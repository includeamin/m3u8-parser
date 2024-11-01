# M3U8 Parser

[![Crates.io](https://img.shields.io/crates/v/m3u8-parser.svg)](https://crates.io/crates/m3u8-parser)
[![Documentation](https://docs.rs/m3u8-parser/badge.svg)](https://docs.rs/m3u8-parser)
[![check](https://github.com/includeamin/m3u8-parser/actions/workflows/rust.yml/badge.svg)](https://github.com/includeamin/m3u8-parser/actions/workflows/rust.yml)

A Rust crate for parsing and creating M3U8 version 7 files for HTTP Live Streaming (HLS), as specified
by [RFC 8216](https://tools.ietf.org/html/rfc8216).

> [!IMPORTANT]
> This project is under active development. Features and APIs may change.

## Features

- Parse M3U8 playlists from strings, files, or readers
- Generate M3U8 playlists and write them to strings, files, or writers
- Support for all tags specified in RFC 8216, including:
    - Basic Tags (e.g., `#EXTM3U`, `#EXT-X-VERSION`)
    - Media Segment Tags (e.g., `#EXTINF`, `#EXT-X-BYTERANGE`)
    - Media Playlist Tags (e.g., `#EXT-X-TARGETDURATION`, `#EXT-X-MEDIA-SEQUENCE`)
    - Master Playlist Tags (e.g., `#EXT-X-STREAM-INF`, `#EXT-X-MEDIA`)
    - Encryption Tags (e.g., `#EXT-X-KEY`, `#EXT-X-SESSION-KEY`)
    - Date Range Tags (e.g., `#EXT-X-DATERANGE`)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
m3u8-parser = "0.4.0"
```

## Usage

### Parsing a Playlist

```rust
use m3u8_parser::Playlist;

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
use m3u8_parser::PlaylistBuilder;

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


### Using PlaylistBuilder

```rust
use m3u8_parser::PlaylistBuilder;

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
        .build();

    playlist.write_to_file("playlist.m3u8")?;
    Ok(())
}
```
