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
- Generate M3U8 playlists and write them to strings, files, or writers
- Support for all tags specified in RFC 8216, including:
    - **Basic Tags**:
        - `#EXTM3U`
        - `#EXT-X-VERSION`
    - **Media Playlist Tags**:
        - `#EXT-X-TARGETDURATION`
        - `#EXT-X-MEDIA-SEQUENCE`
        - `#EXT-X-ALLOW-CACHE`
        - `#EXT-X-DISCONTINUITY-SEQUENCE`
        - `#EXT-X-MEDIA`
        - `#EXT-X-STREAM-INF`
        - `#EXT-X-I-FRAME-STREAM-INF`
        - `#EXT-X-INDEPENDENT-SEGMENTS`
        - `#EXT-X-BYTERANGE`
        - `#EXT-X-SESSION-DATA`
        - `#EXT-X-SESSION-KEY`
        - `#EXT-X-DEFINE`
    - **Media Segment Tags**:
        - `#EXTINF`
        - `#EXT-X-KEY`
        - `#EXT-X-BYTERANGE`
        - `#EXT-X-MAP`
        - `#EXT-X-GAP`
        - `#EXT-X-PROGRAM-DATE-TIME`
        - `#EXT-X-PART`
        - `#EXT-X-PRELOAD-HINT`
        - `#EXT-X-START`
        - `#EXT-X-DATERANGE`
    - **Encryption Tags**:
        - `#EXT-X-KEY`
        - `#EXT-X-SESSION-KEY`
    - **Date Range Tags**:
        - `#EXT-X-DATERANGE`
    - **End Playlist Tags**:
        - `#EXT-X-ENDLIST`
    - **Master Playlist Tags**:
        - `#EXT-X-STREAM-INF`
        - `#EXT-X-MEDIA`
        - `#EXT-X-STREAM-INF`
        - `#EXT-X-I-FRAME-STREAM-INF`
    - **Program Date and Time**:
        - `#EXT-X-PROGRAM-DATE-TIME`

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
m3u8-parser = "0.6.0"
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
  println!("{:?}", playlist);
  Ok(())
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
          .extinf(5.009, None)
          .uri("https://media.example.com/first.ts".to_string())
          .extinf(5.009, None)
          .uri("https://media.example.com/second.ts".to_string())
          .extinf(3.003, None)
          .uri("https://media.example.com/third.ts".to_string())
          .end_list()
          .build()?;

  playlist.write_to_file("playlist.m3u8")?;
  Ok(())
}
```

