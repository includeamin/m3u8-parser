# Changelog

All notable changes to this project will be documented in this file.

## [0.19.0] - 2026-11-23

### 🚀 Features

- Add `Playlist::to_delta_update`, the inverse of `apply_delta_update`, which builds the Playlist Delta Update a server sends for an `_HLS_skip` request by replacing the segments older than `CAN-SKIP-UNTIL` with an `EXT-X-SKIP` tag
- `SkipRequest::SegmentsAndDateRanges` also drops the skipped date ranges and writes `RECENTLY-REMOVED-DATERANGES`, and `EXT-X-VERSION` is raised to the level the delta needs
- Add `Playlist::reload_response` to pick the delta update or the full playlist for a `ReloadRequest`

## [0.18.0] - 2026-11-23

### 🚀 Features

- Add `ReloadRequest` and `SkipRequest` to build and parse the `_HLS_msn`, `_HLS_part` and `_HLS_skip` query parameters of Blocking Playlist Reload and delta update requests, with `ReloadRequestError` for invalid directives
- Add `Playlist::can_block_reload`, `can_skip_until`, `can_skip_dateranges`, `supports_skip` and `skip_request` for the capabilities the server advertises
- Add `Playlist::next_reload_request` for the request that blocks on the next part or segment, and `Playlist::satisfies` to check whether a playlist answers a request
- Validation requires `CAN-SKIP-UNTIL` to be at least six times `EXT-X-TARGETDURATION`

## [0.17.0] - 2026-11-22

### 🚀 Features

- [**breaking**] `EXT-X-DATERANGE` `START-DATE` and `END-DATE` are now `DateTime`, `PlaylistBuilder::date_range` takes a `DateTime`, and `ValidationError::InvalidDateRangeStartDate` and `InvalidDateRangeEndDate` are removed
- Validation reports a date range whose `END-DATE` is earlier than its `START-DATE`, and `DateTime::unix_timestamp_nanos` is added for comparing instants
- [**breaking**] Add typed `MediaType`, `PreloadHintType`, `ClosedCaptions`, `InstreamId`, `Channels`, `Characteristics`, `LanguageTag`, `VideoRange`, `HdcpLevel` and `HexSequence` values, used by the tags, `Variant`, `Rendition` and the builder
- [**breaking**] `VIDEO-RANGE` and `HDCP-LEVEL` are now the `video_range` and `hdcp_level` fields of `EXT-X-STREAM-INF` and `EXT-X-I-FRAME-STREAM-INF` instead of `extra_attributes`
- [**breaking**] `EXT-X-DEFINE` is now a typed `Define` of `Value`, `Import` or `QueryParam`, and `PlaylistBuilder::define` takes one
- Add `ValidationError::InvalidLanguage`, and report a malformed `INSTREAM-ID` through `InvalidMediaAttributes`

### 🔄 Changes

- [**breaking**] Invalid `START-DATE` and `END-DATE` values, a non-numeric `CHANNELS` count, malformed `SCTE35-*` hex sequences and an `EXT-X-DEFINE` that is not exactly one form now fail at parse time with a `SyntaxError`; a malformed `EXT-X-DEFINE` was previously `SubstitutionError::InvalidDefine`
- `EXT-X-DEFINE` no longer keeps unknown attributes

## [0.16.0] - 2026-11-22

### 🚀 Features

- [**breaking**] Add a typed `Codecs` for the `CODECS` attribute: the `codecs` field of `EXT-X-STREAM-INF` and `EXT-X-I-FRAME-STREAM-INF`, `Variant::codecs` and the `PlaylistBuilder::stream_inf` and `iframe_stream_inf` parameters now use it instead of a string
- `Codecs` lists the codec identifiers with `iter`, `len`, `is_empty` and `contains_family`
- Add `ValidationError::InvalidCodecs` for empty `CODECS` entries

## [0.15.0] - 2026-11-22

### 🚀 Features

- [**breaking**] Add a typed `DateTime` for `EXT-X-PROGRAM-DATE-TIME`: `Tag::ExtXProgramDateTime`, `MediaSegment::program_date_time` and `PlaylistBuilder::program_date_time` now use it instead of a string
- `DateTime` exposes its fields, converts to and from Unix timestamps with `unix_timestamp` and `from_unix_timestamp`, and keeps the fraction precision when written

### 🔄 Changes

- [**breaking**] An invalid `EXT-X-PROGRAM-DATE-TIME` now fails at parse time with a `SyntaxError`, and `ValidationError::InvalidProgramDateTime` is removed

## [0.14.0] - 2026-11-22

### 🚀 Features

- [**breaking**] Add `MediaSegment::parts`, the `Part` partial segments that precede each segment's `EXTINF`
- Add `Playlist::pending_parts`, `pending_segment_sequence` and `latest_part_position` for the segment still being produced and the newest part's `PartPosition`

### 🐛 Bug Fixes

- `media_segments` now adds `SKIPPED-SEGMENTS` to the sequence at an `EXT-X-SKIP`, so the segments of a delta playlist are numbered correctly

## [0.13.0] - 2026-11-22

### 🚀 Features

- Merge Playlist Delta Updates with `Playlist::apply_delta_update`, replacing `EXT-X-SKIP` with the skipped segments and their tags from the previous playlist
- Add `DeltaUpdateError` for deltas that cannot be applied

### 📚 Documentation

- Add runnable examples for media playlists, master playlists, building and URI resolution

## [0.12.0] - 2026-11-22

### 🚀 Features

- Add `Playlist::total_duration` for the summed `EXTINF` durations
- Add `Playlist::variants` and `variants_by_bandwidth`, returning `Variant` values for each `EXT-X-STREAM-INF` and its URI
- Add `Playlist::renditions` and `renditions_in_group`, returning `Rendition` values for `EXT-X-MEDIA` tags

## [0.11.0] - 2026-11-22

### 🚀 Features

- [**breaking**] Type `EXT-X-PLAYLIST-TYPE` as `PlaylistType` and `METHOD` as `KeyMethod`; `PlaylistBuilder::key`, `session_key` and `playlist_type` take the new types
- [**breaking**] Type `RESOLUTION` as `Resolution { width, height }` on `ExtXStreamInf` and `ExtXIFrameStreamInf`; malformed values are now a syntax error, and `PlaylistBuilder::stream_inf` and `iframe_stream_inf` take `Option<Resolution>`

## [0.10.0] - 2026-11-21

### 🚀 Features

- Resolve relative URIs against a base URI with `Playlist::resolve_uris` and `uri::resolve_uri`, following RFC 3986 section 5 with no new dependencies
- Add `UriError::InvalidBase` for bases that are not absolute URIs

## [0.9.0] - 2026-11-21

### 🚀 Features

- Support `EXT-X-CONTENT-STEERING` with `Tag::ExtXContentSteering` and `PlaylistBuilder::content_steering`
- [**breaking**] Add a typed `pathway_id` field to `ExtXStreamInf`, `ExtXIFrameStreamInf` and `ExtXMedia`; `PATHWAY-ID` is no longer kept in `extra_attributes`
- Validate content steering: master playlist tag, single occurrence, non-empty `SERVER-URI` and `PATHWAY-ID` characters, reported as the new `ValidationError::InvalidContentSteering`

## [0.8.1] - 2026-10-17

### 🐛 Bug Fixes

- Strip a byte order mark at the start of a line so written playlists parse back identically
- Reject a double quote inside an unquoted attribute value, which could not be written back

### 🧪 Testing

- Add cargo-fuzz targets for parsing and round-tripping

## [0.8.0] - 2026-10-17

### 🚀 Features

- Increase support for RFC (#7)
- [**breaking**] Write playlists to strings and writers; drop unused errors
- [**breaking**] Type byte ranges and start offsets; validate date-times
- [**breaking**] Extend PlaylistBuilder and drop shared mutable state
- Substitute EXT-X-DEFINE variables; flag removed ALLOW-CACHE
- [**breaking**] Preserve unmodeled attributes on STREAM-INF, I-FRAME-STREAM-INF and MEDIA
- Flag tags whose strings break serialization during validation
- Validate uniqueness, rendition group, low-latency and ordering rules

### 🐛 Bug Fixes

- [**breaking**] Align parsing, serialization, and validation with the HLS spec (#8)
- Avoid validate panic on non-ASCII time zones
- Reject NaN and infinite durations, bare value tags and impossible dates
- Reject variable values that could inject tags or attributes during substitution

### 🚜 Refactor

- [**breaking**] Use named fields for ExtInf and f64 for durations

### ⚙️ Miscellaneous Tasks

- Publish the crate and create a GitHub release on version tags
- Add MSRV (1.82), docs and package checks; update actions

## [0.6.1] - 2024-11-05

### 🐛 Bug Fixes

- Fix EXTINF duration (#5)

## [0.6.0] - 2024-11-05

### 🚀 Features

- Add support for more tags from rfc (#3)

## [0.5.0] - 2024-11-02

### ⚙️ Miscellaneous Tasks

- [**breaking**] Restructure project (#2)

## [0.4.0] - 2024-11-01

### ⚙️ Miscellaneous Tasks

- Add a Note about the status of the project
- Fix `Crates.io` badge in `README.md`

## [0.3.0] - 2024-11-01

### 🚀 Features

- Add `PlaylistBuilder`
- Add example to README for `PlaylistBuilder`
- Add `validate` method to the `Playlist` to validate the Playlist on build

### ⚙️ Miscellaneous Tasks

- Update README.md

## [0.2.0] - 2024-11-01

### 🚀 Features

- Creat the first version of m3u8-parser

### ⚙️ Miscellaneous Tasks

- Implement the first version
- [**breaking**] Rename crate from `m3u8` to `m3u8_parser`
- Update README.md

<!-- generated by git-cliff -->
