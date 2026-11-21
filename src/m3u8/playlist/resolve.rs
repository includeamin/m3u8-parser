//! Resolution of relative URIs against a base URI.

use super::Playlist;
use crate::m3u8::error::UriError;
use crate::m3u8::tags::{Key, Map, Tag};
use crate::m3u8::uri::{resolve_uri, validate_base};

impl Playlist {
    /// Returns a copy of the playlist in which every relative URI is resolved
    /// against `base`, normally the URL the playlist was fetched from.
    ///
    /// This covers segment and variant URIs, `EXT-X-KEY`, `EXT-X-SESSION-KEY`,
    /// `EXT-X-MAP`, `EXT-X-MEDIA`, `EXT-X-I-FRAME-STREAM-INF`,
    /// `EXT-X-SESSION-DATA`, `EXT-X-PART`, `EXT-X-PRELOAD-HINT`,
    /// `EXT-X-RENDITION-REPORT`, and the `EXT-X-CONTENT-STEERING` server URI.
    /// Empty URIs are left alone, since validation reports them.
    ///
    /// Call [`Playlist::substitute_variables`] first if the playlist uses
    /// `EXT-X-DEFINE`, because `{$name}` references are not resolved.
    ///
    /// ```
    /// use m3u8_parser::m3u8::playlist::Playlist;
    ///
    /// let playlist: Playlist = "#EXTM3U\n#EXT-X-TARGETDURATION:6\n#EXTINF:6,\nseg/1.ts\n"
    ///     .parse()
    ///     .unwrap();
    /// let resolved = playlist
    ///     .resolve_uris("https://cdn.example.com/live/index.m3u8")
    ///     .unwrap();
    /// assert!(resolved.to_string().contains("https://cdn.example.com/live/seg/1.ts"));
    /// ```
    pub fn resolve_uris(&self, base: &str) -> Result<Playlist, UriError> {
        validate_base(base)?;
        let mut playlist = Playlist {
            tags: self.tags.clone(),
        };
        for tag in &mut playlist.tags {
            resolve_tag(tag, base)?;
        }
        Ok(playlist)
    }
}

fn resolve_tag(tag: &mut Tag, base: &str) -> Result<(), UriError> {
    match tag {
        Tag::ExtInf { uri, .. }
        | Tag::Uri(uri)
        | Tag::ExtXMap(Map { uri, .. })
        | Tag::ExtXIFrameStreamInf { uri, .. }
        | Tag::ExtXPreloadHint { uri, .. }
        | Tag::ExtXRenditionReport { uri, .. }
        | Tag::ExtXPart { uri, .. }
        | Tag::ExtXContentSteering {
            server_uri: uri, ..
        } => resolve(uri, base),
        Tag::ExtXKey(Key { uri: Some(uri), .. })
        | Tag::ExtXSessionKey(Key { uri: Some(uri), .. })
        | Tag::ExtXMedia { uri: Some(uri), .. }
        | Tag::ExtXSessionData { uri: Some(uri), .. } => resolve(uri, base),
        _ => Ok(()),
    }
}

fn resolve(uri: &mut String, base: &str) -> Result<(), UriError> {
    if !uri.is_empty() {
        *uri = resolve_uri(base, uri)?;
    }
    Ok(())
}
