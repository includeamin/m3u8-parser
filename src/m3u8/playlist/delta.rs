//! Playlist Delta Updates (draft-pantos-hls-rfc8216bis section 6.2.5.1).

use super::{Playlist, ReloadRequest, SkipRequest};
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

    /// Builds the Playlist Delta Update a server sends for a request with
    /// `_HLS_skip`, the inverse of [`apply_delta_update`](Playlist::apply_delta_update).
    ///
    /// The leading segments that end more than `CAN-SKIP-UNTIL` seconds before
    /// the end of the playlist are replaced by an `EXT-X-SKIP` tag, together
    /// with the tags that belong to them. The duration after a segment counts
    /// the later segments and the parts of the segment in progress, so a
    /// segment is never skipped unless the Skip Boundary of media follows it.
    ///
    /// With [`SkipRequest::Segments`] the `EXT-X-DATERANGE` tags of the skipped
    /// segments are kept, since that request only skips segments. With
    /// [`SkipRequest::SegmentsAndDateRanges`] they are dropped, and
    /// `recently_removed_dateranges` lists the IDs of date ranges the server
    /// removed from the playlist recently, for `RECENTLY-REMOVED-DATERANGES`.
    /// The `EXT-X-VERSION` is raised to 9, or 10 when that list is not empty.
    ///
    /// Returns `None`, meaning the server should send the full playlist, when:
    /// the playlist does not advertise support for `skip` (see
    /// [`supports_skip`](Playlist::supports_skip)), it is itself a delta update,
    /// no segment is far enough from the end to skip, or an ID in
    /// `recently_removed_dateranges` is empty or contains a tab, quote, or line
    /// break, which the attribute cannot hold.
    ///
    /// ```
    /// use m3u8_parser::m3u8::playlist::{Playlist, SkipRequest};
    ///
    /// let full: Playlist = "#EXTM3U\n#EXT-X-VERSION:9\n#EXT-X-TARGETDURATION:4\n\
    /// #EXT-X-SERVER-CONTROL:CAN-SKIP-UNTIL=24\n#EXT-X-MEDIA-SEQUENCE:7\n\
    /// #EXTINF:4,\na.ts\n#EXTINF:4,\nb.ts\n#EXTINF:4,\nc.ts\n#EXTINF:4,\nd.ts\n\
    /// #EXTINF:4,\ne.ts\n#EXTINF:4,\nf.ts\n#EXTINF:4,\ng.ts\n#EXTINF:4,\nh.ts\n"
    ///     .parse()
    ///     .unwrap();
    ///
    /// let delta = full.to_delta_update(SkipRequest::Segments, &[]).unwrap();
    /// assert!(delta.to_string().contains("#EXT-X-SKIP:SKIPPED-SEGMENTS=2\n"));
    /// assert_eq!(full.apply_delta_update(&delta).unwrap().to_string(), full.to_string());
    /// ```
    pub fn to_delta_update(
        &self,
        skip: SkipRequest,
        recently_removed_dateranges: &[&str],
    ) -> Option<Playlist> {
        if !self.supports_skip(skip)
            || self
                .tags
                .iter()
                .any(|tag| matches!(tag, Tag::ExtXSkip { .. }))
            || recently_removed_dateranges
                .iter()
                .any(|id| id.is_empty() || id.contains(['\t', '"', '\n', '\r']))
        {
            return None;
        }
        let boundary = self.can_skip_until()?;

        // A segment can go when at least `boundary` seconds of media follow it.
        let segments = self.media_segments();
        let mut following: f64 = self.pending_parts().iter().map(|part| part.duration).sum();
        let mut skippable = vec![false; segments.len()];
        for (index, segment) in segments.iter().enumerate().rev() {
            skippable[index] = following >= boundary;
            following += segment.duration;
        }
        let skipped = skippable.iter().take_while(|skippable| **skippable).count();
        if skipped == 0 {
            return None;
        }

        let ends: Vec<usize> = self
            .tags
            .iter()
            .enumerate()
            .filter(|(_, tag)| matches!(tag, Tag::ExtInf { .. }))
            .map(|(index, _)| index)
            .collect();
        let skipped_end = ends[skipped - 1];

        let removed =
            skip == SkipRequest::SegmentsAndDateRanges && !recently_removed_dateranges.is_empty();
        let skip_tag = Tag::ExtXSkip {
            skipped_segments: u32::try_from(skipped).ok()?,
            recently_removed_dateranges: removed.then(|| recently_removed_dateranges.join("\t")),
        };

        // The header before the first segment stays; everything else in the
        // skipped part is what the client already holds.
        let mut tags: Vec<Tag> = self.tags[..=ends[0]]
            .iter()
            .filter(|tag| !is_segment_tag(tag))
            .cloned()
            .collect();
        tags.push(skip_tag);
        if skip == SkipRequest::Segments {
            tags.extend(
                self.tags[..=skipped_end]
                    .iter()
                    .filter(|tag| matches!(tag, Tag::ExtXDateRange { .. }))
                    .cloned(),
            );
        }
        tags.extend_from_slice(&self.tags[skipped_end + 1..]);

        let version = if removed { 10 } else { 9 };
        match tags
            .iter_mut()
            .find(|tag| matches!(tag, Tag::ExtXVersion(_)))
        {
            Some(Tag::ExtXVersion(current)) => *current = (*current).max(version),
            _ => {
                let position = tags
                    .iter()
                    .position(|tag| matches!(tag, Tag::ExtM3U))
                    .map_or(0, |index| index + 1);
                tags.insert(position, Tag::ExtXVersion(version));
            }
        }
        Some(Playlist { tags })
    }

    /// The playlist a server sends for `request`: a delta update when the
    /// request has `_HLS_skip` and [`to_delta_update`](Playlist::to_delta_update)
    /// can build one, and a copy of the full playlist otherwise.
    ///
    /// This does not wait for the playlist to contain the requested segment or
    /// part; check [`satisfies`](Playlist::satisfies) for that first.
    pub fn reload_response(
        &self,
        request: &ReloadRequest,
        recently_removed_dateranges: &[&str],
    ) -> Playlist {
        request
            .skip
            .and_then(|skip| self.to_delta_update(skip, recently_removed_dateranges))
            .unwrap_or_else(|| Playlist {
                tags: self.tags.clone(),
            })
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
