#[cfg(test)]
mod tests {
    use crate::m3u8::error::{ParseError, SubstitutionError, SyntaxError};
    use crate::m3u8::playlist::builder::PlaylistBuilder;
    use crate::m3u8::playlist::Playlist;
    use crate::m3u8::tags::{AttributeValue, ByteRange, Key, Map, Tag};
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
                    method: "AES-128".to_string(),
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
            segments[0].program_date_time.as_deref(),
            Some("2024-01-01T00:00:00Z")
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
                    method: "AES-128".to_string(),
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
                    method: "AES-128".to_string(),
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
                Tag::ExtXProgramDateTime("2020-01-01T00:00:00Z".to_string()),
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
                Tag::ExtXProgramDateTime("2020-01-01T00:00:00Z".to_string()),
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
                    method: "NONE".to_string(),
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
                    autoselect: Some(true),
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
                method: "AES-128".to_string(),
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
                    method: "AES-128".to_string(),
                    uri: Some("key.bin".to_string()),
                    iv: Some("0x01".to_string()),
                    keyformat: None,
                    keyformatversions: None,
                }),
                Tag::ExtXKey(Key {
                    method: "AES-128".to_string(),
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
                method: method.to_string(),
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
        let media = |type_: &str, uri: Option<&str>, instream_id: Option<&str>| Tag::ExtXMedia {
            type_: type_.to_string(),
            group_id: "group".to_string(),
            name: Some("name".to_string()),
            uri: uri.map(str::to_string),
            default: None,
            autoselect: None,
            characteristics: None,
            language: None,
            instream_id: instream_id.map(str::to_string),
            language_codec: None,
            forced: None,
            channels: None,
        };
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(6),
                media("CLOSED-CAPTIONS", Some("cc.m3u8"), None),
                media("AUDIO", None, Some("CC1")),
                media("CLOSED-CAPTIONS", None, Some("CC1")),
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
                    bandwidth: 1_000_000,
                    average_bandwidth: Some(900_000),
                    codecs: None,
                    resolution: None,
                    frame_rate: None,
                    audio: Some("group".to_string()),
                    video: None,
                    subtitle: None,
                    closed_captions: None,
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
                    method: "SAMPLE-AES".to_string(),
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
                Tag::ExtXProgramDateTime("2010-02-19T14:54:23.031+08:00".to_string()),
                date_range("2024-01-01T00:00:00Z", Some("2024-01-01T00:00:30.5Z")),
                Tag::ExtXProgramDateTime("2024-12-31T23:59:60-0500".to_string()),
            ],
        };
        let invalid = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXTargetDuration(10),
                Tag::ExtXProgramDateTime("yesterday".to_string()),
                date_range("2024-13-01T00:00:00Z", Some("2024-01-01 00:00:00Z")),
            ],
        };

        assert_eq!(valid.validate(), Ok(()));
        assert_eq!(
            invalid.validate(),
            Err(vec![
                ValidationError::InvalidProgramDateTime,
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
                Some("avc1.640020,mp4a.40.2"),
                Some("1280x720"),
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
            .server_control(Some(24.0), Some(1.5), true)
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
#EXT-X-SERVER-CONTROL:CAN-SKIP-UNTIL=24,PART-HOLD-BACK=1.5,CAN-BLOCK-RELOAD=YES
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
    fn test_validate_non_ascii_time_zone_does_not_panic() {
        let playlist: Playlist =
            "#EXTM3U\n#EXT-X-PROGRAM-DATE-TIME:2020-01-01T00:00:00+a\u{20ac}\n"
                .parse()
                .unwrap();
        let errors = playlist.validate().unwrap_err();
        assert!(errors.contains(&ValidationError::InvalidProgramDateTime));
    }

    #[test]
    fn test_validate_rejects_nonexistent_day() {
        for date in ["2021-02-31", "2021-04-31", "2023-02-29"] {
            let playlist: Playlist =
                format!("#EXTM3U\n#EXT-X-PROGRAM-DATE-TIME:{date}T00:00:00Z\n")
                    .parse()
                    .unwrap();
            assert!(playlist
                .validate()
                .unwrap_err()
                .contains(&ValidationError::InvalidProgramDateTime));
        }
        let leap: Playlist =
            "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXT-X-PROGRAM-DATE-TIME:2024-02-29T00:00:00Z\n"
                .parse()
                .unwrap();
        assert_eq!(leap.validate(), Ok(()));
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
}
