//! Partial segments (draft-pantos-hls-rfc8216bis section 4.4.4.9).

use super::Playlist;
use crate::m3u8::tags::{ByteRange, Tag};

/// A Partial Segment declared by an EXT-X-PART tag.
#[derive(Debug, PartialEq, Clone)]
pub struct Part {
    pub uri: String,
    /// Duration in seconds.
    pub duration: f64,
    /// Whether the part can be decoded without any earlier part.
    pub independent: bool,
    pub byterange: Option<ByteRange>,
    pub gap: bool,
}

impl Part {
    pub(super) fn from_tag(tag: &Tag) -> Option<Part> {
        match tag {
            Tag::ExtXPart {
                uri,
                duration,
                independent,
                byterange,
                gap,
            } => Some(Part {
                uri: uri.clone(),
                duration: *duration,
                independent: independent.unwrap_or(false),
                byterange: *byterange,
                gap: gap.unwrap_or(false),
            }),
            _ => None,
        }
    }
}

/// The position of a part in a playlist: the Media Sequence Number of its
/// segment and its zero-based index within that segment.
///
/// This is what a client names in the `_HLS_msn` and `_HLS_part` parameters of
/// a Blocking Playlist Reload, and what EXT-X-RENDITION-REPORT carries as
/// `LAST-MSN` and `LAST-PART`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct PartPosition {
    pub sequence: u64,
    pub part: u64,
}

impl Playlist {
    /// The parts of the segment that is still being produced: the EXT-X-PART
    /// tags after the last `EXTINF`, which have no complete segment yet.
    ///
    /// Parts of completed segments are in [`MediaSegment::parts`](super::MediaSegment::parts).
    pub fn pending_parts(&self) -> Vec<Part> {
        let start = self
            .tags
            .iter()
            .rposition(|tag| matches!(tag, Tag::ExtInf { .. }))
            .map_or(0, |index| index + 1);
        self.tags[start..]
            .iter()
            .filter_map(Part::from_tag)
            .collect()
    }

    /// The Media Sequence Number of the segment that follows the last
    /// `EXTINF`, which the [`pending_parts`](Playlist::pending_parts) belong to.
    pub fn pending_segment_sequence(&self) -> u64 {
        let segments = self.media_segments();
        match segments.last() {
            Some(last) => last.sequence + 1,
            None => self.first_sequence(),
        }
    }

    /// The position of the newest part, whether it belongs to the segment in
    /// progress or to the last completed segment. `None` when the playlist has
    /// no parts.
    pub fn latest_part_position(&self) -> Option<PartPosition> {
        let pending = self.pending_parts();
        if !pending.is_empty() {
            return Some(PartPosition {
                sequence: self.pending_segment_sequence(),
                part: pending.len() as u64 - 1,
            });
        }
        let segments = self.media_segments();
        let last = segments.last()?;
        (!last.parts.is_empty()).then(|| PartPosition {
            sequence: last.sequence,
            part: last.parts.len() as u64 - 1,
        })
    }

    /// The first segment's Media Sequence Number, accounting for a delta
    /// update's EXT-X-SKIP before any segment.
    fn first_sequence(&self) -> u64 {
        let mut sequence = 0;
        for tag in &self.tags {
            match tag {
                Tag::ExtXMediaSequence(first) => sequence = *first,
                Tag::ExtXSkip {
                    skipped_segments, ..
                } => sequence += u64::from(*skipped_segments),
                Tag::ExtInf { .. } => break,
                _ => {}
            }
        }
        sequence
    }
}
