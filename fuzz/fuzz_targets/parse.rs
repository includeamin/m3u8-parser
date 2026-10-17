#![no_main]

use libfuzzer_sys::fuzz_target;
use m3u8_parser::m3u8::playlist::Playlist;
use std::collections::HashMap;

// Arbitrary input must never panic, whether or not it parses.
fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(playlist) = text.parse::<Playlist>() else {
        return;
    };
    let _ = playlist.validate();
    let _ = playlist.media_segments();
    let _ = playlist.to_string();
    let _ = playlist.substitute_variables(&HashMap::new(), Some("a=1&b=%zz"));
});
