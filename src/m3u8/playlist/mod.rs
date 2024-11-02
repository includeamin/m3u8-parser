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
use std::io;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Represents a playlist containing multiple tags.
#[derive(Debug, PartialEq)]
pub struct Playlist {
    pub tags: Vec<Tag>,
}

impl Playlist {
    /// Creates a new `Playlist` by reading tags from a buffered reader.
    ///
    /// # Arguments
    ///
    /// * `reader` - A buffered reader providing lines of the playlist.
    ///
    /// # Returns
    ///
    /// A result containing a `Playlist` or an error message as a string.
    pub fn from_reader<R: BufRead>(reader: R) -> Result<Self, String> {
        let mut tags = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?;
            if line.is_empty() {
                continue;
            }
            if line.starts_with("#EXTM3U") {
                tags.push(Tag::ExtM3U);
            } else if let Some(stripped) = line.strip_prefix("#EXT-X-VERSION:") {
                let version = stripped.parse().unwrap();
                tags.push(Tag::ExtXVersion(version));
            } else if let Some(stripped) = line.strip_prefix("#EXTINF:") {
                let parts: Vec<&str> = stripped.splitn(2, ',').collect();
                let duration = parts[0].parse().unwrap();
                let title = if parts.len() > 1 && !parts[1].to_string().is_empty() {
                    Some(parts[1].to_string())
                } else {
                    None
                };
                tags.push(Tag::ExtInf(duration, title));
            } else if let Some(stripped) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
                let duration = stripped.parse().unwrap();
                tags.push(Tag::ExtXTargetDuration(duration));
            } else if let Some(stripped) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
                let sequence = stripped.parse().unwrap();
                tags.push(Tag::ExtXMediaSequence(sequence));
            } else if let Some(stripped) = line.strip_prefix("#EXT-X-DISCONTINUITY-SEQUENCE:") {
                let sequence = stripped.parse().unwrap();
                tags.push(Tag::ExtXDiscontinuitySequence(sequence));
            } else if line.starts_with("#EXT-X-ENDLIST") {
                tags.push(Tag::ExtXEndList);
            } else if let Some(stripped) = line.strip_prefix("#EXT-X-KEY:") {
                let attributes = parse_attributes(stripped)?;
                tags.push(Tag::ExtXKey {
                    method: attributes
                        .get("METHOD")
                        .ok_or("Missing METHOD attribute")?
                        .clone(),
                    uri: attributes.get("URI").cloned(),
                    iv: attributes.get("IV").cloned(),
                    keyformat: attributes.get("KEYFORMAT").cloned(),
                    keyformatversions: attributes.get("KEYFORMATVERSIONS").cloned(),
                });
            } else if let Some(stripped) = line.strip_prefix("#EXT-X-MAP:") {
                let attributes = parse_attributes(stripped)?;
                tags.push(Tag::ExtXMap {
                    uri: attributes
                        .get("URI")
                        .ok_or("Missing URI attribute")?
                        .clone(),
                    byterange: attributes.get("BYTERANGE").cloned(),
                });
            } else if let Some(stripped) = line.strip_prefix("#EXT-X-PROGRAM-DATE-TIME:") {
                tags.push(Tag::ExtXProgramDateTime(stripped.to_string()));
            } else if let Some(stripped) = line.strip_prefix("#EXT-X-DATERANGE:") {
                let attributes = parse_attributes(stripped)?;
                tags.push(Tag::ExtXDateRange {
                    id: attributes.get("ID").ok_or("Missing ID attribute")?.clone(),
                    start_date: attributes
                        .get("START-DATE")
                        .ok_or("Missing START-DATE attribute")?
                        .clone(),
                    end_date: attributes.get("END-DATE").cloned(),
                    duration: attributes
                        .get("DURATION")
                        .map(|s| s.parse::<f32>().unwrap()),
                    planned_duration: attributes
                        .get("PLANNED-DURATION")
                        .map(|s| s.parse().unwrap()),
                    scte35_cmd: attributes.get("SCTE35-CMD").cloned(),
                    scte35_out: attributes.get("SCTE35-OUT").cloned(),
                    scte35_in: attributes.get("SCTE35-IN").cloned(),
                    end_on_next: attributes.get("END-ON-NEXT").map(|s| s == "YES"),
                });
            } else if !line.starts_with('#') {
                tags.push(Tag::Uri(line));
            }
        }
        Ok(Playlist { tags })
    }

    /// Creates a new `Playlist` by reading tags from a file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file containing the playlist.
    ///
    /// # Returns
    ///
    /// A result containing a `Playlist` or an error message as a string.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }

    /// Writes the playlist to a file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path where the playlist should be saved.
    ///
    /// # Returns
    ///
    /// A result indicating success or an error if the write fails.
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = File::create(path)?;
        for tag in &self.tags {
            writeln!(file, "{}", tag)?;
        }
        Ok(())
    }

    /// Validates the playlist according to RFC 8216.
    ///
    /// # Returns
    ///
    /// A result indicating success or a list of validation errors.
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Ensure the playlist starts with #EXTM3U
        match self.tags.first() {
            Some(Tag::ExtM3U) => {}
            _ => errors.push(ValidationError::MissingExtM3U),
        }

        // Validate each tag according to its rules
        for tag in &self.tags {
            match tag {
                Tag::ExtXVersion(version) => {
                    if *version < 1 || *version > 7 {
                        errors.push(ValidationError::InvalidVersion(*version));
                    }
                }
                Tag::ExtInf(duration, _) => {
                    if *duration <= 0.0 {
                        errors.push(ValidationError::InvalidDuration(*duration));
                    }
                }
                Tag::ExtXTargetDuration(duration) => {
                    if *duration == 0 {
                        errors.push(ValidationError::InvalidTargetDuration(*duration));
                    }
                }
                Tag::ExtXMediaSequence(sequence) => {
                    if *sequence == 0 {
                        errors.push(ValidationError::InvalidMediaSequence(*sequence));
                    }
                }
                Tag::ExtXKey { method, .. } => {
                    if method != "NONE" && method != "AES-128" && method != "SAMPLE-AES" {
                        errors.push(ValidationError::InvalidKeyMethod(method.clone()));
                    }
                }
                Tag::ExtXMap { uri, .. } => {
                    if uri.is_empty() {
                        errors.push(ValidationError::InvalidMapUri);
                    }
                }
                Tag::ExtXProgramDateTime(date_time) => {
                    if date_time.is_empty() {
                        errors.push(ValidationError::InvalidProgramDateTime);
                    }
                }
                Tag::ExtXDateRange {
                    id,
                    start_date,
                    end_date,
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
                    if let Some(end_date) = end_date {
                        if end_date < start_date {
                            errors.push(ValidationError::InvalidDateRangeEndDate);
                        }
                    }
                    if let Some(duration) = duration {
                        if *duration < 0.0 {
                            errors.push(ValidationError::InvalidDateRangeDuration(*duration));
                        }
                    }
                    if let Some(planned_duration) = planned_duration {
                        if *planned_duration < 0.0 {
                            errors.push(ValidationError::InvalidDateRangePlannedDuration(
                                *planned_duration,
                            ));
                        }
                    }
                }
                _ => {}
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
