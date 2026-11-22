//! Parses a Media Playlist, validates it, and walks its segments.
//!
//! Run with `cargo run --example media_playlist`.

use m3u8_parser::m3u8::playlist::Playlist;

const PLAYLIST: &str = r#"#EXTM3U
#EXT-X-VERSION:6
#EXT-X-TARGETDURATION:6
#EXT-X-MEDIA-SEQUENCE:100
#EXT-X-MAP:URI="init.mp4"
#EXTINF:6.0,
seg100.m4s
#EXT-X-DISCONTINUITY
#EXTINF:5.5,
seg101.m4s
#EXT-X-ENDLIST
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playlist: Playlist = PLAYLIST.parse()?;

    if let Err(errors) = playlist.validate() {
        eprintln!("playlist is not RFC 8216 compliant: {errors:?}");
    }

    println!("total duration: {}s", playlist.total_duration());
    for segment in playlist.media_segments() {
        println!(
            "#{} {} ({}s){}",
            segment.sequence,
            segment.uri,
            segment.duration,
            if segment.discontinuity {
                " after a discontinuity"
            } else {
                ""
            }
        );
    }
    Ok(())
}
