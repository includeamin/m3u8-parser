//! Read-only views over a playlist: total duration, variants and renditions.

use super::Playlist;
use crate::m3u8::tags::{Codecs, Resolution, Tag};

/// A variant stream of a Multivariant Playlist: an EXT-X-STREAM-INF tag and the
/// URI line that follows it.
#[derive(Debug, PartialEq, Clone)]
pub struct Variant {
    pub uri: String,
    pub bandwidth: u32,
    pub average_bandwidth: Option<u32>,
    pub codecs: Option<Codecs>,
    pub resolution: Option<Resolution>,
    pub frame_rate: Option<f64>,
    /// The `AUDIO` rendition group.
    pub audio: Option<String>,
    /// The `VIDEO` rendition group.
    pub video: Option<String>,
    /// The `SUBTITLES` rendition group.
    pub subtitles: Option<String>,
    /// The `CLOSED-CAPTIONS` rendition group, or `NONE`.
    pub closed_captions: Option<String>,
    pub pathway_id: Option<String>,
}

/// An alternative rendition declared by an EXT-X-MEDIA tag.
#[derive(Debug, PartialEq, Clone)]
pub struct Rendition {
    /// `AUDIO`, `VIDEO`, `SUBTITLES`, or `CLOSED-CAPTIONS`.
    pub type_: String,
    pub group_id: String,
    pub name: Option<String>,
    pub uri: Option<String>,
    pub language: Option<String>,
    pub default: bool,
    pub autoselect: bool,
    pub forced: bool,
    pub characteristics: Option<String>,
    pub instream_id: Option<String>,
    pub channels: Option<String>,
    pub pathway_id: Option<String>,
}

impl Playlist {
    /// The sum of the `EXTINF` durations in seconds, or `0.0` when there are none.
    ///
    /// Segments marked with `EXT-X-GAP` still count, since they occupy time.
    pub fn total_duration(&self) -> f64 {
        self.tags
            .iter()
            .map(|tag| match tag {
                Tag::ExtInf { duration, .. } => *duration,
                _ => 0.0,
            })
            .sum()
    }

    /// The variant streams in playlist order. An EXT-X-STREAM-INF tag that is
    /// not followed by a URI line is skipped; validation reports it.
    pub fn variants(&self) -> Vec<Variant> {
        let mut variants = Vec::new();
        for pair in self.tags.windows(2) {
            if let (
                Tag::ExtXStreamInf {
                    bandwidth,
                    average_bandwidth,
                    codecs,
                    resolution,
                    frame_rate,
                    audio,
                    video,
                    subtitle,
                    closed_captions,
                    pathway_id,
                    ..
                },
                Tag::Uri(uri),
            ) = (&pair[0], &pair[1])
            {
                variants.push(Variant {
                    uri: uri.clone(),
                    bandwidth: *bandwidth,
                    average_bandwidth: *average_bandwidth,
                    codecs: codecs.clone(),
                    resolution: *resolution,
                    frame_rate: *frame_rate,
                    audio: audio.clone(),
                    video: video.clone(),
                    subtitles: subtitle.clone(),
                    closed_captions: closed_captions.clone(),
                    pathway_id: pathway_id.clone(),
                });
            }
        }
        variants
    }

    /// The variant streams from the lowest to the highest `BANDWIDTH`. Variants
    /// with equal bandwidth keep their playlist order.
    pub fn variants_by_bandwidth(&self) -> Vec<Variant> {
        let mut variants = self.variants();
        variants.sort_by_key(|variant| variant.bandwidth);
        variants
    }

    /// All EXT-X-MEDIA renditions in playlist order.
    pub fn renditions(&self) -> Vec<Rendition> {
        self.tags
            .iter()
            .filter_map(|tag| match tag {
                Tag::ExtXMedia {
                    type_,
                    group_id,
                    name,
                    uri,
                    default,
                    autoselect,
                    characteristics,
                    language,
                    instream_id,
                    forced,
                    channels,
                    pathway_id,
                    ..
                } => Some(Rendition {
                    type_: type_.clone(),
                    group_id: group_id.clone(),
                    name: name.clone(),
                    uri: uri.clone(),
                    language: language.clone(),
                    default: default.unwrap_or(false),
                    autoselect: autoselect.unwrap_or(false),
                    forced: forced.unwrap_or(false),
                    characteristics: characteristics.clone(),
                    instream_id: instream_id.clone(),
                    channels: channels.clone(),
                    pathway_id: pathway_id.clone(),
                }),
                _ => None,
            })
            .collect()
    }

    /// The renditions of one group, for example the ones a variant's
    /// [`audio`](Variant::audio) group refers to.
    pub fn renditions_in_group(&self, type_: &str, group_id: &str) -> Vec<Rendition> {
        let mut renditions = self.renditions();
        renditions.retain(|rendition| rendition.type_ == type_ && rendition.group_id == group_id);
        renditions
    }
}
