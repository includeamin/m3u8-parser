//! Playlist Delta Updates (draft-pantos-hls-rfc8216bis section 6.2.5.1).

use super::Playlist;
use crate::m3u8::error::DeltaUpdateError;
use crate::m3u8::tags::Tag;
use std::collections::HashSet;

impl Playlist {
    /// Applies a Playlist Delta Update to this, the previously loaded, playlist
    /// and returns the full playlist it describes.
    ///
    /// The `EXT-X-SKIP` tag of `delta` stands for the first
    /// `SKIPPED-SEGMENTS` segments of the playlist, starting at its
    /// `EXT-X-MEDIA-SEQUENCE`. They are taken from `self` together with the
    /// tags that belong to them (`EXT-X-KEY`, `EXT-X-MAP`,
    /// `EXT-X-DISCONTINUITY`, `EXT-X-PROGRAM-DATE-TIME`, `EXT-X-DATERANGE` and so
    /// on) and put where the `EXT-X-SKIP` tag was. `EXT-X-DATERANGE` tags named
    /// by `RECENTLY-REMOVED-DATERANGES`, or already present in `delta`, are not
    /// copied.
    ///
    /// A `delta` without an `EXT-X-SKIP` tag is already complete and is
    /// returned unchanged.
    ///
    /// ```
    /// use m3u8_parser::m3u8::playlist::Playlist;
    ///
    /// let previous: Playlist = "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-MEDIA-SEQUENCE:7\n\
    /// #EXTINF:4,\na.ts\n#EXTINF:4,\nb.ts\n".parse().unwrap();
    /// let delta: Playlist = "#EXTM3U\n#EXT-X-VERSION:9\n#EXT-X-TARGETDURATION:4\n\
    /// #EXT-X-MEDIA-SEQUENCE:7\n#EXT-X-SKIP:SKIPPED-SEGMENTS=2\n#EXTINF:4,\nc.ts\n".parse().unwrap();
    ///
    /// let full = previous.apply_delta_update(&delta).unwrap();
    /// let uris: Vec<_> = full.media_segments().into_iter().map(|s| s.uri).collect();
    /// assert_eq!(uris, ["a.ts", "b.ts", "c.ts"]);
    /// ```
    pub fn apply_delta_update(&self, delta: &Playlist) -> Result<Playlist, DeltaUpdateError> {
        let mut skips = delta
            .tags
            .iter()
            .enumerate()
            .filter(|(_, tag)| matches!(tag, Tag::ExtXSkip { .. }));
        let Some((
            skip_index,
            Tag::ExtXSkip {
                skipped_segments,
                recently_removed_dateranges,
            },
        )) = skips.next()
        else {
            return Ok(Playlist {
                tags: delta.tags.clone(),
            });
        };
        if skips.next().is_some() {
            return Err(DeltaUpdateError::MultipleSkipTags);
        }
        if self
            .tags
            .iter()
            .any(|tag| matches!(tag, Tag::ExtXSkip { .. }))
        {
            return Err(DeltaUpdateError::PreviousIsDeltaUpdate);
        }

        let first_skipped = delta
            .tags
            .iter()
            .find_map(|tag| match tag {
                Tag::ExtXMediaSequence(sequence) => Some(*sequence),
                _ => None,
            })
            .unwrap_or(0);

        let segments = self.media_segments();
        let ends: Vec<usize> = self
            .tags
            .iter()
            .enumerate()
            .filter(|(_, tag)| matches!(tag, Tag::ExtInf { .. }))
            .map(|(index, _)| index)
            .collect();
        let previous_first = segments.first().map(|segment| segment.sequence);

        let mut excluded: HashSet<&str> = recently_removed_dateranges
            .as_deref()
            .map(|ids| ids.split('\t').collect())
            .unwrap_or_default();
        for tag in &delta.tags {
            if let Tag::ExtXDateRange { id, .. } = tag {
                excluded.insert(id);
            }
        }

        let mut spliced = Vec::new();
        for offset in 0..u64::from(*skipped_segments) {
            let sequence = first_skipped + offset;
            let position = previous_first
                .and_then(|first| sequence.checked_sub(first))
                .and_then(|position| usize::try_from(position).ok())
                .filter(|position| *position < ends.len())
                .ok_or(DeltaUpdateError::MissingSegment(sequence))?;

            let end = ends[position];
            let start = if position == 0 {
                0
            } else {
                ends[position - 1] + 1
            };
            let span = &self.tags[start..=end];

            if offset == 0 && position > 0 {
                // The segments before this one are gone from the full playlist,
                // so restate the key and map they established.
                let segment = &segments[position];
                if let Some(key) = &segment.key {
                    if !span.iter().any(|tag| matches!(tag, Tag::ExtXKey(_))) {
                        spliced.push(Tag::ExtXKey(key.clone()));
                    }
                }
                if let Some(map) = &segment.map {
                    if !span.iter().any(|tag| matches!(tag, Tag::ExtXMap(_))) {
                        spliced.push(Tag::ExtXMap(map.clone()));
                    }
                }
            }

            for tag in span {
                // Before the first segment only its own tags belong to it, not
                // the playlist header.
                if position == 0 && !is_segment_tag(tag) {
                    continue;
                }
                if matches!(tag, Tag::ExtXDateRange { id, .. } if excluded.contains(id.as_str())) {
                    continue;
                }
                spliced.push(tag.clone());
            }
        }

        let mut tags = Vec::with_capacity(delta.tags.len() + spliced.len());
        tags.extend_from_slice(&delta.tags[..skip_index]);
        tags.extend(spliced);
        tags.extend_from_slice(&delta.tags[skip_index + 1..]);
        Ok(Playlist { tags })
    }
}

/// Whether `tag` applies to a single media segment rather than the playlist.
fn is_segment_tag(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::ExtInf { .. }
            | Tag::ExtXKey(_)
            | Tag::ExtXMap(_)
            | Tag::ExtXProgramDateTime(_)
            | Tag::ExtXByteRange(_)
            | Tag::ExtXDateRange { .. }
            | Tag::ExtXGap
            | Tag::ExtXDiscontinuity
            | Tag::ExtXBitrate(_)
            | Tag::ExtXPart { .. }
    )
}
