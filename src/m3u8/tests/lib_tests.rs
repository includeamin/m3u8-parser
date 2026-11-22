#[cfg(test)]
mod tests {
    use crate::m3u8::datetime::DateTime;
    use crate::m3u8::error::{ParseError, SubstitutionError, SyntaxError};
    use crate::m3u8::playlist::builder::PlaylistBuilder;
    use crate::m3u8::playlist::Playlist;
    use crate::m3u8::tags::{
        AttributeValue, ByteRange, Codecs, Key, KeyMethod, Map, PlaylistType, Resolution, Tag,
    };
    use crate::m3u8::validation::ValidationError;
    use std::collections::HashMap;
    use std::io::Write;

    fn syntax_error(data: &str) -> (usize, SyntaxError) {
        match Playlist::from_reader(data.as_bytes()) {
            Err(ParseError::Syntax { line, kind }) => (line, kind),
            other => panic!("expected a syntax error, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_simple_playlist() {
        let data = r#"
#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:10
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
                Tag::ExtInf {
                    uri: "https://media.example.com/first.ts".to_string(),
                    duration: 5.005,
                    title: None
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/second.ts".to_string(),
                    duration: 5.005,
                    title: None
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/third.ts".to_string(),
                    duration: 3.003,
                    title: None
                },
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
                Tag::ExtXKey(Key {
                    method: KeyMethod::Aes128,
                    uri: Some("key.bin".to_string()),
                    iv: Some("0x1234".to_string()),
                    keyformat: Some("identity".to_string()),
                    keyformatversions: None,
                }),
                Tag::ExtXMap(Map {
                    uri: "init.mp4".to_string(),
                    byterange: Some(ByteRange {
                        length: 720,
                        offset: Some(0)
                    }),
                }),
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
            syntax_error(data),
            (
                2,
                SyntaxError::MissingAttribute {
                    tag: "EXT-X-MEDIA",
                    attribute: "NAME"
                }
            )
        );
    }

    #[test]
    fn test_validate_media_requires_name() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(6),
                Tag::ExtXMedia {
                    pathway_id: None,
                    type_: "AUDIO".to_string(),
                    group_id: "audio".to_string(),
                    name: None,
                    uri: None,
                    default: None,
                    autoselect: None,
                    characteristics: None,
                    language: None,
                    instream_id: None,
                    language_codec: None,
                    forced: None,
                    channels: None,
                    extra_attributes: Vec::new(),
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
            Tag::ExtInf { uri, duration, title: Some(title) } if uri == "segment.ts" && *duration == 6.0 && title == "segment"
        ));
    }

    #[test]
    fn test_parse_extension_tag_variants() {
        let data = "#EXTM3U
#EXT-X-BITRATE:1200000
#EXT-X-SERVER-CONTROL:CAN-SKIP-UNTIL=24,CAN-SKIP-DATERANGES=YES,HOLD-BACK=12,PART-HOLD-BACK=1.5,CAN-BLOCK-RELOAD=YES
#EXT-X-PART-INF:PART-TARGET=0.5
#EXT-X-SKIP:SKIPPED-SEGMENTS=3,RECENTLY-REMOVED-DATERANGES=\"ad-1\tad-2\"
#EXT-X-PART:DURATION=0.5,URI=\"part0.ts\",INDEPENDENT=YES,BYTERANGE=\"100@0\",GAP=YES
#EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"next.ts\",BYTERANGE-START=0,BYTERANGE-LENGTH=100
#EXT-X-RENDITION-REPORT:URI=\"../1M/waitForMSN.php\",LAST-MSN=273,LAST-PART=2";

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();

        assert_eq!(
            playlist.tags[1..],
            [
                Tag::ExtXBitrate(1_200_000),
                Tag::ExtXServerControl {
                    can_skip_until: Some(24.0),
                    can_skip_dateranges: Some(true),
                    hold_back: Some(12.0),
                    part_hold_back: Some(1.5),
                    can_block_reload: Some(true),
                },
                Tag::ExtXPartInf {
                    part_target_duration: 0.5,
                },
                Tag::ExtXSkip {
                    skipped_segments: 3,
                    recently_removed_dateranges: Some("ad-1\tad-2".to_string()),
                },
                Tag::ExtXPart {
                    uri: "part0.ts".to_string(),
                    duration: 0.5,
                    independent: Some(true),
                    byterange: Some(ByteRange {
                        length: 100,
                        offset: Some(0)
                    }),
                    gap: Some(true),
                },
                Tag::ExtXPreloadHint {
                    type_: "PART".to_string(),
                    uri: "next.ts".to_string(),
                    byterange_start: Some(0),
                    byterange_length: Some(100),
                },
                Tag::ExtXRenditionReport {
                    uri: "../1M/waitForMSN.php".to_string(),
                    last_msn: Some(273),
                    last_part: Some(2),
                },
            ]
        );

        let output: Vec<String> = playlist.tags.iter().map(Tag::to_string).collect();
        assert_eq!(output.join("\n"), data);
    }

    #[test]
    fn test_parse_extinf_allows_tags_and_comments_before_uri() {
        let data = "#EXTM3U
#EXT-X-VERSION:4
#EXT-X-TARGETDURATION:10
#EXTINF:10,
#EXT-X-BYTERANGE:75232@0
# a comment
seg.ts
#EXTINF:10,
#EXT-X-DISCONTINUITY
seg2.ts
";

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();

        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(4),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXByteRange(ByteRange {
                    length: 75232,
                    offset: Some(0)
                }),
                Tag::Unknown(" a comment".to_string()),
                Tag::ExtInf {
                    uri: "seg.ts".to_string(),
                    duration: 10.0,
                    title: None
                },
                Tag::ExtXDiscontinuity,
                Tag::ExtInf {
                    uri: "seg2.ts".to_string(),
                    duration: 10.0,
                    title: None
                },
            ]
        );
        assert_eq!(playlist.validate(), Ok(()));

        let segments = playlist.media_segments();
        assert_eq!(
            segments[0].byterange,
            Some(ByteRange {
                length: 75232,
                offset: Some(0)
            })
        );
        assert_eq!(segments[1].byterange, None);
    }

    #[test]
    fn test_parse_consecutive_extinf_reports_first_extinf_line() {
        let data = "#EXTM3U\n#EXTINF:6.0,\n#EXTINF:6.0,\nsegment.ts\n";

        assert_eq!(syntax_error(data), (2, SyntaxError::MissingUriAfterExtInf));
    }

    #[test]
    fn test_parse_skips_utf8_bom() {
        let data = "\u{feff}#EXTM3U\n#EXT-X-VERSION:3\n";

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();

        assert_eq!(playlist.tags, vec![Tag::ExtM3U, Tag::ExtXVersion(3)]);
    }

    #[test]
    fn test_write_extinf_with_title_has_no_leading_space_before_uri() {
        let tag = Tag::ExtInf {
            uri: "segment.ts".to_string(),
            duration: 6.0,
            title: Some("title".to_string()),
        };

        assert_eq!(tag.to_string(), "#EXTINF:6,title\nsegment.ts");
    }

    #[test]
    fn test_daterange_preserves_unknown_attributes_and_value_quoting() {
        let data = "#EXTM3U
#EXT-X-DATERANGE:ID=\"splice-6FFFFFF0\",START-DATE=\"2014-03-05T11:15:00Z\",CUE=\"PRE,ONCE\",SCTE35-OUT=0xFC002F0000,X-HEX=0x1A,X-FLOAT=1.5,X-STR=\"text\",FUTURE-ATTR=7";

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();

        assert_eq!(
            playlist.tags[1],
            Tag::ExtXDateRange {
                id: "splice-6FFFFFF0".to_string(),
                class: None,
                start_date: "2014-03-05T11:15:00Z".to_string(),
                cue: Some("PRE,ONCE".to_string()),
                end_date: None,
                duration: None,
                planned_duration: None,
                end_on_next: None,
                scte35_cmd: None,
                scte35_out: Some("0xFC002F0000".to_string()),
                scte35_in: None,
                extra_attributes: vec![
                    (
                        "X-HEX".to_string(),
                        AttributeValue::Unquoted("0x1A".to_string())
                    ),
                    (
                        "X-FLOAT".to_string(),
                        AttributeValue::Unquoted("1.5".to_string())
                    ),
                    (
                        "X-STR".to_string(),
                        AttributeValue::Quoted("text".to_string())
                    ),
                    (
                        "FUTURE-ATTR".to_string(),
                        AttributeValue::Unquoted("7".to_string())
                    ),
                ],
            }
        );

        let output: Vec<String> = playlist.tags.iter().map(Tag::to_string).collect();
        assert_eq!(output.join("\n"), data);
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
        assert_eq!(
            segments[0].byterange,
            Some(ByteRange {
                length: 1000,
                offset: Some(0)
            })
        );
        assert_eq!(
            segments[0].program_date_time,
            Some("2024-01-01T00:00:00Z".parse().unwrap())
        );
        assert!(segments[0].gap);
        assert_eq!(
            segments[0].key.as_ref().map(|key| key.method.as_str()),
            Some("AES-128")
        );
        assert_eq!(
            segments[0].map.as_ref().map(|map| map.uri.as_str()),
            Some("init.mp4")
        );

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

        assert_eq!(syntax_error(data), (2, SyntaxError::MissingUriAfterExtInf));
    }

    #[test]
    fn test_write_simple_playlist() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(7),
                Tag::ExtXTargetDuration(10),
                Tag::ExtInf {
                    uri: "https://media.example.com/first.ts".to_string(),
                    duration: 5.005,
                    title: None,
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/second.ts".to_string(),
                    duration: 5.005,
                    title: None,
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/third.ts".to_string(),
                    duration: 3.003,
                    title: None,
                },
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
#EXTINF:5.005,
https://media.example.com/first.ts
#EXTINF:5.005,
https://media.example.com/second.ts
#EXTINF:3.003,
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
                Tag::ExtXKey(Key {
                    method: KeyMethod::Aes128,
                    uri: Some("https://priv.example.com/key.php?r=52".to_string()),
                    iv: None,
                    keyformat: None,
                    keyformatversions: None,
                }),
                Tag::ExtInf {
                    uri: "https://media.example.com/first.ts".to_string(),
                    duration: 5.005,
                    title: None
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/second.ts".to_string(),
                    duration: 5.005,
                    title: None
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/third.ts".to_string(),
                    duration: 3.003,
                    title: None
                },
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
                Tag::ExtXKey(Key {
                    method: KeyMethod::Aes128,
                    uri: Some("https://priv.example.com/key.php?r=52".to_string()),
                    iv: None,
                    keyformat: None,
                    keyformatversions: None,
                }),
                Tag::ExtInf {
                    uri: "https://media.example.com/first.ts".to_string(),
                    duration: 5.005,
                    title: None,
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/second.ts".to_string(),
                    duration: 5.005,
                    title: None,
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/third.ts".to_string(),
                    duration: 3.003,
                    title: None,
                },
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
#EXTINF:5.005,
https://media.example.com/first.ts
#EXTINF:5.005,
https://media.example.com/second.ts
#EXTINF:3.003,
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
                Tag::ExtXMap(Map {
                    uri: "init.mp4".to_string(),
                    byterange: None,
                }),
                Tag::ExtInf {
                    uri: "https://media.example.com/first.ts".to_string(),
                    duration: 5.005,
                    title: None
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/second.ts".to_string(),
                    duration: 5.005,
                    title: None
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/third.ts".to_string(),
                    duration: 3.003,
                    title: None
                },
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
                Tag::ExtXMap(Map {
                    uri: "init.mp4".to_string(),
                    byterange: None,
                }),
                Tag::ExtInf {
                    uri: "https://media.example.com/first.ts".to_string(),
                    duration: 5.005,
                    title: None,
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/second.ts".to_string(),
                    duration: 5.005,
                    title: None,
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/third.ts".to_string(),
                    duration: 3.003,
                    title: None,
                },
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
#EXTINF:5.005,
https://media.example.com/first.ts
#EXTINF:5.005,
https://media.example.com/second.ts
#EXTINF:3.003,
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
                Tag::ExtXProgramDateTime("2020-01-01T00:00:00Z".parse().unwrap()),
                Tag::ExtInf {
                    uri: "https://media.example.com/first.ts".to_string(),
                    duration: 5.005,
                    title: None
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/second.ts".to_string(),
                    duration: 5.005,
                    title: None
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/third.ts".to_string(),
                    duration: 3.003,
                    title: None
                },
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
                Tag::ExtXProgramDateTime("2020-01-01T00:00:00Z".parse().unwrap()),
                Tag::ExtInf {
                    uri: "https://media.example.com/first.ts".to_string(),
                    duration: 5.005,
                    title: None,
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/second.ts".to_string(),
                    duration: 5.005,
                    title: None,
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/third.ts".to_string(),
                    duration: 3.003,
                    title: None,
                },
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
#EXTINF:5.005,
https://media.example.com/first.ts
#EXTINF:5.005,
https://media.example.com/second.ts
#EXTINF:3.003,
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
                    cue: None,
                    end_date: None,
                    duration: None,
                    planned_duration: Some(30.5),
                    end_on_next: Some(false),
                    scte35_cmd: None,
                    scte35_out: Some("0xFC".to_string()),
                    scte35_in: None,
                    extra_attributes: vec![(
                        "X-CUSTOM".to_string(),
                        AttributeValue::Quoted("metadata".to_string())
                    )],
                },
                Tag::ExtXEndList,
            ]
        );

        assert_eq!(
            playlist.tags[3].to_string(),
            "#EXT-X-DATERANGE:ID=\"ad-1\",START-DATE=\"2020-01-01T00:00:00Z\",CLASS=\"com.example.ad\",PLANNED-DURATION=30.5,END-ON-NEXT=NO,SCTE35-OUT=0xFC,X-CUSTOM=\"metadata\""
        );
    }

    #[test]
    fn test_parse_daterange_requires_start_date() {
        let data = "#EXTM3U\n#EXT-X-DATERANGE:ID=\"ad-1\"\n";

        assert_eq!(
            syntax_error(data),
            (
                2,
                SyntaxError::MissingAttribute {
                    tag: "EXT-X-DATERANGE",
                    attribute: "START-DATE"
                }
            )
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
                Tag::ExtInf {
                    uri: "https://media.example.com/first.ts".to_string(),
                    duration: 5.005,
                    title: None
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/second.ts".to_string(),
                    duration: 5.005,
                    title: None
                },
                Tag::ExtInf {
                    uri: "https://media.example.com/third.ts".to_string(),
                    duration: 3.003,
                    title: None
                },
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
#EXTINF:5.005,
https://media.example.com/first.ts
#EXTINF:5.005,
https://media.example.com/second.ts
#EXTINF:3.003,
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
            .version(13) // Invalid version
            .target_duration(10)
            .extinf("https://media.example.com/first.ts", 5.005, None)
            .extinf("https://media.example.com/second.ts", 5.005, None)
            .extinf("https://media.example.com/third.ts", 3.003, None)
            .end_list()
            .build();

        assert_eq!(playlist, Err(vec![ValidationError::InvalidVersion(13)]));
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
                KeyMethod::from("INVALID-METHOD"), // Invalid key method
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
            .version(6)
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
    fn test_validate_extm3u_must_be_first() {
        let playlist = Playlist {
            tags: vec![Tag::ExtXIndependentSegments, Tag::ExtM3U],
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
                    pathway_id: None,
                    bandwidth: 1_000_000,
                    average_bandwidth: None,
                    codecs: None,
                    resolution: None,
                    frame_rate: None,
                    audio: None,
                    video: None,
                    subtitle: None,
                    closed_captions: None,
                    extra_attributes: Vec::new(),
                },
                Tag::Uri("variant.m3u8".to_string()),
                Tag::ExtXTargetDuration(6),
                Tag::ExtInf {
                    uri: "segment.ts".to_string(),
                    duration: 6.0,
                    title: None,
                },
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
                    pathway_id: None,
                    bandwidth: 1_000_000,
                    average_bandwidth: None,
                    codecs: None,
                    resolution: None,
                    frame_rate: None,
                    audio: None,
                    video: None,
                    subtitle: None,
                    closed_captions: None,
                    extra_attributes: Vec::new(),
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
                Tag::ExtXByteRange(ByteRange {
                    length: 1000,
                    offset: Some(0),
                }),
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
                Tag::ExtInf {
                    uri: "segment.ts".to_string(),
                    duration: 6.0,
                    title: None,
                },
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
                Tag::ExtXVersion(3),
                Tag::ExtXTargetDuration(6),
                Tag::ExtInf {
                    uri: "segment.ts".to_string(),
                    duration: 6.5,
                    title: None,
                },
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
                Tag::ExtXVersion(3),
                Tag::ExtXTargetDuration(6),
                Tag::ExtInf {
                    uri: "segment.ts".to_string(),
                    duration: 6.49,
                    title: None,
                },
            ],
        };

        assert_eq!(playlist.validate(), Ok(()));
    }

    #[test]
    fn test_validate_key_attribute_combinations() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXTargetDuration(6),
                Tag::ExtXKey(Key {
                    method: KeyMethod::None,
                    uri: Some("key.bin".to_string()),
                    iv: None,
                    keyformat: None,
                    keyformatversions: None,
                }),
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![ValidationError::InvalidKeyAttributes(
                "EXT-X-KEY METHOD=NONE must not include URI, IV, KEYFORMAT, or KEYFORMATVERSIONS"
                    .to_string(),
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
                    cue: None,
                    end_date: None,
                    duration: Some(30.0),
                    planned_duration: None,
                    end_on_next: Some(true),
                    scte35_cmd: None,
                    scte35_out: None,
                    scte35_in: None,
                    extra_attributes: Vec::new(),
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
                    pathway_id: None,
                    bandwidth: 1_000_000,
                    average_bandwidth: None,
                    codecs: None,
                    resolution: None,
                    frame_rate: None,
                    audio: Some("audio-main".to_string()),
                    video: None,
                    subtitle: None,
                    closed_captions: None,
                    extra_attributes: Vec::new(),
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
                    pathway_id: None,
                    type_: "AUDIO".to_string(),
                    group_id: "audio-main".to_string(),
                    name: Some("English".to_string()),
                    uri: Some("audio.m3u8".to_string()),
                    default: Some(true),
                    autoselect: Some(true),
                    characteristics: None,
                    language: Some("en".to_string()),
                    instream_id: None,
                    language_codec: None,
                    forced: None,
                    channels: None,
                    extra_attributes: Vec::new(),
                },
                Tag::ExtXStreamInf {
                    pathway_id: None,
                    bandwidth: 1_000_000,
                    average_bandwidth: None,
                    codecs: None,
                    resolution: None,
                    frame_rate: None,
                    audio: Some("audio-main".to_string()),
                    video: None,
                    subtitle: None,
                    closed_captions: None,
                    extra_attributes: Vec::new(),
                },
                Tag::Uri("variant.m3u8".to_string()),
            ],
        };

        assert_eq!(playlist.validate(), Ok(()));
    }

    fn assert_round_trip(data: &str) -> Playlist {
        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();
        let output: Vec<String> = playlist.tags.iter().map(Tag::to_string).collect();
        assert_eq!(output.join("\n"), data.trim_end());
        playlist
    }

    #[test]
    fn test_round_trip_master_playlist() {
        let playlist = assert_round_trip(
            r#"#EXTM3U
#EXT-X-VERSION:6
#EXT-X-INDEPENDENT-SEGMENTS
#EXT-X-SESSION-DATA:DATA-ID="com.example.title",VALUE="Example",LANGUAGE="en"
#EXT-X-SESSION-KEY:METHOD=SAMPLE-AES,URI="skd://key",KEYFORMAT="com.apple.streamingkeydelivery",KEYFORMATVERSIONS="1"
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="aac",URI="audio/en.m3u8",NAME="English",DEFAULT=YES,AUTOSELECT=YES,LANGUAGE="en",CHANNELS="2"
#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID="subs",URI="subs/en.m3u8",NAME="English",FORCED=NO,CHARACTERISTICS="public.accessibility.describes-music-and-sound",LANGUAGE="en"
#EXT-X-STREAM-INF:BANDWIDTH=2177116,AVERAGE-BANDWIDTH=2168183,CODECS="avc1.640020,mp4a.40.2",RESOLUTION=960x540,FRAME-RATE=60,AUDIO="aac",SUBTITLES="subs",CLOSED-CAPTIONS=NONE
v5/prog_index.m3u8
#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=187492,CODECS="avc1.640020",RESOLUTION=960x540,URI="v5/iframe_index.m3u8"
"#,
        );

        assert_eq!(playlist.validate(), Ok(()));
    }

    #[test]
    fn test_round_trip_media_playlist() {
        let playlist = assert_round_trip(
            r#"#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:10
#EXT-X-MEDIA-SEQUENCE:100
#EXT-X-PLAYLIST-TYPE:VOD
#EXT-X-MAP:URI="init.mp4",BYTERANGE="720@0"
#EXT-X-KEY:METHOD=AES-128,URI="https://example.com/key",IV=0x0123456789ABCDEF0123456789ABCDEF,KEYFORMAT="identity",KEYFORMATVERSIONS="1"
#EXT-X-PROGRAM-DATE-TIME:2024-01-01T00:00:00.000Z
#EXTINF:9.009,first
first.mp4
#EXT-X-DISCONTINUITY
#EXT-X-KEY:METHOD=NONE
#EXT-X-BYTERANGE:1000@0
#EXTINF:10,
second.mp4
#EXT-X-ENDLIST
"#,
        );

        assert_eq!(playlist.validate(), Ok(()));
    }

    #[test]
    fn test_media_segments_track_sequence_discontinuity_and_key_reset() {
        let data = "#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:10
#EXT-X-MEDIA-SEQUENCE:100
#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"
#EXTINF:9.5,
first.ts
#EXT-X-DISCONTINUITY
#EXT-X-KEY:METHOD=NONE
#EXTINF:9.5,
second.ts
#EXTINF:9.5,
third.ts
";
        let segments = Playlist::from_reader(data.as_bytes())
            .unwrap()
            .media_segments();

        let sequences: Vec<u64> = segments.iter().map(|segment| segment.sequence).collect();
        assert_eq!(sequences, vec![100, 101, 102]);
        let discontinuities: Vec<bool> = segments
            .iter()
            .map(|segment| segment.discontinuity)
            .collect();
        assert_eq!(discontinuities, vec![false, true, false]);
        assert_eq!(
            segments[0].key,
            Some(Key {
                method: KeyMethod::Aes128,
                uri: Some("key.bin".to_string()),
                iv: None,
                keyformat: None,
                keyformatversions: None,
            })
        );
        assert_eq!(segments[1].key, None);
        assert_eq!(segments[2].key, None);
    }

    #[test]
    fn test_validate_reports_tag_and_playlist_errors_together() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXTargetDuration(10),
                Tag::ExtXTargetDuration(10),
                Tag::ExtInf {
                    uri: "segment.ts".to_string(),
                    duration: -1.0,
                    title: None,
                },
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![
                ValidationError::InvalidDuration(-1.0),
                ValidationError::DuplicateTag("EXT-X-TARGETDURATION".to_string()),
            ])
        );
    }

    #[test]
    fn test_validate_version_gates_for_iv_decimal_extinf_and_keyformatversions() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXTargetDuration(10),
                Tag::ExtXKey(Key {
                    method: KeyMethod::Aes128,
                    uri: Some("key.bin".to_string()),
                    iv: Some("0x01".to_string()),
                    keyformat: None,
                    keyformatversions: None,
                }),
                Tag::ExtXKey(Key {
                    method: KeyMethod::Aes128,
                    uri: Some("key.bin".to_string()),
                    iv: None,
                    keyformat: None,
                    keyformatversions: Some("1".to_string()),
                }),
                Tag::ExtInf {
                    uri: "segment.ts".to_string(),
                    duration: 9.5,
                    title: None,
                },
            ],
        };

        let insufficient = |tag: &str, required| ValidationError::InsufficientVersion {
            tag: tag.to_string(),
            required,
            actual: 1,
        };
        assert_eq!(
            playlist.validate(),
            Err(vec![
                insufficient("EXT-X-KEY", 2),
                insufficient("EXT-X-KEY", 5),
                insufficient("EXTINF", 3),
            ])
        );
    }

    #[test]
    fn test_validate_low_latency_version_gates() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(9),
                Tag::ExtXTargetDuration(4),
                Tag::ExtXDefine("NAME=\"base\",VALUE=\"https://example.com\"".to_string()),
                Tag::ExtXSkip {
                    skipped_segments: 3,
                    recently_removed_dateranges: Some("ad-1".to_string()),
                },
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![ValidationError::InsufficientVersion {
                tag: "EXT-X-SKIP".to_string(),
                required: 10,
                actual: 9,
            }])
        );
    }

    #[test]
    fn test_validate_session_key_rules() {
        let session_key = |method: &str, uri: Option<&str>| {
            Tag::ExtXSessionKey(Key {
                method: method.into(),
                uri: uri.map(str::to_string),
                iv: None,
                keyformat: None,
                keyformatversions: None,
            })
        };
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                session_key("NONE", None),
                session_key("AES-256", Some("key.bin")),
                session_key("AES-128", None),
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![
                ValidationError::InvalidKeyAttributes(
                    "EXT-X-SESSION-KEY METHOD must not be NONE".to_string()
                ),
                ValidationError::InvalidKeyMethod("AES-256".to_string()),
                ValidationError::InvalidKeyAttributes(
                    "EXT-X-SESSION-KEY encryption methods require URI".to_string()
                ),
            ])
        );
    }

    #[test]
    fn test_validate_closed_captions_media_rules() {
        let media = |type_: &str, name: &str, uri: Option<&str>, instream_id: Option<&str>| {
            Tag::ExtXMedia {
                pathway_id: None,
                type_: type_.to_string(),
                group_id: "group".to_string(),
                name: Some(name.to_string()),
                uri: uri.map(str::to_string),
                default: None,
                autoselect: None,
                characteristics: None,
                language: None,
                instream_id: instream_id.map(str::to_string),
                language_codec: None,
                forced: None,
                channels: None,
                extra_attributes: Vec::new(),
            }
        };
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(6),
                media("CLOSED-CAPTIONS", "a", Some("cc.m3u8"), None),
                media("AUDIO", "b", None, Some("CC1")),
                media("CLOSED-CAPTIONS", "c", None, Some("CC1")),
            ],
        };

        let invalid = |reason: &str| ValidationError::InvalidMediaAttributes(reason.to_string());
        assert_eq!(
            playlist.validate(),
            Err(vec![
                invalid("TYPE=CLOSED-CAPTIONS requires INSTREAM-ID"),
                invalid("TYPE=CLOSED-CAPTIONS must not include URI"),
                invalid("INSTREAM-ID is only allowed with TYPE=CLOSED-CAPTIONS"),
            ])
        );
    }

    #[test]
    fn test_parse_does_not_confuse_tag_name_prefixes() {
        let data =
            "#EXTM3U\n#EXT-X-PARTX:URI=\"a\"\n#EXT-X-ENDLIST:junk\n#EXT-X-PART-INF:PART-TARGET=1\n";

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();

        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::Unknown("EXT-X-PARTX:URI=\"a\"".to_string()),
                Tag::Unknown("EXT-X-ENDLIST:junk".to_string()),
                Tag::ExtXPartInf {
                    part_target_duration: 1.0
                },
            ]
        );
    }

    #[test]
    fn test_parse_errors_are_typed_with_line_numbers() {
        assert_eq!(
            syntax_error("#EXTM3U\n\n#EXT-X-VERSION:abc\n"),
            (
                3,
                SyntaxError::InvalidTagValue {
                    tag: "EXT-X-VERSION",
                    value: "abc".to_string()
                }
            )
        );
        assert_eq!(
            syntax_error("#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=fast\n"),
            (
                2,
                SyntaxError::InvalidAttributeValue {
                    tag: "EXT-X-STREAM-INF",
                    attribute: "BANDWIDTH",
                    value: "fast".to_string()
                }
            )
        );
        assert_eq!(
            syntax_error("#EXTM3U\n#EXTINF:abc,\nsegment.ts\n"),
            (2, SyntaxError::InvalidExtInf("abc,".to_string()))
        );

        let error = Playlist::from_reader("#EXTM3U\n#EXT-X-MAP:URI=\"init".as_bytes()).unwrap_err();
        assert_eq!(
            error.to_string(),
            "line 2: malformed attribute list (unterminated quoted string): URI=\"init"
        );
    }

    #[test]
    fn test_from_file_reports_io_errors() {
        let error = Playlist::from_file("does/not/exist.m3u8").unwrap_err();

        assert!(matches!(error, ParseError::Io(_)));
        assert_eq!(error.line(), None);
    }

    #[test]
    fn test_parse_attributes_handles_quoted_commas() {
        use crate::m3u8::parser::parse_attributes;

        let attributes =
            parse_attributes(r#"BANDWIDTH=1000,CODECS="avc1.4d401f,mp4a.40.2""#).unwrap();

        assert_eq!(attributes.len(), 2);
        assert_eq!(attributes["CODECS"], "avc1.4d401f,mp4a.40.2");
        assert!(parse_attributes(r#"URI="unterminated"#).is_err());
    }

    fn media_tag(type_: &str, instream_id: Option<&str>) -> Tag {
        Tag::ExtXMedia {
            pathway_id: None,
            type_: type_.to_string(),
            group_id: "group".to_string(),
            name: Some("name".to_string()),
            uri: None,
            default: None,
            autoselect: None,
            characteristics: None,
            language: None,
            instream_id: instream_id.map(str::to_string),
            language_codec: None,
            forced: None,
            channels: None,
            extra_attributes: Vec::new(),
        }
    }

    #[test]
    fn test_validate_does_not_gate_tags_the_spec_allows_in_version_1() {
        let master = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXIndependentSegments,
                Tag::ExtXStart {
                    time_offset: 0.0,
                    precise: None,
                },
                media_tag("AUDIO", None),
                Tag::ExtXStreamInf {
                    pathway_id: None,
                    bandwidth: 1_000_000,
                    average_bandwidth: Some(900_000),
                    codecs: None,
                    resolution: None,
                    frame_rate: None,
                    audio: Some("group".to_string()),
                    video: None,
                    subtitle: None,
                    closed_captions: None,
                    extra_attributes: Vec::new(),
                },
                Tag::Uri("variant.m3u8".to_string()),
            ],
        };
        let media = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXTargetDuration(6),
                Tag::ExtXDateRange {
                    id: "ad".to_string(),
                    class: None,
                    start_date: "2024-01-01T00:00:00Z".to_string(),
                    cue: None,
                    end_date: None,
                    duration: None,
                    planned_duration: None,
                    end_on_next: None,
                    scte35_cmd: None,
                    scte35_out: None,
                    scte35_in: None,
                    extra_attributes: Vec::new(),
                },
                Tag::ExtInf {
                    uri: "segment.ts".to_string(),
                    duration: 6.0,
                    title: None,
                },
            ],
        };

        assert_eq!(master.validate(), Ok(()));
        assert_eq!(media.validate(), Ok(()));
    }

    #[test]
    fn test_validate_map_version_depends_on_i_frames_only() {
        let map = Tag::ExtXMap(Map {
            uri: "init.mp4".to_string(),
            byterange: None,
        });
        let regular = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(5),
                Tag::ExtXTargetDuration(6),
                map.clone(),
                Tag::ExtInf {
                    uri: "segment.mp4".to_string(),
                    duration: 6.0,
                    title: None,
                },
            ],
        };
        let i_frames = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(5),
                Tag::ExtXTargetDuration(6),
                Tag::ExtXIFramesOnly,
                map,
                Tag::ExtInf {
                    uri: "segment.mp4".to_string(),
                    duration: 6.0,
                    title: None,
                },
            ],
        };

        assert_eq!(
            regular.validate(),
            Err(vec![ValidationError::InsufficientVersion {
                tag: "EXT-X-MAP".to_string(),
                required: 6,
                actual: 5,
            }])
        );
        assert_eq!(i_frames.validate(), Ok(()));
    }

    #[test]
    fn test_validate_version_gates_for_sample_aes_service_queryparam_and_req() {
        let master = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(6),
                media_tag("CLOSED-CAPTIONS", Some("SERVICE1")),
                Tag::ExtXDefine("QUERYPARAM=\"token\"".to_string()),
            ],
        };
        let media = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(4),
                Tag::ExtXTargetDuration(6),
                Tag::ExtXKey(Key {
                    method: KeyMethod::SampleAes,
                    uri: Some("key.bin".to_string()),
                    iv: None,
                    keyformat: None,
                    keyformatversions: None,
                }),
                Tag::ExtXDateRange {
                    id: "ad".to_string(),
                    class: None,
                    start_date: "2024-01-01T00:00:00Z".to_string(),
                    cue: None,
                    end_date: None,
                    duration: None,
                    planned_duration: None,
                    end_on_next: None,
                    scte35_cmd: None,
                    scte35_out: None,
                    scte35_in: None,
                    extra_attributes: vec![(
                        "REQ-VIDEO-LAYOUT".to_string(),
                        AttributeValue::Quoted("CH-STEREO".to_string()),
                    )],
                },
                Tag::ExtInf {
                    uri: "segment.ts".to_string(),
                    duration: 6.0,
                    title: None,
                },
            ],
        };

        let insufficient = |tag: &str, required, actual| ValidationError::InsufficientVersion {
            tag: tag.to_string(),
            required,
            actual,
        };
        assert_eq!(
            master.validate(),
            Err(vec![
                insufficient("EXT-X-MEDIA", 7, 6),
                insufficient("EXT-X-DEFINE", 11, 6),
            ])
        );
        assert_eq!(
            media.validate(),
            Err(vec![
                insufficient("EXT-X-KEY", 5, 4),
                insufficient("EXT-X-DATERANGE", 12, 4),
            ])
        );
    }

    #[test]
    fn test_playlist_writes_to_strings_and_writers() {
        let data = "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:10\n#EXTINF:9.5,first\nfirst.ts\n#EXT-X-ENDLIST\n";
        let playlist: Playlist = data.parse().unwrap();

        assert_eq!(playlist.to_string(), data);

        let mut buffer = Vec::new();
        playlist.write_to(&mut buffer).unwrap();
        assert_eq!(String::from_utf8(buffer).unwrap(), data);
    }

    #[test]
    fn test_parse_typed_byte_ranges_and_time_offset() {
        let data = "#EXTM3U
#EXT-X-VERSION:6
#EXT-X-TARGETDURATION:10
#EXT-X-START:TIME-OFFSET=-12.5,PRECISE=YES
#EXT-X-MAP:URI=\"init.mp4\",BYTERANGE=\"720@0\"
#EXT-X-BYTERANGE:1000@720
#EXTINF:10,
media.mp4
#EXT-X-BYTERANGE:2000
#EXTINF:10,
media.mp4
";
        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();

        assert_eq!(
            playlist.tags[3],
            Tag::ExtXStart {
                time_offset: -12.5,
                precise: Some(true)
            }
        );
        assert_eq!(
            playlist.tags[4],
            Tag::ExtXMap(Map {
                uri: "init.mp4".to_string(),
                byterange: Some(ByteRange {
                    length: 720,
                    offset: Some(0)
                }),
            })
        );
        let segments = playlist.media_segments();
        assert_eq!(
            segments[0].byterange,
            Some(ByteRange {
                length: 1000,
                offset: Some(720)
            })
        );
        assert_eq!(
            segments[1].byterange,
            Some(ByteRange {
                length: 2000,
                offset: None
            })
        );
        assert_eq!(playlist.validate(), Ok(()));
        assert_eq!(playlist.to_string(), data);
    }

    #[test]
    fn test_parse_rejects_malformed_byte_ranges_and_time_offsets() {
        assert_eq!(
            syntax_error("#EXTM3U\n#EXT-X-BYTERANGE:1000@\n"),
            (
                2,
                SyntaxError::InvalidTagValue {
                    tag: "EXT-X-BYTERANGE",
                    value: "1000@".to_string()
                }
            )
        );
        assert_eq!(
            syntax_error("#EXTM3U\n#EXT-X-MAP:URI=\"a\",BYTERANGE=\"x\"\n"),
            (
                2,
                SyntaxError::InvalidAttributeValue {
                    tag: "EXT-X-MAP",
                    attribute: "BYTERANGE",
                    value: "x".to_string()
                }
            )
        );
        assert_eq!(
            syntax_error("#EXTM3U\n#EXT-X-START:TIME-OFFSET=soon\n"),
            (
                2,
                SyntaxError::InvalidAttributeValue {
                    tag: "EXT-X-START",
                    attribute: "TIME-OFFSET",
                    value: "soon".to_string()
                }
            )
        );
    }

    #[test]
    fn test_validate_byte_range_without_offset_needs_previous_sub_range() {
        let segment = |uri: &str| Tag::ExtInf {
            uri: uri.to_string(),
            duration: 10.0,
            title: None,
        };
        let range = |offset| {
            Tag::ExtXByteRange(ByteRange {
                length: 100,
                offset,
            })
        };
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(4),
                Tag::ExtXTargetDuration(10),
                range(None),
                segment("a.ts"),
                range(Some(0)),
                segment("a.ts"),
                range(None),
                segment("a.ts"),
                range(None),
                segment("b.ts"),
            ],
        };

        assert_eq!(
            playlist.validate(),
            Err(vec![
                ValidationError::InvalidByteRange("a.ts".to_string()),
                ValidationError::InvalidByteRange("b.ts".to_string()),
            ])
        );
    }

    #[test]
    fn test_validate_date_time_formats() {
        let date_range = |start_date: &str, end_date: Option<&str>| Tag::ExtXDateRange {
            id: "ad".to_string(),
            class: None,
            start_date: start_date.to_string(),
            cue: None,
            end_date: end_date.map(str::to_string),
            duration: None,
            planned_duration: None,
            end_on_next: None,
            scte35_cmd: None,
            scte35_out: None,
            scte35_in: None,
            extra_attributes: Vec::new(),
        };
        let valid = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXTargetDuration(10),
                Tag::ExtXProgramDateTime("2010-02-19T14:54:23.031+08:00".parse().unwrap()),
                date_range("2024-01-01T00:00:00Z", Some("2024-01-01T00:00:30.5Z")),
                Tag::ExtXProgramDateTime("2024-12-31T23:59:60-0500".parse().unwrap()),
            ],
        };
        let invalid = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXTargetDuration(10),
                date_range("2024-13-01T00:00:00Z", Some("2024-01-01 00:00:00Z")),
            ],
        };

        assert_eq!(valid.validate(), Ok(()));
        assert_eq!(
            invalid.validate(),
            Err(vec![
                ValidationError::InvalidDateRangeStartDate,
                ValidationError::InvalidDateRangeEndDate,
            ])
        );
    }

    #[test]
    fn test_builder_creates_master_playlist_with_variant_uris() {
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .version(6)
            .stream_inf(
                2_000_000,
                Some(1_800_000),
                Some("avc1.640020,mp4a.40.2".into()),
                Some(Resolution {
                    width: 1280,
                    height: 720,
                }),
                Some(30.0),
                None,
                None,
                None,
                Some("NONE"),
            )
            .uri("720p.m3u8")
            .iframe_stream_inf(
                200_000,
                Some(180_000),
                None,
                None,
                None,
                "720p-iframes.m3u8",
            )
            .build()
            .unwrap();

        assert_eq!(
            playlist.to_string(),
            "#EXTM3U
#EXT-X-VERSION:6
#EXT-X-STREAM-INF:BANDWIDTH=2000000,AVERAGE-BANDWIDTH=1800000,CODECS=\"avc1.640020,mp4a.40.2\",RESOLUTION=1280x720,FRAME-RATE=30,CLOSED-CAPTIONS=NONE
720p.m3u8
#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=200000,AVERAGE-BANDWIDTH=180000,URI=\"720p-iframes.m3u8\"
"
        );
    }

    #[test]
    fn test_builder_supports_date_range_low_latency_and_arbitrary_tags() {
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .version(9)
            .target_duration(4)
            .server_control(Some(24.0), Some(2.0), true)
            .part_inf(1.0)
            .skip(3)
            .date_range("ad-1", "2024-01-01T00:00:00Z")
            .part("seg1.0.mp4", 1.0)
            .extinf("seg1.mp4", 4.0, None)
            .discontinuity()
            .tag(Tag::ExtXGap)
            .extinf("seg2.mp4", 4.0, None)
            .preload_hint("PART", "seg3.0.mp4")
            .rendition_report("../low/index.m3u8", Some(2), Some(0))
            .build()
            .unwrap();

        assert_eq!(
            playlist.to_string(),
            "#EXTM3U
#EXT-X-VERSION:9
#EXT-X-TARGETDURATION:4
#EXT-X-SERVER-CONTROL:CAN-SKIP-UNTIL=24,PART-HOLD-BACK=2,CAN-BLOCK-RELOAD=YES
#EXT-X-PART-INF:PART-TARGET=1
#EXT-X-SKIP:SKIPPED-SEGMENTS=3
#EXT-X-DATERANGE:ID=\"ad-1\",START-DATE=\"2024-01-01T00:00:00Z\"
#EXT-X-PART:DURATION=1,URI=\"seg1.0.mp4\"
#EXTINF:4,
seg1.mp4
#EXT-X-DISCONTINUITY
#EXT-X-GAP
#EXTINF:4,
seg2.mp4
#EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"seg3.0.mp4\"
#EXT-X-RENDITION-REPORT:URI=\"../low/index.m3u8\",LAST-MSN=2,LAST-PART=0
"
        );
    }

    #[test]
    fn test_builder_clones_are_independent() {
        let base = PlaylistBuilder::new().extm3u();
        let with_version = base.clone().version(3);

        assert_eq!(base.build().unwrap().tags, vec![Tag::ExtM3U]);
        assert_eq!(
            with_version.build().unwrap().tags,
            vec![Tag::ExtM3U, Tag::ExtXVersion(3)]
        );
    }

    #[test]
    fn test_validate_flags_allow_cache_from_version_7() {
        let playlist = |version| Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(version),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXAllowCache(true),
            ],
        };

        assert_eq!(playlist(6).validate(), Ok(()));
        assert_eq!(
            playlist(7).validate(),
            Err(vec![ValidationError::RemovedTag {
                tag: "EXT-X-ALLOW-CACHE".to_string(),
                removed_in: 7,
            }])
        );
    }

    #[test]
    fn test_substitute_variables_from_value_import_and_query() {
        let data = "#EXTM3U
#EXT-X-VERSION:11
#EXT-X-TARGETDURATION:4
#EXT-X-DEFINE:NAME=\"base\",VALUE=\"https://cdn.example.com\"
#EXT-X-DEFINE:IMPORT=\"session\"
#EXT-X-DEFINE:QUERYPARAM=\"token\"
#EXT-X-MAP:URI=\"{$base}/init.mp4\"
#EXTINF:4,
{$base}/seg.mp4?s={$session}&t={$token}
";
        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();
        let imports = HashMap::from([("session".to_string(), "abc".to_string())]);

        let resolved = playlist
            .substitute_variables(&imports, Some("x=1&token=a%2Fb"))
            .unwrap();

        assert_eq!(
            resolved.tags[6],
            Tag::ExtXMap(Map {
                uri: "https://cdn.example.com/init.mp4".to_string(),
                byterange: None,
            })
        );
        assert_eq!(
            resolved.media_segments()[0].uri,
            "https://cdn.example.com/seg.mp4?s=abc&t=a/b"
        );
        assert_eq!(resolved.tags[3], playlist.tags[3]);
        assert_eq!(playlist.to_string(), data);
    }

    #[test]
    fn test_substitute_variables_errors() {
        let substitute = |data: &str, imports: &[(&str, &str)], query: Option<&str>| {
            let imports = imports
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect::<HashMap<_, _>>();
            Playlist::from_reader(data.as_bytes())
                .unwrap()
                .substitute_variables(&imports, query)
                .unwrap_err()
        };

        assert!(matches!(
            substitute("#EXTM3U\n{$nope}/a.m3u8\n", &[], None),
            SubstitutionError::UndefinedVariable(name) if name == "nope"
        ));
        assert!(matches!(
            substitute("#EXTM3U\n#EXT-X-DEFINE:IMPORT=\"x\"\n", &[], None),
            SubstitutionError::MissingImport(name) if name == "x"
        ));
        assert!(matches!(
            substitute("#EXTM3U\n#EXT-X-DEFINE:QUERYPARAM=\"t\"\n", &[], Some("u=1")),
            SubstitutionError::MissingQueryParameter(name) if name == "t"
        ));
        assert!(matches!(
            substitute(
                "#EXTM3U\n#EXT-X-DEFINE:NAME=\"a\",VALUE=\"1\"\n#EXT-X-DEFINE:NAME=\"a\",VALUE=\"2\"\n",
                &[],
                None
            ),
            SubstitutionError::DuplicateVariable(name) if name == "a"
        ));
        assert!(matches!(
            substitute("#EXTM3U\n#EXT-X-DEFINE:NAME=\"a\"\n", &[], None),
            SubstitutionError::InvalidDefine(_)
        ));
    }

    #[test]
    fn test_parse_rejects_invalid_program_date_time() {
        for value in [
            "",
            "yesterday",
            "2020-01-01T00:00:00+a\u{20ac}",
            "2020-01-01T00:00:00Zjunk",
            "2020-01-01T24:00:00Z",
            "2020-01-01T00:00:00.Z",
            "2020-01-01T00:00.5Z",
            "2020-01-01 00:00:00Z",
            "2021-02-31T00:00:00Z",
            "2021-04-31T00:00:00Z",
            "2023-02-29T00:00:00Z",
        ] {
            let result = format!("#EXTM3U\n#EXT-X-PROGRAM-DATE-TIME:{value}\n").parse::<Playlist>();
            assert!(result.is_err(), "{value:?} should not parse");
        }
        let leap: Playlist = "#EXTM3U\n#EXT-X-PROGRAM-DATE-TIME:2024-02-29T00:00:00Z\n"
            .parse()
            .unwrap();
        assert_eq!(leap.tags.len(), 2);
    }

    #[test]
    fn test_codecs_are_typed_on_stream_tags() {
        let playlist: Playlist = "#EXTM3U\n\
#EXT-X-STREAM-INF:BANDWIDTH=1000,CODECS=\"avc1.640028, mp4a.40.2\"\n\
v.m3u8\n\
#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=100,CODECS=\"avc1.640028\",URI=\"i.m3u8\"\n"
            .parse()
            .unwrap();
        let variants = playlist.variants();
        let codecs = variants[0].codecs.as_ref().unwrap();
        assert_eq!(
            codecs.iter().collect::<Vec<_>>(),
            ["avc1.640028", "mp4a.40.2"]
        );
        assert_eq!(codecs.len(), 2);
        assert!(!codecs.is_empty());
        assert!(codecs.contains_family("avc1"));
        assert!(codecs.contains_family("MP4A"));
        assert!(!codecs.contains_family("hvc1"));
        assert!(!codecs.contains_family("avc"));
        assert_eq!(codecs.to_string(), "avc1.640028,mp4a.40.2");
        assert_eq!(
            playlist.to_string(),
            "#EXTM3U\n\
#EXT-X-STREAM-INF:BANDWIDTH=1000,CODECS=\"avc1.640028,mp4a.40.2\"\n\
v.m3u8\n\
#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=100,CODECS=\"avc1.640028\",URI=\"i.m3u8\"\n"
        );
        assert_eq!(
            Codecs::from("hvc1.2.4.L93.B0"),
            ["hvc1.2.4.L93.B0"].into_iter().collect::<Codecs>()
        );
    }

    #[test]
    fn test_validate_rejects_empty_codec_entries() {
        for value in ["", "avc1,,mp4a", "avc1,"] {
            let playlist: Playlist =
                format!("#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1000,CODECS=\"{value}\"\nv.m3u8\n")
                    .parse()
                    .unwrap();
            let errors = playlist.validate().unwrap_err();
            assert!(
                errors
                    .iter()
                    .any(|error| matches!(error, ValidationError::InvalidCodecs(_))),
                "{value:?}: {errors:?}"
            );
            assert!(playlist
                .to_string()
                .contains(&format!("CODECS=\"{value}\"")));
        }
    }

    #[test]
    fn test_builder_codecs() {
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .stream_inf(
                1000,
                None,
                Some(["avc1.4d401f", "mp4a.40.2"].into_iter().collect()),
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .uri("v.m3u8")
            .build()
            .unwrap();
        assert!(playlist
            .to_string()
            .contains("CODECS=\"avc1.4d401f,mp4a.40.2\""));
    }

    #[test]
    fn test_date_time_fields_and_display() {
        let date_time: DateTime = "2010-02-19T14:54:23.031+08:00".parse().unwrap();
        assert_eq!(
            (
                date_time.year(),
                date_time.month(),
                date_time.day(),
                date_time.hour(),
                date_time.minute(),
                date_time.second(),
                date_time.nanosecond(),
                date_time.offset_minutes()
            ),
            (2010, 2, 19, 14, 54, 23, 31_000_000, Some(480))
        );
        assert_eq!(date_time.to_string(), "2010-02-19T14:54:23.031+08:00");

        for (input, output) in [
            ("2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z"),
            ("2024-01-01T00:00:00.000Z", "2024-01-01T00:00:00.000Z"),
            ("2024-01-01T00:00:00+00:00", "2024-01-01T00:00:00Z"),
            ("2024-01-01T10:30Z", "2024-01-01T10:30:00Z"),
            ("2024-12-31T23:59:60-0500", "2024-12-31T23:59:60-05:00"),
            ("2024-01-01T00:00:00+05:30", "2024-01-01T00:00:00+05:30"),
            (
                "2024-01-01T00:00:00.1234567Z",
                "2024-01-01T00:00:00.1234567Z",
            ),
            ("2024-01-01T00:00:00.5", "2024-01-01T00:00:00.5"),
            (
                "2024-01-01T00:00:00.123456789123Z",
                "2024-01-01T00:00:00.123456789Z",
            ),
            ("2024-01-01T00:00:00+09", "2024-01-01T00:00:00+09:00"),
        ] {
            let parsed: DateTime = input.parse().unwrap();
            assert_eq!(parsed.to_string(), output, "{input}");
            assert_eq!(parsed.to_string().parse::<DateTime>().unwrap(), parsed);
        }
        assert_eq!(
            "2024-01-01T00:00:00"
                .parse::<DateTime>()
                .unwrap()
                .offset_minutes(),
            None
        );
    }

    #[test]
    fn test_date_time_unix_timestamp() {
        let timestamp = |text: &str| text.parse::<DateTime>().unwrap().unix_timestamp();
        assert_eq!(timestamp("1970-01-01T00:00:00Z"), 0);
        assert_eq!(timestamp("2000-03-01T00:00:00Z"), 951_868_800);
        assert_eq!(timestamp("2024-02-29T12:00:00Z"), 1_709_208_000);
        assert_eq!(timestamp("2024-02-29T12:00:00+02:00"), 1_709_200_800);
        assert_eq!(timestamp("2024-02-29T12:00:00"), 1_709_208_000);
        assert_eq!(timestamp("1969-12-31T23:59:59Z"), -1);
        assert_eq!(timestamp("0000-01-01T00:00:00Z"), -62_167_219_200);

        for seconds in [
            0,
            -1,
            951_868_800,
            1_709_208_000,
            -62_167_219_200,
            253_402_300_799,
        ] {
            let date_time = DateTime::from_unix_timestamp(seconds, 500_000_000).unwrap();
            assert_eq!(date_time.unix_timestamp(), seconds);
            assert_eq!(date_time.nanosecond(), 500_000_000);
            assert_eq!(date_time.offset_minutes(), Some(0));
        }
        assert_eq!(
            DateTime::from_unix_timestamp(1_709_208_000, 0)
                .unwrap()
                .to_string(),
            "2024-02-29T12:00:00Z"
        );
        assert_eq!(DateTime::from_unix_timestamp(253_402_300_800, 0), None);
        assert_eq!(DateTime::from_unix_timestamp(-62_167_219_201, 0), None);
        assert_eq!(DateTime::from_unix_timestamp(0, 1_000_000_000), None);
    }

    #[test]
    fn test_date_time_new_validates_fields() {
        assert!(DateTime::new(2024, 2, 29, 23, 59, 60, 999_999_999, Some(-1439)).is_some());
        assert!(DateTime::new(2023, 2, 29, 0, 0, 0, 0, None).is_none());
        assert!(DateTime::new(2024, 13, 1, 0, 0, 0, 0, None).is_none());
        assert!(DateTime::new(2024, 1, 0, 0, 0, 0, 0, None).is_none());
        assert!(DateTime::new(2024, 1, 1, 24, 0, 0, 0, None).is_none());
        assert!(DateTime::new(2024, 1, 1, 0, 60, 0, 0, None).is_none());
        assert!(DateTime::new(2024, 1, 1, 0, 0, 61, 0, None).is_none());
        assert!(DateTime::new(2024, 1, 1, 0, 0, 0, 0, Some(1440)).is_none());
        assert!(DateTime::new(10_000, 1, 1, 0, 0, 0, 0, None).is_none());
    }

    #[test]
    fn test_builder_program_date_time_round_trips() {
        let start = DateTime::from_unix_timestamp(1_709_208_000, 250_000_000).unwrap();
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .target_duration(10)
            .program_date_time(start)
            .extinf("a.ts", 9.0, None)
            .end_list()
            .build()
            .unwrap();
        let text = playlist.to_string();
        assert!(text.contains("#EXT-X-PROGRAM-DATE-TIME:2024-02-29T12:00:00.250Z\n"));
        let reparsed: Playlist = text.parse().unwrap();
        assert_eq!(reparsed.media_segments()[0].program_date_time, Some(start));
    }

    #[test]
    fn test_parse_rejects_non_finite_extinf_duration() {
        for duration in ["NaN", "inf", "-inf"] {
            let (line, kind) = syntax_error(&format!("#EXTM3U\n#EXTINF:{duration},\na.ts\n"));
            assert_eq!(line, 2);
            assert!(matches!(kind, SyntaxError::InvalidExtInf(_)));
        }
    }

    #[test]
    fn test_validate_rejects_non_finite_duration_from_builder_tags() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXTargetDuration(10),
                Tag::ExtInf {
                    uri: "a.ts".to_string(),
                    duration: f64::NAN,
                    title: None,
                },
            ],
        };
        assert!(playlist
            .validate()
            .unwrap_err()
            .iter()
            .any(|error| matches!(error, ValidationError::InvalidDuration(_))));
    }

    #[test]
    fn test_parse_rejects_bare_value_tags() {
        for tag in ["EXT-X-TARGETDURATION", "EXT-X-KEY", "EXT-X-VERSION"] {
            let (line, kind) = syntax_error(&format!("#EXTM3U\n#{tag}\n"));
            assert_eq!(line, 2);
            assert!(matches!(kind, SyntaxError::MissingTagValue { .. }));
        }
    }

    #[test]
    fn test_substitute_rejects_values_that_would_inject_tags() {
        let playlist: Playlist = "#EXTM3U\n#EXT-X-VERSION:11\n#EXT-X-DEFINE:QUERYPARAM=\"t\"\n#EXT-X-TARGETDURATION:10\n#EXTINF:5,\n{$t}\n"
            .parse()
            .unwrap();
        for query in ["t=a.ts%0A%23EXT-X-ENDLIST", "t=a%0D.ts", "t=a%22.ts"] {
            assert!(matches!(
                playlist.substitute_variables(&HashMap::new(), Some(query)),
                Err(SubstitutionError::InvalidValue(name)) if name == "t"
            ));
        }
        let ok = playlist
            .substitute_variables(&HashMap::new(), Some("t=a%20b.ts"))
            .unwrap();
        assert!(ok.tags.contains(&Tag::ExtInf {
            uri: "a b.ts".to_string(),
            duration: 5.0,
            title: None,
        }));
    }

    #[test]
    fn test_unmodeled_attributes_round_trip_on_variant_and_media_tags() {
        let data = "#EXTM3U\n\
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"x\",ASSOC-LANGUAGE=\"en\",SAMPLE-RATE=48000\n\
#EXT-X-STREAM-INF:BANDWIDTH=1000,CODECS=\"hvc1\",VIDEO-RANGE=PQ,HDCP-LEVEL=TYPE-1,SCORE=1.5\n\
v.m3u8\n\
#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=100,URI=\"i.m3u8\",VIDEO-RANGE=PQ\n";
        let playlist: Playlist = data.parse().unwrap();
        assert!(playlist.tags.iter().any(|tag| matches!(
            tag,
            Tag::ExtXStreamInf { extra_attributes, .. }
                if extra_attributes[0] == ("VIDEO-RANGE".to_string(), AttributeValue::Unquoted("PQ".to_string()))
                    && extra_attributes.len() == 3
        )));
        assert_eq!(playlist.to_string(), data.replace("\\\n", ""));
    }

    #[test]
    fn test_validate_rejects_strings_that_break_serialization() {
        let injected = PlaylistBuilder::new()
            .extm3u()
            .target_duration(10)
            .extinf("a.ts\n#EXT-X-ENDLIST", 5.0, None)
            .build();
        assert!(matches!(
            injected.unwrap_err().as_slice(),
            [ValidationError::UnserializableTag(_)]
        ));

        let quoted = PlaylistBuilder::new()
            .extm3u()
            .media(
                "AUDIO",
                "a",
                Some("x\",DEFAULT=YES,FOO=\"y"),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .build();
        assert!(matches!(
            quoted.unwrap_err().as_slice(),
            [ValidationError::UnserializableTag(_)]
        ));
    }

    #[test]
    fn test_validate_accepts_parsed_playlist_sample() {
        let playlist = Playlist::from_file("src/m3u8/tests/test_data/playlist.m3u8").unwrap();
        if let Err(errors) = playlist.validate() {
            assert!(!errors
                .iter()
                .any(|error| matches!(error, ValidationError::UnserializableTag(_))));
        }
    }

    fn validation_errors(data: &str) -> Vec<ValidationError> {
        data.parse::<Playlist>().unwrap().validate().unwrap_err()
    }

    #[test]
    fn test_validate_orphan_uri() {
        let errors = validation_errors("#EXTM3U\n#EXT-X-TARGETDURATION:10\nstray.ts\n");
        assert!(errors.contains(&ValidationError::OrphanUri("stray.ts".to_string())));
        let ok = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\nv.m3u8\n";
        assert_eq!(ok.parse::<Playlist>().unwrap().validate(), Ok(()));
    }

    #[test]
    fn test_validate_duplicate_date_range_and_session_data() {
        let errors = validation_errors(
            "#EXTM3U\n#EXT-X-TARGETDURATION:10\n\
#EXT-X-DATERANGE:ID=\"a\",START-DATE=\"2024-01-01T00:00:00Z\"\n\
#EXT-X-DATERANGE:ID=\"a\",START-DATE=\"2024-01-01T00:00:00Z\"\n",
        );
        assert!(errors.contains(&ValidationError::DuplicateDateRangeId("a".to_string())));

        let errors = validation_errors(
            "#EXTM3U\n#EXT-X-SESSION-DATA:DATA-ID=\"d\",VALUE=\"1\"\n#EXT-X-SESSION-DATA:DATA-ID=\"d\",VALUE=\"2\"\n",
        );
        assert!(errors.contains(&ValidationError::DuplicateSessionData {
            id: "d".to_string(),
            language: None,
        }));
        let ok = "#EXTM3U\n#EXT-X-SESSION-DATA:DATA-ID=\"d\",VALUE=\"1\",LANGUAGE=\"en\"\n#EXT-X-SESSION-DATA:DATA-ID=\"d\",VALUE=\"2\",LANGUAGE=\"fr\"\n";
        assert_eq!(ok.parse::<Playlist>().unwrap().validate(), Ok(()));
    }

    #[test]
    fn test_content_steering_round_trip_and_builder() {
        let data = "#EXTM3U\n#EXT-X-CONTENT-STEERING:SERVER-URI=\"https://example.com/steer?a=1\",PATHWAY-ID=\"CDN-A\"\n";
        let playlist: Playlist = data.parse().unwrap();
        assert_eq!(
            playlist.tags[1],
            Tag::ExtXContentSteering {
                server_uri: "https://example.com/steer?a=1".to_string(),
                pathway_id: Some("CDN-A".to_string()),
            }
        );
        assert_eq!(playlist.to_string(), data);
        assert_eq!(playlist.validate(), Ok(()));

        let built = PlaylistBuilder::new()
            .extm3u()
            .content_steering("https://example.com/steer", None)
            .build()
            .unwrap();
        assert_eq!(
            built.to_string(),
            "#EXTM3U\n#EXT-X-CONTENT-STEERING:SERVER-URI=\"https://example.com/steer\"\n"
        );
    }

    #[test]
    fn test_content_steering_errors_and_validation() {
        assert!(matches!(
            syntax_error("#EXTM3U\n#EXT-X-CONTENT-STEERING:PATHWAY-ID=\"a\"\n").1,
            SyntaxError::MissingAttribute { .. }
        ));
        assert!(matches!(
            syntax_error("#EXTM3U\n#EXT-X-CONTENT-STEERING\n").1,
            SyntaxError::MissingTagValue { .. }
        ));

        let errors = validation_errors(
            "#EXTM3U\n#EXT-X-CONTENT-STEERING:SERVER-URI=\"https://a\",PATHWAY-ID=\"bad id!\"\n\
#EXT-X-CONTENT-STEERING:SERVER-URI=\"https://b\"\n",
        );
        assert!(errors.contains(&ValidationError::DuplicateTag(
            "EXT-X-CONTENT-STEERING".to_string()
        )));
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidContentSteering(_))));

        let errors = validation_errors(
            "#EXTM3U\n#EXT-X-TARGETDURATION:6\n#EXT-X-CONTENT-STEERING:SERVER-URI=\"https://a\"\n#EXTINF:6,\nseg.ts\n",
        );
        assert!(errors.contains(&ValidationError::MixedPlaylistTypes));
    }

    #[test]
    fn test_pathway_id_is_typed_on_stream_and_media_tags() {
        let data = "#EXTM3U\n\
#EXT-X-CONTENT-STEERING:SERVER-URI=\"https://example.com/steer\",PATHWAY-ID=\"CDN-A\"\n\
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",URI=\"a.m3u8\",NAME=\"en\",PATHWAY-ID=\"CDN-B\"\n\
#EXT-X-STREAM-INF:BANDWIDTH=1000,AUDIO=\"a\",PATHWAY-ID=\"CDN-A\"\n\
v.m3u8\n\
#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=100,URI=\"i.m3u8\",PATHWAY-ID=\"CDN-B\"\n";
        let playlist: Playlist = data.parse().unwrap();
        let pathways: Vec<_> = playlist
            .tags
            .iter()
            .filter_map(|tag| match tag {
                Tag::ExtXMedia {
                    pathway_id,
                    extra_attributes,
                    ..
                }
                | Tag::ExtXStreamInf {
                    pathway_id,
                    extra_attributes,
                    ..
                }
                | Tag::ExtXIFrameStreamInf {
                    pathway_id,
                    extra_attributes,
                    ..
                } => {
                    assert!(extra_attributes.is_empty());
                    pathway_id.as_deref()
                }
                _ => None,
            })
            .collect();
        assert_eq!(pathways, ["CDN-B", "CDN-A", "CDN-B"]);
        assert_eq!(playlist.to_string(), data);
        assert_eq!(playlist.validate(), Ok(()));

        let errors = validation_errors(
            "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1000,PATHWAY-ID=\"bad id\"\nv.m3u8\n",
        );
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidContentSteering(_))));
    }

    #[test]
    fn test_resolve_uri_rfc3986_examples() {
        use crate::m3u8::uri::resolve_uri;
        let base = "http://a/b/c/d;p?q";
        for (reference, expected) in [
            ("g:h", "g:h"),
            ("g", "http://a/b/c/g"),
            ("./g", "http://a/b/c/g"),
            ("g/", "http://a/b/c/g/"),
            ("/g", "http://a/g"),
            ("//g", "http://g"),
            ("?y", "http://a/b/c/d;p?y"),
            ("g?y", "http://a/b/c/g?y"),
            ("#s", "http://a/b/c/d;p?q#s"),
            ("g#s", "http://a/b/c/g#s"),
            ("", "http://a/b/c/d;p?q"),
            (".", "http://a/b/c/"),
            ("./", "http://a/b/c/"),
            ("..", "http://a/b/"),
            ("../", "http://a/b/"),
            ("../g", "http://a/b/g"),
            ("../..", "http://a/"),
            ("../../g", "http://a/g"),
            ("../../../g", "http://a/g"),
            ("/./g", "http://a/g"),
            ("g.", "http://a/b/c/g."),
            ("./g/.", "http://a/b/c/g/"),
            ("g/./h", "http://a/b/c/g/h"),
            ("g/../h", "http://a/b/c/h"),
            ("http:g", "http:g"),
        ] {
            assert_eq!(
                resolve_uri(base, reference).unwrap(),
                expected,
                "{reference}"
            );
        }
        assert_eq!(
            resolve_uri("https://example.com", "x.ts").unwrap(),
            "https://example.com/x.ts"
        );
        for bad in ["relative/path", "", "https://a/b c", "https://a/\"b"] {
            assert!(resolve_uri(bad, "x").is_err(), "{bad:?}");
        }
    }

    #[test]
    fn test_resolve_uris_in_playlist() {
        use crate::m3u8::error::UriError;
        let playlist: Playlist = "#EXTM3U\n\
#EXT-X-CONTENT-STEERING:SERVER-URI=\"/steer\"\n\
#EXT-X-SESSION-KEY:METHOD=AES-128,URI=\"k/s.key\"\n\
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"en\",URI=\"audio/en.m3u8\"\n\
#EXT-X-MEDIA:TYPE=CLOSED-CAPTIONS,GROUP-ID=\"c\",NAME=\"cc\",INSTREAM-ID=\"CC1\"\n\
#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=1,URI=\"i.m3u8\"\n\
#EXT-X-STREAM-INF:BANDWIDTH=2,AUDIO=\"a\"\n\
../v/v.m3u8\n"
            .parse()
            .unwrap();
        let resolved = playlist
            .resolve_uris("https://cdn.example.com/m/master.m3u8?token=1")
            .unwrap();
        assert_eq!(
            resolved.to_string(),
            "#EXTM3U\n\
#EXT-X-CONTENT-STEERING:SERVER-URI=\"https://cdn.example.com/steer\"\n\
#EXT-X-SESSION-KEY:METHOD=AES-128,URI=\"https://cdn.example.com/m/k/s.key\"\n\
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",URI=\"https://cdn.example.com/m/audio/en.m3u8\",NAME=\"en\"\n\
#EXT-X-MEDIA:TYPE=CLOSED-CAPTIONS,GROUP-ID=\"c\",NAME=\"cc\",INSTREAM-ID=\"CC1\"\n\
#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=1,URI=\"https://cdn.example.com/m/i.m3u8\"\n\
#EXT-X-STREAM-INF:BANDWIDTH=2,AUDIO=\"a\"\n\
https://cdn.example.com/v/v.m3u8\n"
        );

        let media: Playlist = "#EXTM3U\n#EXT-X-VERSION:6\n#EXT-X-TARGETDURATION:6\n\
#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:6,\nseg/1.ts\n#EXTINF:6,\nhttps://other/2.ts\n"
            .parse()
            .unwrap();
        let resolved = media.resolve_uris("https://a.example/live/x.m3u8").unwrap();
        let segments = resolved.media_segments();
        assert_eq!(segments[0].uri, "https://a.example/live/seg/1.ts");
        assert_eq!(segments[1].uri, "https://other/2.ts");
        assert_eq!(resolved.validate(), Ok(()));

        assert_eq!(
            media.resolve_uris("live/x.m3u8"),
            Err(UriError::InvalidBase("live/x.m3u8".to_string()))
        );
    }

    #[test]
    fn test_playlist_type_and_key_method_are_typed() {
        let data = "#EXTM3U\n#EXT-X-VERSION:5\n#EXT-X-TARGETDURATION:6\n#EXT-X-PLAYLIST-TYPE:EVENT\n\
#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"k.key\"\n#EXTINF:6,\na.ts\n#EXT-X-KEY:METHOD=NONE\n#EXTINF:6,\nb.ts\n";
        let playlist: Playlist = data.parse().unwrap();
        assert!(playlist
            .tags
            .contains(&Tag::ExtXPlaylistType(PlaylistType::Event)));
        let methods: Vec<_> = playlist
            .tags
            .iter()
            .filter_map(|tag| match tag {
                Tag::ExtXKey(key) => Some(key.method.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(methods, [KeyMethod::SampleAes, KeyMethod::None]);
        assert_eq!(playlist.to_string(), data);
        assert_eq!(playlist.validate(), Ok(()));

        assert_eq!("VOD".parse(), Ok(PlaylistType::Vod));
        assert_eq!(PlaylistType::Event.to_string(), "EVENT");
        assert!("vod".parse::<PlaylistType>().is_err());
        assert!(matches!(
            syntax_error("#EXTM3U\n#EXT-X-PLAYLIST-TYPE:LIVE\n").1,
            SyntaxError::InvalidTagValue { .. }
        ));

        let unknown: Playlist = "#EXTM3U\n#EXT-X-TARGETDURATION:6\n#EXT-X-KEY:METHOD=AES-256,URI=\"k\"\n#EXTINF:6,\na.ts\n"
            .parse()
            .unwrap();
        assert_eq!(KeyMethod::from("AES-256").as_str(), "AES-256");
        assert!(unknown
            .validate()
            .unwrap_err()
            .contains(&ValidationError::InvalidKeyMethod("AES-256".to_string())));
        assert_eq!(unknown.to_string().matches("METHOD=AES-256").count(), 1);

        let built = PlaylistBuilder::new()
            .extm3u()
            .target_duration(6)
            .playlist_type(PlaylistType::Vod)
            .build()
            .unwrap();
        assert!(built.to_string().contains("#EXT-X-PLAYLIST-TYPE:VOD"));
    }

    #[test]
    fn test_resolution_is_typed() {
        let data = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1000,RESOLUTION=1920x1080\nv.m3u8\n\
#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=100,RESOLUTION=640x360,URI=\"i.m3u8\"\n";
        let playlist: Playlist = data.parse().unwrap();
        let resolutions: Vec<_> = playlist
            .tags
            .iter()
            .filter_map(|tag| match tag {
                Tag::ExtXStreamInf { resolution, .. }
                | Tag::ExtXIFrameStreamInf { resolution, .. } => *resolution,
                _ => None,
            })
            .collect();
        assert_eq!(
            resolutions,
            [
                Resolution {
                    width: 1920,
                    height: 1080
                },
                Resolution {
                    width: 640,
                    height: 360
                }
            ]
        );
        assert_eq!(playlist.to_string(), data);
        assert_eq!(playlist.validate(), Ok(()));

        for bad in [
            "1920",
            "1920x",
            "x1080",
            "1920X1080",
            "-1x5",
            "+1x5",
            "1x2x3",
            "1.5x2",
            "99999999999x1",
        ] {
            let data = format!("#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1,RESOLUTION={bad}\nv.m3u8\n");
            assert!(
                matches!(
                    syntax_error(&data).1,
                    SyntaxError::InvalidAttributeValue {
                        attribute: "RESOLUTION",
                        ..
                    }
                ),
                "{bad}"
            );
        }

        let built = PlaylistBuilder::new()
            .extm3u()
            .stream_inf(
                1000,
                None,
                None,
                Some(Resolution {
                    width: 1280,
                    height: 720,
                }),
                None,
                None,
                None,
                None,
                None,
            )
            .uri("v.m3u8")
            .build()
            .unwrap();
        assert!(built.to_string().contains("RESOLUTION=1280x720"));
    }

    #[test]
    fn test_total_duration_variants_and_renditions() {
        let media: Playlist =
            "#EXTM3U\n#EXT-X-TARGETDURATION:6\n#EXTINF:5.5,\na.ts\n#EXT-X-GAP\n#EXTINF:4,\nb.ts\n"
                .parse()
                .unwrap();
        assert_eq!(media.total_duration(), 9.5);
        assert_eq!(Playlist { tags: vec![] }.total_duration(), 0.0);
        assert!(media.variants().is_empty());

        let master: Playlist = "#EXTM3U\n\
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aud\",NAME=\"English\",LANGUAGE=\"en\",DEFAULT=YES,AUTOSELECT=YES,URI=\"en.m3u8\"\n\
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aud\",NAME=\"French\",LANGUAGE=\"fr\",URI=\"fr.m3u8\"\n\
#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"aud\",NAME=\"Subs\",URI=\"s.m3u8\"\n\
#EXT-X-STREAM-INF:BANDWIDTH=3000,RESOLUTION=1280x720,AUDIO=\"aud\",CODECS=\"avc1.4d401f\"\n\
hi.m3u8\n\
#EXT-X-STREAM-INF:BANDWIDTH=1000,AUDIO=\"aud\"\n\
lo.m3u8\n\
#EXT-X-STREAM-INF:BANDWIDTH=1000,AVERAGE-BANDWIDTH=900\n\
lo2.m3u8\n"
            .parse()
            .unwrap();
        let uris: Vec<_> = master.variants().into_iter().map(|v| v.uri).collect();
        assert_eq!(uris, ["hi.m3u8", "lo.m3u8", "lo2.m3u8"]);
        let sorted = master.variants_by_bandwidth();
        let uris: Vec<_> = sorted.iter().map(|v| v.uri.as_str()).collect();
        assert_eq!(uris, ["lo.m3u8", "lo2.m3u8", "hi.m3u8"]);
        assert_eq!(
            sorted[2].resolution,
            Some(Resolution {
                width: 1280,
                height: 720
            })
        );
        assert_eq!(sorted[2].audio.as_deref(), Some("aud"));
        assert_eq!(sorted[1].average_bandwidth, Some(900));

        assert_eq!(master.renditions().len(), 3);
        let audio = master.renditions_in_group("AUDIO", "aud");
        assert_eq!(audio.len(), 2);
        assert!(audio[0].default && audio[0].autoselect && !audio[1].default);
        assert_eq!(audio[1].language.as_deref(), Some("fr"));
        assert!(master.renditions_in_group("AUDIO", "other").is_empty());

        let dangling: Playlist = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\n".parse().unwrap();
        assert!(dangling.variants().is_empty());
    }

    #[test]
    fn test_apply_delta_update() {
        use crate::m3u8::error::DeltaUpdateError;
        let parse = |text: &str| text.parse::<Playlist>().unwrap();
        let date = "START-DATE=\"2024-01-01T00:00:00Z\"";

        let previous = parse(&format!(
            "#EXTM3U\n#EXT-X-VERSION:10\n#EXT-X-TARGETDURATION:4\n#EXT-X-MEDIA-SEQUENCE:10\n\
#EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n#EXTINF:4,\ns10.ts\n\
#EXT-X-DATERANGE:ID=\"a\",{date}\n#EXT-X-DATERANGE:ID=\"gone\",{date}\n#EXTINF:4,\ns11.ts\n\
#EXT-X-DISCONTINUITY\n#EXTINF:4,\ns12.ts\n#EXTINF:4,\ns13.ts\n"
        ));
        let delta = parse(
            "#EXTM3U\n#EXT-X-VERSION:10\n#EXT-X-TARGETDURATION:4\n#EXT-X-MEDIA-SEQUENCE:10\n\
#EXT-X-SKIP:SKIPPED-SEGMENTS=3,RECENTLY-REMOVED-DATERANGES=\"gone\"\n\
#EXTINF:4,\ns13.ts\n#EXTINF:4,\ns14.ts\n",
        );
        let expected = parse(&format!(
            "#EXTM3U\n#EXT-X-VERSION:10\n#EXT-X-TARGETDURATION:4\n#EXT-X-MEDIA-SEQUENCE:10\n\
#EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n#EXTINF:4,\ns10.ts\n\
#EXT-X-DATERANGE:ID=\"a\",{date}\n#EXTINF:4,\ns11.ts\n\
#EXT-X-DISCONTINUITY\n#EXTINF:4,\ns12.ts\n#EXTINF:4,\ns13.ts\n#EXTINF:4,\ns14.ts\n"
        ));
        let merged = previous.apply_delta_update(&delta).unwrap();
        assert_eq!(merged, expected);
        let sequences: Vec<_> = merged.media_segments().iter().map(|s| s.sequence).collect();
        assert_eq!(sequences, [10, 11, 12, 13, 14]);

        // The previous playlist reaches back further than the skipped range, so
        // the key declared in the dropped part is restated.
        let previous = parse(
            "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-MEDIA-SEQUENCE:8\n\
#EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n#EXTINF:4,\ns8.ts\n#EXTINF:4,\ns9.ts\n\
#EXTINF:4,\ns10.ts\n#EXTINF:4,\ns11.ts\n",
        );
        let delta = parse(
            "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-MEDIA-SEQUENCE:10\n\
#EXT-X-SKIP:SKIPPED-SEGMENTS=2\n#EXTINF:4,\ns12.ts\n",
        );
        let merged = previous.apply_delta_update(&delta).unwrap();
        let segments = merged.media_segments();
        let uris: Vec<_> = segments.iter().map(|s| s.uri.as_str()).collect();
        assert_eq!(uris, ["s10.ts", "s11.ts", "s12.ts"]);
        assert!(segments.iter().all(|s| s.key.is_some()));
        assert_eq!(segments[0].sequence, 10);

        // A delta that skips nothing is already complete.
        let complete = parse("#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXTINF:4,\na.ts\n");
        assert_eq!(previous.apply_delta_update(&complete).unwrap(), complete);

        let too_far = parse(
            "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-MEDIA-SEQUENCE:11\n\
#EXT-X-SKIP:SKIPPED-SEGMENTS=3\n#EXTINF:4,\ns14.ts\n",
        );
        assert_eq!(
            previous.apply_delta_update(&too_far),
            Err(DeltaUpdateError::MissingSegment(12))
        );
        let too_early = parse(
            "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-MEDIA-SEQUENCE:5\n\
#EXT-X-SKIP:SKIPPED-SEGMENTS=1\n#EXTINF:4,\ns6.ts\n",
        );
        assert_eq!(
            previous.apply_delta_update(&too_early),
            Err(DeltaUpdateError::MissingSegment(5))
        );
        assert_eq!(
            Playlist { tags: vec![] }.apply_delta_update(&too_early),
            Err(DeltaUpdateError::MissingSegment(5))
        );
        assert_eq!(
            delta.apply_delta_update(&delta),
            Err(DeltaUpdateError::PreviousIsDeltaUpdate)
        );
        let twice =
            parse("#EXTM3U\n#EXT-X-SKIP:SKIPPED-SEGMENTS=1\n#EXT-X-SKIP:SKIPPED-SEGMENTS=1\n");
        assert_eq!(
            previous.apply_delta_update(&twice),
            Err(DeltaUpdateError::MultipleSkipTags)
        );
    }

    #[test]
    fn test_partial_segments() {
        use crate::m3u8::playlist::PartPosition;
        let header = "#EXTM3U\n#EXT-X-VERSION:9\n#EXT-X-TARGETDURATION:4\n\
#EXT-X-PART-INF:PART-TARGET=1\n#EXT-X-MEDIA-SEQUENCE:20\n";
        let playlist: Playlist = format!(
            "{header}#EXT-X-PART:DURATION=1,URI=\"s20.0.m4s\",INDEPENDENT=YES\n\
#EXT-X-PART:DURATION=1,URI=\"s20.1.m4s\",BYTERANGE=\"100@0\"\n#EXTINF:2,\ns20.m4s\n\
#EXTINF:4,\ns21.m4s\n\
#EXT-X-PART:DURATION=1,URI=\"s22.0.m4s\",INDEPENDENT=YES\n\
#EXT-X-PART:DURATION=0.5,URI=\"s22.1.m4s\",GAP=YES\n\
#EXT-X-PRELOAD-HINT:TYPE=PART,URI=\"s22.2.m4s\"\n"
        )
        .parse()
        .unwrap();

        let segments = playlist.media_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].parts.len(), 2);
        assert_eq!(segments[0].parts[0].uri, "s20.0.m4s");
        assert!(segments[0].parts[0].independent && !segments[0].parts[1].independent);
        assert_eq!(
            segments[0].parts[1].byterange,
            Some(ByteRange {
                length: 100,
                offset: Some(0)
            })
        );
        assert!(segments[1].parts.is_empty());

        let pending = playlist.pending_parts();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[1].duration, 0.5);
        assert!(pending[1].gap && !pending[0].gap);
        assert_eq!(playlist.pending_segment_sequence(), 22);
        assert_eq!(
            playlist.latest_part_position(),
            Some(PartPosition {
                sequence: 22,
                part: 1
            })
        );

        // Without pending parts the newest part is the last completed segment's.
        let completed: Playlist = format!(
            "{header}#EXT-X-PART:DURATION=1,URI=\"a\"\n#EXT-X-PART:DURATION=1,URI=\"b\"\n\
#EXT-X-PART:DURATION=1,URI=\"c\"\n#EXTINF:3,\ns20.m4s\n"
        )
        .parse()
        .unwrap();
        assert!(completed.pending_parts().is_empty());
        assert_eq!(
            completed.latest_part_position(),
            Some(PartPosition {
                sequence: 20,
                part: 2
            })
        );

        let plain: Playlist = format!("{header}#EXTINF:4,\ns20.m4s\n").parse().unwrap();
        assert_eq!(plain.latest_part_position(), None);
        assert_eq!(plain.pending_segment_sequence(), 21);

        // A delta update's skipped segments still advance the sequence.
        let delta: Playlist = format!(
            "{header}#EXT-X-SKIP:SKIPPED-SEGMENTS=3\n#EXTINF:4,\ns23.m4s\n\
#EXT-X-PART:DURATION=1,URI=\"p\"\n"
        )
        .parse()
        .unwrap();
        assert_eq!(delta.media_segments()[0].sequence, 23);
        assert_eq!(
            delta.latest_part_position(),
            Some(PartPosition {
                sequence: 24,
                part: 0
            })
        );
        let only_skip: Playlist = format!("{header}#EXT-X-SKIP:SKIPPED-SEGMENTS=3\n")
            .parse()
            .unwrap();
        assert_eq!(only_skip.pending_segment_sequence(), 23);
    }

    #[test]
    fn test_validate_media_group_rules() {
        let media =
            |attributes: &str| format!("#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"g\",{attributes}\n");
        let invalid = |reason: &str| ValidationError::InvalidMediaAttributes(reason.to_string());

        let errors = validation_errors(&format!(
            "#EXTM3U\n{}{}",
            media("NAME=\"a\",DEFAULT=YES"),
            media("NAME=\"b\",DEFAULT=YES")
        ));
        assert!(errors.contains(&invalid(
            "GROUP-ID=g has more than one DEFAULT=YES rendition"
        )));

        let errors = validation_errors(&format!(
            "#EXTM3U\n{}{}",
            media("NAME=\"a\""),
            media("NAME=\"a\"")
        ));
        assert!(errors.contains(&invalid("GROUP-ID=g has more than one rendition named a")));

        let errors = validation_errors(&format!(
            "#EXTM3U\n{}",
            media("NAME=\"a\",DEFAULT=YES,AUTOSELECT=NO,FORCED=NO")
        ));
        assert!(errors.contains(&invalid(
            "DEFAULT=YES requires AUTOSELECT=YES when AUTOSELECT is present"
        )));
        assert!(errors.contains(&invalid("FORCED is only allowed with TYPE=SUBTITLES")));
    }

    #[test]
    fn test_validate_low_latency_rules() {
        let errors = validation_errors(
            "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-PART:URI=\"p.mp4\",DURATION=1\n",
        );
        assert!(errors.contains(&ValidationError::MissingPartInf));

        let errors = validation_errors(
            "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-PART-INF:PART-TARGET=1\n\
#EXT-X-SERVER-CONTROL:HOLD-BACK=8,PART-HOLD-BACK=1\n",
        );
        assert!(errors.contains(&ValidationError::InvalidServerControl(
            "HOLD-BACK must be at least three times EXT-X-TARGETDURATION".to_string()
        )));
        assert!(errors.contains(&ValidationError::InvalidServerControl(
            "PART-HOLD-BACK must be at least twice PART-TARGET".to_string()
        )));
    }

    #[test]
    fn test_validate_sequence_tags_precede_first_segment() {
        let errors = validation_errors(
            "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXTINF:5,\na.ts\n#EXT-X-MEDIA-SEQUENCE:3\n#EXT-X-DISCONTINUITY-SEQUENCE:1\n",
        );
        assert!(errors.contains(&ValidationError::TagAfterFirstSegment(
            "EXT-X-MEDIA-SEQUENCE".to_string()
        )));
        assert!(errors.contains(&ValidationError::TagAfterFirstSegment(
            "EXT-X-DISCONTINUITY-SEQUENCE".to_string()
        )));
    }

    #[test]
    fn test_byte_order_mark_after_whitespace_does_not_survive_round_trip() {
        let playlist: Playlist = "\u{b}\u{feff}a.ts\n".parse().unwrap();
        assert_eq!(playlist.tags, vec![Tag::Uri("a.ts".to_string())]);
        let reparsed: Playlist = playlist.to_string().parse().unwrap();
        assert_eq!(reparsed, playlist);
    }

    #[test]
    fn test_parse_rejects_quote_inside_unquoted_attribute_value() {
        let (line, kind) = syntax_error(
            "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"n\",CHANNELS=2\"\"\n",
        );
        assert_eq!(line, 2);
        assert!(matches!(kind, SyntaxError::MalformedAttributeList { .. }));
    }
}
