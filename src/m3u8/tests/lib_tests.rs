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
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/first.ts".to_string()),
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("https://media.example.com/third.ts".to_string()),
                Tag::ExtXEndList,
            ]
        );
    }

    #[test]
    fn test_write_simple_playlist() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(7),
                Tag::ExtXTargetDuration(10),
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/first.ts".to_string()),
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("https://media.example.com/third.ts".to_string()),
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
#EXT-X-URI:https://media.example.com/first.ts
#EXTINF:5.005,
#EXT-X-URI:https://media.example.com/second.ts
#EXTINF:3.003,
#EXT-X-URI:https://media.example.com/third.ts
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
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/first.ts".to_string()),
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("https://media.example.com/third.ts".to_string()),
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
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/first.ts".to_string()),
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("https://media.example.com/third.ts".to_string()),
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
#EXT-X-URI:https://media.example.com/first.ts
#EXTINF:5.005,
#EXT-X-URI:https://media.example.com/second.ts
#EXTINF:3.003,
#EXT-X-URI:https://media.example.com/third.ts
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
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/first.ts".to_string()),
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("https://media.example.com/third.ts".to_string()),
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
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/first.ts".to_string()),
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("https://media.example.com/third.ts".to_string()),
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
#EXT-X-URI:https://media.example.com/first.ts
#EXTINF:5.005,
#EXT-X-URI:https://media.example.com/second.ts
#EXTINF:3.003,
#EXT-X-URI:https://media.example.com/third.ts
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
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/first.ts".to_string()),
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("https://media.example.com/third.ts".to_string()),
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
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/first.ts".to_string()),
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("https://media.example.com/third.ts".to_string()),
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
#EXT-X-URI:https://media.example.com/first.ts
#EXTINF:5.005,
#EXT-X-URI:https://media.example.com/second.ts
#EXTINF:3.003,
#EXT-X-URI:https://media.example.com/third.ts
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
#EXT-X-DATERANGE:ID="ad-break",START-DATE="2020-01-01T00:00:00Z",DURATION=60.0
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
                Tag::ExtXDateRange {
                    id: "ad-break".to_string(),
                    start_date: "2020-01-01T00:00:00Z".to_string(),
                    end_date: None,
                    duration: Some(60.0),
                    planned_duration: None,
                    scte35_cmd: None,
                    scte35_out: None,
                    scte35_in: None,
                    end_on_next: None,
                },
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/first.ts".to_string()),
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("https://media.example.com/third.ts".to_string()),
                Tag::ExtXEndList,
            ]
        );
    }

    #[test]
    fn test_write_playlist_with_daterange() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(7),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXDateRange {
                    id: "ad-break".to_string(),
                    start_date: "2020-01-01T00:00:00Z".to_string(),
                    end_date: None,
                    duration: Some(60.6),
                    planned_duration: None,
                    scte35_cmd: None,
                    scte35_out: None,
                    scte35_in: None,
                    end_on_next: None,
                },
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/first.ts".to_string()),
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("https://media.example.com/third.ts".to_string()),
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
#EXT-X-DATERANGE:ID="ad-break",START-DATE="2020-01-01T00:00:00Z",DURATION=60.6
#EXTINF:5.005,
#EXT-X-URI:https://media.example.com/first.ts
#EXTINF:5.005,
#EXT-X-URI:https://media.example.com/second.ts
#EXTINF:3.003,
#EXT-X-URI:https://media.example.com/third.ts
#EXT-X-ENDLIST
"#;

        assert_eq!(output, expected);
    }

    #[test]
    fn test_playlist_builder() {
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .version(7)
            .target_duration(10)
            .extinf(5.005, None)
            .uri("https://media.example.com/first.ts".to_string())
            .extinf(5.005, None)
            .uri("https://media.example.com/second.ts".to_string())
            .extinf(3.003, None)
            .uri("https://media.example.com/third.ts".to_string())
            .end_list()
            .build()
            .unwrap();

        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(7),
                Tag::ExtXTargetDuration(10),
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/first.ts".to_string()),
                Tag::ExtInf(5.005, None),
                Tag::Uri("https://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("https://media.example.com/third.ts".to_string()),
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
#EXT-X-URI:https://media.example.com/first.ts
#EXTINF:5.005,
#EXT-X-URI:https://media.example.com/second.ts
#EXTINF:3.003,
#EXT-X-URI:https://media.example.com/third.ts
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
            .extinf(5.005, None)
            .uri("https://media.example.com/first.ts".to_string())
            .extinf(5.005, None)
            .uri("https://media.example.com/second.ts".to_string())
            .extinf(3.003, None)
            .uri("https://media.example.com/third.ts".to_string())
            .end_list()
            .build();

        assert!(playlist.is_ok());
    }

    #[test]
    fn test_validate_playlist_missing_extm3u() {
        let playlist = PlaylistBuilder::new()
            .version(3)
            .target_duration(10)
            .extinf(5.005, None)
            .uri("https://media.example.com/first.ts".to_string())
            .extinf(5.005, None)
            .uri("https://media.example.com/second.ts".to_string())
            .extinf(3.003, None)
            .uri("https://media.example.com/third.ts".to_string())
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
            .extinf(5.005, None)
            .uri("https://media.example.com/first.ts".to_string())
            .extinf(5.005, None)
            .uri("https://media.example.com/second.ts".to_string())
            .extinf(3.003, None)
            .uri("https://media.example.com/third.ts".to_string())
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
            .extinf(-5.005, None) // Invalid duration
            .uri("https://media.example.com/first.ts".to_string())
            .extinf(5.005, None)
            .uri("https://media.example.com/second.ts".to_string())
            .extinf(3.003, None)
            .uri("https://media.example.com/third.ts".to_string())
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
            .extinf(5.005, None)
            .uri("https://media.example.com/first.ts".to_string())
            .extinf(5.005, None)
            .uri("https://media.example.com/second.ts".to_string())
            .extinf(3.003, None)
            .uri("https://media.example.com/third.ts".to_string())
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
                "INVALID-METHOD".to_string(), // Invalid key method
                Some("https://priv.example.com/key.php?r=52".to_string()),
                None,
                None,
                None,
            )
            .extinf(5.005, None)
            .uri("https://media.example.com/first.ts".to_string())
            .extinf(5.005, None)
            .uri("https://media.example.com/second.ts".to_string())
            .extinf(3.003, None)
            .uri("https://media.example.com/third.ts".to_string())
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
            .map("".to_string(), None) // Invalid map URI
            .extinf(5.005, None)
            .uri("https://media.example.com/first.ts".to_string())
            .extinf(5.005, None)
            .uri("https://media.example.com/second.ts".to_string())
            .extinf(3.003, None)
            .uri("https://media.example.com/third.ts".to_string())
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
            .program_date_time("".to_string()) // Invalid program date time
            .extinf(5.005, None)
            .uri("https://media.example.com/first.ts".to_string())
            .extinf(5.005, None)
            .uri("https://media.example.com/second.ts".to_string())
            .extinf(3.003, None)
            .uri("https://media.example.com/third.ts".to_string())
            .end_list()
            .build();

        assert_eq!(playlist, Err(vec![ValidationError::InvalidProgramDateTime]));
    }

    #[test]
    fn test_validate_playlist_invalid_date_range() {
        let playlist = PlaylistBuilder::new()
            .extm3u()
            .version(3)
            .target_duration(10)
            .date_range(
                "".to_string(), // Invalid date range ID
                "2020-01-01T00:00:00Z".to_string(),
                None,
                Some(-60.0), // Invalid date range duration
                None,
                None,
                None,
                None,
                None,
            )
            .extinf(5.005, None)
            .uri("https://media.example.com/first.ts".to_string())
            .extinf(5.005, None)
            .uri("https://media.example.com/second.ts".to_string())
            .extinf(3.003, None)
            .uri("https://media.example.com/third.ts".to_string())
            .end_list()
            .build();

        assert_eq!(
            playlist,
            Err(vec![
                ValidationError::InvalidDateRangeId,
                ValidationError::InvalidDateRangeDuration(-60.0)
            ])
        );
    }
}
