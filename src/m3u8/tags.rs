/// Represents different types of tags found in an M3U8 playlist.
///
/// Each variant corresponds to a specific type of tag defined in the M3U8 specification.
/// This enum allows for easy manipulation and representation of these tags in a playlist.
#[derive(Debug, PartialEq)]
pub enum Tag {
    /// Indicates the start of an M3U8 file.
    ExtM3U,
    /// Specifies the version of the M3U8 playlist.
    ExtXVersion(u8),
    /// Represents a media segment with a duration and an optional title.
    ExtInf(f32, Option<String>),
    /// Indicates the target duration for media segments.
    ExtXTargetDuration(u32),
    /// Specifies the media sequence number.
    ExtXMediaSequence(u64),
    /// Represents a discontinuity sequence number.
    ExtXDiscontinuitySequence(u32),
    /// Marks the end of the playlist.
    ExtXEndList,
    /// Contains information about encryption keys.
    ExtXKey {
        method: String,
        uri: Option<String>,
        iv: Option<String>,
        keyformat: Option<String>,
        keyformatversions: Option<String>,
    },
    /// Represents a mapping to an initialization segment.
    ExtXMap {
        uri: String,
        byterange: Option<String>,
    },
    /// Specifies the program date and time.
    ExtXProgramDateTime(String),
    /// Represents a date range for events within the playlist.
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
    /// Represents a URI to a media segment.
    Uri(String),
}

impl std::fmt::Display for Tag {
    /// Formats the tag as a string for output.
    ///
    /// This method implements the `Display` trait for the `Tag` enum, allowing each
    /// variant to be converted into a string representation that conforms to the M3U8
    /// specification.
    ///
    /// # Arguments
    ///
    /// * `f` - A mutable reference to a formatter.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure of formatting.
    ///
    /// # Example
    ///
    /// ```
    /// use m3u8_parser::m3u8::tags::Tag;
    /// let tag = Tag::ExtXVersion(3);
    /// println!("{}", tag); // Outputs: #EXT-X-VERSION:3
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tag::ExtM3U => write!(f, "#EXTM3U"),
            Tag::ExtXVersion(version) => write!(f, "#EXT-X-VERSION:{}", version),
            Tag::ExtInf(duration, title) => {
                if let Some(title) = title {
                    write!(f, "#EXTINF:{},{},", duration, title)
                } else {
                    write!(f, "#EXTINF:{},", duration)
                }
            }
            Tag::ExtXTargetDuration(duration) => {
                write!(f, "#EXT-X-TARGETDURATION:{}", duration)
            }
            Tag::ExtXMediaSequence(sequence) => {
                write!(f, "#EXT-X-MEDIA-SEQUENCE:{}", sequence)
            }
            Tag::ExtXDiscontinuitySequence(sequence) => {
                write!(f, "#EXT-X-DISCONTINUITY-SEQUENCE:{}", sequence)
            }
            Tag::ExtXEndList => write!(f, "#EXT-X-ENDLIST"),
            Tag::ExtXKey {
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
                    write!(f, ",KEYFORMAT={}", keyformat)?;
                }
                if let Some(keyformatversions) = keyformatversions {
                    write!(f, ",KEYFORMATVERSIONS={}", keyformatversions)?;
                }
                Ok(())
            }
            Tag::ExtXMap { uri, byterange } => {
                write!(f, "#EXT-X-MAP:URI=\"{}\"", uri)?;
                if let Some(byterange) = byterange {
                    write!(f, ",BYTERANGE={}", byterange)?;
                }
                Ok(())
            }
            Tag::ExtXProgramDateTime(date_time) => {
                write!(f, "#EXT-X-PROGRAM-DATE-TIME:{}", date_time)
            }
            Tag::ExtXDateRange {
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
            Tag::Uri(uri) => write!(f, "{}", uri),
        }
    }
}
