//! Parses a Multivariant Playlist and lists its variants and renditions.
//!
//! Run with `cargo run --example master_playlist`.

use m3u8_parser::m3u8::playlist::Playlist;

const PLAYLIST: &str = r#"#EXTM3U
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="aud",NAME="English",LANGUAGE="en",DEFAULT=YES,URI="audio/en.m3u8"
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="aud",NAME="French",LANGUAGE="fr",URI="audio/fr.m3u8"
#EXT-X-STREAM-INF:BANDWIDTH=5000000,RESOLUTION=1920x1080,CODECS="avc1.640028,mp4a.40.2",AUDIO="aud"
video/1080p.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=1200000,RESOLUTION=640x360,CODECS="avc1.4d401e,mp4a.40.2",AUDIO="aud"
video/360p.m3u8
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playlist: Playlist = PLAYLIST.parse()?;
    playlist
        .validate()
        .map_err(|errors| format!("invalid playlist: {errors:?}"))?;

    for variant in playlist.variants_by_bandwidth() {
        let size = variant
            .resolution
            .map_or_else(|| "unknown size".to_string(), |r| r.to_string());
        println!("{} bit/s, {size}: {}", variant.bandwidth, variant.uri);

        if let Some(group) = &variant.audio {
            for rendition in playlist.renditions_in_group("AUDIO", group) {
                println!(
                    "  audio {:?} ({}){}",
                    rendition.name.unwrap_or_default(),
                    rendition.language.unwrap_or_default(),
                    if rendition.default { " [default]" } else { "" }
                );
            }
        }
    }
    Ok(())
}
