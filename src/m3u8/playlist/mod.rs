//! Represents a playlist containing multiple tags for M3U8 files.
//!
//! This module defines the `Playlist` struct, which represents an M3U8 playlist
//! consisting of various tags. The `Playlist` struct provides methods for reading
//! playlists from files or buffered readers, writing playlists to files, and
//! validating the playlist structure according to the M3U8 specification (RFC 8216).
//!
//! # Example
//!
//! ```
//! use m3u8_parser::m3u8::playlist::Playlist;
//!
//! let playlist = Playlist::from_file("src/m3u8/tests/test_data/playlist.m3u8")
//!     .expect("Failed to read playlist");
//!
//! playlist.validate().expect("Playlist is invalid");
//! playlist.write_to_file("src/m3u8/tests/test_data/out.m3u8")
//!     .expect("Failed to write playlist");
//! ```
//!
//! ## Structs
//!
//! - `Playlist`: A struct representing an M3U8 playlist that contains a vector of `Tag` items.
//!
//! ## Methods
//!
//! - `from_reader<R: BufRead>(reader: R) -> Result<Self, String>`: Creates a new `Playlist` by reading tags from a buffered reader.
//! - `from_file<P: AsRef<Path>>(path: P) -> Result<Self, String>`: Creates a new `Playlist` by reading tags from a specified file.
//! - `write_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()>`: Writes the playlist to a specified file.
//! - `validate(&self) -> Result<(), Vec<ValidationError>>`: Validates the playlist according to RFC 8216, returning any validation errors.

pub mod builder;

use crate::m3u8::tags::Tag;
use crate::m3u8::validation::ValidationError;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

type TagPredicate = fn(&Tag) -> bool;

fn parse_attribute_list(input: &str) -> Result<Vec<(String, String)>, String> {
    let mut attributes = Vec::new();
    let mut remaining = input.trim();

    while !remaining.is_empty() {
        let Some(equals_index) = remaining.find('=') else {
            return Err(format!("invalid attribute list: {input}"));
        };
        let name = remaining[..equals_index].trim();
        if name.is_empty() || name.contains(',') {
            return Err(format!("invalid attribute name in: {input}"));
        }

        let value_start = remaining[equals_index + 1..].trim_start();
        let (value, after_value) = if let Some(quoted) = value_start.strip_prefix('"') {
            let Some(end) = quoted.find('"') else {
                return Err(format!("unterminated quoted attribute in: {input}"));
            };
            (&quoted[..end], &quoted[end + 1..])
        } else {
            let end = value_start.find(',').unwrap_or(value_start.len());
            (&value_start[..end], &value_start[end..])
        };
        attributes.push((name.to_string(), value.to_string()));

        let after_value = after_value.trim_start();
        if after_value.is_empty() {
            break;
        }
        let Some(next) = after_value.strip_prefix(',') else {
            return Err(format!("missing comma between attributes in: {input}"));
        };
        remaining = next.trim_start();
    }

    Ok(attributes)
}

fn attribute<'a>(attributes: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|(attribute_name, _)| attribute_name == name)
        .map(|(_, value)| value.as_str())
}

fn required_attribute(
    attributes: &[(String, String)],
    name: &str,
    tag_name: &str,
) -> Result<String, String> {
    attribute(attributes, name)
        .map(str::to_owned)
        .ok_or_else(|| format!("{tag_name} requires {name}"))
}

fn optional_boolean(
    attributes: &[(String, String)],
    name: &str,
    tag_name: &str,
) -> Result<Option<bool>, String> {
    attribute(attributes, name)
        .map(|value| match value {
            "YES" => Ok(true),
            "NO" => Ok(false),
            _ => Err(format!("invalid {tag_name} {name}: {value}")),
        })
        .transpose()
}

fn optional_number<T: std::str::FromStr>(
    attributes: &[(String, String)],
    name: &str,
    tag_name: &str,
) -> Result<Option<T>, String> {
    attribute(attributes, name)
        .map(|value| {
            value
                .parse()
                .map_err(|_| format!("invalid {tag_name} {name}: {value}"))
        })
        .transpose()
}

/// Represents a playlist containing multiple tags.
#[derive(Debug, PartialEq)]
pub struct Playlist {
    pub tags: Vec<Tag>,
}

/// A media segment and the state established by preceding Media Playlist tags.
#[derive(Debug, PartialEq, Clone)]
pub struct MediaSegment {
    pub uri: String,
    pub duration: f32,
    pub title: Option<String>,
    pub key: Option<Tag>,
    pub map: Option<Tag>,
    pub byterange: Option<String>,
    pub program_date_time: Option<String>,
    pub gap: bool,
}

impl Playlist {
    /// Creates a new `Playlist` by reading tags from a buffered reader.
    pub fn from_reader<R: BufRead>(mut reader: R) -> Result<Self, String> {
        let mut tags = Vec::new();
        let mut pending_extinf = None;

        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .map_err(|e| e.to_string())?;

        for (line_number, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some(extinf) = trimmed.strip_prefix("#EXTINF:") {
                if pending_extinf.is_some() {
                    return Err(format!("missing URI after EXTINF on line {}", line_number));
                }
                let (duration, title) = extinf
                    .split_once(',')
                    .ok_or_else(|| format!("invalid EXTINF on line {}", line_number + 1))?;
                let duration = duration
                    .parse()
                    .map_err(|_| format!("invalid EXTINF duration on line {}", line_number + 1))?;
                let title = (!title.trim().is_empty()).then(|| title.trim().to_string());
                pending_extinf = Some((duration, title));
                continue;
            }

            if let Some((duration, title)) = pending_extinf.take() {
                if trimmed.starts_with('#') {
                    return Err(format!("missing URI after EXTINF on line {}", line_number));
                }
                tags.push(Tag::ExtInf(trimmed.to_string(), duration, title));
                continue;
            }

            if let Some(tag_line) = trimmed.strip_prefix('#') {
                if let Some(tag) = Self::parse_line(tag_line)? {
                    tags.push(tag);
                } else {
                    tags.push(Tag::Unknown(tag_line.to_string()));
                }
            } else {
                tags.push(Tag::Uri(trimmed.to_string()));
            }
        }

        if pending_extinf.is_some() {
            return Err("missing URI after EXTINF at end of playlist".to_string());
        }

        Ok(Playlist { tags })
    }

    /// Creates a new `Playlist` by reading tags from a file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        Self::from_reader(BufReader::new(file))
    }

    /// Writes the playlist to a file.
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = File::create(path)?;
        for tag in &self.tags {
            writeln!(file, "{}", tag)?;
        }
        Ok(())
    }

    /// Returns media segments with effective state from preceding tags.
    pub fn media_segments(&self) -> Vec<MediaSegment> {
        let mut segments = Vec::new();
        let mut key = None;
        let mut map = None;
        let mut byterange = None;
        let mut program_date_time = None;
        let mut gap = false;

        for tag in &self.tags {
            match tag {
                Tag::ExtXKey { .. } => key = Some(tag.clone()),
                Tag::ExtXMap { .. } => map = Some(tag.clone()),
                Tag::ExtXByteRange(value) => byterange = Some(value.clone()),
                Tag::ExtXProgramDateTime(value) => program_date_time = Some(value.clone()),
                Tag::ExtXGap => gap = true,
                Tag::ExtInf(uri, duration, title) => {
                    segments.push(MediaSegment {
                        uri: uri.clone(),
                        duration: *duration,
                        title: title.clone(),
                        key: key.clone(),
                        map: map.clone(),
                        byterange: byterange.take(),
                        program_date_time: program_date_time.take(),
                        gap: std::mem::take(&mut gap),
                    });
                }
                _ => {}
            }
        }

        segments
    }

    /// Validates the playlist according to RFC 8216.
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        if !self.tags.iter().any(|tag| matches!(tag, Tag::ExtM3U)) {
            errors.push(ValidationError::MissingExtM3U);
        }

        for tag in &self.tags {
            self.validate_tag(tag, &mut errors);
        }

        if errors.is_empty() {
            self.validate_playlist_rules(&mut errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn parse_line(line: &str) -> Result<Option<Tag>, String> {
        let trimmed = line.trim();

        if trimmed == "EXTM3U" {
            return Ok(Some(Tag::ExtM3U));
        }

        if trimmed == "EXT-X-I-FRAMES-ONLY" {
            return Ok(Some(Tag::ExtXIFramesOnly));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-ALLOW-CACHE:") {
            return match value {
                "YES" => Ok(Some(Tag::ExtXAllowCache(true))),
                "NO" => Ok(Some(Tag::ExtXAllowCache(false))),
                _ => Err(format!("invalid EXT-X-ALLOW-CACHE value: {value}")),
            };
        }

        Self::parse_existing_line(trimmed)
    }

    fn parse_existing_line(trimmed: &str) -> Result<Option<Tag>, String> {
        if let Some(value) = trimmed.strip_prefix("EXT-X-VERSION:") {
            return value
                .parse()
                .map(Tag::ExtXVersion)
                .map(Some)
                .map_err(|_| format!("invalid EXT-X-VERSION: {value}"));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-TARGETDURATION:") {
            return value
                .parse()
                .map(Tag::ExtXTargetDuration)
                .map(Some)
                .map_err(|_| format!("invalid EXT-X-TARGETDURATION: {value}"));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-PLAYLIST-TYPE:") {
            return match value {
                "EVENT" | "VOD" => Ok(Some(Tag::ExtXPlaylistType(value.to_string()))),
                _ => Err(format!("invalid EXT-X-PLAYLIST-TYPE: {value}")),
            };
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-MEDIA-SEQUENCE:") {
            return value
                .parse()
                .map(Tag::ExtXMediaSequence)
                .map(Some)
                .map_err(|_| format!("invalid EXT-X-MEDIA-SEQUENCE: {value}"));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-DISCONTINUITY-SEQUENCE:") {
            return value
                .parse()
                .map(Tag::ExtXDiscontinuitySequence)
                .map(Some)
                .map_err(|_| format!("invalid EXT-X-DISCONTINUITY-SEQUENCE: {value}"));
        }

        if trimmed == "EXT-X-ENDLIST" {
            return Ok(Some(Tag::ExtXEndList));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-KEY:") {
            let attributes = parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXKey {
                method: required_attribute(&attributes, "METHOD", "EXT-X-KEY")?,
                uri: attribute(&attributes, "URI").map(str::to_owned),
                iv: attribute(&attributes, "IV").map(str::to_owned),
                keyformat: attribute(&attributes, "KEYFORMAT").map(str::to_owned),
                keyformatversions: attribute(&attributes, "KEYFORMATVERSIONS").map(str::to_owned),
            }));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-MAP:") {
            let attributes = parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXMap {
                uri: required_attribute(&attributes, "URI", "EXT-X-MAP")?,
                byterange: attribute(&attributes, "BYTERANGE")
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned),
            }));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-PROGRAM-DATE-TIME:") {
            if value.is_empty() {
                return Err("EXT-X-PROGRAM-DATE-TIME requires a date".to_string());
            }
            return Ok(Some(Tag::ExtXProgramDateTime(value.to_string())));
        }

        if trimmed == "EXT-X-DISCONTINUITY" {
            return Ok(Some(Tag::ExtXDiscontinuity));
        }

        if trimmed == "EXT-X-GAP" {
            return Ok(Some(Tag::ExtXGap));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-BYTERANGE:") {
            if value.is_empty() {
                return Err("EXT-X-BYTERANGE requires a value".to_string());
            }
            return Ok(Some(Tag::ExtXByteRange(value.to_string())));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-DEFINE:") {
            parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXDefine(value.to_string())));
        }

        if let Some(attributes) = trimmed.strip_prefix("EXT-X-DATERANGE:") {
            let attributes = parse_attribute_list(attributes)?;
            let mut id = None;
            let mut class = None;
            let mut start_date = None;
            let mut end_date = None;
            let mut duration = None;
            let mut planned_duration = None;
            let mut end_on_next = None;
            let mut scte35_cmd = None;
            let mut scte35_out = None;
            let mut scte35_in = None;
            let mut client_attributes = Vec::new();

            for (name, value) in attributes {
                match name.as_str() {
                    "ID" => id = Some(value),
                    "CLASS" => class = Some(value),
                    "START-DATE" => start_date = Some(value),
                    "END-DATE" => end_date = Some(value),
                    "DURATION" => {
                        duration =
                            Some(value.parse().map_err(|_| {
                                format!("invalid EXT-X-DATERANGE DURATION: {value}")
                            })?)
                    }
                    "PLANNED-DURATION" => {
                        planned_duration = Some(value.parse().map_err(|_| {
                            format!("invalid EXT-X-DATERANGE PLANNED-DURATION: {value}")
                        })?)
                    }
                    "END-ON-NEXT" => {
                        end_on_next = Some(match value.as_str() {
                            "YES" => true,
                            "NO" => false,
                            _ => {
                                return Err(format!("invalid EXT-X-DATERANGE END-ON-NEXT: {value}"))
                            }
                        })
                    }
                    "SCTE35-CMD" => scte35_cmd = Some(value),
                    "SCTE35-OUT" => scte35_out = Some(value),
                    "SCTE35-IN" => scte35_in = Some(value),
                    name if name.starts_with("X-") => {
                        client_attributes.push((name.to_string(), value))
                    }
                    _ => return Err(format!("unknown EXT-X-DATERANGE attribute: {name}")),
                }
            }

            let id = id.ok_or_else(|| "EXT-X-DATERANGE requires ID".to_string())?;
            let start_date =
                start_date.ok_or_else(|| "EXT-X-DATERANGE requires START-DATE".to_string())?;
            return Ok(Some(Tag::ExtXDateRange {
                id,
                class,
                start_date,
                end_date,
                duration,
                planned_duration,
                end_on_next,
                scte35_cmd,
                scte35_out,
                scte35_in,
                client_attributes,
            }));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-PART-INF:") {
            let attributes = parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXPartInf {
                part_target_duration: required_attribute(
                    &attributes,
                    "PART-TARGET",
                    "EXT-X-PART-INF",
                )?
                .parse()
                .map_err(|_| "invalid EXT-X-PART-INF PART-TARGET".to_string())?,
                part_hold_back: optional_number(&attributes, "PART-HOLD-BACK", "EXT-X-PART-INF")?,
                part_number: optional_number(&attributes, "PART-NUMBER", "EXT-X-PART-INF")?,
            }));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-PART:") {
            let attributes = parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXPart {
                uri: required_attribute(&attributes, "URI", "EXT-X-PART")?,
                duration: optional_number(&attributes, "DURATION", "EXT-X-PART")?,
            }));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-SERVER-CONTROL:") {
            let attributes = parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXServerControl {
                can_play: optional_boolean(&attributes, "CAN-PLAY", "EXT-X-SERVER-CONTROL")?,
                can_seek: optional_boolean(&attributes, "CAN-SEEK", "EXT-X-SERVER-CONTROL")?,
                can_pause: optional_boolean(&attributes, "CAN-PAUSE", "EXT-X-SERVER-CONTROL")?,
                min_buffer_time: optional_number(
                    &attributes,
                    "MIN-BUFFER-TIME",
                    "EXT-X-SERVER-CONTROL",
                )?,
            }));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-SKIP:") {
            let attributes = parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXSkip {
                uri: attribute(&attributes, "URI")
                    .unwrap_or_default()
                    .to_string(),
                duration: optional_number(&attributes, "DURATION", "EXT-X-SKIP")?,
                skipped_segments: required_attribute(
                    &attributes,
                    "SKIPPED-SEGMENTS",
                    "EXT-X-SKIP",
                )?
                .parse()
                .map_err(|_| "invalid EXT-X-SKIP SKIPPED-SEGMENTS".to_string())?,
                reason: attribute(&attributes, "REASON").map(str::to_owned),
            }));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-BITRATE:") {
            return value
                .parse()
                .map(Tag::ExtXBitrate)
                .map(Some)
                .map_err(|_| format!("invalid EXT-X-BITRATE: {value}"));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-START:") {
            let attributes = parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXStart {
                time_offset: required_attribute(&attributes, "TIME-OFFSET", "EXT-X-START")?,
                precise: optional_boolean(&attributes, "PRECISE", "EXT-X-START")?,
            }));
        }

        if trimmed == "EXT-X-INDEPENDENT-SEGMENTS" {
            return Ok(Some(Tag::ExtXIndependentSegments));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-STREAM-INF:") {
            let attributes = parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXStreamInf {
                bandwidth: required_attribute(&attributes, "BANDWIDTH", "EXT-X-STREAM-INF")?
                    .parse()
                    .map_err(|_| "invalid EXT-X-STREAM-INF BANDWIDTH".to_string())?,
                average_bandwidth: optional_number(
                    &attributes,
                    "AVERAGE-BANDWIDTH",
                    "EXT-X-STREAM-INF",
                )?,
                codecs: attribute(&attributes, "CODECS").map(str::to_owned),
                resolution: attribute(&attributes, "RESOLUTION").map(str::to_owned),
                frame_rate: optional_number(&attributes, "FRAME-RATE", "EXT-X-STREAM-INF")?,
                audio: attribute(&attributes, "AUDIO").map(str::to_owned),
                video: attribute(&attributes, "VIDEO").map(str::to_owned),
                subtitle: attribute(&attributes, "SUBTITLES").map(str::to_owned),
                closed_captions: attribute(&attributes, "CLOSED-CAPTIONS").map(str::to_owned),
            }));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-MEDIA:") {
            let attributes = parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXMedia {
                type_: required_attribute(&attributes, "TYPE", "EXT-X-MEDIA")?,
                group_id: required_attribute(&attributes, "GROUP-ID", "EXT-X-MEDIA")?,
                name: Some(required_attribute(&attributes, "NAME", "EXT-X-MEDIA")?),
                uri: attribute(&attributes, "URI").map(str::to_owned),
                default: optional_boolean(&attributes, "DEFAULT", "EXT-X-MEDIA")?,
                autoplay: optional_boolean(&attributes, "AUTOSELECT", "EXT-X-MEDIA")?,
                characteristics: attribute(&attributes, "CHARACTERISTICS").map(str::to_owned),
                language: attribute(&attributes, "LANGUAGE").map(str::to_owned),
                instream_id: attribute(&attributes, "INSTREAM-ID").map(str::to_owned),
                language_codec: attribute(&attributes, "LANGUAGE-CODEC").map(str::to_owned),
                forced: optional_boolean(&attributes, "FORCED", "EXT-X-MEDIA")?,
                channels: attribute(&attributes, "CHANNELS").map(str::to_owned),
            }));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-RENDITION-REPORT:") {
            let attributes = parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXRenditionReport {
                uri: required_attribute(&attributes, "URI", "EXT-X-RENDITION-REPORT")?,
                bandwidth: required_attribute(&attributes, "BANDWIDTH", "EXT-X-RENDITION-REPORT")?
                    .parse()
                    .map_err(|_| "invalid EXT-X-RENDITION-REPORT BANDWIDTH".to_string())?,
            }));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-I-FRAME-STREAM-INF:") {
            let attributes = parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXIFrameStreamInf {
                bandwidth: required_attribute(
                    &attributes,
                    "BANDWIDTH",
                    "EXT-X-I-FRAME-STREAM-INF",
                )?
                .parse()
                .map_err(|_| "invalid EXT-X-I-FRAME-STREAM-INF BANDWIDTH".to_string())?,
                average_bandwidth: optional_number(
                    &attributes,
                    "AVERAGE-BANDWIDTH",
                    "EXT-X-I-FRAME-STREAM-INF",
                )?,
                codecs: attribute(&attributes, "CODECS").map(str::to_owned),
                resolution: attribute(&attributes, "RESOLUTION").map(str::to_owned),
                frame_rate: optional_number(&attributes, "FRAME-RATE", "EXT-X-I-FRAME-STREAM-INF")?,
                uri: required_attribute(&attributes, "URI", "EXT-X-I-FRAME-STREAM-INF")?,
            }));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-SESSION-DATA:") {
            let attributes = parse_attribute_list(value)?;
            let value = attribute(&attributes, "VALUE").map(str::to_owned);
            let uri = attribute(&attributes, "URI").map(str::to_owned);
            if value.is_some() == uri.is_some() {
                return Err("EXT-X-SESSION-DATA requires exactly one of VALUE or URI".to_string());
            }
            return Ok(Some(Tag::ExtXSessionData {
                id: required_attribute(&attributes, "DATA-ID", "EXT-X-SESSION-DATA")?,
                value,
                uri,
                language: attribute(&attributes, "LANGUAGE").map(str::to_owned),
            }));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-PRELOAD-HINT:") {
            let attributes = parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXPreloadHint {
                uri: required_attribute(&attributes, "URI", "EXT-X-PRELOAD-HINT")?,
                byterange: attribute(&attributes, "BYTERANGE").map(str::to_owned),
            }));
        }

        if let Some(value) = trimmed.strip_prefix("EXT-X-SESSION-KEY:") {
            let attributes = parse_attribute_list(value)?;
            return Ok(Some(Tag::ExtXSessionKey {
                method: required_attribute(&attributes, "METHOD", "EXT-X-SESSION-KEY")?,
                uri: attribute(&attributes, "URI").map(str::to_owned),
                iv: attribute(&attributes, "IV").map(str::to_owned),
                keyformat: attribute(&attributes, "KEYFORMAT").map(str::to_owned),
                keyformatversions: attribute(&attributes, "KEYFORMATVERSIONS").map(str::to_owned),
            }));
        }

        Ok(None)
    }

    fn validate_tag(&self, tag: &Tag, errors: &mut Vec<ValidationError>) {
        match tag {
            Tag::ExtXVersion(version) => {
                if *version < 1 || *version > 7 {
                    errors.push(ValidationError::InvalidVersion(*version));
                }
            }
            Tag::ExtInf(_, duration, _) if *duration <= 0.0 => {
                errors.push(ValidationError::InvalidDuration(*duration));
            }
            Tag::ExtXTargetDuration(duration) if *duration == 0 => {
                errors.push(ValidationError::InvalidTargetDuration(*duration));
            }
            Tag::ExtXKey { method, .. }
                if !matches!(method.as_str(), "NONE" | "AES-128" | "SAMPLE-AES") =>
            {
                errors.push(ValidationError::InvalidKeyMethod(method.clone()));
            }
            Tag::ExtXKey {
                method,
                uri,
                iv,
                keyformat,
                keyformatversions,
            } => {
                if method == "NONE"
                    && (uri.is_some()
                        || iv.is_some()
                        || keyformat.is_some()
                        || keyformatversions.is_some())
                {
                    errors.push(ValidationError::InvalidKeyAttributes(
                        "METHOD=NONE must not include URI, IV, KEYFORMAT, or KEYFORMATVERSIONS"
                            .to_string(),
                    ));
                }
                if method != "NONE" && uri.as_deref().is_none_or(str::is_empty) {
                    errors.push(ValidationError::InvalidKeyAttributes(
                        "encryption methods require URI".to_string(),
                    ));
                }
            }
            Tag::ExtXMap { uri, .. } if uri.is_empty() => {
                errors.push(ValidationError::InvalidMapUri);
            }
            Tag::ExtXMedia {
                type_,
                group_id,
                name,
                ..
            } if type_.is_empty()
                || group_id.is_empty()
                || name.as_deref().is_none_or(str::is_empty) =>
            {
                errors.push(ValidationError::MissingMediaFields);
            }
            Tag::ExtXProgramDateTime(date_time) if date_time.is_empty() => {
                errors.push(ValidationError::InvalidProgramDateTime);
            }
            Tag::ExtXDateRange {
                id,
                start_date,
                end_date,
                duration,
                planned_duration,
                class,
                end_on_next,
                ..
            } => {
                if id.is_empty() {
                    errors.push(ValidationError::InvalidDateRangeId);
                }
                if start_date.is_empty() {
                    errors.push(ValidationError::InvalidDateRangeStartDate);
                }
                if end_date.as_deref().is_some_and(str::is_empty) {
                    errors.push(ValidationError::InvalidDateRangeEndDate);
                }
                if let Some(duration) = duration {
                    if *duration < 0.0 {
                        errors.push(ValidationError::InvalidDuration(*duration as f32));
                    }
                }
                if let Some(planned_duration) = planned_duration {
                    if *planned_duration < 0.0 {
                        errors.push(ValidationError::InvalidDateRangePlannedDuration(
                            *planned_duration as f32,
                        ));
                    }
                }
                if *end_on_next == Some(true)
                    && (class.as_deref().is_none_or(str::is_empty)
                        || end_date.is_some()
                        || duration.is_some())
                {
                    errors.push(ValidationError::InvalidDateRangeAttributes(
                        "END-ON-NEXT requires CLASS and forbids END-DATE and DURATION".to_string(),
                    ));
                }
            }
            Tag::ExtXGap => {
                // Validation for EXT-X-GAP if necessary
                // TODO: maybe we can make it configurable?
            }
            Tag::ExtXBitrate(bitrate) if bitrate < &0 => {
                errors.push(ValidationError::InvalidBitrate(*bitrate));
            }
            Tag::ExtXIndependentSegments => {
                // No specific validation needed
            }
            Tag::ExtXStart { time_offset, .. } if time_offset.is_empty() => {
                errors.push(ValidationError::InvalidStartOffset);
            }
            Tag::ExtXSkip {
                duration: Some(duration),
                ..
            } if *duration <= 0.0 => {
                errors.push(ValidationError::InvalidSkipTag(
                    "Duration must be positive".to_string(),
                ));
            }
            Tag::ExtXPreloadHint { uri, .. } if uri.is_empty() => {
                errors.push(ValidationError::InvalidPreloadHintUri);
            }
            Tag::ExtXRenditionReport { uri, .. } if uri.is_empty() => {
                errors.push(ValidationError::InvalidRenditionReportUri);
            }
            Tag::ExtXServerControl { .. } => {
                // Add specific validations if needed
                // TODO: maybe we can make it configurable?
            }
            _ => {}
        }
    }

    fn validate_playlist_rules(&self, errors: &mut Vec<ValidationError>) {
        if self.tags.iter().any(|tag| matches!(tag, Tag::ExtM3U))
            && self
                .tags
                .first()
                .is_some_and(|tag| !matches!(tag, Tag::ExtM3U))
        {
            errors.push(ValidationError::ExtM3UNotFirst);
        }

        let singleton_tags: [(&str, TagPredicate); 9] = [
            ("EXT-X-VERSION", is_version),
            ("EXT-X-TARGETDURATION", is_target_duration),
            ("EXT-X-MEDIA-SEQUENCE", is_media_sequence),
            ("EXT-X-DISCONTINUITY-SEQUENCE", is_discontinuity_sequence),
            ("EXT-X-PLAYLIST-TYPE", is_playlist_type),
            ("EXT-X-I-FRAMES-ONLY", is_i_frames_only),
            ("EXT-X-INDEPENDENT-SEGMENTS", is_independent_segments),
            ("EXT-X-START", is_start),
            ("EXT-X-ENDLIST", is_end_list),
        ];
        for (name, predicate) in singleton_tags {
            if self.tags.iter().filter(|tag| predicate(tag)).count() > 1 {
                errors.push(ValidationError::DuplicateTag(name.to_string()));
            }
        }

        let has_master_tag = self.tags.iter().any(is_master_tag);
        let has_media_segment_tag = self.tags.iter().any(is_media_segment_tag);
        if has_master_tag && has_media_segment_tag {
            errors.push(ValidationError::MixedPlaylistTypes);
        }

        if has_media_segment_tag && !self.tags.iter().any(is_target_duration) {
            errors.push(ValidationError::MissingTargetDuration);
        }

        for pair in self.tags.windows(2) {
            if matches!(pair[0], Tag::ExtXStreamInf { .. }) && !matches!(pair[1], Tag::Uri(_)) {
                errors.push(ValidationError::MissingVariantUri);
            }
        }
        if matches!(self.tags.last(), Some(Tag::ExtXStreamInf { .. })) {
            errors.push(ValidationError::MissingVariantUri);
        }

        for tag in &self.tags {
            if let Tag::ExtXStreamInf {
                audio,
                video,
                subtitle,
                closed_captions,
                ..
            } = tag
            {
                for (attribute, group_id, media_type) in [
                    ("AUDIO", audio, "AUDIO"),
                    ("VIDEO", video, "VIDEO"),
                    ("SUBTITLES", subtitle, "SUBTITLES"),
                    ("CLOSED-CAPTIONS", closed_captions, "CLOSED-CAPTIONS"),
                ] {
                    if let Some(group_id) = group_id {
                        if group_id != "NONE"
                            && !self.tags.iter().any(|candidate| {
                                matches!(
                                    candidate,
                                    Tag::ExtXMedia {
                                        type_,
                                        group_id: candidate_group_id,
                                        ..
                                    } if type_ == media_type && candidate_group_id == group_id
                                )
                            })
                        {
                            errors.push(ValidationError::UnresolvedRenditionGroup {
                                attribute: attribute.to_string(),
                                group_id: group_id.clone(),
                            });
                        }
                    }
                }
            }
        }

        let version = self
            .tags
            .iter()
            .find_map(|tag| match tag {
                Tag::ExtXVersion(version) => Some(*version),
                _ => None,
            })
            .unwrap_or(1);
        for tag in &self.tags {
            if let Some(required) = minimum_version(tag) {
                if version < required {
                    errors.push(ValidationError::InsufficientVersion {
                        tag: tag_name(tag).to_string(),
                        required,
                        actual: version,
                    });
                }
            }
        }

        if let Some(target_duration) = self.tags.iter().find_map(|tag| match tag {
            Tag::ExtXTargetDuration(duration) if *duration > 0 => Some(*duration),
            _ => None,
        }) {
            for tag in &self.tags {
                if let Tag::ExtInf(_, duration, _) = tag {
                    if duration.round() as u64 > target_duration {
                        errors.push(ValidationError::SegmentDurationExceedsTarget {
                            duration: *duration,
                            target_duration,
                        });
                    }
                }
            }
        }
    }
}

fn is_version(tag: &Tag) -> bool {
    matches!(tag, Tag::ExtXVersion(_))
}
fn is_target_duration(tag: &Tag) -> bool {
    matches!(tag, Tag::ExtXTargetDuration(_))
}
fn is_media_sequence(tag: &Tag) -> bool {
    matches!(tag, Tag::ExtXMediaSequence(_))
}
fn is_discontinuity_sequence(tag: &Tag) -> bool {
    matches!(tag, Tag::ExtXDiscontinuitySequence(_))
}
fn is_playlist_type(tag: &Tag) -> bool {
    matches!(tag, Tag::ExtXPlaylistType(_))
}
fn is_i_frames_only(tag: &Tag) -> bool {
    matches!(tag, Tag::ExtXIFramesOnly)
}
fn is_independent_segments(tag: &Tag) -> bool {
    matches!(tag, Tag::ExtXIndependentSegments)
}
fn is_start(tag: &Tag) -> bool {
    matches!(tag, Tag::ExtXStart { .. })
}
fn is_end_list(tag: &Tag) -> bool {
    matches!(tag, Tag::ExtXEndList)
}

fn is_master_tag(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::ExtXMedia { .. }
            | Tag::ExtXStreamInf { .. }
            | Tag::ExtXIFrameStreamInf { .. }
            | Tag::ExtXSessionData { .. }
            | Tag::ExtXSessionKey { .. }
    )
}

fn is_media_segment_tag(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::ExtXTargetDuration(_)
            | Tag::ExtXMediaSequence(_)
            | Tag::ExtXDiscontinuitySequence(_)
            | Tag::ExtXEndList
            | Tag::ExtInf(..)
            | Tag::ExtXKey { .. }
            | Tag::ExtXMap { .. }
            | Tag::ExtXProgramDateTime(_)
            | Tag::ExtXByteRange(_)
            | Tag::ExtXDateRange { .. }
            | Tag::ExtXGap
            | Tag::ExtXIFramesOnly
    )
}

fn minimum_version(tag: &Tag) -> Option<u8> {
    match tag {
        Tag::ExtXByteRange(_) | Tag::ExtXIFramesOnly => Some(4),
        Tag::ExtXMap { .. }
        | Tag::ExtXKey {
            keyformat: Some(_), ..
        }
        | Tag::ExtXSessionKey {
            keyformat: Some(_), ..
        } => Some(5),
        Tag::ExtXDateRange { .. }
        | Tag::ExtXStart { .. }
        | Tag::ExtXIndependentSegments
        | Tag::ExtXMedia { .. }
        | Tag::ExtXStreamInf {
            average_bandwidth: Some(_),
            ..
        }
        | Tag::ExtXIFrameStreamInf {
            average_bandwidth: Some(_),
            ..
        } => Some(6),
        _ => None,
    }
}

fn tag_name(tag: &Tag) -> &'static str {
    match tag {
        Tag::ExtXByteRange(_) => "EXT-X-BYTERANGE",
        Tag::ExtXIFramesOnly => "EXT-X-I-FRAMES-ONLY",
        Tag::ExtXMap { .. } => "EXT-X-MAP",
        Tag::ExtXKey { .. } => "EXT-X-KEY",
        Tag::ExtXSessionKey { .. } => "EXT-X-SESSION-KEY",
        Tag::ExtXDateRange { .. } => "EXT-X-DATERANGE",
        Tag::ExtXStart { .. } => "EXT-X-START",
        Tag::ExtXIndependentSegments => "EXT-X-INDEPENDENT-SEGMENTS",
        Tag::ExtXMedia { .. } => "EXT-X-MEDIA",
        Tag::ExtXStreamInf { .. } => "EXT-X-STREAM-INF",
        Tag::ExtXIFrameStreamInf { .. } => "EXT-X-I-FRAME-STREAM-INF",
        _ => "tag",
    }
}
