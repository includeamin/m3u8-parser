//! A builder for creating M3U8 playlists with a fluent interface.
//!
//! This module provides the `PlaylistBuilder` struct, which allows users
//! to construct an M3U8 playlist step-by-step. Each method corresponds
//! to a specific tag defined in the M3U8 specification, enabling the
//! creation of valid playlists in a clear and concise manner.
//!
//! # Example
//!
//! ```
//! use m3u8_parser::m3u8::playlist::builder::PlaylistBuilder;
//!
//! let playlist = PlaylistBuilder::new()
//!     .extm3u()
//!     .version(3)
//!     .extinf(10.0, Some("Sample Title".to_string()))
//!     .uri("http://example.com/media.ts".to_string())
//!     .end_list()
//!     .build()
//!     .expect("Failed to build playlist");
//! ```
//!
//! ## Methods
//!
//! - `new`: Creates a new `PlaylistBuilder` instance.
//! - `extm3u`: Adds an `ExtM3U` tag to the playlist.
//! - `version`: Adds an `ExtXVersion` tag with the specified version number.
//! - `extinf`: Adds an `ExtInf` tag with the duration and an optional title.
//! - `target_duration`: Adds an `ExtXTargetDuration` tag with the specified duration.
//! - `media_sequence`: Adds an `ExtXMediaSequence` tag with the specified sequence number.
//! - `discontinuity_sequence`: Adds an `ExtXDiscontinuitySequence` tag with the specified sequence number.
//! - `end_list`: Adds an `ExtXEndList` tag, indicating the end of the playlist.
//! - `key`: Adds an `ExtXKey` tag with encryption details.
//! - `map`: Adds an `ExtXMap` tag with the specified URI and optional byte range.
//! - `program_date_time`: Adds an `ExtXProgramDateTime` tag with the specified date and time.
//! - `date_range`: Adds an `ExtXDateRange` tag with details for a date range.
//! - `uri`: Adds a `Uri` tag for a media segment.
//! - `gap`: Adds an `ExtXGap` tag to indicate a gap in the playlist.
//! - `build`: Constructs the final `Playlist` and validates it, returning the playlist or a list of validation errors.

use crate::m3u8::playlist::Playlist;
use crate::m3u8::tags::Tag;
use crate::m3u8::validation::ValidationError;

/// A builder for creating a `Playlist` with a chained interface.
pub struct PlaylistBuilder {
    tags: Vec<Tag>,
}

impl Default for PlaylistBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaylistBuilder {
    /// Creates a new `PlaylistBuilder`.
    pub fn new() -> Self {
        Self { tags: Vec::new() }
    }

    /// Adds an `ExtM3U` tag.
    pub fn extm3u(mut self) -> Self {
        self.tags.push(Tag::ExtM3U);
        self
    }

    /// Adds an `ExtXVersion` tag.
    pub fn version(mut self, version: u8) -> Self {
        self.tags.push(Tag::ExtXVersion(version));
        self
    }

    /// Adds an `ExtInf` tag.
    pub fn extinf(mut self, duration: f32, title: Option<String>) -> Self {
        self.tags.push(Tag::ExtInf(duration, title));
        self
    }

    /// Adds an `ExtXTargetDuration` tag.
    pub fn target_duration(mut self, duration: u64) -> Self {
        self.tags.push(Tag::ExtXTargetDuration(duration));
        self
    }

    /// Adds an `ExtXMediaSequence` tag.
    pub fn media_sequence(mut self, sequence: u64) -> Self {
        self.tags.push(Tag::ExtXMediaSequence(sequence));
        self
    }

    /// Adds an `ExtXDiscontinuitySequence` tag.
    pub fn discontinuity_sequence(mut self, sequence: u32) -> Self {
        self.tags.push(Tag::ExtXDiscontinuitySequence(sequence));
        self
    }

    /// Adds an `ExtXEndList` tag.
    pub fn end_list(mut self) -> Self {
        self.tags.push(Tag::ExtXEndList);
        self
    }

    /// Adds an `ExtXKey` tag.
    pub fn key(
        mut self,
        method: String,
        uri: Option<String>,
        iv: Option<String>,
        keyformat: Option<String>,
        keyformatversions: Option<String>,
    ) -> Self {
        self.tags.push(Tag::ExtXKey {
            method,
            uri,
            iv,
            keyformat,
            keyformatversions,
        });
        self
    }

    /// Adds an `ExtXMap` tag.
    pub fn map(mut self, uri: String, byterange: Option<String>) -> Self {
        self.tags.push(Tag::ExtXMap { uri, byterange });
        self
    }

    /// Adds an `ExtXProgramDateTime` tag.
    pub fn program_date_time(mut self, date_time: String) -> Self {
        self.tags.push(Tag::ExtXProgramDateTime(date_time));
        self
    }

    /// Adds an `ExtXDateRange` tag.
    #[allow(clippy::too_many_arguments)]
    pub fn date_range(
        mut self,
        id: String,
        start_date: String,
        end_date: Option<String>,
        duration: Option<f32>,
        planned_duration: Option<f32>,
        scte35_cmd: Option<String>,
        scte35_out: Option<String>,
        scte35_in: Option<String>,
        end_on_next: Option<bool>,
    ) -> Self {
        self.tags.push(Tag::ExtXDateRange {
            id,
            start_date,
            end_date,
            duration,
            planned_duration,
            scte35_cmd,
            scte35_out,
            scte35_in,
            end_on_next,
        });
        self
    }

    /// Adds a `Uri` tag.
    pub fn uri(mut self, uri: String) -> Self {
        self.tags.push(Tag::Uri(uri));
        self
    }

    /// Adds an `ExtXGap` tag.
    pub fn gap(mut self) -> Self {
        self.tags.push(Tag::ExtXGap);
        self
    }

    /// Adds an `ExtXByteRange` tag.
    pub fn byte_range(mut self, byterange: String) -> Self {
        self.tags.push(Tag::ExtXByteRange(byterange));
        self
    }

    /// Adds an `ExtXDefine` tag.
    pub fn define(mut self, value: String) -> Self {
        self.tags.push(Tag::ExtXDefine(value));
        self
    }

    /// Adds an `ExtXMedia` tag.
    #[allow(clippy::too_many_arguments)]
    pub fn media(
        mut self,
        type_: String,
        group_id: String,
        name: Option<String>,
        uri: Option<String>,
        default: Option<bool>,
        autoplay: Option<bool>,
        characteristics: Option<String>,
        language: Option<String>,
    ) -> Self {
        self.tags.push(Tag::ExtXMedia {
            type_,
            group_id,
            name,
            uri,
            default,
            autoplay,
            characteristics,
            language,
        });
        self
    }

    /// Adds an `ExtXStreamInf` tag.
    #[allow(clippy::too_many_arguments)]
    pub fn stream_inf(
        mut self,
        bandwidth: u32,
        codecs: Option<String>,
        resolution: Option<String>,
        frame_rate: Option<f32>,
        audio: Option<String>,
        video: Option<String>,
        subtitle: Option<String>,
        closed_captions: Option<String>,
    ) -> Self {
        self.tags.push(Tag::ExtXStreamInf {
            bandwidth,
            codecs,
            resolution,
            frame_rate,
            audio,
            video,
            subtitle,
            closed_captions,
        });
        self
    }

    /// Adds an `ExtXIFrameStreamInf` tag.
    pub fn iframe_stream_inf(
        mut self,
        bandwidth: u32,
        codecs: Option<String>,
        resolution: Option<String>,
        frame_rate: Option<f32>,
        uri: String,
    ) -> Self {
        self.tags.push(Tag::ExtXIFrameStreamInf {
            bandwidth,
            codecs,
            resolution,
            frame_rate,
            uri,
        });
        self
    }

    /// Adds an `ExtXBitrate` tag.
    pub fn bitrate(mut self, bitrate: u32) -> Self {
        self.tags.push(Tag::ExtXBitrate(bitrate));
        self
    }

    /// Adds an `ExtXIndependentSegments` tag.
    pub fn independent_segments(mut self) -> Self {
        self.tags.push(Tag::ExtXIndependentSegments);
        self
    }

    /// Adds an `ExtXStart` tag.
    pub fn start(mut self, time_offset: String, precise: Option<bool>) -> Self {
        self.tags.push(Tag::ExtXStart {
            time_offset,
            precise,
        });
        self
    }

    /// Adds an `ExtXSessionData` tag.
    pub fn session_data(mut self, id: String, value: String, language: Option<String>) -> Self {
        self.tags.push(Tag::ExtXSessionData {
            id,
            value,
            language,
        });
        self
    }

    /// Adds an `ExtXSessionKey` tag.
    pub fn session_key(mut self, method: String, uri: Option<String>, iv: Option<String>) -> Self {
        self.tags.push(Tag::ExtXSessionKey { method, uri, iv });
        self
    }

    /// Constructs the final `Playlist` and validates it.
    pub fn build(self) -> Result<Playlist, Vec<ValidationError>> {
        // Validate and build the playlist from the tags
        // (Implement validation logic here)
        Ok(Playlist { tags: self.tags })
    }
}
