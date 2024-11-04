use crate::m3u8::playlist::Playlist;
use crate::m3u8::tags::Tag;
use crate::m3u8::validation::ValidationError;
use std::cell::RefCell;
use std::rc::Rc;

/// A builder for creating a `Playlist` with a chained interface.
#[derive(Clone)]
pub struct PlaylistBuilder {
    tags: Rc<RefCell<Vec<Tag>>>,
}

impl Default for PlaylistBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaylistBuilder {
    /// Creates a new `PlaylistBuilder`.
    pub fn new() -> Self {
        Self {
            tags: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Adds an `ExtM3U` tag.
    pub fn extm3u(self) -> Self {
        self.tags.borrow_mut().push(Tag::ExtM3U);
        self
    }

    /// Adds an `ExtXVersion` tag.
    pub fn version(self, version: u8) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXVersion(version));
        self
    }

    /// Adds an `ExtInf` tag.
    pub fn extinf(self, duration: f32, title: Option<String>) -> Self {
        self.tags.borrow_mut().push(Tag::ExtInf(duration, title));
        self
    }

    /// Adds an `ExtXTargetDuration` tag.
    pub fn target_duration(self, duration: u64) -> Self {
        self.tags
            .borrow_mut()
            .push(Tag::ExtXTargetDuration(duration));
        self
    }

    /// Adds an `ExtXMediaSequence` tag.
    pub fn media_sequence(self, sequence: u64) -> Self {
        self.tags
            .borrow_mut()
            .push(Tag::ExtXMediaSequence(sequence));
        self
    }

    /// Adds an `ExtXDiscontinuitySequence` tag.
    pub fn discontinuity_sequence(self, sequence: u32) -> Self {
        self.tags
            .borrow_mut()
            .push(Tag::ExtXDiscontinuitySequence(sequence));
        self
    }

    /// Adds an `ExtXEndList` tag.
    pub fn end_list(self) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXEndList);
        self
    }

    /// Adds an `ExtXKey` tag.
    pub fn key(
        self,
        method: &str,
        uri: Option<&str>,
        iv: Option<&str>,
        keyformat: Option<&str>,
        keyformatversions: Option<&str>,
    ) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXKey {
            method: method.to_string(),
            uri: uri.map(|s| s.to_string()),
            iv: iv.map(|s| s.to_string()),
            keyformat: keyformat.map(|s| s.to_string()),
            keyformatversions: keyformatversions.map(|s| s.to_string()),
        });
        self
    }

    /// Adds an `ExtXMap` tag.
    pub fn map(self, uri: &str, byterange: Option<&str>) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXMap {
            uri: uri.to_string(),
            byterange: byterange.map(|s| s.to_string()),
        });
        self
    }

    /// Adds an `ExtXProgramDateTime` tag.
    pub fn program_date_time(self, date_time: &str) -> Self {
        self.tags
            .borrow_mut()
            .push(Tag::ExtXProgramDateTime(date_time.to_string()));
        self
    }

    /// Adds an `ExtXDateRange` tag.
    #[allow(clippy::too_many_arguments)]
    pub fn date_range(
        self,
        id: &str,
        start_date: &str,
        end_date: Option<&str>,
        duration: Option<f32>,
        planned_duration: Option<f32>,
        scte35_cmd: Option<&str>,
        scte35_out: Option<&str>,
        scte35_in: Option<&str>,
        end_on_next: Option<bool>,
    ) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXDateRange {
            id: id.to_string(),
            start_date: start_date.to_string(),
            end_date: end_date.map(|s| s.to_string()),
            duration,
            planned_duration,
            scte35_cmd: scte35_cmd.map(|s| s.to_string()),
            scte35_out: scte35_out.map(|s| s.to_string()),
            scte35_in: scte35_in.map(|s| s.to_string()),
            end_on_next,
        });
        self
    }

    /// Adds a `Uri` tag.
    pub fn uri(self, uri: &str) -> Self {
        self.tags.borrow_mut().push(Tag::Uri(uri.to_string()));
        self
    }

    /// Adds an `ExtXGap` tag.
    pub fn gap(self) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXGap);
        self
    }

    /// Adds an `ExtXByteRange` tag.
    pub fn byte_range(self, byterange: &str) -> Self {
        self.tags
            .borrow_mut()
            .push(Tag::ExtXByteRange(byterange.to_string()));
        self
    }

    /// Adds an `ExtXDefine` tag.
    pub fn define(self, value: &str) -> Self {
        self.tags
            .borrow_mut()
            .push(Tag::ExtXDefine(value.to_string()));
        self
    }

    /// Adds an `ExtXMedia` tag.
    #[allow(clippy::too_many_arguments)]
    pub fn media(
        self,
        type_: &str,
        group_id: &str,
        name: Option<&str>,
        uri: Option<&str>,
        default: Option<bool>,
        autoplay: Option<bool>,
        characteristics: Option<&str>,
        language: Option<&str>,
    ) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXMedia {
            type_: type_.to_string(),
            group_id: group_id.to_string(),
            name: name.map(|s| s.to_string()),
            uri: uri.map(|s| s.to_string()),
            default,
            autoplay,
            characteristics: characteristics.map(|s| s.to_string()),
            language: language.map(|s| s.to_string()),
        });
        self
    }

    /// Adds an `ExtXStreamInf` tag.
    #[allow(clippy::too_many_arguments)]
    pub fn stream_inf(
        self,
        bandwidth: u32,
        codecs: Option<&str>,
        resolution: Option<&str>,
        frame_rate: Option<f32>,
        audio: Option<&str>,
        video: Option<&str>,
        subtitle: Option<&str>,
        closed_captions: Option<&str>,
    ) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXStreamInf {
            bandwidth,
            codecs: codecs.map(|s| s.to_string()),
            resolution: resolution.map(|s| s.to_string()),
            frame_rate,
            audio: audio.map(|s| s.to_string()),
            video: video.map(|s| s.to_string()),
            subtitle: subtitle.map(|s| s.to_string()),
            closed_captions: closed_captions.map(|s| s.to_string()),
        });
        self
    }

    /// Adds an `ExtXIFrameStreamInf` tag.
    pub fn iframe_stream_inf(
        self,
        bandwidth: u32,
        codecs: Option<&str>,
        resolution: Option<&str>,
        frame_rate: Option<f32>,
        uri: &str,
    ) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXIFrameStreamInf {
            bandwidth,
            codecs: codecs.map(|s| s.to_string()),
            resolution: resolution.map(|s| s.to_string()),
            frame_rate,
            uri: uri.to_string(),
        });
        self
    }

    /// Adds an `ExtXBitrate` tag.
    pub fn bitrate(self, bitrate: u32) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXBitrate(bitrate));
        self
    }

    /// Adds an `ExtXIndependentSegments` tag.
    pub fn independent_segments(self) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXIndependentSegments);
        self
    }

    /// Adds an `ExtXStart` tag.
    pub fn start(self, time_offset: &str, precise: Option<bool>) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXStart {
            time_offset: time_offset.to_string(),
            precise,
        });
        self
    }

    /// Adds an `ExtXSessionData` tag.
    pub fn session_data(self, id: &str, value: &str, language: Option<&str>) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXSessionData {
            id: id.to_string(),
            value: value.to_string(),
            language: language.map(|s| s.to_string()),
        });
        self
    }

    /// Adds an `ExtXSessionKey` tag.
    pub fn session_key(self, method: &str, uri: Option<&str>, iv: Option<&str>) -> Self {
        self.tags.borrow_mut().push(Tag::ExtXSessionKey {
            method: method.to_string(),
            uri: uri.map(|s| s.to_string()),
            iv: iv.map(|s| s.to_string()),
        });
        self
    }

    /// Constructs the final `Playlist` and validates it.
    pub fn build(self) -> Result<Playlist, Vec<ValidationError>> {
        let playlist = Playlist {
            tags: self.tags.borrow().clone(),
        };
        match playlist.validate() {
            Ok(_) => Ok(playlist),
            Err(errors) => Err(errors),
        }
    }

    /// Adds an `ExtXPlaylistType` tag.
    pub fn playlist_type(self, playlist_type: &str) -> Self {
        self.tags
            .borrow_mut()
            .push(Tag::ExtXPlaylistType(playlist_type.to_string()));
        self
    }
}
