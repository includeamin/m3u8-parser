use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

pub mod m3u8 {
    #[derive(Debug, PartialEq)]
    pub enum Tag {
        ExtM3U,
        ExtXVersion(u8),
        ExtInf(f32, Option<String>),
        ExtXTargetDuration(u32),
        ExtXMediaSequence(u64),
        ExtXDiscontinuitySequence(u32),
        ExtXEndList,
        ExtXKey {
            method: String,
            uri: Option<String>,
            iv: Option<String>,
            keyformat: Option<String>,
            keyformatversions: Option<String>,
        },
        ExtXMap {
            uri: String,
            byterange: Option<String>,
        },
        ExtXProgramDateTime(String),
        ExtXDateRange {
            id: String,
            start_date: String,
            end_date: Option<String>,
            duration: Option<f32>,
            planned_duration: Option<f32>,
            scte35_cmd: Option<String>,
            scte35_out: Option<String>,
            scte35_in: Option<String>,
            end_on_next: Option<bool>,
        },
        Uri(String),
    }

    #[derive(Debug, PartialEq)]
    pub struct Playlist {
        pub tags: Vec<Tag>,
    }
}

impl m3u8::Playlist {
    pub fn from_reader<R: BufRead>(reader: R) -> Result<Self, String> {
        let mut tags = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?;
            if line.is_empty() {
                continue;
            }
            if line.starts_with("#EXTM3U") {
                tags.push(m3u8::Tag::ExtM3U);
            } else if line.starts_with("#EXT-X-VERSION:") {
                let version = line[15..].parse().unwrap();
                tags.push(m3u8::Tag::ExtXVersion(version));
            } else if line.starts_with("#EXTINF:") {
                let parts: Vec<&str> = line[8..].splitn(2, ',').collect();
                let duration = parts[0].parse().unwrap();
                let title = if parts.len() > 1 && !parts[1].to_string().is_empty() {
                    Some(parts[1].to_string())
                } else {
                    None
                };
                tags.push(m3u8::Tag::ExtInf(duration, title));
            } else if line.starts_with("#EXT-X-TARGETDURATION:") {
                let duration = line[22..].parse().unwrap();
                tags.push(m3u8::Tag::ExtXTargetDuration(duration));
            } else if line.starts_with("#EXT-X-MEDIA-SEQUENCE:") {
                let sequence = line[22..].parse().unwrap();
                tags.push(m3u8::Tag::ExtXMediaSequence(sequence));
            } else if line.starts_with("#EXT-X-DISCONTINUITY-SEQUENCE:") {
                let sequence = line[30..].parse().unwrap();
                tags.push(m3u8::Tag::ExtXDiscontinuitySequence(sequence));
            } else if line.starts_with("#EXT-X-ENDLIST") {
                tags.push(m3u8::Tag::ExtXEndList);
            } else if line.starts_with("#EXT-X-KEY:") {
                let attributes = parse_attributes(&line[11..])?;
                tags.push(m3u8::Tag::ExtXKey {
                    method: attributes
                        .get("METHOD")
                        .ok_or("Missing METHOD attribute")?
                        .clone(),
                    uri: attributes.get("URI").cloned(),
                    iv: attributes.get("IV").cloned(),
                    keyformat: attributes.get("KEYFORMAT").cloned(),
                    keyformatversions: attributes.get("KEYFORMATVERSIONS").cloned(),
                });
            } else if line.starts_with("#EXT-X-MAP:") {
                let attributes = parse_attributes(&line[11..])?;
                tags.push(m3u8::Tag::ExtXMap {
                    uri: attributes
                        .get("URI")
                        .ok_or("Missing URI attribute")?
                        .clone(),
                    byterange: attributes.get("BYTERANGE").cloned(),
                });
            } else if line.starts_with("#EXT-X-PROGRAM-DATE-TIME:") {
                tags.push(m3u8::Tag::ExtXProgramDateTime(line[25..].to_string()));
            } else if line.starts_with("#EXT-X-DATERANGE:") {
                let attributes = parse_attributes(&line[17..])?;
                tags.push(m3u8::Tag::ExtXDateRange {
                    id: attributes.get("ID").ok_or("Missing ID attribute")?.clone(),
                    start_date: attributes
                        .get("START-DATE")
                        .ok_or("Missing START-DATE attribute")?
                        .clone(),
                    end_date: attributes.get("END-DATE").cloned(),
                    duration: attributes
                        .get("DURATION")
                        .map(|s| s.parse::<f32>().unwrap()),
                    planned_duration: attributes
                        .get("PLANNED-DURATION")
                        .map(|s| s.parse().unwrap()),
                    scte35_cmd: attributes.get("SCTE35-CMD").cloned(),
                    scte35_out: attributes.get("SCTE35-OUT").cloned(),
                    scte35_in: attributes.get("SCTE35-IN").cloned(),
                    end_on_next: attributes.get("END-ON-NEXT").map(|s| s == "YES"),
                });
            } else if !line.starts_with('#') {
                tags.push(m3u8::Tag::Uri(line));
            }
        }
        Ok(m3u8::Playlist { tags })
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }

    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = File::create(path)?;
        for tag in &self.tags {
            writeln!(file, "{}", tag)?;
        }
        Ok(())
    }
}

fn parse_attributes(input: &str) -> Result<std::collections::HashMap<String, String>, String> {
    let mut attributes = std::collections::HashMap::new();
    for part in input.split(',') {
        let parts: Vec<&str> = part.splitn(2, '=').collect();
        if parts.len() == 2 {
            attributes.insert(parts[0].to_string(), parts[1].trim_matches('"').to_string());
        }
    }
    Ok(attributes)
}

impl std::fmt::Display for m3u8::Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            m3u8::Tag::ExtM3U => write!(f, "#EXTM3U"),
            m3u8::Tag::ExtXVersion(version) => write!(f, "#EXT-X-VERSION:{}", version),
            m3u8::Tag::ExtInf(duration, title) => {
                if let Some(title) = title {
                    write!(f, "#EXTINF:{},{},", duration, title)
                } else {
                    write!(f, "#EXTINF:{},", duration)
                }
            }
            m3u8::Tag::ExtXTargetDuration(duration) => {
                write!(f, "#EXT-X-TARGETDURATION:{}", duration)
            }
            m3u8::Tag::ExtXMediaSequence(sequence) => {
                write!(f, "#EXT-X-MEDIA-SEQUENCE:{}", sequence)
            }
            m3u8::Tag::ExtXDiscontinuitySequence(sequence) => {
                write!(f, "#EXT-X-DISCONTINUITY-SEQUENCE:{}", sequence)
            }
            m3u8::Tag::ExtXEndList => write!(f, "#EXT-X-ENDLIST"),
            m3u8::Tag::ExtXKey {
                method,
                uri,
                iv,
                keyformat,
                keyformatversions,
            } => {
                write!(f, "#EXT-X-KEY:METHOD={}", method)?;
                if let Some(uri) = uri {
                    write!(f, ",URI=\"{}\"", uri)?;
                }
                if let Some(iv) = iv {
                    write!(f, ",IV={}", iv)?;
                }
                if let Some(keyformat) = keyformat {
                    write!(f, ",KEYFORMAT=\"{}\"", keyformat)?;
                }
                if let Some(keyformatversions) = keyformatversions {
                    write!(f, ",KEYFORMATVERSIONS=\"{}\"", keyformatversions)?;
                }
                Ok(())
            }
            m3u8::Tag::ExtXMap { uri, byterange } => {
                write!(f, "#EXT-X-MAP:URI=\"{}\"", uri)?;
                if let Some(byterange) = byterange {
                    write!(f, ",BYTERANGE={}", byterange)?;
                }
                Ok(())
            }
            m3u8::Tag::ExtXProgramDateTime(datetime) => {
                write!(f, "#EXT-X-PROGRAM-DATE-TIME:{}", datetime)
            }
            m3u8::Tag::ExtXDateRange {
                id,
                start_date,
                end_date,
                duration,
                planned_duration,
                scte35_cmd,
                scte35_out,
                scte35_in,
                end_on_next,
            } => {
                write!(
                    f,
                    "#EXT-X-DATERANGE:ID=\"{}\",START-DATE=\"{}\"",
                    id, start_date
                )?;
                if let Some(end_date) = end_date {
                    write!(f, ",END-DATE=\"{}\"", end_date)?;
                }
                if let Some(duration) = duration {
                    write!(f, ",DURATION={}", duration)?;
                }
                if let Some(planned_duration) = planned_duration {
                    write!(f, ",PLANNED-DURATION={}", planned_duration)?;
                }
                if let Some(scte35_cmd) = scte35_cmd {
                    write!(f, ",SCTE35-CMD={}", scte35_cmd)?;
                }
                if let Some(scte35_out) = scte35_out {
                    write!(f, ",SCTE35-OUT={}", scte35_out)?;
                }
                if let Some(scte35_in) = scte35_in {
                    write!(f, ",SCTE35-IN={}", scte35_in)?;
                }
                if let Some(end_on_next) = end_on_next {
                    write!(
                        f,
                        ",END-ON-NEXT={}",
                        if *end_on_next { "YES" } else { "NO" }
                    )?;
                }
                Ok(())
            }
            m3u8::Tag::Uri(uri) => write!(f, "{}", uri),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::m3u8::{Playlist, Tag};
    use std::io::Write;

    #[test]
    fn test_parse_simple_playlist() {
        let data = r#"
#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:10
#EXTINF:9.009,
http://media.example.com/first.ts
#EXTINF:9.009,
http://media.example.com/second.ts
#EXTINF:3.003,
http://media.example.com/third.ts
#EXT-X-ENDLIST
"#;

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();
        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(3),
                Tag::ExtXTargetDuration(10),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/first.ts".to_string()),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("http://media.example.com/third.ts".to_string()),
                Tag::ExtXEndList,
            ]
        );
    }

    #[test]
    fn test_write_simple_playlist() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(3),
                Tag::ExtXTargetDuration(10),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/first.ts".to_string()),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("http://media.example.com/third.ts".to_string()),
                Tag::ExtXEndList,
            ],
        };

        let mut output = Vec::new();
        for tag in &playlist.tags {
            writeln!(output, "{}", tag).unwrap();
        }
        let output = String::from_utf8(output).unwrap();

        let expected = "#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:10
#EXTINF:9.009,
http://media.example.com/first.ts
#EXTINF:9.009,
http://media.example.com/second.ts
#EXTINF:3.003,
http://media.example.com/third.ts
#EXT-X-ENDLIST
";

        assert_eq!(output, expected);
    }

    #[test]
    fn test_parse_playlist_with_key() {
        let data = r#"
#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:10
#EXT-X-KEY:METHOD=AES-128,URI="https://priv.example.com/key.php?r=52"
#EXTINF:9.009,
http://media.example.com/first.ts
#EXTINF:9.009,
http://media.example.com/second.ts
#EXTINF:3.003,
http://media.example.com/third.ts
#EXT-X-ENDLIST
"#;

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();
        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(3),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXKey {
                    method: "AES-128".to_string(),
                    uri: Some("https://priv.example.com/key.php?r=52".to_string()),
                    iv: None,
                    keyformat: None,
                    keyformatversions: None,
                },
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/first.ts".to_string()),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("http://media.example.com/third.ts".to_string()),
                Tag::ExtXEndList,
            ]
        );
    }

    #[test]
    fn test_write_playlist_with_key() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(3),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXKey {
                    method: "AES-128".to_string(),
                    uri: Some("https://priv.example.com/key.php?r=52".to_string()),
                    iv: None,
                    keyformat: None,
                    keyformatversions: None,
                },
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/first.ts".to_string()),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("http://media.example.com/third.ts".to_string()),
                Tag::ExtXEndList,
            ],
        };

        let mut output = Vec::new();
        for tag in &playlist.tags {
            writeln!(output, "{}", tag).unwrap();
        }
        let output = String::from_utf8(output).unwrap();

        let expected = "#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:10
#EXT-X-KEY:METHOD=AES-128,URI=\"https://priv.example.com/key.php?r=52\"
#EXTINF:9.009,
http://media.example.com/first.ts
#EXTINF:9.009,
http://media.example.com/second.ts
#EXTINF:3.003,
http://media.example.com/third.ts
#EXT-X-ENDLIST
";

        assert_eq!(output, expected);
    }

    #[test]
    fn test_parse_playlist_with_map() {
        let data = r#"
#EXTM3U
#EXT-X-VERSION:6
#EXT-X-TARGETDURATION:10
#EXT-X-MAP:URI="init.mp4"
#EXTINF:9.009,
http://media.example.com/first.ts
#EXTINF:9.009,
http://media.example.com/second.ts
#EXTINF:3.003,
http://media.example.com/third.ts
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
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/first.ts".to_string()),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("http://media.example.com/third.ts".to_string()),
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
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/first.ts".to_string()),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("http://media.example.com/third.ts".to_string()),
                Tag::ExtXEndList,
            ],
        };

        let mut output = Vec::new();
        for tag in &playlist.tags {
            writeln!(output, "{}", tag).unwrap();
        }
        let output = String::from_utf8(output).unwrap();

        let expected = "#EXTM3U
#EXT-X-VERSION:6
#EXT-X-TARGETDURATION:10
#EXT-X-MAP:URI=\"init.mp4\"
#EXTINF:9.009,
http://media.example.com/first.ts
#EXTINF:9.009,
http://media.example.com/second.ts
#EXTINF:3.003,
http://media.example.com/third.ts
#EXT-X-ENDLIST
";

        assert_eq!(output, expected);
    }

    #[test]
    fn test_parse_playlist_with_program_date_time() {
        let data = r#"
#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:10
#EXT-X-PROGRAM-DATE-TIME:2020-01-01T00:00:00Z
#EXTINF:9.009,
http://media.example.com/first.ts
#EXTINF:9.009,
http://media.example.com/second.ts
#EXTINF:3.003,
http://media.example.com/third.ts
#EXT-X-ENDLIST
"#;

        let playlist = Playlist::from_reader(data.as_bytes()).unwrap();
        assert_eq!(
            playlist.tags,
            vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(3),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXProgramDateTime("2020-01-01T00:00:00Z".to_string()),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/first.ts".to_string()),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("http://media.example.com/third.ts".to_string()),
                Tag::ExtXEndList,
            ]
        );
    }

    #[test]
    fn test_write_playlist_with_program_date_time() {
        let playlist = Playlist {
            tags: vec![
                Tag::ExtM3U,
                Tag::ExtXVersion(3),
                Tag::ExtXTargetDuration(10),
                Tag::ExtXProgramDateTime("2020-01-01T00:00:00Z".to_string()),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/first.ts".to_string()),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("http://media.example.com/third.ts".to_string()),
                Tag::ExtXEndList,
            ],
        };

        let mut output = Vec::new();
        for tag in &playlist.tags {
            writeln!(output, "{}", tag).unwrap();
        }
        let output = String::from_utf8(output).unwrap();

        let expected = "#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:10
#EXT-X-PROGRAM-DATE-TIME:2020-01-01T00:00:00Z
#EXTINF:9.009,
http://media.example.com/first.ts
#EXTINF:9.009,
http://media.example.com/second.ts
#EXTINF:3.003,
http://media.example.com/third.ts
#EXT-X-ENDLIST
";

        assert_eq!(output, expected);
    }

    #[test]
    fn test_parse_playlist_with_daterange() {
        let data = r#"
#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:10
#EXT-X-DATERANGE:ID="ad-break",START-DATE="2020-01-01T00:00:00Z",DURATION=60.0
#EXTINF:9.009,
http://media.example.com/first.ts
#EXTINF:9.009,
http://media.example.com/second.ts
#EXTINF:3.003,
http://media.example.com/third.ts
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
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/first.ts".to_string()),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("http://media.example.com/third.ts".to_string()),
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
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/first.ts".to_string()),
                Tag::ExtInf(9.009, None),
                Tag::Uri("http://media.example.com/second.ts".to_string()),
                Tag::ExtInf(3.003, None),
                Tag::Uri("http://media.example.com/third.ts".to_string()),
                Tag::ExtXEndList,
            ],
        };

        let mut output = Vec::new();
        for tag in &playlist.tags {
            writeln!(output, "{}", tag).unwrap();
        }
        let output = String::from_utf8(output).unwrap();

        let expected = "#EXTM3U
#EXT-X-VERSION:7
#EXT-X-TARGETDURATION:10
#EXT-X-DATERANGE:ID=\"ad-break\",START-DATE=\"2020-01-01T00:00:00Z\",DURATION=60.6
#EXTINF:9.009,
http://media.example.com/first.ts
#EXTINF:9.009,
http://media.example.com/second.ts
#EXTINF:3.003,
http://media.example.com/third.ts
#EXT-X-ENDLIST
";

        assert_eq!(output, expected);
    }
}
