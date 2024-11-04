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

use crate::m3u8::parser::parse_attributes;
use crate::m3u8::tags::Tag;
use crate::m3u8::validation::ValidationError;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

/// Represents a playlist containing multiple tags.
#[derive(Debug, PartialEq)]
pub struct Playlist {
    pub tags: Vec<Tag>,
}

impl Playlist {
    /// Creates a new `Playlist` by reading tags from a buffered reader.
    pub fn from_reader<R: BufRead>(reader: R) -> Result<Self, String> {
        let mut tags = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?.trim().to_string();
            if line.is_empty() {
                continue;
            }

            if let Some(tag) = Self::parse_line(&line)? {
                tags.push(tag);
            }
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
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn parse_line(line: &str) -> Result<Option<Tag>, String> {
        if line.starts_with("#EXTM3U") {
            return Ok(Some(Tag::ExtM3U));
        }
        if line.starts_with("#EXT-X-ENDLIST") {
            return Ok(Some(Tag::ExtXEndList));
        }

        if let Some((prefix, stripped)) = line.split_once(':') {
            match prefix {
                "#EXT-X-VERSION" => {
                    let version = stripped
                        .parse()
                        .map_err(|_| "Invalid version".to_string())?;
                    return Ok(Some(Tag::ExtXVersion(version)));
                }
                "#EXTINF" => {
                    let parts: Vec<&str> = stripped.splitn(2, ',').collect();
                    let duration = parts[0]
                        .parse()
                        .map_err(|_| "Invalid duration".to_string())?;
                    let title = parts.get(1).map(|&s| s.to_string());
                    if title.clone().is_some_and(move |t| !t.is_empty()) {
                        return Ok(Some(Tag::ExtInf(duration, title)));
                    }
                    return Ok(Some(Tag::ExtInf(duration, None)));
                }
                "#EXT-X-BYTERANGE" => {
                    let byterange = stripped.to_string(); // Parse the byterange format as needed
                    return Ok(Some(Tag::ExtXByteRange(byterange)));
                }
                "#EXT-X-DEFINE" => {
                    // Define your structure for EXT-X-DEFINE
                    return Ok(Some(Tag::ExtXDefine(stripped.to_string())));
                }
                "#EXT-X-MEDIA" => {
                    let attributes = parse_attributes(stripped)?;

                    let media_type = attributes
                        .get("TYPE")
                        .ok_or("Missing TYPE attribute")?
                        .clone();
                    let uri = attributes.get("URI").cloned();
                    let group_id = attributes
                        .get("GROUP-ID")
                        .ok_or("Missing GROUP-ID attribute")?
                        .clone();
                    let name = attributes.get("NAME").cloned();
                    let language = attributes.get("LANGUAGE").cloned();
                    let autoselect = attributes.get("AUTOSELECT").map(|s| s == "YES");
                    let is_default = attributes.get("DEFAULT").map(|s| s == "YES");
                    let characteristics = attributes.get("CHARACTERISTICS").cloned();

                    return Ok(Some(Tag::ExtXMedia {
                        type_: media_type,
                        group_id,
                        name,
                        uri, // uri can be None, so no need to unwrap
                        default: is_default,
                        autoplay: autoselect,
                        language, // Include language property
                        characteristics,
                    }));
                }
                "#EXT-X-STREAM-INF" => {
                    let attributes = parse_attributes(stripped)?;

                    let bandwidth = attributes
                        .get("BANDWIDTH")
                        .ok_or("Missing BANDWIDTH attribute")?
                        .parse::<u32>()
                        .map_err(|_| "Invalid BANDWIDTH value")?;
                    let codecs = attributes.get("CODECS").cloned();
                    let resolution = attributes.get("RESOLUTION").cloned();
                    let frame_rate = attributes
                        .get("FRAME-RATE")
                        .and_then(|s| s.parse::<f32>().ok());
                    let audio = attributes.get("AUDIO").cloned();
                    let video = attributes.get("VIDEO").cloned();
                    let subtitle = attributes.get("SUBTITLES").cloned();
                    let closed_captions = attributes.get("CLOSED-CAPTIONS").cloned();

                    return Ok(Some(Tag::ExtXStreamInf {
                        bandwidth,
                        codecs,
                        resolution,
                        frame_rate,
                        audio,
                        video,
                        subtitle,
                        closed_captions,
                    }));
                }

                "#EXT-X-I-FRAME-STREAM-INF" => {
                    let attributes = parse_attributes(stripped)?;

                    let bandwidth = attributes
                        .get("BANDWIDTH")
                        .ok_or("Missing BANDWIDTH attribute")?
                        .parse::<u32>()
                        .map_err(|_| "Invalid BANDWIDTH value")?;
                    let codecs = attributes.get("CODECS").cloned();
                    let resolution = attributes.get("RESOLUTION").cloned();
                    let frame_rate = attributes
                        .get("FRAME-RATE")
                        .and_then(|s| s.parse::<f32>().ok());
                    let uri = attributes
                        .get("URI")
                        .ok_or("Missing URI attribute")?
                        .clone();

                    return Ok(Some(Tag::ExtXIFrameStreamInf {
                        bandwidth,
                        codecs,
                        resolution,
                        frame_rate,
                        uri,
                    }));
                }
                "#EXT-X-KEY" => {
                    let attributes = parse_attributes(stripped)?;
                    let method = attributes
                        .get("METHOD")
                        .ok_or("Missing METHOD attribute")?
                        .clone();
                    let uri = attributes.get("URI").cloned();
                    return Ok(Some(Tag::ExtXKey {
                        method,
                        uri,
                        iv: attributes.get("IV").cloned(),
                        keyformat: attributes.get("KEYFORMAT").cloned(),
                        keyformatversions: attributes.get("KEYFORMATVERSIONS").cloned(),
                    }));
                }
                "#EXT-X-MAP" => {
                    let attributes = parse_attributes(stripped)?;
                    let uri = attributes
                        .get("URI")
                        .ok_or("Missing URI attribute")?
                        .clone();
                    return Ok(Some(Tag::ExtXMap {
                        uri,
                        byterange: attributes.get("BYTERANGE").cloned(),
                    }));
                }
                "#EXT-X-PROGRAM-DATE-TIME" => {
                    return Ok(Some(Tag::ExtXProgramDateTime(stripped.to_string())));
                }
                "#EXT-X-DATERANGE" => {
                    let attributes = parse_attributes(stripped)?;
                    let id = attributes.get("ID").ok_or("Missing ID attribute")?.clone();
                    let start_date = attributes
                        .get("START-DATE")
                        .ok_or("Missing START-DATE attribute")?
                        .clone();
                    let end_date = attributes.get("END-DATE").cloned();
                    let duration = attributes
                        .get("DURATION")
                        .map(|s| s.parse::<f32>().map_err(|_| "Invalid duration".to_string()))
                        .transpose()?;
                    let planned_duration = attributes
                        .get("PLANNED-DURATION")
                        .map(|s| {
                            s.parse()
                                .map_err(|_| "Invalid planned duration".to_string())
                        })
                        .transpose()?;

                    return Ok(Some(Tag::ExtXDateRange {
                        id,
                        start_date,
                        end_date,
                        duration,
                        planned_duration,
                        scte35_cmd: attributes.get("SCTE35-CMD").cloned(),
                        scte35_out: attributes.get("SCTE35-OUT").cloned(),
                        scte35_in: attributes.get("SCTE35-IN").cloned(),
                        end_on_next: attributes.get("END-ON-NEXT").map(|s| s == "YES"),
                    }));
                }
                "#EXT-X-GAP" => {
                    return Ok(Some(Tag::ExtXGap));
                }
                "#EXT-X-BITRATE" => {
                    let bitrate = stripped
                        .parse()
                        .map_err(|_| "Invalid bitrate".to_string())?;
                    return Ok(Some(Tag::ExtXBitrate(bitrate)));
                }
                "#EXT-X-INDEPENDENT-SEGMENTS" => {
                    return Ok(Some(Tag::ExtXIndependentSegments));
                }
                "#EXT-X-START" => {
                    let attributes = parse_attributes(stripped)?;
                    let time_offset = attributes
                        .get("TIME-OFFSET")
                        .ok_or("Missing TIME-OFFSET attribute")?
                        .clone();
                    let precise = attributes
                        .get("PRECISE")
                        .map(|s| s == "YES")
                        .unwrap_or(false);
                    return Ok(Some(Tag::ExtXStart {
                        time_offset,
                        precise: Some(precise),
                    }));
                }
                "#EXT-X-SERVER-CONTROL" => {
                    let attributes = parse_attributes(stripped)?;

                    let can_play = attributes.get("CAN-PLAY").map(|s| s == "YES");
                    let can_seek = attributes.get("CAN-SEEK").map(|s| s == "YES");
                    let can_pause = attributes.get("CAN-PAUSE").map(|s| s == "YES");
                    let min_buffer_time = attributes
                        .get("MIN-BUFFER-TIME")
                        .map(|s| {
                            s.parse::<f32>()
                                .map_err(|_| "Invalid MIN-BUFFER-TIME value")
                        })
                        .transpose()?;

                    return Ok(Some(Tag::ExtXServerControl {
                        can_play,
                        can_seek,
                        can_pause,
                        min_buffer_time,
                    }));
                }
                "#EXT-X-PART-INF" => {
                    let attributes = parse_attributes(stripped)?;

                    let part_target_duration = attributes
                        .get("PART-TARGET-DURATION")
                        .ok_or("Missing PART-TARGET-DURATION attribute")?
                        .parse::<f32>()
                        .map_err(|_| "Invalid PART-TARGET-DURATION value")?;

                    let part_hold_back = attributes
                        .get("PART-HOLD-BACK")
                        .map(|s| s.parse::<f32>().map_err(|_| "Invalid PART-HOLD-BACK value"))
                        .transpose()?;

                    let part_number = attributes
                        .get("PART-NUMBER")
                        .map(|s| s.parse::<u64>().map_err(|_| "Invalid PART-NUMBER value"))
                        .transpose()?;

                    return Ok(Some(Tag::ExtXPartInf {
                        part_target_duration,
                        part_hold_back,
                        part_number,
                    }));
                }
                "#EXT-X-PRELOAD-HINT" => {
                    let attributes = parse_attributes(stripped)?;

                    let uri = attributes
                        .get("URI")
                        .ok_or("Missing URI attribute")?
                        .clone();

                    let byterange = attributes.get("BYTERANGE").cloned();

                    return Ok(Some(Tag::ExtXPreloadHint { uri, byterange }));
                }
                "#EXT-X-RENDITION-REPORT" => {
                    let attributes = parse_attributes(stripped)?;

                    let uri = attributes
                        .get("URI")
                        .ok_or("Missing URI attribute")?
                        .clone();
                    let bandwidth = attributes
                        .get("BANDWIDTH")
                        .ok_or("Missing BANDWIDTH attribute")?
                        .parse::<u32>()
                        .map_err(|_| "Invalid BANDWIDTH value")?;

                    return Ok(Some(Tag::ExtXRenditionReport { uri, bandwidth }));
                }
                "#EXT-X-PART" => {
                    let attributes = parse_attributes(stripped)?;

                    let uri = attributes
                        .get("URI")
                        .ok_or("Missing URI attribute")?
                        .clone();
                    let duration = attributes
                        .get("DURATION")
                        .map(|s| s.parse::<f32>().map_err(|_| "Invalid DURATION value"))
                        .transpose()?;

                    return Ok(Some(Tag::ExtXPart { uri, duration }));
                }
                "#EXT-X-SKIP" => {
                    let attributes = parse_attributes(stripped)?;

                    let uri = attributes
                        .get("URI")
                        .ok_or("Missing URI attribute")?
                        .clone();
                    let skipped_segments = attributes
                        .get("SKIPPED-SEGMENTS")
                        .ok_or("Missing SKIPPED-SEGMENTS attribute")?
                        .parse::<u32>()
                        .map_err(|_| "Invalid SKIPPED-SEGMENTS value")?;

                    // Optional fields
                    let duration = attributes
                        .get("DURATION")
                        .and_then(|d| d.parse::<f32>().ok());
                    let reason = attributes.get("REASON").cloned();

                    return Ok(Some(Tag::ExtXSkip {
                        uri,
                        duration,
                        skipped_segments,
                        reason,
                    }));
                }
                "#EXT-X-DISCONTINUITY" => {
                    return Ok(Some(Tag::ExtXDiscontinuity));
                }
                "#EXT-X-SESSION-DATA" => {
                    // Example input: #EXT-X-SESSION-DATA:ID="sessionId",VALUE="someValue",LANGUAGE="en"
                    // Remove the prefix and split by commas
                    let parts: Vec<&str> = stripped.split(',').collect();
                    let mut id = None;
                    let mut value = None;
                    let mut language = None;

                    for part in parts {
                        // Split each part into key-value pairs
                        let kv: Vec<&str> = part.splitn(2, '=').collect();
                        if kv.len() == 2 {
                            let key = kv[0].trim();
                            let val = kv[1].trim().trim_matches('"'); // Remove surrounding quotes

                            match key {
                                "ID" => id = Some(val.to_string()),
                                "VALUE" => value = Some(val.to_string()),
                                "LANGUAGE" => language = Some(val.to_string()),
                                _ => {
                                    return Err(format!(
                                        "Unknown key in EXT-X-SESSION-DATA: {}",
                                        key
                                    ))
                                }
                            }
                        } else {
                            return Err(format!("Invalid format for EXT-X-SESSION-DATA: {}", part));
                        }
                    }

                    // Ensure required fields are present
                    return if let (Some(id), Some(value)) = (id, value) {
                        // Create the ExtXSessionData tag
                        let session_data = Tag::ExtXSessionData {
                            id,
                            value,
                            language,
                        };
                        Ok(Some(session_data))
                    } else {
                        Err("Missing required fields in EXT-X-SESSION-DATA".to_string())
                    };
                }
                "#EXT-X-SESSION-KEY" => {
                    // Example input: #EXT-X-SESSION-KEY:METHOD=AES-128,URI="https://example.com/key",IV="0x1234567890ABCDEF"

                    // Parse the attributes
                    let attributes = parse_attributes(stripped)?;

                    // Extract required fields
                    let method = attributes
                        .get("METHOD")
                        .ok_or("Missing METHOD attribute")?
                        .clone();

                    let uri = attributes.get("URI").cloned();
                    let iv = attributes.get("IV").cloned();

                    // Create the ExtXSessionKey tag
                    let session_key = Tag::ExtXSessionKey { method, uri, iv };

                    return Ok(Some(session_key));
                }
                "#EXT-X-TARGETDURATION" => {
                    // Example input: #EXT-X-TARGETDURATION:10
                    let target_duration: u64 =
                        stripped.parse().map_err(|_| "Invalid target duration")?;

                    // Create the ExtXTargetDuration tag
                    let target_duration_tag = Tag::ExtXTargetDuration(target_duration);

                    return Ok(Some(target_duration_tag));
                }
                "#EXT-X-PLAYLIST-TYPE" => {
                    let attributes = parse_attributes(stripped)?;
                    let playlist_type = attributes
                        .get("PLAYLIST-TYPE")
                        .ok_or("Missing PLAYLIST-TYPE attribute")?
                        .clone();

                    if playlist_type != "EVENT" && playlist_type != "VOD" {
                        return Err("Invalid PLAYLIST-TYPE value".to_string());
                    }

                    return Ok(Some(Tag::ExtXPlaylistType(playlist_type)));
                }
                _ => {}
            }
        }

        if !line.starts_with('#') {
            return Ok(Some(Tag::Uri(line.to_string())));
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
            Tag::ExtInf(duration, _) if *duration <= 0.0 => {
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
            Tag::ExtXMap { uri, .. } if uri.is_empty() => {
                errors.push(ValidationError::InvalidMapUri);
            }
            Tag::ExtXProgramDateTime(date_time) if date_time.is_empty() => {
                errors.push(ValidationError::InvalidProgramDateTime);
            }
            Tag::ExtXDateRange {
                id,
                start_date,
                duration,
                planned_duration,
                ..
            } => {
                if id.is_empty() {
                    errors.push(ValidationError::InvalidDateRangeId);
                }
                if start_date.is_empty() {
                    errors.push(ValidationError::InvalidDateRangeStartDate);
                }
                if let Some(dur) = duration {
                    if *dur < 0.0 {
                        errors.push(ValidationError::InvalidDateRangeDuration(*dur));
                    }
                }
                if let Some(planned_dur) = planned_duration {
                    if *planned_dur < 0.0 {
                        errors.push(ValidationError::InvalidDateRangePlannedDuration(
                            *planned_dur,
                        ));
                    }
                }

                // if end_date.is_some_and(move |t| {t.is_empty()})  {
                //     // Add checks for end_date format or validity as needed
                // } else {
                //     errors.push(ValidationError::InvalidDateRangeEndDate);
                // }
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
            Tag::ExtXSkip { duration, .. } if duration.unwrap() <= 0.0 => {
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
}
