//! Resolves the relative URIs of a playlist against the URL it was fetched from.
//!
//! Run with `cargo run --example resolve_uris`.

use m3u8_parser::m3u8::playlist::Playlist;

const PLAYLIST: &str =
    "#EXTM3U\n#EXT-X-TARGETDURATION:6\n#EXTINF:6,\nsegments/1.ts\n#EXTINF:6,\n../shared/2.ts\n";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playlist: Playlist = PLAYLIST.parse()?;

    // If the playlist uses EXT-X-DEFINE, call `substitute_variables` first.
    let resolved = playlist.resolve_uris("https://cdn.example.com/live/index.m3u8")?;

    for segment in resolved.media_segments() {
        println!("{}", segment.uri);
    }
    Ok(())
}
