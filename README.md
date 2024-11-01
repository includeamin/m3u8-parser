# m3u8

[![Crates.io](https://img.shields.io/crates/v/m3u8.svg)](https://crates.io/crates/m3u8)
[![Documentation](https://docs.rs/m3u8/badge.svg)](https://docs.rs/m3u8)
[![check](https://github.com/includeamin/m3u-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/includeamin/m3u-rs/actions/workflows/rust.yml)
A Rust crate for parsing and creating M3U8 version 7 files for HTTP Live Streaming (HLS), as specified by [RFC 8216](https://tools.ietf.org/html/rfc8216).

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
m3u8 = "0.1.0"
```


## Usage
### Parsing a Playlist

```rust
use m3u8::Playlist;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = r#"
    #EXTM3U
    #EXT-X-VERSION:3
    #EXT-X-TARGETDURATION:10
    #EXTINF:9.009,
    http://media.example.com/first.ts
    #EXTINF:9.009,
    http://media.example.com/second.ts
    #EXTINF:3.003,
    http://media.example.com/third.ts
    #EXT-X-ENDLIST
    "#;

    let playlist = Playlist::from_reader(data.as_bytes())?;
    println!("{:?}", playlist);
    Ok(())
}
```

### Creating a Playlist

```rust
use m3u8::{Playlist, Tag};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut playlist = Playlist::new();
    playlist.tags.push(Tag::ExtM3U);
    playlist.tags.push(Tag::ExtXVersion(3));
    playlist.tags.push(Tag::ExtXTargetDuration(10));
    playlist.tags.push(Tag::ExtInf(9.009, None));
    playlist.tags.push(Tag::Uri("http://media.example.com/first.ts".to_string()));
    playlist.tags.push(Tag::ExtInf(9.009, None));
    playlist.tags.push(Tag::Uri("http://media.example.com/second.ts".to_string()));
    playlist.tags.push(Tag::ExtInf(3.003, None));
    playlist.tags.push(Tag::Uri("http://media.example.com/third.ts".to_string()));
    playlist.tags.push(Tag::ExtXEndList);

    playlist.write_to_file("playlist.m3u8")?;
    Ok(())
}
```
