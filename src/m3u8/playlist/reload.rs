//! Blocking Playlist Reload and Playlist Delta Updates, from the client's side
//! (draft-pantos-hls-rfc8216bis sections 6.2.5.1 and 6.2.5.2).
//!
//! A client asks for a newer playlist by adding `_HLS_msn`, `_HLS_part` and
//! `_HLS_skip` query parameters to the playlist URI. [`ReloadRequest`] builds
//! and parses those parameters, and the [`Playlist`] methods here say what a
//! playlist allows and which request to make next.

use super::parts::PartPosition;
use super::Playlist;
use crate::m3u8::error::ReloadRequestError;
use crate::m3u8::tags::Tag;
use std::fmt;

const MSN: &str = "_HLS_msn";
const PART: &str = "_HLS_part";
const SKIP: &str = "_HLS_skip";

/// What the `_HLS_skip` parameter asks the server to leave out.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum SkipRequest {
    /// `_HLS_skip=YES`: replace old segments with an `EXT-X-SKIP` tag.
    Segments,
    /// `_HLS_skip=v2`: also replace old date ranges, listing the removed ones.
    SegmentsAndDateRanges,
}

impl SkipRequest {
    fn as_str(self) -> &'static str {
        match self {
            SkipRequest::Segments => "YES",
            SkipRequest::SegmentsAndDateRanges => "v2",
        }
    }
}

/// The directives of a playlist reload request, from the `_HLS_msn`,
/// `_HLS_part` and `_HLS_skip` query parameters.
///
/// The default request has no directives and asks for the playlist as it is.
/// `Display` writes the query string, for example
/// `_HLS_msn=101&_HLS_part=2&_HLS_skip=YES`.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Hash)]
pub struct ReloadRequest {
    /// `_HLS_msn`: hold the response until the playlist contains the segment
    /// with this Media Sequence Number, or a later one.
    pub msn: Option<u64>,
    /// `_HLS_part`: with `msn`, hold the response until the playlist contains
    /// this part of that segment, or a later one. Requires `msn`.
    pub part: Option<u64>,
    /// `_HLS_skip`: ask for a delta update.
    pub skip: Option<SkipRequest>,
}

impl From<PartPosition> for ReloadRequest {
    /// A request that blocks until `position` is available.
    fn from(position: PartPosition) -> Self {
        ReloadRequest {
            msn: Some(position.sequence),
            part: Some(position.part),
            skip: None,
        }
    }
}

impl ReloadRequest {
    /// A request that blocks until the segment with Media Sequence Number `msn`
    /// is in the playlist.
    pub fn segment(msn: u64) -> ReloadRequest {
        ReloadRequest {
            msn: Some(msn),
            ..ReloadRequest::default()
        }
    }

    /// The same request with `skip` added.
    pub fn with_skip(self, skip: SkipRequest) -> ReloadRequest {
        ReloadRequest {
            skip: Some(skip),
            ..self
        }
    }

    /// Whether the request has no directives.
    pub fn is_empty(&self) -> bool {
        self.msn.is_none() && self.part.is_none() && self.skip.is_none()
    }

    /// Reads the directives from a query string without the leading `?`. Other
    /// parameters are ignored, and the last value of a repeated directive wins.
    pub fn from_query(query: &str) -> Result<ReloadRequest, ReloadRequestError> {
        let mut request = ReloadRequest::default();
        for pair in query.split('&') {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            let invalid = || ReloadRequestError::InvalidValue {
                parameter: name.to_string(),
                value: value.to_string(),
            };
            let number = || {
                if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(invalid());
                }
                value.parse::<u64>().map_err(|_| invalid())
            };
            match name {
                MSN => request.msn = Some(number()?),
                PART => request.part = Some(number()?),
                SKIP => {
                    request.skip = Some(match value {
                        "YES" => SkipRequest::Segments,
                        "v2" => SkipRequest::SegmentsAndDateRanges,
                        _ => return Err(invalid()),
                    })
                }
                _ => {}
            }
        }
        if request.part.is_some() && request.msn.is_none() {
            return Err(ReloadRequestError::PartWithoutMsn);
        }
        Ok(request)
    }

    /// Reads the directives from the query of a request URI. A URI without a
    /// query gives the default request.
    pub fn from_uri(uri: &str) -> Result<ReloadRequest, ReloadRequestError> {
        let (query, _) = split_query(uri);
        ReloadRequest::from_query(query.unwrap_or_default())
    }

    /// Returns `uri` with these directives added to its query. Directives
    /// already on the URI are replaced, other parameters and the fragment are
    /// kept, and the order is `_HLS_msn`, `_HLS_part`, `_HLS_skip`.
    pub fn apply_to_uri(&self, uri: &str) -> String {
        let (fragment, rest) = match uri.split_once('#') {
            Some((rest, fragment)) => (Some(fragment), rest),
            None => (None, uri),
        };
        let (path, query) = match rest.split_once('?') {
            Some((path, query)) => (path, query),
            None => (rest, ""),
        };
        let mut parameters: Vec<String> = query
            .split('&')
            .filter(|pair| !pair.is_empty())
            .filter(|pair| {
                let name = pair.split_once('=').map_or(*pair, |(name, _)| name);
                ![MSN, PART, SKIP].contains(&name)
            })
            .map(str::to_string)
            .collect();
        let directives = self.to_string();
        if !directives.is_empty() {
            parameters.push(directives);
        }

        let mut result = path.to_string();
        if !parameters.is_empty() {
            result.push('?');
            result.push_str(&parameters.join("&"));
        }
        if let Some(fragment) = fragment {
            result.push('#');
            result.push_str(fragment);
        }
        result
    }
}

impl fmt::Display for ReloadRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parameters = Vec::new();
        if let Some(msn) = self.msn {
            parameters.push(format!("{MSN}={msn}"));
        }
        if let Some(part) = self.part {
            parameters.push(format!("{PART}={part}"));
        }
        if let Some(skip) = self.skip {
            parameters.push(format!("{SKIP}={}", skip.as_str()));
        }
        f.write_str(&parameters.join("&"))
    }
}

/// The query of a URI, without the `?`, and the rest of it.
fn split_query(uri: &str) -> (Option<&str>, &str) {
    let without_fragment = uri.split_once('#').map_or(uri, |(rest, _)| rest);
    match without_fragment.split_once('?') {
        Some((path, query)) => (Some(query), path),
        None => (None, without_fragment),
    }
}

impl Playlist {
    /// Whether the server accepts blocking reload requests
    /// (`CAN-BLOCK-RELOAD=YES` on `EXT-X-SERVER-CONTROL`).
    pub fn can_block_reload(&self) -> bool {
        self.tags.iter().any(|tag| {
            matches!(
                tag,
                Tag::ExtXServerControl {
                    can_block_reload: Some(true),
                    ..
                }
            )
        })
    }

    /// The Skip Boundary in seconds (`CAN-SKIP-UNTIL`), when the server can
    /// produce delta updates.
    pub fn can_skip_until(&self) -> Option<f64> {
        self.tags.iter().find_map(|tag| match tag {
            Tag::ExtXServerControl { can_skip_until, .. } => *can_skip_until,
            _ => None,
        })
    }

    /// Whether the server can also skip date ranges (`CAN-SKIP-DATERANGES=YES`).
    pub fn can_skip_dateranges(&self) -> bool {
        self.tags.iter().any(|tag| {
            matches!(
                tag,
                Tag::ExtXServerControl {
                    can_skip_dateranges: Some(true),
                    ..
                }
            )
        })
    }

    /// Whether the server supports `skip`: it needs a Skip Boundary, and
    /// skipping date ranges also needs `CAN-SKIP-DATERANGES=YES`.
    pub fn supports_skip(&self, skip: SkipRequest) -> bool {
        self.can_skip_until().is_some()
            && (skip == SkipRequest::Segments || self.can_skip_dateranges())
    }

    /// The strongest delta update the server supports, to put in the `skip` of
    /// the next request, or `None` when it supports none.
    pub fn skip_request(&self) -> Option<SkipRequest> {
        [SkipRequest::SegmentsAndDateRanges, SkipRequest::Segments]
            .into_iter()
            .find(|skip| self.supports_skip(*skip))
    }

    /// The request a client makes to get the next update of this playlist.
    ///
    /// It blocks until the next part, when the playlist uses partial segments,
    /// or the next segment otherwise. It has no directives when the server does
    /// not accept blocking reloads or the playlist has ended. It never asks for a
    /// delta update; combine it with [`skip_request`](Playlist::skip_request) for
    /// that:
    ///
    /// ```
    /// use m3u8_parser::m3u8::playlist::Playlist;
    /// use m3u8_parser::m3u8::playlist::ReloadRequest;
    ///
    /// let playlist: Playlist = "#EXTM3U\n#EXT-X-TARGETDURATION:4\n\
    /// #EXT-X-SERVER-CONTROL:CAN-BLOCK-RELOAD=YES,CAN-SKIP-UNTIL=24\n\
    /// #EXT-X-MEDIA-SEQUENCE:100\n#EXTINF:4,\na.ts\n"
    ///     .parse()
    ///     .unwrap();
    /// let request = ReloadRequest {
    ///     skip: playlist.skip_request(),
    ///     ..playlist.next_reload_request()
    /// };
    /// assert_eq!(
    ///     request.apply_to_uri("https://example.com/live.m3u8"),
    ///     "https://example.com/live.m3u8?_HLS_msn=101&_HLS_skip=YES"
    /// );
    /// ```
    pub fn next_reload_request(&self) -> ReloadRequest {
        let ended = self.tags.iter().any(|tag| matches!(tag, Tag::ExtXEndList));
        if ended || !self.can_block_reload() {
            return ReloadRequest::default();
        }
        let pending = self.pending_parts();
        let uses_parts = !pending.is_empty()
            || self
                .tags
                .iter()
                .any(|tag| matches!(tag, Tag::ExtXPartInf { .. } | Tag::ExtXPart { .. }));
        ReloadRequest {
            msn: Some(self.pending_segment_sequence()),
            part: uses_parts.then_some(pending.len() as u64),
            skip: None,
        }
    }

    /// Whether this playlist is new enough to answer `request`: it contains the
    /// segment `msn`, or the part `part` of it, or something later. A playlist
    /// that has ended answers every request, and a request with no `msn` is
    /// always answered.
    ///
    /// This is the condition a server waits for before responding.
    pub fn satisfies(&self, request: &ReloadRequest) -> bool {
        let Some(msn) = request.msn else {
            return true;
        };
        if self.tags.iter().any(|tag| matches!(tag, Tag::ExtXEndList)) {
            return true;
        }
        let last_complete = self.media_segments().last().map(|segment| segment.sequence);
        if last_complete.is_some_and(|sequence| sequence >= msn) {
            return true;
        }
        match (request.part, self.latest_part_position()) {
            (Some(part), Some(latest)) => (latest.sequence, latest.part) >= (msn, part),
            _ => false,
        }
    }
}
