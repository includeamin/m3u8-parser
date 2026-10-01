use crate::m3u8::playlist::Playlist;
use crate::m3u8::tags::{ByteRange, Key, Map, Tag};
use crate::m3u8::validation::ValidationError;

/// A builder for creating a `Playlist` with a chained interface.
///
/// Tags are emitted in call order. Use [`PlaylistBuilder::tag`] for any tag or
/// attribute combination that has no dedicated method.
#[derive(Debug, Clone, Default)]
pub struct PlaylistBuilder {
    tags: Vec<Tag>,
}

impl PlaylistBuilder {
    /// Creates a new `PlaylistBuilder`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds any tag.
    pub fn tag(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self
    }

    /// Adds a URI line, such as the variant URI that follows `stream_inf`.
    pub fn uri(self, uri: &str) -> Self {
        self.tag(Tag::Uri(uri.to_string()))
    }

    /// Adds an `ExtXDiscontinuity` tag.
    pub fn discontinuity(self) -> Self {
        self.tag(Tag::ExtXDiscontinuity)
    }

    /// Adds an `ExtXIFramesOnly` tag.
    pub fn i_frames_only(self) -> Self {
        self.tag(Tag::ExtXIFramesOnly)
    }

    /// Adds an `ExtXDateRange` tag with only its required attributes.
    pub fn date_range(self, id: &str, start_date: &str) -> Self {
        self.tag(Tag::ExtXDateRange {
            id: id.to_string(),
            class: None,
            start_date: start_date.to_string(),
            cue: None,
            end_date: None,
            duration: None,
            planned_duration: None,
            end_on_next: None,
            scte35_cmd: None,
            scte35_out: None,
            scte35_in: None,
            extra_attributes: Vec::new(),
        })
    }

    /// Adds an `ExtXServerControl` tag.
    pub fn server_control(
        self,
        can_skip_until: Option<f64>,
        part_hold_back: Option<f64>,
        can_block_reload: bool,
    ) -> Self {
        self.tag(Tag::ExtXServerControl {
            can_skip_until,
            can_skip_dateranges: None,
            hold_back: None,
            part_hold_back,
            can_block_reload: can_block_reload.then_some(true),
        })
    }

    /// Adds an `ExtXPartInf` tag.
    pub fn part_inf(self, part_target_duration: f64) -> Self {
        self.tag(Tag::ExtXPartInf {
            part_target_duration,
        })
    }

    /// Adds an `ExtXPart` tag.
    pub fn part(self, uri: &str, duration: f64) -> Self {
        self.tag(Tag::ExtXPart {
            uri: uri.to_string(),
            duration,
            independent: None,
            byterange: None,
            gap: None,
        })
    }

    /// Adds an `ExtXPreloadHint` tag; `type_` is `PART` or `MAP`.
    pub fn preload_hint(self, type_: &str, uri: &str) -> Self {
        self.tag(Tag::ExtXPreloadHint {
            type_: type_.to_string(),
            uri: uri.to_string(),
            byterange_start: None,
            byterange_length: None,
        })
    }

    /// Adds an `ExtXRenditionReport` tag.
    pub fn rendition_report(
        self,
        uri: &str,
        last_msn: Option<u64>,
        last_part: Option<u64>,
    ) -> Self {
        self.tag(Tag::ExtXRenditionReport {
            uri: uri.to_string(),
            last_msn,
            last_part,
        })
    }

    /// Adds an `ExtXSkip` tag.
    pub fn skip(self, skipped_segments: u32) -> Self {
        self.tag(Tag::ExtXSkip {
            skipped_segments,
            recently_removed_dateranges: None,
        })
    }

    /// Adds an `ExtM3U` tag.
    pub fn extm3u(self) -> Self {
        self.tag(Tag::ExtM3U)
    }

    /// Adds an `ExtXVersion` tag.
    pub fn version(self, version: u8) -> Self {
        self.tag(Tag::ExtXVersion(version))
    }

    /// Adds an `ExtInf` tag.
    pub fn extinf(self, url: &str, duration: f64, title: Option<String>) -> Self {
        self.tag(Tag::ExtInf {
            uri: url.to_string(),
            duration,
            title,
        })
    }

    /// Adds an `ExtXTargetDuration` tag.
    pub fn target_duration(self, duration: u64) -> Self {
        self.tag(Tag::ExtXTargetDuration(duration))
    }

    /// Adds an `ExtXMediaSequence` tag.
    pub fn media_sequence(self, sequence: u64) -> Self {
        self.tag(Tag::ExtXMediaSequence(sequence))
    }

    /// Adds an `ExtXDiscontinuitySequence` tag.
    pub fn discontinuity_sequence(self, sequence: u32) -> Self {
        self.tag(Tag::ExtXDiscontinuitySequence(sequence))
    }

    /// Adds an `ExtXEndList` tag.
    pub fn end_list(self) -> Self {
        self.tag(Tag::ExtXEndList)
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
        self.tag(Tag::ExtXKey(Key {
            method: method.to_string(),
            uri: uri.map(|s| s.to_string()),
            iv: iv.map(|s| s.to_string()),
            keyformat: keyformat.map(|s| s.to_string()),
            keyformatversions: keyformatversions.map(|s| s.to_string()),
        }))
    }

    /// Adds an `ExtXMap` tag.
    pub fn map(self, uri: &str, byterange: Option<ByteRange>) -> Self {
        self.tag(Tag::ExtXMap(Map {
            uri: uri.to_string(),
            byterange,
        }))
    }

    /// Adds an `ExtXProgramDateTime` tag.
    pub fn program_date_time(self, date_time: &str) -> Self {
        self.tag(Tag::ExtXProgramDateTime(date_time.to_string()))
    }

    /// Adds an `ExtXGap` tag.
    pub fn gap(self) -> Self {
        self.tag(Tag::ExtXGap)
    }

    /// Adds an `ExtXByteRange` tag.
    pub fn byte_range(self, byterange: ByteRange) -> Self {
        self.tag(Tag::ExtXByteRange(byterange))
    }

    /// Adds an `ExtXDefine` tag.
    pub fn define(self, value: &str) -> Self {
        self.tag(Tag::ExtXDefine(value.to_string()))
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
        autoselect: Option<bool>,
        characteristics: Option<&str>,
        language: Option<&str>,
        forced: Option<bool>,
        language_codec: Option<&str>,
        instream_id: Option<&str>,
    ) -> Self {
        self.tag(Tag::ExtXMedia {
            type_: type_.to_string(),
            group_id: group_id.to_string(),
            name: name.map(|s| s.to_string()),
            uri: uri.map(|s| s.to_string()),
            default,
            autoselect,
            characteristics: characteristics.map(|s| s.to_string()),
            language: language.map(|s| s.to_string()),
            instream_id: instream_id.map(|s| s.to_string()),
            language_codec: language_codec.map(|s| s.to_string()),
            forced,
            channels: None,
        })
    }

    /// Adds an `ExtXStreamInf` tag.
    #[allow(clippy::too_many_arguments)]
    pub fn stream_inf(
        self,
        bandwidth: u32,
        average_bandwidth: Option<u32>,
        codecs: Option<&str>,
        resolution: Option<&str>,
        frame_rate: Option<f64>,
        audio: Option<&str>,
        video: Option<&str>,
        subtitle: Option<&str>,
        closed_captions: Option<&str>,
    ) -> Self {
        self.tag(Tag::ExtXStreamInf {
            bandwidth,
            average_bandwidth,
            codecs: codecs.map(|s| s.to_string()),
            resolution: resolution.map(|s| s.to_string()),
            frame_rate,
            audio: audio.map(|s| s.to_string()),
            video: video.map(|s| s.to_string()),
            subtitle: subtitle.map(|s| s.to_string()),
            closed_captions: closed_captions.map(|s| s.to_string()),
        })
    }

    /// Adds an `ExtXIFrameStreamInf` tag.
    #[allow(clippy::too_many_arguments)]
    pub fn iframe_stream_inf(
        self,
        bandwidth: u32,
        average_bandwidth: Option<u32>,
        codecs: Option<&str>,
        resolution: Option<&str>,
        frame_rate: Option<f64>,
        uri: &str,
    ) -> Self {
        self.tag(Tag::ExtXIFrameStreamInf {
            bandwidth,
            average_bandwidth,
            codecs: codecs.map(|s| s.to_string()),
            resolution: resolution.map(|s| s.to_string()),
            frame_rate,
            uri: uri.to_string(),
        })
    }

    /// Adds an `ExtXBitrate` tag.
    pub fn bitrate(self, bitrate: u32) -> Self {
        self.tag(Tag::ExtXBitrate(bitrate))
    }

    /// Adds an `ExtXIndependentSegments` tag.
    pub fn independent_segments(self) -> Self {
        self.tag(Tag::ExtXIndependentSegments)
    }

    /// Adds an `ExtXStart` tag.
    pub fn start(self, time_offset: f64, precise: Option<bool>) -> Self {
        self.tag(Tag::ExtXStart {
            time_offset,
            precise,
        })
    }

    /// Adds an `ExtXSessionData` tag.
    pub fn session_data(self, id: &str, value: &str, language: Option<&str>) -> Self {
        self.tag(Tag::ExtXSessionData {
            id: id.to_string(),
            value: Some(value.to_string()),
            uri: None,
            language: language.map(|s| s.to_string()),
        })
    }

    /// Adds an `ExtXSessionKey` tag.
    pub fn session_key(self, method: &str, uri: Option<&str>, iv: Option<&str>) -> Self {
        self.tag(Tag::ExtXSessionKey(Key {
            method: method.to_string(),
            uri: uri.map(|s| s.to_string()),
            iv: iv.map(|s| s.to_string()),
            keyformat: None,
            keyformatversions: None,
        }))
    }

    /// Constructs the final `Playlist` and validates it.
    pub fn build(self) -> Result<Playlist, Vec<ValidationError>> {
        let playlist = Playlist { tags: self.tags };
        playlist.validate()?;
        Ok(playlist)
    }

    /// Adds an `ExtXPlaylistType` tag.
    pub fn playlist_type(self, playlist_type: &str) -> Self {
        self.tag(Tag::ExtXPlaylistType(playlist_type.to_string()))
    }
}
