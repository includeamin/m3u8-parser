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

use regex::Regex;

/// Represents a playlist containing multiple tags.
#[derive(Debug, PartialEq)]
pub struct Playlist {
    pub tags: Vec<Tag>,
}

impl Playlist {
    /// Creates a new `Playlist` by reading tags from a buffered reader.
    pub fn from_reader<R: BufRead>(mut reader: R) -> Result<Self, String> {
        let mut tags = Vec::new();

        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .map_err(|e| e.to_string())?;

        for line in content.split("#") {
            if line.is_empty() {
                continue;
            }

            if let Some(tag) = Self::parse_line(line)? {
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
        let trimmed = line.trim();

        if trimmed.starts_with("EXTM3U") {
            return Ok(Some(Tag::ExtM3U));
        }

        if trimmed.starts_with("EXT-X-VERSION") {
            // Example: #EXT-X-VERSION:7
            let version_re = Regex::new(r#"EXT-X-VERSION:(\d+)"#).unwrap();
            if let Some(caps) = version_re.captures(trimmed) {
                let version = caps.get(1).unwrap().as_str();
                return Ok(Some(Tag::ExtXVersion(version.parse().unwrap())));
            }
        }

        if trimmed.starts_with("EXT-X-TARGETDURATION") {
            // Example #EXT-X-TARGETDURATION:10
            let target_duration_re = Regex::new(r#"EXT-X-TARGETDURATION:(\d+)"#).unwrap();
            if let Some(caps) = target_duration_re.captures(trimmed) {
                let target = caps.get(1).unwrap().as_str();
                return Ok(Some(Tag::ExtXTargetDuration(target.parse().unwrap())));
            }
        }

        if trimmed.starts_with("EXT-X-PLAYLIST-TYPE") {
            // Example: #EXT-X-PLAYLIST-TYPE:EVENT
            let playlist_type_re = Regex::new(r#"EXT-X-PLAYLIST-TYPE:(\w+)"#).unwrap();
            if let Some(caps) = playlist_type_re.captures(trimmed) {
                let playlist_type = caps.get(1).unwrap().as_str();
                return Ok(Some(Tag::ExtXPlaylistType(playlist_type.to_string())));
            }
        }

        if trimmed.starts_with("EXT-X-MEDIA-SEQUENCE") {
            // Example: #EXT-X-MEDIA-SEQUENCE:0
            let media_sequence_re = Regex::new(r#"EXT-X-MEDIA-SEQUENCE:(\d+)"#).unwrap();
            if let Some(caps) = media_sequence_re.captures(trimmed) {
                let sequence = caps.get(1).unwrap().as_str();
                return Ok(Some(Tag::ExtXMediaSequence(sequence.parse().unwrap())));
            }
        }

        if trimmed.starts_with("EXT-X-DISCONTINUITY-SEQUENCE") {
            // Example: #EXT-X-DISCONTINUITY-SEQUENCE:0
            let discontinuity_seq_re = Regex::new(r#"EXT-X-DISCONTINUITY-SEQUENCE:(\d+)"#).unwrap();
            if let Some(caps) = discontinuity_seq_re.captures(trimmed) {
                let sequence = caps.get(1).unwrap().as_str();
                return Ok(Some(Tag::ExtXDiscontinuitySequence(
                    sequence.parse().unwrap(),
                )));
            }
        }

        if trimmed.starts_with("EXT-X-ENDLIST") {
            return Ok(Some(Tag::ExtXEndList));
        }

        if trimmed.starts_with("EXT-X-KEY") {
            // Example: #EXT-X-KEY:METHOD=AES-128,URI="https://example.com/key",IV="0x1234567890ABCDEF",KEYFORMAT="identity",KEYFORMATVERSIONS="1"
            let key_re = Regex::new(r#"EXT-X-KEY:METHOD=([A-Za-z0-9\-]+),URI="([^"]+)"(?:,IV="([^"]*)")?(?:,KEYFORMAT="([^"]+)")?(?:,KEYFORMATVERSIONS="([^"]+)")?"#).unwrap();

            if let Some(caps) = key_re.captures(trimmed) {
                let method = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
                let uri = caps.get(2).map(|m| m.as_str().to_string());
                let iv = caps.get(3).map(|m| m.as_str().to_string());
                let keyformat = caps.get(4).map(|m| m.as_str().to_string());
                let keyformatversions = caps.get(5).map(|m| m.as_str().to_string());

                return Ok(Some(Tag::ExtXKey {
                    method: method.to_string(),
                    uri,
                    iv,
                    keyformat,
                    keyformatversions,
                }));
            }
        }

        if trimmed.starts_with("EXT-X-MAP") {
            // Example: #EXT-X-MAP:URI="init.mp4",BYTERANGE="800@0"
            let map_re = Regex::new(r#"EXT-X-MAP:URI="([^"]+)"(?:,BYTERANGE="([^"]+)")?"#).unwrap();
            if let Some(caps) = map_re.captures(trimmed) {
                let uri = caps.get(1).unwrap().as_str();
                let byterange = caps.get(2).map(|m| m.as_str().to_string());
                if byterange.clone().is_none() || byterange.clone().unwrap() == "" {
                    return Ok(Some(Tag::ExtXMap {
                        uri: uri.to_string(),
                        byterange: None,
                    }));
                }

                return Ok(Some(Tag::ExtXMap {
                    uri: uri.to_string(),
                    byterange,
                }));
            }
        }

        if trimmed.starts_with("EXT-X-PROGRAM-DATE-TIME") {
            // Example: #EXT-X-PROGRAM-DATE-TIME:2024-11-05T12:00:00Z
            let datetime_re = Regex::new(r#"EXT-X-PROGRAM-DATE-TIME:([^\s]+)"#).unwrap();
            if let Some(caps) = datetime_re.captures(trimmed) {
                let datetime = caps.get(1).unwrap().as_str();
                return Ok(Some(Tag::ExtXProgramDateTime(datetime.to_string())));
            }
        }

        if trimmed.starts_with("EXT-X-DISCONTINUITY") {
            return Ok(Some(Tag::ExtXDiscontinuity));
        }

        if trimmed.starts_with("EXT-X-PART") {
            // Example: #EXT-X-PART:URI="part1.ts",DURATION=5.0
            let part_re = Regex::new(r#"EXT-X-PART:URI="([^\"]+)",DURATION=([\d\.]+)"#).unwrap();
            if let Some(caps) = part_re.captures(trimmed) {
                let uri = caps.get(1).unwrap().as_str();
                let duration = caps.get(2).unwrap().as_str().parse().unwrap();
                return Ok(Some(Tag::ExtXPart {
                    uri: uri.to_string(),
                    duration: Some(duration),
                    independent: None,
                }));
            }
        }

        if trimmed.starts_with("EXT-X-PART-INF") {
            // Example: #EXT-X-PART-INF:PART-TARGET=5.0
            // Note: PART-HOLD-BACK is now in EXT-X-SERVER-CONTROL, not here
            let part_inf_re = Regex::new(
                r#"EXT-X-PART-INF:PART-TARGET=([\d\.]+)"#,
            )
            .unwrap();
            if let Some(caps) = part_inf_re.captures(trimmed) {
                let part_target_duration = caps.get(1).unwrap().as_str().parse().unwrap();
                return Ok(Some(Tag::ExtXPartInf {
                    part_target_duration,
                    part_number: None,
                }));
            }
            // Also try PART-TARGET-DURATION for backwards compatibility
            let part_inf_re_alt = Regex::new(
                r#"EXT-X-PART-INF:PART-TARGET-DURATION=([\d\.]+)"#,
            )
            .unwrap();
            if let Some(caps) = part_inf_re_alt.captures(trimmed) {
                let part_target_duration = caps.get(1).unwrap().as_str().parse().unwrap();
                return Ok(Some(Tag::ExtXPartInf {
                    part_target_duration,
                    part_number: None,
                }));
            }
        }

        if trimmed.starts_with("EXT-X-SERVER-CONTROL") {
            // Example: #EXT-X-SERVER-CONTROL:CAN-BLOCK-RELOAD=YES,PART-HOLD-BACK=1.0,CAN-SKIP-UNTIL=24.0
            // Parse attributes flexibly
            let mut can_play = None;
            let mut can_seek = None;
            let mut can_pause = None;
            let mut min_buffer_time = None;
            let mut can_block_reload = None;
            let mut part_hold_back = None;
            let mut can_skip_until = None;

            // Extract all key=value pairs
            let attr_re = Regex::new(r#"([A-Z-]+)=([^,]+)"#).unwrap();
            for cap in attr_re.captures_iter(trimmed) {
                let key = cap.get(1).unwrap().as_str();
                let value = cap.get(2).unwrap().as_str();
                match key {
                    "CAN-PLAY" => can_play = Some(value == "YES"),
                    "CAN-SEEK" => can_seek = Some(value == "YES"),
                    "CAN-PAUSE" => can_pause = Some(value == "YES"),
                    "MIN-BUFFER-TIME" => {
                        if let Ok(val) = value.parse::<f32>() {
                            min_buffer_time = Some(val);
                        }
                    }
                    "CAN-BLOCK-RELOAD" => can_block_reload = Some(value == "YES"),
                    "PART-HOLD-BACK" => {
                        if let Ok(val) = value.parse::<f32>() {
                            part_hold_back = Some(val);
                        }
                    }
                    "CAN-SKIP-UNTIL" => {
                        if let Ok(val) = value.parse::<f32>() {
                            can_skip_until = Some(val);
                        }
                    }
                    _ => {}
                }
            }

            // Only create tag if at least one attribute was found
            if can_play.is_some()
                || can_seek.is_some()
                || can_pause.is_some()
                || min_buffer_time.is_some()
                || can_block_reload.is_some()
                || part_hold_back.is_some()
                || can_skip_until.is_some()
            {
                return Ok(Some(Tag::ExtXServerControl {
                    can_play,
                    can_seek,
                    can_pause,
                    min_buffer_time,
                    can_block_reload,
                    part_hold_back,
                    can_skip_until,
                }));
            }
        }

        if trimmed.starts_with("EXT-X-SKIP") {
            // Example: #EXT-X-SKIP:SKIPPED-SEGMENTS=3
            // Optional: ,RECENTLY-REMOVED-DATERANGES="id1\tid2"
            let mut skipped_segments = None;
            let mut recently_removed_dateranges = None;

            let attr_re = Regex::new(r#"([A-Z-]+)=("([^"]*)"|([^,]+))"#).unwrap();
            for cap in attr_re.captures_iter(trimmed) {
                let key = cap.get(1).unwrap().as_str();
                // Use quoted value (group 3) if present, otherwise unquoted (group 4)
                let value = cap.get(3).or(cap.get(4)).unwrap().as_str();
                match key {
                    "SKIPPED-SEGMENTS" => {
                        skipped_segments = value.parse::<u32>().ok();
                    }
                    "RECENTLY-REMOVED-DATERANGES" => {
                        recently_removed_dateranges = Some(value.to_string());
                    }
                    _ => {}
                }
            }

            if let Some(skipped) = skipped_segments {
                return Ok(Some(Tag::ExtXSkip {
                    skipped_segments: skipped,
                    recently_removed_dateranges,
                }));
            }
        }

        if trimmed.starts_with("EXT-X-START") {
            // Example: #EXT-X-START:TIME-OFFSET=0.0,PRECISE=YES
            let start_re =
                Regex::new(r#"EXT-X-START:TIME-OFFSET=([\d\.]+),PRECISE=(\w+)"#).unwrap();
            if let Some(caps) = start_re.captures(trimmed) {
                let time_offset = caps.get(1).unwrap().as_str().to_string();
                let precise = caps.get(2).unwrap().as_str() == "YES";
                return Ok(Some(Tag::ExtXStart {
                    time_offset,
                    precise: Some(precise),
                }));
            }
        }

        if trimmed.starts_with("EXT-X-INDEPENDENT-SEGMENTS") {
            return Ok(Some(Tag::ExtXIndependentSegments));
        }

        if trimmed.starts_with("EXT-X-STREAM-INF") {
            // Example: #EXT-X-STREAM-INF:BANDWIDTH=500000,RESOLUTION=640x360,CODECS="avc1.42c01e,mp4a.40.2"
            let stream_inf_re = Regex::new(
                r#"EXT-X-STREAM-INF:BANDWIDTH=(\d+),RESOLUTION=([^,]+),CODECS="([^"]+)"\s*(\S+)"#,
            )
            .unwrap();
            if let Some(caps) = stream_inf_re.captures(trimmed) {
                let bandwidth = caps.get(1).unwrap().as_str().parse().unwrap();
                let resolution = caps.get(2).unwrap().as_str().to_string();
                let codecs = caps.get(3).unwrap().as_str().to_string();
                return Ok(Some(Tag::ExtXStreamInf {
                    bandwidth,
                    resolution: Some(resolution),
                    codecs: Some(codecs),
                    frame_rate: None,
                    audio: None,
                    video: None,
                    subtitle: None,
                    closed_captions: None,
                }));
            }
        }

        if trimmed.starts_with("EXT-X-MEDIA") {
            // Example: #EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="audio",NAME="English",LANGUAGE="en",DEFAULT=YES,AUTOSELECT=YES,URI="audio_en.m3u8"
            let media_re = Regex::new(r#"EXT-X-MEDIA:TYPE=(\w+),GROUP-ID="([^"]+)",(?:NAME="([^"]+)")?,(?:LANGUAGE="([^"]+)")?,(?:DEFAULT=(YES|NO))?,(?:AUTOSELECT=(YES|NO))?,(?:URI="([^"]+)")?,(?:CHARACTERISTICS=([^,]+))?,(?:LANGUAGE-CODEC="([^"]+)")?,(?:INSTREAM-ID="([^"]+)")?,(?:FORCED=(YES|NO))?"#).unwrap();
            if let Some(caps) = media_re.captures(trimmed) {
                let type_ = caps.get(1).unwrap().as_str().to_string();
                let group_id = caps.get(2).unwrap().as_str().to_string();
                let name = Some(caps.get(3).unwrap().as_str().to_string());
                let language = Some(caps.get(4).unwrap().as_str().to_string());
                let default = Some(caps.get(5).unwrap().as_str() == "YES");
                let auto_select = Some(caps.get(6).unwrap().as_str() == "YES");
                let uri = Some(caps.get(7).unwrap().as_str().to_string());
                let instream_id = Some(caps.get(8).unwrap().as_str().to_string());
                let language_codec = Some(caps.get(9).unwrap().as_str().to_string());
                let characteristics = Some(caps.get(10).unwrap().as_str().to_string());
                let forced = Some(caps.get(11).unwrap().as_str() == "YES");

                return Ok(Some(Tag::ExtXMedia {
                    type_,
                    group_id,
                    name,
                    language,
                    instream_id,
                    language_codec,
                    default,
                    autoplay: auto_select,
                    characteristics,
                    uri,
                    forced,
                }));
            }
        }

        if trimmed.starts_with("EXT-X-RENDITION-REPORT") {
            // Example: #EXT-X-RENDITION-REPORT:URI="rendition_report.m3u8",BANDWIDTH=1000000
            let rendition_report_re =
                Regex::new(r#"EXT-X-RENDITION-REPORT:URI="([^"]+)",BANDWIDTH=(\d+)"#).unwrap();
            if let Some(caps) = rendition_report_re.captures(trimmed) {
                let uri = caps.get(1).unwrap().as_str().to_string();
                let bandwidth = caps.get(2).unwrap().as_str().parse().unwrap();
                return Ok(Some(Tag::ExtXRenditionReport { uri, bandwidth }));
            }
        }

        if trimmed.starts_with("EXT-X-BYTERANGE") {
            // Example: #EXT-X-BYTERANGE:500@1000
            let byte_range_re = Regex::new(r#"EXT-X-BYTERANGE:([^\s]+)"#).unwrap();
            if let Some(caps) = byte_range_re.captures(trimmed) {
                let byte_range = caps.get(1).unwrap().as_str().to_string();
                return Ok(Some(Tag::ExtXByteRange(byte_range)));
            }
        }

        if trimmed.starts_with("EXT-X-I-FRAME-STREAM-INF") {
            // Example: #EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=300000,URI="iframe.m3u8"
            let iframe_re =
                Regex::new(r#"EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=(\d+),URI="([^"]+)""#).unwrap();
            if let Some(caps) = iframe_re.captures(trimmed) {
                let bandwidth = caps.get(1).unwrap().as_str().parse().unwrap();
                let uri = caps.get(2).unwrap().as_str().to_string();
                return Ok(Some(Tag::ExtXIFrameStreamInf {
                    bandwidth,
                    codecs: None,
                    resolution: None,
                    frame_rate: None,
                    uri,
                }));
            }
        }

        if trimmed.starts_with("EXT-X-SESSION-DATA") {
            // Example: #EXT-X-SESSION-DATA:ID="session1",VALUE="value1",LANGUAGE="en"
            let session_data_re =
                Regex::new(r#"EXT-X-SESSION-DATA:ID="([^"]+)",VALUE="([^"]+)",LANGUAGE="([^"]+)""#)
                    .unwrap();
            if let Some(caps) = session_data_re.captures(trimmed) {
                let id = caps.get(1).unwrap().as_str().to_string();
                let value = caps.get(2).unwrap().as_str().to_string();
                let language = Some(caps.get(3).unwrap().as_str().to_string());
                return Ok(Some(Tag::ExtXSessionData {
                    id,
                    value,
                    language,
                }));
            }
        }

        if trimmed.starts_with("EXT-X-PRELOAD-HINT") {
            // Example: #EXT-X-PRELOAD-HINT:URI="preload_segment.ts",BYTERANGE="1000@2000"
            let preload_hint_re =
                Regex::new(r#"EXT-X-PRELOAD-HINT:URI="([^"]+)",BYTERANGE="([^"]+)""#).unwrap();
            if let Some(caps) = preload_hint_re.captures(trimmed) {
                let uri = caps.get(1).unwrap().as_str().to_string();
                let byterange = Some(caps.get(2).unwrap().as_str().to_string());
                return Ok(Some(Tag::ExtXPreloadHint { type_: None, uri, byterange }));
            }
        }

        if trimmed.starts_with("EXTINF") {
            // let split = trimmed.split("\n").collect::<Vec<_>>();
            //
            // let metadata_line = split.get(0).unwrap();
            // let segment = split.get(1).unwrap();

            let extinf_re = Regex::new(r#"EXTINF:(\d+(\.\d+)?),?\s*(.*?),?\s*(\S+)"#).unwrap();
            if let Some(caps) = extinf_re.captures(trimmed) {
                let duration: f32 = caps.get(1).unwrap().as_str().parse().unwrap();
                let title = caps
                    .get(3)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_else(|| "".to_string());
                let segment = caps.get(4).unwrap().as_str().trim().to_string();

                if title.is_empty() {
                    return Ok(Some(Tag::ExtInf(segment, duration, None)));
                }

                // Return parsed values wrapped in Tag::ExtInf
                return Ok(Some(Tag::ExtInf(segment, duration, Some(title))));
            }
        }

        if trimmed.starts_with("EXT-X-SESSION-KEY") {
            // Example: #EXT-X-SESSION-KEY:METHOD=AES-128,URI="https://example.com/session_key",IV="0x9876543210ABCDEF"
            let session_key_re =
                Regex::new(r#"EXT-X-SESSION-KEY:METHOD=([^,]+),URI="([^"]+)",IV="([^"]+)""#)
                    .unwrap();
            if let Some(caps) = session_key_re.captures(trimmed) {
                let method = caps.get(1).unwrap().as_str().to_string();
                let uri = Some(caps.get(2).unwrap().as_str().to_string());
                let iv = Some(caps.get(3).unwrap().as_str().to_string());
                return Ok(Some(Tag::ExtXSessionKey { method, uri, iv }));
            }
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
            Tag::ExtXMap { uri, .. } if uri.is_empty() => {
                errors.push(ValidationError::InvalidMapUri);
            }
            Tag::ExtXProgramDateTime(date_time) if date_time.is_empty() => {
                errors.push(ValidationError::InvalidProgramDateTime);
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
            Tag::ExtXSkip { skipped_segments, .. } if *skipped_segments == 0 => {
                errors.push(ValidationError::InvalidSkipTag(
                    "SKIPPED-SEGMENTS must be positive".to_string(),
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
