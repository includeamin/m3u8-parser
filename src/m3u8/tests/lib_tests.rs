#[cfg(test)]
mod tests {
    use crate::m3u8::playlist::builder::PlaylistBuilder;
    use crate::m3u8::playlist::Playlist;
    use crate::m3u8::tags::Tag;
    use crate::m3u8::validation::ValidationError;
    use std::io::Write;

    #[test]
    fn test_parse_simple_playlist() {
        let data = r#"
#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:10
#EXTINF:5.0050,
https://media.example.com/first.ts
#EXTINF:5.0050,
https://media.example.com/second.ts
#EXTINF:3.0030,
https://media.example.com/third.ts
#EXT-X-ENDLIST
"#;

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();
        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(7),
                Tag::ExtXTargetDuration(10),
                Tag::ExtInf(
                    "https://media.example.com/first.ts".to_string(),
                    5.005,
                    None
                ),
                Tag::ExtInf(
                    "https://media.example.com/second.ts".to_string(),
                    5.005,
                    None
                ),
                Tag::ExtInf(
                    "https://media.example.com/third.ts".to_string(),
                    3.003,
                    None
                ),
                Tag::ExtXEndList,
            ]
        );
    }

    #[test]
    fn test_parse_preserves_uri_unknown_and_core_tags() {
        let data = r#"#EXTM3U
#EXT-X-ALLOW-CACHE:NO
#EXT-X-I-FRAMES-ONLY
#EXT-X-CUSTOM:VALUE=1
variant.m3u8
"#;

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();

        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::ExtXAllowCache(false),
                Tag::ExtXIFramesOnly,
                Tag::Unknown("EXT-X-CUSTOM:VALUE=1".to_string()),
                Tag::Uri("variant.m3u8".to_string()),
            ]
        );

        let output = playlist
            .tags
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            output,
            "#EXTM3U\n#EXT-X-ALLOW-CACHE:NO\n#EXT-X-I-FRAMES-ONLY\n#EXT-X-CUSTOM:VALUE=1\nvariant.m3u8"
        );
    }

    #[test]
    fn test_parse_key_and_map_with_reordered_attributes() {
        let data = r#"#EXTM3U
#EXT-X-KEY:KEYFORMAT="identity",IV=0x1234,METHOD=AES-128,URI="key.bin"
#EXT-X-MAP:BYTERANGE="720@0",URI="init.mp4"
"#;

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();

        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::ExtXKey {
                    method: "AES-128".to_string(),
                    uri: Some("key.bin".to_string()),
                    iv: Some("0x1234".to_string()),
                    keyformat: Some("identity".to_string()),
                    keyformatversions: None,
                },
                Tag::ExtXMap {
                    uri: "init.mp4".to_string(),
                    byterange: Some("720@0".to_string()),
                },
            ]
        );
    }

    #[test]
    fn test_parse_rfc8216_master_playlist_tags() {
        let data = r#"#EXTM3U
#EXT-X-VERSION:7
#EXT-X-DEFINE:NAME="host",VALUE="https://example.com"
#EXT-X-INDEPENDENT-SEGMENTS
#EXT-X-MEDIA:URI="audio.m3u8",TYPE=AUDIO,GROUP-ID="audio",NAME="English",DEFAULT=YES,AUTOSELECT=YES,CHANNELS="2"
#EXT-X-SESSION-DATA:DATA-ID="com.example.title",VALUE="Example",LANGUAGE="en"
#EXT-X-SESSION-KEY:KEYFORMAT="identity",METHOD=AES-128,URI="key.bin",KEYFORMATVERSIONS="1"
#EXT-X-STREAM-INF:CODECS="avc1.4d401f,mp4a.40.2",AVERAGE-BANDWIDTH=900000,BANDWIDTH=1000000,AUDIO="audio",CLOSED-CAPTIONS=NONE
main.m3u8
#EXT-X-I-FRAME-STREAM-INF:URI="iframe.m3u8",BANDWIDTH=250000,AVERAGE-BANDWIDTH=200000,CODECS="avc1.4d401f"
"#;

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();

        assert_eq!(playlist.tags.len(), 10);
        assert!(playlist
            .tags
            .iter()
            .all(|tag| !matches!(tag, Tag::Unknown(_))));
        assert!(matches!(
            &playlist.tags[5],
            Tag::ExtXSessionData {
                id,
                value: Some(value),
                uri: None,
                language: Some(language),
            } if id == "com.example.title" && value == "Example" && language == "en"
        ));
        assert_eq!(
            playlist.tags[5].to_string(),
            "#EXT-X-SESSION-DATA:DATA-ID=\"com.example.title\",VALUE=\"Example\",LANGUAGE=\"en\""
        );
    }

    #[test]
    fn test_parse_media_requires_name() {
        let data = "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\"\n";

        assert_eq!(
            Playlist::from_reader(data.as_bytes()),
            Err("EXT-X-MEDIA requires NAME".to_string())
        );
    }

    #[test]
    fn test_validate_media_requires_name() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXMedia {
                    type_: "AUDIO".to_string(),
                    group_id: "audio".to_string(),
                    name: None,
                    uri: None,
                    default: None,
                    autoplay: None,
                    characteristics: None,
                    language: None,
                    instream_id: None,
                    language_codec: None,
                    forced: None,
                    channels: None,
                },
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![ValidationError::MissingMediaFields])
        );
    }

    #[test]
    fn test_parse_rfc8216_media_playlist_tags() {
        let data = r#"#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:6
#EXT-X-MEDIA-SEQUENCE:10
#EXT-X-DISCONTINUITY-SEQUENCE:3
#EXT-X-PLAYLIST-TYPE:VOD
#EXT-X-I-FRAMES-ONLY
#EXT-X-ALLOW-CACHE:NO
#EXT-X-START:PRECISE=YES,TIME-OFFSET=-12.5
#EXT-X-KEY:METHOD=AES-128,URI="key.bin",IV=0x01
#EXT-X-MAP:BYTERANGE="720@0",URI="init.mp4"
#EXT-X-PROGRAM-DATE-TIME:2024-01-01T00:00:00Z
#EXT-X-DATERANGE:ID="ad-1",START-DATE="2024-01-01T00:00:00Z",DURATION=30
#EXT-X-GAP
#EXT-X-BYTERANGE:1000@720
#EXTINF:6.0,segment
segment.ts
#EXT-X-DISCONTINUITY
#EXT-X-ENDLIST
"#;

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();

        assert_eq!(playlist.tags.len(), 18);
        assert!(playlist
            .tags
            .iter()
            .all(|tag| !matches!(tag, Tag::Unknown(_))));
        assert!(matches!(
            &playlist.tags[15],
            Tag::ExtInf(uri, duration, Some(title)) if uri == "segment.ts" && *duration == 6.0 && title == "segment"
        ));
    }

    #[test]
    fn test_parse_extension_tag_variants() {
        let data = r#"#EXTM3U
#EXT-X-BITRATE:1200000
#EXT-X-PART-INF:PART-NUMBER=4,PART-TARGET=0.5,PART-HOLD-BACK=1.5
#EXT-X-PART:DURATION=0.5,URI="part0.ts"
#EXT-X-SERVER-CONTROL:CAN-PAUSE=NO,MIN-BUFFER-TIME=2.0,CAN-PLAY=YES,CAN-SEEK=NO
#EXT-X-SKIP:REASON="delta",SKIPPED-SEGMENTS=3,URI="skip.ts"
#EXT-X-PRELOAD-HINT:BYTERANGE="100@0",URI="next.ts"
#EXT-X-RENDITION-REPORT:BANDWIDTH=2000000,URI="other.m3u8"
"#;

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();

        assert_eq!(playlist.tags.len(), 8);
        assert!(playlist
            .tags
            .iter()
            .all(|tag| !matches!(tag, Tag::Unknown(_))));
        assert!(matches!(playlist.tags[1], Tag::ExtXBitrate(1_200_000)));
        assert!(matches!(
            &playlist.tags[2],
            Tag::ExtXPartInf {
                part_target_duration,
                part_hold_back: Some(part_hold_back),
                part_number: Some(4),
            } if *part_target_duration == 0.5 && *part_hold_back == 1.5
        ));
        assert!(matches!(
            &playlist.tags[3],
            Tag::ExtXPart {
                uri,
                duration: Some(duration),
            } if uri == "part0.ts" && *duration == 0.5
        ));
        assert!(matches!(
            &playlist.tags[7],
            Tag::ExtXRenditionReport { uri, bandwidth }
                if uri == "other.m3u8" && *bandwidth == 2_000_000
        ));
    }

    #[test]
    fn test_media_segments_carry_preceding_state() {
        let data = r#"#EXTM3U
#EXT-X-TARGETDURATION:6
#EXT-X-KEY:METHOD=AES-128,URI="key-1.bin"
#EXT-X-MAP:URI="init.mp4"
#EXT-X-BYTERANGE:1000@0
#EXT-X-PROGRAM-DATE-TIME:2024-01-01T00:00:00Z
#EXT-X-GAP
#EXTINF:6.0,first
first.ts
#EXTINF:6.0,second
second.ts
"#;
        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();
        let segments = playlist.media_segments();

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].uri, "first.ts");
        assert_eq!(segments[0].byterange.as_deref(), Some("1000@0"));
        assert_eq!(
            segments[0].program_date_time.as_deref(),
            Some("2024-01-01T00:00:00Z")
        );
        assert!(segments[0].gap);
        assert!(matches!(
            segments[0].key,
            Some(Tag::ExtXKey { ref method, .. }) if method == "AES-128"
        ));
        assert!(matches!(segments[0].map, Some(Tag::ExtXMap { .. })));

        assert_eq!(segments[1].uri, "second.ts");
        assert!(segments[1].key.is_some());
        assert!(segments[1].map.is_some());
        assert_eq!(segments[1].byterange, None);
        assert_eq!(segments[1].program_date_time, None);
        assert!(!segments[1].gap);
    }

    #[test]
    fn test_parse_extinf_requires_following_uri() {
        let data = "#EXTM3U\n#EXTINF:6.0,segment\n#EXT-X-ENDLIST\n";

        assert_eq!(
            Playlist::from_reader(data.as_bytes()),
            Err("missing URI after EXTINF on line 2".to_string())
        );
    }

    #[test]
    fn test_write_simple_playlist() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(7),
                Tag::ExtXTargetDuration(10),
                Tag::ExtInf(
                    "https://media.example.com/first.ts".to_string(),
                    5.005,
                    None,
                ),
                Tag::ExtInf(
                    "https://media.example.com/second.ts".to_string(),
                    5.005,
                    None,
                ),
                Tag::ExtInf(
                    "https://media.example.com/third.ts".to_string(),
                    3.003,
                    None,
                ),
                Tag::ExtXEndList,
            ],
        };

        let mut output = Vec::new();
        for tag in &playlist.tags {
            writeln!(output, "{}", tag).unwrap();
        }
        let output = String::from_utf8(output).unwrap();

        let expected = r#"#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:10
#EXTINF:5.0050,
https://media.example.com/first.ts
#EXTINF:5.0050,
https://media.example.com/second.ts
#EXTINF:3.0030,
https://media.example.com/third.ts
#EXT-X-ENDLIST
"#;

        assert_eq!(output, expected);
    }

    #[test]
    fn test_parse_playlist_with_key() {
        let data = r#"
#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:10
#EXT-X-KEY:METHOD=AES-128,URI="https://priv.example.com/key.php?r=52"
#EXTINF:5.005,
https://media.example.com/first.ts
#EXTINF:5.005,
https://media.example.com/second.ts
#EXTINF:3.003,
https://media.example.com/third.ts
#EXT-X-ENDLIST
"#;

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();
        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(7),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXKey {
                    method: "AES-128".to_string(),
                    uri: Some("https://priv.example.com/key.php?r=52".to_string()),
                    iv: None,
                    keyformat: None,
                    keyformatversions: None,
                },
                Tag::ExtInf(
                    "https://media.example.com/first.ts".to_string(),
                    5.005,
                    None
                ),
                Tag::ExtInf(
                    "https://media.example.com/second.ts".to_string(),
                    5.005,
                    None
                ),
                Tag::ExtInf(
                    "https://media.example.com/third.ts".to_string(),
                    3.003,
                    None
                ),
                Tag::ExtXEndList,
            ]
        );
    }

    #[test]
    fn test_write_playlist_with_key() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(7),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXKey {
                    method: "AES-128".to_string(),
                    uri: Some("https://priv.example.com/key.php?r=52".to_string()),
                    iv: None,
                    keyformat: None,
                    keyformatversions: None,
                },
                Tag::ExtInf(
                    "https://media.example.com/first.ts".to_string(),
                    5.005,
                    None,
                ),
                Tag::ExtInf(
                    "https://media.example.com/second.ts".to_string(),
                    5.005,
                    None,
                ),
                Tag::ExtInf(
                    "https://media.example.com/third.ts".to_string(),
                    3.003,
                    None,
                ),
                Tag::ExtXEndList,
            ],
        };

        let mut output = Vec::new();
        for tag in &playlist.tags {
            writeln!(output, "{}", tag).unwrap();
        }
        let output = String::from_utf8(output).unwrap();

        let expected = r#"#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:10
#EXT-X-KEY:METHOD=AES-128,URI="https://priv.example.com/key.php?r=52"
#EXTINF:5.0050,
https://media.example.com/first.ts
#EXTINF:5.0050,
https://media.example.com/second.ts
#EXTINF:3.0030,
https://media.example.com/third.ts
#EXT-X-ENDLIST
"#;

        assert_eq!(output, expected);
    }

    #[test]
    fn test_parse_playlist_with_map() {
        let data = r#"
#EXTM3U
#EXT-X-VERSION:6
#EXT-X-TARGETDURATION:10
#EXT-X-MAP:URI="init.mp4"
#EXTINF:5.005,
https://media.example.com/first.ts
#EXTINF:5.005,
https://media.example.com/second.ts
#EXTINF:3.003,
https://media.example.com/third.ts
#EXT-X-ENDLIST
"#;

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();
        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(6),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXMap {
                    uri: "init.mp4".to_string(),
                    byterange: None,
                },
                Tag::ExtInf(
                    "https://media.example.com/first.ts".to_string(),
                    5.005,
                    None
                ),
                Tag::ExtInf(
                    "https://media.example.com/second.ts".to_string(),
                    5.005,
                    None
                ),
                Tag::ExtInf(
                    "https://media.example.com/third.ts".to_string(),
                    3.003,
                    None
                ),
                Tag::ExtXEndList,
            ]
        );
    }

    #[test]
    fn test_write_playlist_with_map() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(6),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXMap {
                    uri: "init.mp4".to_string(),
                    byterange: None,
                },
                Tag::ExtInf(
                    "https://media.example.com/first.ts".to_string(),
                    5.005,
                    None,
                ),
                Tag::ExtInf(
                    "https://media.example.com/second.ts".to_string(),
                    5.005,
                    None,
                ),
                Tag::ExtInf(
                    "https://media.example.com/third.ts".to_string(),
                    3.003,
                    None,
                ),
                Tag::ExtXEndList,
            ],
        };

        let mut output = Vec::new();
        for tag in &playlist.tags {
            writeln!(output, "{}", tag).unwrap();
        }
        let output = String::from_utf8(output).unwrap();

        let expected = r#"#EXTM3U
#EXT-X-VERSION:6
#EXT-X-TARGETDURATION:10
#EXT-X-MAP:URI="init.mp4"
#EXTINF:5.0050,
https://media.example.com/first.ts
#EXTINF:5.0050,
https://media.example.com/second.ts
#EXTINF:3.0030,
https://media.example.com/third.ts
#EXT-X-ENDLIST
"#;

        assert_eq!(output, expected);
    }

    #[test]
    fn test_parse_playlist_with_program_date_time() {
        let data = r#"
#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:10
#EXT-X-PROGRAM-DATE-TIME:2020-01-01T00:00:00Z
#EXTINF:5.005,
https://media.example.com/first.ts
#EXTINF:5.005,
https://media.example.com/second.ts
#EXTINF:3.003,
https://media.example.com/third.ts
#EXT-X-ENDLIST
"#;

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();
        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(7),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXProgramDateTime("2020-01-01T00:00:00Z".to_string()),
                Tag::ExtInf(
                    "https://media.example.com/first.ts".to_string(),
                    5.005,
                    None
                ),
                Tag::ExtInf(
                    "https://media.example.com/second.ts".to_string(),
                    5.005,
                    None
                ),
                Tag::ExtInf(
                    "https://media.example.com/third.ts".to_string(),
                    3.003,
                    None
                ),
                Tag::ExtXEndList,
            ]
        );
    }

    #[test]
    fn test_write_playlist_with_program_date_time() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(7),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXProgramDateTime("2020-01-01T00:00:00Z".to_string()),
                Tag::ExtInf(
                    "https://media.example.com/first.ts".to_string(),
                    5.005,
                    None,
                ),
                Tag::ExtInf(
                    "https://media.example.com/second.ts".to_string(),
                    5.005,
                    None,
                ),
                Tag::ExtInf(
                    "https://media.example.com/third.ts".to_string(),
                    3.003,
                    None,
                ),
                Tag::ExtXEndList,
            ],
        };

        let mut output = Vec::new();
        for tag in &playlist.tags {
            writeln!(output, "{}", tag).unwrap();
        }
        let output = String::from_utf8(output).unwrap();

        let expected = r#"#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:10
#EXT-X-PROGRAM-DATE-TIME:2020-01-01T00:00:00Z
#EXTINF:5.0050,
https://media.example.com/first.ts
#EXTINF:5.0050,
https://media.example.com/second.ts
#EXTINF:3.0030,
https://media.example.com/third.ts
#EXT-X-ENDLIST
"#;

        assert_eq!(output, expected);
    }

    #[test]
    fn test_parse_playlist_with_daterange() {
        let data = r#"
#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:10
#EXT-X-DATERANGE:PLANNED-DURATION=30.5,ID="ad-1",START-DATE="2020-01-01T00:00:00Z",CLASS="com.example.ad",END-ON-NEXT=NO,SCTE35-OUT="0xFC",X-CUSTOM="metadata"
#EXT-X-ENDLIST
"#;

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();
        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(7),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXDateRange {
                    id: "ad-1".to_string(),
                    class: Some("com.example.ad".to_string()),
                    start_date: "2020-01-01T00:00:00Z".to_string(),
                    end_date: None,
                    duration: None,
                    planned_duration: Some(30.5),
                    end_on_next: Some(false),
                    scte35_cmd: None,
                    scte35_out: Some("0xFC".to_string()),
                    scte35_in: None,
                    client_attributes: vec![("X-CUSTOM".to_string(), "metadata".to_string())],
                },
                Tag::ExtXEndList,
            ]
        );

        assert_eq!(
            playlist.tags[3].to_string(),
            "#EXT-X-DATERANGE:ID=\"ad-1\",START-DATE=\"2020-01-01T00:00:00Z\",CLASS=\"com.example.ad\",PLANNED-DURATION=30.5,END-ON-NEXT=NO,SCTE35-OUT=\"0xFC\",X-CUSTOM=\"metadata\""
        );
    }

    #[test]
    fn test_parse_daterange_requires_start_date() {
        let data = "#EXTM3U\n#EXT-X-DATERANGE:ID=\"ad-1\"\n";

        assert_eq!(
            Playlist::from_reader(data.as_bytes()),
            Err("EXT-X-DATERANGE requires START-DATE".to_string())
        );
    }

    #[test]
    fn test_playlist_builder() {
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .version(7)
            .target_duration(10)
            .extinf("https://media.example.com/first.ts", 5.005, None)
            .extinf("https://media.example.com/second.ts", 5.005, None)
            .extinf("https://media.example.com/third.ts", 3.003, None)
            .end_list()
            .build()
            .unwrap();

        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(7),
                Tag::ExtXTargetDuration(10),
                Tag::ExtInf(
                    "https://media.example.com/first.ts".to_string(),
                    5.005,
                    None
                ),
                Tag::ExtInf(
                    "https://media.example.com/second.ts".to_string(),
                    5.005,
                    None
                ),
                Tag::ExtInf(
                    "https://media.example.com/third.ts".to_string(),
                    3.003,
                    None
                ),
                Tag::ExtXEndList,
            ]
        );

        let mut output = Vec::new();
        for tag in &playlist.tags {
            writeln!(output, "{}", tag).unwrap();
        }
        let output = String::from_utf8(output).unwrap();

        let expected = "#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:10
#EXTINF:5.0050,
https://media.example.com/first.ts
#EXTINF:5.0050,
https://media.example.com/second.ts
#EXTINF:3.0030,
https://media.example.com/third.ts
#EXT-X-ENDLIST
";

        assert_eq!(output, expected);
    }

    #[test]
    fn test_validate_playlist() {
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .version(3)
            .target_duration(10)
            .extinf("https://media.example.com/first.ts", 5.005, None)
            .extinf("https://media.example.com/second.ts", 5.005, None)
            .extinf("https://media.example.com/third.ts", 3.003, None)
            .end_list()
            .build();

        assert!(playlist.is_ok());
    }

    #[test]
    fn test_validate_playlist_missing_extm3u() {
        let playlist = PlaylistBuilder::new()
            .version(3)
            .target_duration(10)
            .extinf("https://media.example.com/first.ts", 5.005, None)
            .extinf("https://media.example.com/second.ts", 5.005, None)
            .extinf("https://media.example.com/third.ts", 3.003, None)
            .end_list()
            .build();

        assert_eq!(playlist, Err(vec![ValidationError::MissingExtM3U]));
    }

    #[test]
    fn test_validate_playlist_invalid_version() {
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .version(8) // Invalid version
            .target_duration(10)
            .extinf("https://media.example.com/first.ts", 5.005, None)
            .extinf("https://media.example.com/second.ts", 5.005, None)
            .extinf("https://media.example.com/third.ts", 3.003, None)
            .end_list()
            .build();

        assert_eq!(playlist, Err(vec![ValidationError::InvalidVersion(8)]));
    }

    #[test]
    fn test_validate_playlist_invalid_duration() {
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .version(3)
            .target_duration(10)
            .extinf("https://media.example.com/first.ts", -5.005, None) // Invalid duration
            .extinf("https://media.example.com/second.ts", 5.005, None)
            .extinf("https://media.example.com/third.ts", 3.003, None)
            .end_list()
            .build();

        assert_eq!(
            playlist,
            Err(vec![ValidationError::InvalidDuration(-5.005)])
        );
    }

    #[test]
    fn test_validate_playlist_invalid_target_duration() {
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .version(3)
            .target_duration(0) // Invalid target duration
            .extinf("https://media.example.com/first.ts", 5.005, None)
            .extinf("https://media.example.com/second.ts", 5.005, None)
            .extinf("https://media.example.com/third.ts", 3.003, None)
            .end_list()
            .build();

        assert_eq!(
            playlist,
            Err(vec![ValidationError::InvalidTargetDuration(0)])
        );
    }

    #[test]
    fn test_validate_playlist_invalid_key_method() {
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .version(3)
            .target_duration(10)
            .key(
                "INVALID-METHOD", // Invalid key method
                Some("https://priv.example.com/key.php?r=52"),
                None,
                None,
                None,
            )
            .extinf("https://media.example.com/first.ts", 5.005, None)
            .extinf("https://media.example.com/second.ts", 5.005, None)
            .extinf("https://media.example.com/third.ts", 3.003, None)
            .end_list()
            .build();

        assert_eq!(
            playlist,
            Err(vec![ValidationError::InvalidKeyMethod(
                "INVALID-METHOD".to_string()
            )])
        );
    }

    #[test]
    fn test_validate_playlist_invalid_map_uri() {
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .version(3)
            .target_duration(10)
            .map("", None) // Invalid map URI
            .extinf("https://media.example.com/first.ts", 5.005, None)
            .extinf("https://media.example.com/second.ts", 5.005, None)
            .extinf("https://media.example.com/third.ts", 3.003, None)
            .end_list()
            .build();

        assert_eq!(playlist, Err(vec![ValidationError::InvalidMapUri]));
    }

    #[test]
    fn test_validate_playlist_invalid_program_date_time() {
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .version(3)
            .target_duration(10)
            .program_date_time("") // Invalid program date time
            .extinf("https://media.example.com/first.ts", 5.005, None)
            .extinf("https://media.example.com/second.ts", 5.005, None)
            .extinf("https://media.example.com/third.ts", 3.003, None)
            .end_list()
            .build();

        assert_eq!(playlist, Err(vec![ValidationError::InvalidProgramDateTime]));
    }

    #[test]
    fn test_validate_extm3u_must_be_first() {
        let playlist = Playlist {
            tags: vec![Tag::Uri("segment.ts".to_string()), Tag::ExtM3U],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![ValidationError::ExtM3UNotFirst])
        );
    }

    #[test]
    fn test_validate_singleton_tags_are_not_duplicated() {
        let playlist = Playlist {
            tags: vec![Tag::ExtM3U, Tag::ExtXVersion(3), Tag::ExtXVersion(3)],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![ValidationError::DuplicateTag(
                "EXT-X-VERSION".to_string()
            )])
        );
    }

    #[test]
    fn test_validate_master_and_media_tags_are_exclusive() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(6),
                Tag::ExtXStreamInf {
                    bandwidth: 1_000_000,
                    average_bandwidth: None,
                    codecs: None,
                    resolution: None,
                    frame_rate: None,
                    audio: None,
                    video: None,
                    subtitle: None,
                    closed_captions: None,
                },
                Tag::Uri("variant.m3u8".to_string()),
                Tag::ExtXTargetDuration(6),
                Tag::ExtInf("segment.ts".to_string(), 6.0, None),
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![ValidationError::MixedPlaylistTypes])
        );
    }

    #[test]
    fn test_validate_stream_inf_requires_uri() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXStreamInf {
                    bandwidth: 1_000_000,
                    average_bandwidth: None,
                    codecs: None,
                    resolution: None,
                    frame_rate: None,
                    audio: None,
                    video: None,
                    subtitle: None,
                    closed_captions: None,
                },
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![ValidationError::MissingVariantUri])
        );
    }

    #[test]
    fn test_validate_version_gates() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(3),
                Tag::ExtXTargetDuration(6),
                Tag::ExtXByteRange("1000@0".to_string()),
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![ValidationError::InsufficientVersion {
                tag: "EXT-X-BYTERANGE".to_string(),
                required: 4,
                actual: 3,
            }])
        );
    }

    #[test]
    fn test_validate_media_playlist_requires_target_duration() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtInf("segment.ts".to_string(), 6.0, None),
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![ValidationError::MissingTargetDuration])
        );
    }

    #[test]
    fn test_validate_segment_duration_does_not_exceed_target() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXTargetDuration(6),
                Tag::ExtInf("segment.ts".to_string(), 6.5, None),
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![ValidationError::SegmentDurationExceedsTarget {
                duration: 6.5,
                target_duration: 6,
            }])
        );
    }

    #[test]
    fn test_validate_target_duration_uses_nearest_integer() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXTargetDuration(6),
                Tag::ExtInf("segment.ts".to_string(), 6.49, None),
            ],
        };

        assert_eq!(playlist.validate(), Ok(()));
    }

    #[test]
    fn test_validate_key_attribute_combinations() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXKey {
                    method: "NONE".to_string(),
                    uri: Some("key.bin".to_string()),
                    iv: None,
                    keyformat: None,
                    keyformatversions: None,
                },
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![ValidationError::InvalidKeyAttributes(
                "METHOD=NONE must not include URI, IV, KEYFORMAT, or KEYFORMATVERSIONS".to_string(),
            )])
        );
    }

    #[test]
    fn test_validate_daterange_end_on_next_constraints() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(6),
                Tag::ExtXTargetDuration(6),
                Tag::ExtXDateRange {
                    id: "ad-1".to_string(),
                    class: None,
                    start_date: "2024-01-01T00:00:00Z".to_string(),
                    end_date: None,
                    duration: Some(30.0),
                    planned_duration: None,
                    end_on_next: Some(true),
                    scte35_cmd: None,
                    scte35_out: None,
                    scte35_in: None,
                    client_attributes: Vec::new(),
                },
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![ValidationError::InvalidDateRangeAttributes(
                "END-ON-NEXT requires CLASS and forbids END-DATE and DURATION".to_string(),
            )])
        );
    }

    #[test]
    fn test_validate_stream_rendition_group_references() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(6),
                Tag::ExtXStreamInf {
                    bandwidth: 1_000_000,
                    average_bandwidth: None,
                    codecs: None,
                    resolution: None,
                    frame_rate: None,
                    audio: Some("audio-main".to_string()),
                    video: None,
                    subtitle: None,
                    closed_captions: None,
                },
                Tag::Uri("variant.m3u8".to_string()),
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![ValidationError::UnresolvedRenditionGroup {
                attribute: "AUDIO".to_string(),
                group_id: "audio-main".to_string(),
            }])
        );
    }

    #[test]
    fn test_validate_stream_rendition_group_reference_resolves() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(6),
                Tag::ExtXMedia {
                    type_: "AUDIO".to_string(),
                    group_id: "audio-main".to_string(),
                    name: Some("English".to_string()),
                    uri: Some("audio.m3u8".to_string()),
                    default: Some(true),
                    autoplay: Some(true),
                    characteristics: None,
                    language: Some("en".to_string()),
                    instream_id: None,
                    language_codec: None,
                    forced: None,
                    channels: None,
                },
                Tag::ExtXStreamInf {
                    bandwidth: 1_000_000,
                    average_bandwidth: None,
                    codecs: None,
                    resolution: None,
                    frame_rate: None,
                    audio: Some("audio-main".to_string()),
                    video: None,
                    subtitle: None,
                    closed_captions: None,
                },
                Tag::Uri("variant.m3u8".to_string()),
            ],
        };

        assert_eq!(playlist.validate(), Ok(()));
    }
}
