# M3U8 Parser

[![Crates.io](https://img.shields.io/crates/v/m3u8-parser.svg)](https://crates.io/crates/m3u8-parser)
[![Documentation](https://docs.rs/m3u8-parser/badge.svg)](https://docs.rs/m3u8-parser)
[![check](https://github.com/includeamin/m3u8-parser/actions/workflows/rust.yml/badge.svg)](https://github.com/includeamin/m3u8-parser/actions/workflows/rust.yml)

A Rust crate for parsing and creating M3U8 version 7 files for HTTP Live Streaming (HLS), as specified
by [RFC 8216](https://tools.ietf.org/html/rfc8216).

> [!IMPORTANT]
> This project is currently under active development. Please note that features and APIs are subject to change. My goal is to ensure full compatibility with [RFC 8216](https://tools.ietf.org/html/rfc8216).

## Features

- Parse M3U8 playlists from strings, files, or readers
- Generate M3U8 playlists with `PlaylistBuilder` and write them to strings
  (`Display`/`FromStr`), files, or any `std::io::Write`
- Resolve `EXT-X-DEFINE` variables (`NAME`/`VALUE`, `IMPORT`, `QUERYPARAM`)
  with `Playlist::substitute_variables`
- Typed attribute values such as `ByteRange`, `Key`, `Map`, and numeric
  `TIME-OFFSET`, with `f64` durations
- Derive `MediaSegment` values from a playlist with the media sequence number,
  discontinuity flag, and effective `KEY`, `MAP`, `BYTERANGE`, `GAP`, and
  `PROGRAM-DATE-TIME` state for each `EXTINF` segment.
- Parses, models, and serializes every RFC 8216 tag family listed below. Standard
  tags use order-independent attribute parsing, and unsupported extension tags and
  unknown `EXT-X-DATERANGE` attributes are preserved for round-tripping.
- Typed parse errors (`ParseError`) carrying the 1-based line number and a
  matchable `SyntaxError` kind.
- RFC 8216 semantic validation enforces `EXTM3U` placement, singleton tags,
  Master/Media Playlist separation, `EXT-X-STREAM-INF` URI pairing, required
  media target durations, RFC target-duration rounding, tag version gates
  (versions 1 through 12), tags removed in later versions, encryption and
  session key attribute combinations, `CLOSED-CAPTIONS` rendition rules,
  ISO 8601 date-times, byte-range continuity, `END-ON-NEXT` date-range
  constraints, and Master Playlist rendition-group references. Validation still does not enforce
  every cross-tag constraint defined by the RFC.
- RFC 8216 tag coverage:
    - **Basic Tags**:
        - `#EXTM3U`
        - `#EXT-X-VERSION`
    - **Media Playlist Tags**:
        - `#EXT-X-TARGETDURATION`
        - `#EXT-X-MEDIA-SEQUENCE`
        - `#EXT-X-ALLOW-CACHE`
        - `#EXT-X-DISCONTINUITY-SEQUENCE`
        - `#EXT-X-ENDLIST`
        - `#EXT-X-PLAYLIST-TYPE`
        - `#EXT-X-I-FRAMES-ONLY`
    - **Media Segment Tags**:
        - `#EXTINF`
        - `#EXT-X-KEY`
        - `#EXT-X-BYTERANGE`
        - `#EXT-X-MAP`
        - `#EXT-X-GAP`
        - `#EXT-X-PROGRAM-DATE-TIME`
        - `#EXT-X-DATERANGE`
    - **Master Playlist Tags**:
        - `#EXT-X-STREAM-INF`
        - `#EXT-X-MEDIA`
        - `#EXT-X-I-FRAME-STREAM-INF`
        - `#EXT-X-SESSION-DATA`
        - `#EXT-X-SESSION-KEY`
        - `#EXT-X-INDEPENDENT-SEGMENTS`
        - `#EXT-X-START`
        - `#EXT-X-DEFINE`
- Low-latency and draft extension tags from draft-pantos-hls-rfc8216bis:
  `#EXT-X-PART`, `#EXT-X-PART-INF`, `#EXT-X-PRELOAD-HINT`,
  `#EXT-X-RENDITION-REPORT`, `#EXT-X-SERVER-CONTROL`, `#EXT-X-SKIP`, and
  `#EXT-X-BITRATE`.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
m3u8-parser = "0.7.0"
```

## Usage

### Parsing a Playlist

```rust
use m3u8_parser::m3u8::playlist::Playlist;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = r#"
#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:6
#EXTINF:5.009,
https://media.example.com/first.ts
#EXTINF:5.009,
https://media.example.com/second.ts
#EXTINF:3.003,
https://media.example.com/third.ts
#EXT-X-ENDLIST
"#;

    let playlist = Playlist::from_reader(data.as_bytes())?;
    if let Err(errors) = playlist.validate() {
        eprintln!("playlist is not RFC 8216 compliant: {errors:?}");
    }
    for segment in playlist.media_segments() {
        println!("#{} {} ({}s)", segment.sequence, segment.uri, segment.duration);
    }
    Ok(())
}
```

Parse failures are reported as `ParseError`, which carries the line number and
a matchable `SyntaxError`:

```rust
use m3u8_parser::m3u8::error::{ParseError, SyntaxError};
use m3u8_parser::m3u8::playlist::Playlist;

let data = "#EXTM3U\n#EXTINF:6.0,\n";
match Playlist::from_reader(data.as_bytes()) {
    Err(ParseError::Syntax { line, kind: SyntaxError::MissingUriAfterExtInf }) => {
        eprintln!("EXTINF on line {line} has no URI");
    }
    other => println!("{other:?}"),
}
```

### Creating a Playlist

```rust
use m3u8_parser::m3u8::playlist::builder::PlaylistBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playlist = PlaylistBuilder::new()
        .extm3u()
        .version(7)
        .target_duration(6)
        .extinf("https://media.example.com/first.ts", 5.009, None)
        .extinf("https://media.example.com/second.ts", 5.009, None)
        .extinf("https://media.example.com/third.ts", 3.003, None)
        .end_list()
        .build()
        .map_err(|errors| format!("invalid playlist: {errors:?}"))?;

    playlist.write_to_file("playlist.m3u8")?;
    Ok(())
}
```

