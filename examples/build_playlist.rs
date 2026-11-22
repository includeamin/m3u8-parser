//! Builds a Media Playlist with `PlaylistBuilder` and prints it.
//!
//! Run with `cargo run --example build_playlist`.

use m3u8_parser::m3u8::playlist::builder::PlaylistBuilder;
use m3u8_parser::m3u8::tags::PlaylistType;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playlist = PlaylistBuilder::new()
        .extm3u()
        .version(3)
        .target_duration(6)
        .playlist_type(PlaylistType::Vod)
        .extinf("first.ts", 5.009, None)
        .extinf("second.ts", 5.009, None)
        .extinf("third.ts", 3.003, None)
        .end_list()
        .build()
        .map_err(|errors| format!("invalid playlist: {errors:?}"))?;

    // `Display` produces the playlist text; `write_to_file` saves it.
    print!("{playlist}");
    Ok(())
}
