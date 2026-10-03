//! Represents a playlist containing multiple tags for M3U8 files.
//!
//! This module defines the `Playlist` struct, which represents an M3U8 playlist
//! consisting of various tags. The `Playlist` struct provides methods for reading
//! playlists from files or readers, writing playlists to files, and
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
//! playlist.write_to_file(std::env::temp_dir().join("m3u8-parser-doc-out.m3u8"))
//!     .expect("Failed to write playlist");
//!
//! // Playlists also parse from and serialize to strings.
//! let text = playlist.to_string();
//! assert_eq!(text.parse::<Playlist>().unwrap(), playlist);
//! ```
//!
//! ## Structs
//!
//! - `Playlist`: A struct representing an M3U8 playlist that contains a vector of `Tag` items.
//! - `MediaSegment`: A media segment with the state established by preceding tags.
//!
//! ## Methods
//!
//! - `from_reader<R: Read>(reader: R) -> Result<Self, ParseError>`: Creates a new `Playlist` by reading tags from a reader.
//! - `from_file<P: AsRef<Path>>(path: P) -> Result<Self, ParseError>`: Creates a new `Playlist` by reading tags from a specified file.
//! - `write_to<W: Write>(&self, writer: W) -> io::Result<()>`: Writes the playlist to any writer.
//! - `write_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()>`: Writes the playlist to a specified file.
//! - `Display` and `FromStr` convert a playlist to and from a string.
//! - `media_segments(&self) -> Vec<MediaSegment>`: Returns the media segments with their effective state.
//! - `validate(&self) -> Result<(), Vec<ValidationError>>`: Validates the playlist according to RFC 8216, returning any validation errors.

pub mod builder;
mod variables;

use crate::m3u8::error::{ParseError, SyntaxError};
use crate::m3u8::parser::{
    attribute, optional_boolean, optional_parsed, optional_string, parse_attribute_list,
    required_parsed, required_string, Attribute,
};
use crate::m3u8::tags::{ByteRange, Key, Map, Tag};
use crate::m3u8::validation::ValidationError;
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;

/// The highest EXT-X-VERSION defined by the HLS specification (draft-pantos-hls-rfc8216bis).
const MAX_VERSION: u8 = 12;

type TagPredicate = fn(&Tag) -> bool;

/// Represents a playlist containing multiple tags.
#[derive(Debug, PartialEq)]
pub struct Playlist {
    pub tags: Vec<Tag>,
}

/// A media segment and the state established by preceding Media Playlist tags.
#[derive(Debug, PartialEq, Clone)]
pub struct MediaSegment {
    pub uri: String,
    pub duration: f64,
    pub title: Option<String>,
    /// The Media Sequence Number, starting from EXT-X-MEDIA-SEQUENCE (default 0).
    pub sequence: u64,
    /// Whether an EXT-X-DISCONTINUITY precedes this segment.
    pub discontinuity: bool,
    /// The effective encryption key; `None` when unencrypted or after `METHOD=NONE`.
    pub key: Option<Key>,
    pub map: Option<Map>,
    pub byterange: Option<ByteRange>,
    pub program_date_time: Option<String>,
    pub gap: bool,
}

impl std::fmt::Display for Playlist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for tag in &self.tags {
            writeln!(f, "{tag}")?;
        }
        Ok(())
    }
}

impl std::str::FromStr for Playlist {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_reader(s.as_bytes())
    }
}

impl Playlist {
    /// Creates a new `Playlist` by reading tags from a reader.
    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self, ParseError> {
        let mut tags = Vec::new();
        let mut pending_extinf = None;

        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        let content = content.strip_prefix('\u{feff}').unwrap_or(&content);

        for (index, line) in content.lines().enumerate() {
            let line_number = index + 1;
            let syntax_error = |kind| ParseError::Syntax {
                line: line_number,
                kind,
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some(extinf) = trimmed.strip_prefix("#EXTINF:") {
                if let Some((_, _, extinf_line)) = pending_extinf {
                    return Err(ParseError::Syntax {
                        line: extinf_line,
                        kind: SyntaxError::MissingUriAfterExtInf,
                    });
                }
                let (duration, title) = parse_extinf(extinf).map_err(syntax_error)?;
                pending_extinf = Some((duration, title, line_number));
                continue;
            }

            // Tags between EXTINF and its URI (e.g. EXT-X-BYTERANGE) are kept
            // ahead of the segment, which RFC 8216 treats equivalently.
            if let Some(tag_line) = trimmed.strip_prefix('#') {
                let tag = parse_tag(tag_line).map_err(syntax_error)?;
                tags.push(tag.unwrap_or_else(|| Tag::Unknown(tag_line.to_string())));
            } else if let Some((duration, title, _)) = pending_extinf.take() {
                tags.push(Tag::ExtInf {
                    uri: trimmed.to_string(),
                    duration,
                    title,
                });
            } else {
                tags.push(Tag::Uri(trimmed.to_string()));
            }
        }

        if let Some((_, _, extinf_line)) = pending_extinf {
            return Err(ParseError::Syntax {
                line: extinf_line,
                kind: SyntaxError::MissingUriAfterExtInf,
            });
        }

        Ok(Playlist { tags })
    }

    /// Creates a new `Playlist` by reading tags from a file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ParseError> {
        Self::from_reader(File::open(path)?)
    }

    /// Writes the playlist to a writer, one tag per line.
    pub fn write_to<W: Write>(&self, mut writer: W) -> io::Result<()> {
        write!(writer, "{self}")?;
        writer.flush()
    }

    /// Writes the playlist to a file.
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        self.write_to(BufWriter::new(File::create(path)?))
    }

    /// Returns media segments with effective state from preceding tags.
    pub fn media_segments(&self) -> Vec<MediaSegment> {
        let mut segments = Vec::new();
        let mut sequence = 0;
        let mut discontinuity = false;
        let mut key = None;
        let mut map = None;
        let mut byterange = None;
        let mut program_date_time = None;
        let mut gap = false;

        for tag in &self.tags {
            match tag {
                Tag::ExtXMediaSequence(first) => sequence = *first,
                Tag::ExtXDiscontinuity => discontinuity = true,
                Tag::ExtXKey(value) => {
                    key = (value.method != "NONE").then(|| value.clone());
                }
                Tag::ExtXMap(value) => map = Some(value.clone()),
                Tag::ExtXByteRange(value) => byterange = Some(*value),
                Tag::ExtXProgramDateTime(value) => program_date_time = Some(value.clone()),
                Tag::ExtXGap => gap = true,
                Tag::ExtInf {
                    uri,
                    duration,
                    title,
                } => {
                    segments.push(MediaSegment {
                        uri: uri.clone(),
                        duration: *duration,
                        title: title.clone(),
                        sequence,
                        discontinuity: std::mem::take(&mut discontinuity),
                        key: key.clone(),
                        map: map.clone(),
                        byterange: byterange.take(),
                        program_date_time: program_date_time.take(),
                        gap: std::mem::take(&mut gap),
                    });
                    sequence += 1;
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

        self.validate_playlist_rules(&mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_tag(&self, tag: &Tag, errors: &mut Vec<ValidationError>) {
        match tag {
            Tag::ExtXVersion(version) => {
                if *version < 1 || *version > MAX_VERSION {
                    errors.push(ValidationError::InvalidVersion(*version));
                }
            }
            Tag::ExtInf { duration, .. } if !duration.is_finite() || *duration <= 0.0 => {
                errors.push(ValidationError::InvalidDuration(*duration));
            }
            Tag::ExtXTargetDuration(duration) if *duration == 0 => {
                errors.push(ValidationError::InvalidTargetDuration(*duration));
            }
            Tag::ExtXKey(key) => validate_key(key, "EXT-X-KEY", errors),
            Tag::ExtXSessionKey(key) => {
                if key.method == "NONE" {
                    errors.push(ValidationError::InvalidKeyAttributes(
                        "EXT-X-SESSION-KEY METHOD must not be NONE".to_string(),
                    ));
                } else {
                    validate_key(key, "EXT-X-SESSION-KEY", errors);
                }
            }
            Tag::ExtXMap(Map { uri, .. }) if uri.is_empty() => {
                errors.push(ValidationError::InvalidMapUri);
            }
            Tag::ExtXMedia {
                type_,
                group_id,
                name,
                uri,
                instream_id,
                ..
            } => {
                if type_.is_empty()
                    || group_id.is_empty()
                    || name.as_deref().is_none_or(str::is_empty)
                {
                    errors.push(ValidationError::MissingMediaFields);
                }
                if type_ == "CLOSED-CAPTIONS" {
                    if instream_id.is_none() {
                        errors.push(ValidationError::InvalidMediaAttributes(
                            "TYPE=CLOSED-CAPTIONS requires INSTREAM-ID".to_string(),
                        ));
                    }
                    if uri.is_some() {
                        errors.push(ValidationError::InvalidMediaAttributes(
                            "TYPE=CLOSED-CAPTIONS must not include URI".to_string(),
                        ));
                    }
                } else if instream_id.is_some() {
                    errors.push(ValidationError::InvalidMediaAttributes(
                        "INSTREAM-ID is only allowed with TYPE=CLOSED-CAPTIONS".to_string(),
                    ));
                }
            }
            Tag::ExtXProgramDateTime(date_time) if !is_date_time(date_time) => {
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
                if !is_date_time(start_date) {
                    errors.push(ValidationError::InvalidDateRangeStartDate);
                }
                if end_date
                    .as_deref()
                    .is_some_and(|end_date| !is_date_time(end_date))
                {
                    errors.push(ValidationError::InvalidDateRangeEndDate);
                }
                if let Some(duration) = duration {
                    if !duration.is_finite() || *duration < 0.0 {
                        errors.push(ValidationError::InvalidDuration(*duration));
                    }
                }
                if let Some(planned_duration) = planned_duration {
                    if !planned_duration.is_finite() || *planned_duration < 0.0 {
                        errors.push(ValidationError::InvalidDateRangePlannedDuration(
                            *planned_duration,
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
            Tag::ExtXPreloadHint { uri, .. } if uri.is_empty() => {
                errors.push(ValidationError::InvalidPreloadHintUri);
            }
            Tag::ExtXRenditionReport { uri, .. } if uri.is_empty() => {
                errors.push(ValidationError::InvalidRenditionReportUri);
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

        // A byte range without an offset continues the previous segment's sub-range.
        let mut pending_range = None;
        let mut previous_segment: Option<(&str, bool)> = None;
        for tag in &self.tags {
            match tag {
                Tag::ExtXByteRange(range) => pending_range = Some(range),
                Tag::ExtInf { uri, .. } => {
                    let range = pending_range.take();
                    if range.is_some_and(|range| range.offset.is_none())
                        && previous_segment != Some((uri.as_str(), true))
                    {
                        errors.push(ValidationError::InvalidByteRange(uri.clone()));
                    }
                    previous_segment = Some((uri.as_str(), range.is_some()));
                }
                _ => {}
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
        if version >= 7
            && self
                .tags
                .iter()
                .any(|tag| matches!(tag, Tag::ExtXAllowCache(_)))
        {
            errors.push(ValidationError::RemovedTag {
                tag: "EXT-X-ALLOW-CACHE".to_string(),
                removed_in: 7,
            });
        }

        let has_i_frames_only = self.tags.iter().any(is_i_frames_only);
        for tag in &self.tags {
            if let Some(required) = minimum_version(tag, has_i_frames_only) {
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
                if let Tag::ExtInf { duration, .. } = tag {
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

fn parse_extinf(value: &str) -> Result<(f64, Option<String>), SyntaxError> {
    let invalid = || SyntaxError::InvalidExtInf(value.to_string());
    let (duration, title) = value.split_once(',').ok_or_else(invalid)?;
    let duration: f64 = duration.trim().parse().map_err(|_| invalid())?;
    if !duration.is_finite() {
        return Err(invalid());
    }
    let title = title.trim();
    Ok((duration, (!title.is_empty()).then(|| title.to_string())))
}

/// Tags that are meaningless without a value; a bare occurrence is a syntax error.
const VALUE_TAGS: [&str; 24] = [
    "EXT-X-VERSION",
    "EXT-X-TARGETDURATION",
    "EXT-X-MEDIA-SEQUENCE",
    "EXT-X-DISCONTINUITY-SEQUENCE",
    "EXT-X-BITRATE",
    "EXT-X-PLAYLIST-TYPE",
    "EXT-X-ALLOW-CACHE",
    "EXT-X-PROGRAM-DATE-TIME",
    "EXT-X-BYTERANGE",
    "EXT-X-DEFINE",
    "EXT-X-KEY",
    "EXT-X-SESSION-KEY",
    "EXT-X-MAP",
    "EXT-X-DATERANGE",
    "EXT-X-START",
    "EXT-X-STREAM-INF",
    "EXT-X-I-FRAME-STREAM-INF",
    "EXT-X-MEDIA",
    "EXT-X-SESSION-DATA",
    "EXT-X-SERVER-CONTROL",
    "EXT-X-PART-INF",
    "EXT-X-PART",
    "EXT-X-SKIP",
    "EXT-X-PRELOAD-HINT",
];

/// Parses a tag line without its leading `#`. Returns `Ok(None)` for unrecognized tags.
fn parse_tag(line: &str) -> Result<Option<Tag>, SyntaxError> {
    let (name, value) = match line.trim().split_once(':') {
        Some((name, value)) => (name, Some(value.trim())),
        None => (line.trim(), None),
    };
    let attributes = || parse_attribute_list(value.unwrap_or_default());

    let tag = match (name, value) {
        ("EXTM3U", None) => Tag::ExtM3U,
        ("EXT-X-ENDLIST", None) => Tag::ExtXEndList,
        ("EXT-X-I-FRAMES-ONLY", None) => Tag::ExtXIFramesOnly,
        ("EXT-X-DISCONTINUITY", None) => Tag::ExtXDiscontinuity,
        ("EXT-X-GAP", None) => Tag::ExtXGap,
        ("EXT-X-INDEPENDENT-SEGMENTS", None) => Tag::ExtXIndependentSegments,
        ("EXT-X-VERSION", Some(value)) => Tag::ExtXVersion(parse_value(value, "EXT-X-VERSION")?),
        ("EXT-X-TARGETDURATION", Some(value)) => {
            Tag::ExtXTargetDuration(parse_value(value, "EXT-X-TARGETDURATION")?)
        }
        ("EXT-X-MEDIA-SEQUENCE", Some(value)) => {
            Tag::ExtXMediaSequence(parse_value(value, "EXT-X-MEDIA-SEQUENCE")?)
        }
        ("EXT-X-DISCONTINUITY-SEQUENCE", Some(value)) => {
            Tag::ExtXDiscontinuitySequence(parse_value(value, "EXT-X-DISCONTINUITY-SEQUENCE")?)
        }
        ("EXT-X-BITRATE", Some(value)) => Tag::ExtXBitrate(parse_value(value, "EXT-X-BITRATE")?),
        ("EXT-X-PLAYLIST-TYPE", Some(value @ ("EVENT" | "VOD"))) => {
            Tag::ExtXPlaylistType(value.to_string())
        }
        ("EXT-X-PLAYLIST-TYPE", Some(value)) => {
            return Err(invalid_value("EXT-X-PLAYLIST-TYPE", value))
        }
        ("EXT-X-ALLOW-CACHE", Some("YES")) => Tag::ExtXAllowCache(true),
        ("EXT-X-ALLOW-CACHE", Some("NO")) => Tag::ExtXAllowCache(false),
        ("EXT-X-ALLOW-CACHE", Some(value)) => {
            return Err(invalid_value("EXT-X-ALLOW-CACHE", value))
        }
        ("EXT-X-PROGRAM-DATE-TIME", Some(value)) => {
            Tag::ExtXProgramDateTime(non_empty(value, "EXT-X-PROGRAM-DATE-TIME")?)
        }
        ("EXT-X-BYTERANGE", Some(value)) => {
            Tag::ExtXByteRange(parse_value(value, "EXT-X-BYTERANGE")?)
        }
        ("EXT-X-DEFINE", Some(value)) => {
            attributes()?;
            Tag::ExtXDefine(value.to_string())
        }
        ("EXT-X-KEY", Some(_)) => Tag::ExtXKey(parse_key(&attributes()?, "EXT-X-KEY")?),
        ("EXT-X-SESSION-KEY", Some(_)) => {
            Tag::ExtXSessionKey(parse_key(&attributes()?, "EXT-X-SESSION-KEY")?)
        }
        ("EXT-X-MAP", Some(_)) => parse_map(&attributes()?)?,
        ("EXT-X-DATERANGE", Some(_)) => parse_date_range(attributes()?)?,
        ("EXT-X-START", Some(_)) => parse_start(&attributes()?)?,
        ("EXT-X-STREAM-INF", Some(_)) => parse_stream_inf(&attributes()?)?,
        ("EXT-X-I-FRAME-STREAM-INF", Some(_)) => parse_i_frame_stream_inf(&attributes()?)?,
        ("EXT-X-MEDIA", Some(_)) => parse_media(&attributes()?)?,
        ("EXT-X-SESSION-DATA", Some(_)) => parse_session_data(&attributes()?)?,
        ("EXT-X-SERVER-CONTROL", Some(_)) => parse_server_control(&attributes()?)?,
        ("EXT-X-PART-INF", Some(_)) => Tag::ExtXPartInf {
            part_target_duration: required_parsed(&attributes()?, "PART-TARGET", "EXT-X-PART-INF")?,
        },
        ("EXT-X-PART", Some(_)) => parse_part(&attributes()?)?,
        ("EXT-X-SKIP", Some(_)) => parse_skip(&attributes()?)?,
        ("EXT-X-PRELOAD-HINT", Some(_)) => parse_preload_hint(&attributes()?)?,
        ("EXT-X-RENDITION-REPORT", Some(_)) => parse_rendition_report(&attributes()?)?,
        (name, None) => match VALUE_TAGS.iter().find(|tag| **tag == name) {
            Some(tag) => return Err(SyntaxError::MissingTagValue { tag }),
            None => return Ok(None),
        },
        _ => return Ok(None),
    };

    Ok(Some(tag))
}

fn invalid_value(tag: &'static str, value: &str) -> SyntaxError {
    SyntaxError::InvalidTagValue {
        tag,
        value: value.to_string(),
    }
}

fn parse_value<T: std::str::FromStr>(value: &str, tag: &'static str) -> Result<T, SyntaxError> {
    value.parse().map_err(|_| invalid_value(tag, value))
}

fn non_empty(value: &str, tag: &'static str) -> Result<String, SyntaxError> {
    if value.is_empty() {
        Err(invalid_value(tag, value))
    } else {
        Ok(value.to_string())
    }
}

fn parse_key(attributes: &[Attribute], tag: &'static str) -> Result<Key, SyntaxError> {
    Ok(Key {
        method: required_string(attributes, "METHOD", tag)?,
        uri: optional_string(attributes, "URI"),
        iv: optional_string(attributes, "IV"),
        keyformat: optional_string(attributes, "KEYFORMAT"),
        keyformatversions: optional_string(attributes, "KEYFORMATVERSIONS"),
    })
}

fn parse_map(attributes: &[Attribute]) -> Result<Tag, SyntaxError> {
    Ok(Tag::ExtXMap(Map {
        uri: required_string(attributes, "URI", "EXT-X-MAP")?,
        byterange: match attribute(attributes, "BYTERANGE") {
            None | Some("") => None,
            Some(_) => optional_parsed(attributes, "BYTERANGE", "EXT-X-MAP")?,
        },
    }))
}

fn parse_date_range(attributes: Vec<Attribute>) -> Result<Tag, SyntaxError> {
    const TAG: &str = "EXT-X-DATERANGE";
    let mut id = None;
    let mut class = None;
    let mut start_date = None;
    let mut cue = None;
    let mut end_date = None;
    let mut duration = None;
    let mut planned_duration = None;
    let mut end_on_next = None;
    let mut scte35_cmd = None;
    let mut scte35_out = None;
    let mut scte35_in = None;
    let mut extra_attributes = Vec::new();

    for attribute in attributes {
        let value = attribute.value.clone();
        match attribute.name.as_str() {
            "ID" => id = Some(value),
            "CLASS" => class = Some(value),
            "START-DATE" => start_date = Some(value),
            "CUE" => cue = Some(value),
            "END-DATE" => end_date = Some(value),
            "DURATION" => duration = Some(parse_attribute_value(&value, TAG, "DURATION")?),
            "PLANNED-DURATION" => {
                planned_duration = Some(parse_attribute_value(&value, TAG, "PLANNED-DURATION")?)
            }
            "END-ON-NEXT" => {
                end_on_next = Some(match value.as_str() {
                    "YES" => true,
                    "NO" => false,
                    _ => {
                        return Err(SyntaxError::InvalidAttributeValue {
                            tag: TAG,
                            attribute: "END-ON-NEXT",
                            value,
                        })
                    }
                })
            }
            "SCTE35-CMD" => scte35_cmd = Some(value),
            "SCTE35-OUT" => scte35_out = Some(value),
            "SCTE35-IN" => scte35_in = Some(value),
            // X- client attributes and attributes from newer revisions of the
            // spec are preserved as written so they round-trip.
            _ => extra_attributes.push((attribute.name.clone(), attribute.into_value())),
        }
    }

    Ok(Tag::ExtXDateRange {
        id: id.ok_or(SyntaxError::MissingAttribute {
            tag: TAG,
            attribute: "ID",
        })?,
        class,
        start_date: start_date.ok_or(SyntaxError::MissingAttribute {
            tag: TAG,
            attribute: "START-DATE",
        })?,
        cue,
        end_date,
        duration,
        planned_duration,
        end_on_next,
        scte35_cmd,
        scte35_out,
        scte35_in,
        extra_attributes,
    })
}

fn parse_attribute_value<T: std::str::FromStr>(
    value: &str,
    tag: &'static str,
    attribute: &'static str,
) -> Result<T, SyntaxError> {
    value
        .parse()
        .map_err(|_| SyntaxError::InvalidAttributeValue {
            tag,
            attribute,
            value: value.to_string(),
        })
}

fn parse_start(attributes: &[Attribute]) -> Result<Tag, SyntaxError> {
    Ok(Tag::ExtXStart {
        time_offset: required_parsed(attributes, "TIME-OFFSET", "EXT-X-START")?,
        precise: optional_boolean(attributes, "PRECISE", "EXT-X-START")?,
    })
}

fn parse_stream_inf(attributes: &[Attribute]) -> Result<Tag, SyntaxError> {
    const TAG: &str = "EXT-X-STREAM-INF";
    Ok(Tag::ExtXStreamInf {
        bandwidth: required_parsed(attributes, "BANDWIDTH", TAG)?,
        average_bandwidth: optional_parsed(attributes, "AVERAGE-BANDWIDTH", TAG)?,
        codecs: optional_string(attributes, "CODECS"),
        resolution: optional_string(attributes, "RESOLUTION"),
        frame_rate: optional_parsed(attributes, "FRAME-RATE", TAG)?,
        audio: optional_string(attributes, "AUDIO"),
        video: optional_string(attributes, "VIDEO"),
        subtitle: optional_string(attributes, "SUBTITLES"),
        closed_captions: optional_string(attributes, "CLOSED-CAPTIONS"),
    })
}

fn parse_i_frame_stream_inf(attributes: &[Attribute]) -> Result<Tag, SyntaxError> {
    const TAG: &str = "EXT-X-I-FRAME-STREAM-INF";
    Ok(Tag::ExtXIFrameStreamInf {
        bandwidth: required_parsed(attributes, "BANDWIDTH", TAG)?,
        average_bandwidth: optional_parsed(attributes, "AVERAGE-BANDWIDTH", TAG)?,
        codecs: optional_string(attributes, "CODECS"),
        resolution: optional_string(attributes, "RESOLUTION"),
        frame_rate: optional_parsed(attributes, "FRAME-RATE", TAG)?,
        uri: required_string(attributes, "URI", TAG)?,
    })
}

fn parse_media(attributes: &[Attribute]) -> Result<Tag, SyntaxError> {
    const TAG: &str = "EXT-X-MEDIA";
    Ok(Tag::ExtXMedia {
        type_: required_string(attributes, "TYPE", TAG)?,
        group_id: required_string(attributes, "GROUP-ID", TAG)?,
        name: Some(required_string(attributes, "NAME", TAG)?),
        uri: optional_string(attributes, "URI"),
        default: optional_boolean(attributes, "DEFAULT", TAG)?,
        autoselect: optional_boolean(attributes, "AUTOSELECT", TAG)?,
        characteristics: optional_string(attributes, "CHARACTERISTICS"),
        language: optional_string(attributes, "LANGUAGE"),
        instream_id: optional_string(attributes, "INSTREAM-ID"),
        language_codec: optional_string(attributes, "LANGUAGE-CODEC"),
        forced: optional_boolean(attributes, "FORCED", TAG)?,
        channels: optional_string(attributes, "CHANNELS"),
    })
}

fn parse_session_data(attributes: &[Attribute]) -> Result<Tag, SyntaxError> {
    const TAG: &str = "EXT-X-SESSION-DATA";
    let id = required_string(attributes, "DATA-ID", TAG)?;
    let value = optional_string(attributes, "VALUE");
    let uri = optional_string(attributes, "URI");
    if value.is_some() == uri.is_some() {
        return Err(SyntaxError::ConflictingAttributes {
            tag: TAG,
            reason: "exactly one of VALUE or URI is required",
        });
    }
    Ok(Tag::ExtXSessionData {
        id,
        value,
        uri,
        language: optional_string(attributes, "LANGUAGE"),
    })
}

fn parse_server_control(attributes: &[Attribute]) -> Result<Tag, SyntaxError> {
    const TAG: &str = "EXT-X-SERVER-CONTROL";
    Ok(Tag::ExtXServerControl {
        can_skip_until: optional_parsed(attributes, "CAN-SKIP-UNTIL", TAG)?,
        can_skip_dateranges: optional_boolean(attributes, "CAN-SKIP-DATERANGES", TAG)?,
        hold_back: optional_parsed(attributes, "HOLD-BACK", TAG)?,
        part_hold_back: optional_parsed(attributes, "PART-HOLD-BACK", TAG)?,
        can_block_reload: optional_boolean(attributes, "CAN-BLOCK-RELOAD", TAG)?,
    })
}

fn parse_part(attributes: &[Attribute]) -> Result<Tag, SyntaxError> {
    const TAG: &str = "EXT-X-PART";
    Ok(Tag::ExtXPart {
        uri: required_string(attributes, "URI", TAG)?,
        duration: required_parsed(attributes, "DURATION", TAG)?,
        independent: optional_boolean(attributes, "INDEPENDENT", TAG)?,
        byterange: optional_parsed(attributes, "BYTERANGE", TAG)?,
        gap: optional_boolean(attributes, "GAP", TAG)?,
    })
}

fn parse_skip(attributes: &[Attribute]) -> Result<Tag, SyntaxError> {
    Ok(Tag::ExtXSkip {
        skipped_segments: required_parsed(attributes, "SKIPPED-SEGMENTS", "EXT-X-SKIP")?,
        recently_removed_dateranges: optional_string(attributes, "RECENTLY-REMOVED-DATERANGES"),
    })
}

fn parse_preload_hint(attributes: &[Attribute]) -> Result<Tag, SyntaxError> {
    const TAG: &str = "EXT-X-PRELOAD-HINT";
    Ok(Tag::ExtXPreloadHint {
        type_: required_string(attributes, "TYPE", TAG)?,
        uri: required_string(attributes, "URI", TAG)?,
        byterange_start: optional_parsed(attributes, "BYTERANGE-START", TAG)?,
        byterange_length: optional_parsed(attributes, "BYTERANGE-LENGTH", TAG)?,
    })
}

fn parse_rendition_report(attributes: &[Attribute]) -> Result<Tag, SyntaxError> {
    const TAG: &str = "EXT-X-RENDITION-REPORT";
    Ok(Tag::ExtXRenditionReport {
        uri: required_string(attributes, "URI", TAG)?,
        last_msn: optional_parsed(attributes, "LAST-MSN", TAG)?,
        last_part: optional_parsed(attributes, "LAST-PART", TAG)?,
    })
}

/// Whether `day` exists in `month` of `year`; all three are already known to be numeric.
fn day_in_month(year: &str, month: &str, day: &str) -> bool {
    let (year, month, day): (u32, u32, u32) = match (year.parse(), month.parse(), day.parse()) {
        (Ok(year), Ok(month), Ok(day)) => (year, month, day),
        _ => return false,
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    day <= days
}

/// Checks the ISO 8601 date-time form HLS uses, e.g. `2010-02-19T14:54:23.031+08:00`.
fn is_date_time(value: &str) -> bool {
    fn number_in(value: &str, digits: usize, min: u32, max: u32) -> bool {
        value.len() == digits
            && value.bytes().all(|byte| byte.is_ascii_digit())
            && value
                .parse()
                .is_ok_and(|number: u32| (min..=max).contains(&number))
    }

    let Some((date, time)) = value.split_once('T') else {
        return false;
    };
    let date_ok = matches!(
        date.split('-').collect::<Vec<_>>().as_slice(),
        [year, month, day] if number_in(year, 4, 0, 9999)
            && number_in(month, 2, 1, 12)
            && number_in(day, 2, 1, 31)
            && day_in_month(year, month, day)
    );

    let (clock, zone) = time.split_at(time.find(['Z', '+', '-']).unwrap_or(time.len()));
    let (clock, fraction) = match clock.split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (clock, None),
    };
    let fraction_ok = fraction.is_none_or(|fraction| {
        !fraction.is_empty() && fraction.bytes().all(|b| b.is_ascii_digit())
    });
    let clock_ok = match clock.split(':').collect::<Vec<_>>().as_slice() {
        [hour, minute] => {
            fraction.is_none() && number_in(hour, 2, 0, 23) && number_in(minute, 2, 0, 59)
        }
        [hour, minute, second] => {
            number_in(hour, 2, 0, 23) && number_in(minute, 2, 0, 59) && number_in(second, 2, 0, 60)
        }
        _ => false,
    };
    let zone_ok = match zone {
        "" | "Z" => true,
        _ => {
            let offset = zone[1..].replace(':', "");
            if !offset.is_ascii() {
                return false;
            }
            match offset.len() {
                2 => number_in(&offset, 2, 0, 23),
                4 => number_in(&offset[..2], 2, 0, 23) && number_in(&offset[2..], 2, 0, 59),
                _ => false,
            }
        }
    };

    date_ok && fraction_ok && clock_ok && zone_ok
}

fn validate_key(key: &Key, tag: &str, errors: &mut Vec<ValidationError>) {
    match key.method.as_str() {
        "NONE" => {
            if key.uri.is_some()
                || key.iv.is_some()
                || key.keyformat.is_some()
                || key.keyformatversions.is_some()
            {
                errors.push(ValidationError::InvalidKeyAttributes(format!(
                    "{tag} METHOD=NONE must not include URI, IV, KEYFORMAT, or KEYFORMATVERSIONS"
                )));
            }
        }
        "AES-128" | "SAMPLE-AES" => {
            if key.uri.as_deref().is_none_or(str::is_empty) {
                errors.push(ValidationError::InvalidKeyAttributes(format!(
                    "{tag} encryption methods require URI"
                )));
            }
        }
        _ => errors.push(ValidationError::InvalidKeyMethod(key.method.clone())),
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
            | Tag::ExtXSessionKey(_)
    )
}

fn is_media_segment_tag(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::ExtXTargetDuration(_)
            | Tag::ExtXMediaSequence(_)
            | Tag::ExtXDiscontinuitySequence(_)
            | Tag::ExtXEndList
            | Tag::ExtInf { .. }
            | Tag::ExtXKey(_)
            | Tag::ExtXMap(_)
            | Tag::ExtXProgramDateTime(_)
            | Tag::ExtXByteRange(_)
            | Tag::ExtXDateRange { .. }
            | Tag::ExtXGap
            | Tag::ExtXIFramesOnly
    )
}

/// The lowest EXT-X-VERSION that permits `tag`, per section 8 of
/// draft-pantos-hls-rfc8216bis.
fn minimum_version(tag: &Tag, has_i_frames_only: bool) -> Option<u8> {
    match tag {
        Tag::ExtXKey(key) | Tag::ExtXSessionKey(key) => {
            if key.method == "SAMPLE-AES"
                || key.keyformat.is_some()
                || key.keyformatversions.is_some()
            {
                Some(5)
            } else if key.iv.is_some() {
                Some(2)
            } else {
                None
            }
        }
        Tag::ExtInf { duration, .. } if duration.fract() != 0.0 => Some(3),
        Tag::ExtXByteRange(_) | Tag::ExtXIFramesOnly => Some(4),
        Tag::ExtXMap(_) if has_i_frames_only => Some(5),
        Tag::ExtXMap(_) => Some(6),
        Tag::ExtXMedia {
            instream_id: Some(instream_id),
            ..
        } if instream_id.starts_with("SERVICE") => Some(7),
        Tag::ExtXDefine(value) => {
            let has_query_param = parse_attribute_list(value)
                .is_ok_and(|attributes| attributes.iter().any(|a| a.name == "QUERYPARAM"));
            Some(if has_query_param { 11 } else { 8 })
        }
        Tag::ExtXSkip {
            recently_removed_dateranges: Some(_),
            ..
        } => Some(10),
        Tag::ExtXSkip { .. } => Some(9),
        Tag::ExtXDateRange {
            extra_attributes, ..
        } if extra_attributes
            .iter()
            .any(|(name, _)| name.starts_with("REQ-")) =>
        {
            Some(12)
        }
        _ => None,
    }
}

fn tag_name(tag: &Tag) -> &'static str {
    match tag {
        Tag::ExtInf { .. } => "EXTINF",
        Tag::ExtXByteRange(_) => "EXT-X-BYTERANGE",
        Tag::ExtXIFramesOnly => "EXT-X-I-FRAMES-ONLY",
        Tag::ExtXMap(_) => "EXT-X-MAP",
        Tag::ExtXKey(_) => "EXT-X-KEY",
        Tag::ExtXSessionKey(_) => "EXT-X-SESSION-KEY",
        Tag::ExtXDateRange { .. } => "EXT-X-DATERANGE",
        Tag::ExtXStart { .. } => "EXT-X-START",
        Tag::ExtXIndependentSegments => "EXT-X-INDEPENDENT-SEGMENTS",
        Tag::ExtXMedia { .. } => "EXT-X-MEDIA",
        Tag::ExtXStreamInf { .. } => "EXT-X-STREAM-INF",
        Tag::ExtXIFrameStreamInf { .. } => "EXT-X-I-FRAME-STREAM-INF",
        Tag::ExtXDefine(_) => "EXT-X-DEFINE",
        Tag::ExtXSkip { .. } => "EXT-X-SKIP",
        _ => "tag",
    }
}
