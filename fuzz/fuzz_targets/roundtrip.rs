#![no_main]

use libfuzzer_sys::fuzz_target;
use m3u8_parser::m3u8::playlist::Playlist;

// Writing a parsed playlist and parsing it again must succeed and be stable:
// the second write equals the first.
fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(playlist) = text.parse::<Playlist>() else {
        return;
    };
    let written = playlist.to_string();
    let reparsed: Playlist = written
        .parse()
        .unwrap_or_else(|error| panic!("written playlist does not parse: {error}\n{written}"));
    assert_eq!(reparsed.to_string(), written);
});
