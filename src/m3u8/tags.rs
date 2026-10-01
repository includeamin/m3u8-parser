/// Represents different types of tags found in an M3U8 playlist.
///
/// Each variant corresponds to a specific type of tag defined in the M3U8 specification.
/// This enum allows for easy manipulation and representation of these tags in a playlist.
#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
pub enum Tag {
    /// Indicates the start of an M3U8 file.
    ExtM3U,
    /// Specifies the version of the M3U8 playlist.
    ExtXVersion(u8),
    /// The EXT-X-PLAYLIST-TYPE tag provides mutability information about the
    //    Media Playlist file.  It applies to the entire Media Playlist file.
    //    It is OPTIONAL.  Its format is:
    ExtXPlaylistType(String),
    /// A media segment: its EXTINF duration and optional title, and the URI line that follows.
    ExtInf {
        uri: String,
        /// Duration in seconds.
        duration: f64,
        title: Option<String>,
    },
    /// Indicates the target duration for media segments.
    ExtXTargetDuration(u64),
    /// Specifies the media sequence number.
    ExtXMediaSequence(u64),
    /// Specifies whether clients may cache media segments.
    ExtXAllowCache(bool),
    /// Represents a discontinuity sequence number.
    ExtXDiscontinuitySequence(u32),
    /// Marks the end of the playlist.
    ExtXEndList,
    /// Indicates that each media segment is an I-frame.
    ExtXIFramesOnly,
    /// Contains information about encryption keys.
    ExtXKey(Key),
    /// Represents a mapping to an initialization segment.
    ExtXMap(Map),
    /// Specifies the program date and time.
    ExtXProgramDateTime(String),
    /// Represents a byte range.
    ExtXByteRange(ByteRange),
    /// Defines a custom tag with a specific value.
    ExtXDefine(String),
    /// Represents media information.
    ExtXMedia {
        type_: String,
        group_id: String,
        name: Option<String>,
        uri: Option<String>,
        default: Option<bool>,
        autoselect: Option<bool>,
        characteristics: Option<String>,
        language: Option<String>,
        instream_id: Option<String>,
        language_codec: Option<String>,
        forced: Option<bool>,
        channels: Option<String>,
    },
    /// Represents stream information.
    ExtXStreamInf {
        bandwidth: u32,
        average_bandwidth: Option<u32>,
        codecs: Option<String>,
        resolution: Option<String>,
        frame_rate: Option<f64>,
        audio: Option<String>,
        video: Option<String>,
        subtitle: Option<String>,
        closed_captions: Option<String>,
    },
    /// Represents an I-frame stream information.
    ExtXIFrameStreamInf {
        bandwidth: u32,
        average_bandwidth: Option<u32>,
        codecs: Option<String>,
        resolution: Option<String>,
        frame_rate: Option<f64>,
        uri: String,
    },
    /// Indicates a gap in the playlist.
    ExtXGap,
    /// Associates a range of media with a date range and optional metadata.
    ExtXDateRange {
        id: String,
        class: Option<String>,
        start_date: String,
        cue: Option<String>,
        end_date: Option<String>,
        duration: Option<f64>,
        planned_duration: Option<f64>,
        end_on_next: Option<bool>,
        scte35_cmd: Option<String>,
        scte35_out: Option<String>,
        scte35_in: Option<String>,
        /// `X-` client attributes and unrecognized attributes, in input order.
        extra_attributes: Vec<(String, AttributeValue)>,
    },
    /// Specifies the bitrate of the stream.
    ExtXBitrate(u32),
    /// Indicates that segments are independent.
    ExtXIndependentSegments,
    /// Specifies the start time offset.
    ExtXStart {
        /// Offset in seconds; negative values count from the end of the playlist.
        time_offset: f64,
        precise: Option<bool>,
    },
    /// Provides server control information.
    ExtXServerControl {
        can_skip_until: Option<f64>,
        can_skip_dateranges: Option<bool>,
        hold_back: Option<f64>,
        part_hold_back: Option<f64>,
        can_block_reload: Option<bool>,
    },
    /// Represents part information.
    ExtXPartInf { part_target_duration: f64 },
    /// Represents a preload hint.
    ExtXPreloadHint {
        /// The hinted resource type, `PART` or `MAP`.
        type_: String,
        uri: String,
        byterange_start: Option<u64>,
        byterange_length: Option<u64>,
    },
    /// Represents a rendition report.
    ExtXRenditionReport {
        uri: String,
        last_msn: Option<u64>,
        last_part: Option<u64>,
    },
    /// Represents a part of a media segment.
    ExtXPart {
        uri: String,
        duration: f64,
        independent: Option<bool>,
        byterange: Option<ByteRange>,
        gap: Option<bool>,
    },
    /// Indicates a skip in the playlist.
    ExtXSkip {
        skipped_segments: u32,
        /// Tab-delimited list of removed EXT-X-DATERANGE IDs.
        recently_removed_dateranges: Option<String>,
    },
    /// Indicates a discontinuity in the media stream.
    ExtXDiscontinuity,
    /// Represents session data for tracking and metadata.
    ExtXSessionData {
        id: String,
        value: Option<String>,
        uri: Option<String>,
        language: Option<String>,
    },
    /// Encryption key information for a Master Playlist.
    ExtXSessionKey(Key),
    /// A URI line associated with a preceding tag or playlist entry.
    Uri(String),
    /// An unrecognized tag, preserved without the leading `#`.
    Unknown(String),
}

/// Encryption key attributes shared by EXT-X-KEY and EXT-X-SESSION-KEY.
#[derive(Debug, PartialEq, Clone)]
pub struct Key {
    /// `NONE`, `AES-128`, or `SAMPLE-AES`.
    pub method: String,
    pub uri: Option<String>,
    pub iv: Option<String>,
    pub keyformat: Option<String>,
    pub keyformatversions: Option<String>,
}

impl std::fmt::Display for Key {
    /// Formats the attribute list, without the tag name.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "METHOD={}", self.method)?;
        if let Some(uri) = &self.uri {
            write!(f, ",URI=\"{}\"", uri)?;
        }
        if let Some(iv) = &self.iv {
            write!(f, ",IV={}", iv)?;
        }
        if let Some(keyformat) = &self.keyformat {
            write!(f, ",KEYFORMAT=\"{}\"", keyformat)?;
        }
        if let Some(keyformatversions) = &self.keyformatversions {
            write!(f, ",KEYFORMATVERSIONS=\"{}\"", keyformatversions)?;
        }
        Ok(())
    }
}

/// The Media Initialization Section declared by EXT-X-MAP.
#[derive(Debug, PartialEq, Clone)]
pub struct Map {
    pub uri: String,
    pub byterange: Option<ByteRange>,
}

/// A sub-range of a resource in `<length>[@<offset>]` form.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ByteRange {
    /// Length of the sub-range in bytes.
    pub length: u64,
    /// Start of the sub-range in bytes; when absent it follows the previous sub-range.
    pub offset: Option<u64>,
}

impl std::str::FromStr for ByteRange {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (length, offset) = match s.split_once('@') {
            Some((length, offset)) => (length, Some(offset.parse()?)),
            None => (s, None),
        };
        Ok(ByteRange {
            length: length.parse()?,
            offset,
        })
    }
}

impl std::fmt::Display for ByteRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.length)?;
        if let Some(offset) = self.offset {
            write!(f, "@{}", offset)?;
        }
        Ok(())
    }
}

/// An attribute value whose type is not modeled, preserving whether it was quoted.
#[derive(Debug, PartialEq, Clone)]
pub enum AttributeValue {
    /// A quoted-string value, stored without the surrounding quotes.
    Quoted(String),
    /// An unquoted value such as a decimal-integer, hexadecimal-sequence, or enumerated-string.
    Unquoted(String),
}

impl std::fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeValue::Quoted(value) => write!(f, "\"{}\"", value),
            AttributeValue::Unquoted(value) => write!(f, "{}", value),
        }
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "YES"
    } else {
        "NO"
    }
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
            Tag::ExtInf {
                uri: url,
                duration,
                title,
            } => {
                // Shortest exact form: whole durations stay integers for version < 3.
                let title = title.as_deref().unwrap_or_default();
                write!(f, "#EXTINF:{},{}\n{}", duration, title, url)
            }
            Tag::ExtXTargetDuration(duration) => {
                write!(f, "#EXT-X-TARGETDURATION:{}", duration)
            }
            Tag::ExtXMediaSequence(sequence) => {
                write!(f, "#EXT-X-MEDIA-SEQUENCE:{}", sequence)
            }
            Tag::ExtXAllowCache(allow_cache) => {
                write!(f, "#EXT-X-ALLOW-CACHE:{}", yes_no(*allow_cache))
            }
            Tag::ExtXDiscontinuitySequence(sequence) => {
                write!(f, "#EXT-X-DISCONTINUITY-SEQUENCE:{}", sequence)
            }
            Tag::ExtXEndList => write!(f, "#EXT-X-ENDLIST"),
            Tag::ExtXIFramesOnly => write!(f, "#EXT-X-I-FRAMES-ONLY"),
            Tag::ExtXKey(key) => write!(f, "#EXT-X-KEY:{}", key),
            Tag::ExtXMap(Map { uri, byterange }) => {
                write!(f, "#EXT-X-MAP:URI=\"{}\"", uri)?;
                if let Some(byterange) = byterange {
                    write!(f, ",BYTERANGE=\"{}\"", byterange)?;
                }
                Ok(())
            }
            Tag::ExtXProgramDateTime(date_time) => {
                write!(f, "#EXT-X-PROGRAM-DATE-TIME:{}", date_time)
            }
            Tag::ExtXByteRange(byterange) => {
                write!(f, "#EXT-X-BYTERANGE:{}", byterange)
            }
            Tag::ExtXDefine(value) => {
                write!(f, "#EXT-X-DEFINE:{}", value)
            }
            Tag::ExtXMedia {
                type_,
                group_id,
                name,
                uri,
                default,
                autoselect,
                characteristics,
                language,
                instream_id,
                language_codec,
                forced,
                channels,
            } => {
                // Basic required fields
                write!(f, "#EXT-X-MEDIA:TYPE={},GROUP-ID=\"{}\"", type_, group_id)?;

                // Optional URI field
                if let Some(uri) = uri {
                    write!(f, ",URI=\"{}\"", uri)?;
                }

                // Optional name field
                if let Some(name) = name {
                    write!(f, ",NAME=\"{}\"", name)?;
                }

                // Optional default field
                if let Some(default) = default {
                    write!(f, ",DEFAULT={}", yes_no(*default))?;
                }

                // Optional autoselect field
                if let Some(autoselect) = autoselect {
                    write!(f, ",AUTOSELECT={}", yes_no(*autoselect))?;
                }

                // Optional forced field
                if let Some(forced) = forced {
                    write!(f, ",FORCED={}", yes_no(*forced))?;
                }

                // Optional instream_id field
                if let Some(instream_id) = instream_id {
                    write!(f, ",INSTREAM-ID=\"{}\"", instream_id)?;
                }

                // Optional characteristics field
                if let Some(characteristics) = characteristics {
                    write!(f, ",CHARACTERISTICS=\"{}\"", characteristics)?;
                }

                // Optional language field
                if let Some(language) = language {
                    write!(f, ",LANGUAGE=\"{}\"", language)?;
                }

                // Optional language_codec field
                if let Some(language_codec) = language_codec {
                    write!(f, ",LANGUAGE-CODEC=\"{}\"", language_codec)?;
                }

                if let Some(channels) = channels {
                    write!(f, ",CHANNELS=\"{}\"", channels)?;
                }

                Ok(())
            }
            Tag::ExtXStreamInf {
                bandwidth,
                average_bandwidth,
                codecs,
                resolution,
                frame_rate,
                audio,
                video,
                subtitle,
                closed_captions,
            } => {
                write!(f, "#EXT-X-STREAM-INF:BANDWIDTH={}", bandwidth)?;
                if let Some(average_bandwidth) = average_bandwidth {
                    write!(f, ",AVERAGE-BANDWIDTH={}", average_bandwidth)?;
                }
                if let Some(codecs) = codecs {
                    write!(f, ",CODECS=\"{}\"", codecs)?;
                }
                if let Some(resolution) = resolution {
                    write!(f, ",RESOLUTION={}", resolution)?;
                }
                if let Some(frame_rate) = frame_rate {
                    write!(f, ",FRAME-RATE={}", frame_rate)?;
                }
                if let Some(audio) = audio {
                    write!(f, ",AUDIO=\"{}\"", audio)?;
                }
                if let Some(video) = video {
                    write!(f, ",VIDEO=\"{}\"", video)?;
                }
                if let Some(subtitle) = subtitle {
                    write!(f, ",SUBTITLES=\"{}\"", subtitle)?;
                }
                // NONE is an enumerated-string; group IDs are quoted-strings.
                match closed_captions.as_deref() {
                    Some("NONE") => write!(f, ",CLOSED-CAPTIONS=NONE")?,
                    Some(closed_captions) => write!(f, ",CLOSED-CAPTIONS=\"{}\"", closed_captions)?,
                    None => {}
                }
                Ok(())
            }
            Tag::ExtXIFrameStreamInf {
                bandwidth,
                average_bandwidth,
                codecs,
                resolution,
                frame_rate,
                uri,
            } => {
                write!(f, "#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH={}", bandwidth)?;
                if let Some(average_bandwidth) = average_bandwidth {
                    write!(f, ",AVERAGE-BANDWIDTH={}", average_bandwidth)?;
                }
                if let Some(codecs) = codecs {
                    write!(f, ",CODECS=\"{}\"", codecs)?;
                }
                if let Some(resolution) = resolution {
                    write!(f, ",RESOLUTION={}", resolution)?;
                }
                if let Some(frame_rate) = frame_rate {
                    write!(f, ",FRAME-RATE={}", frame_rate)?;
                }
                write!(f, ",URI=\"{}\"", uri)?;
                Ok(())
            }
            Tag::ExtXGap => write!(f, "#EXT-X-GAP"),
            Tag::ExtXDateRange {
                id,
                class,
                start_date,
                cue,
                end_date,
                duration,
                planned_duration,
                end_on_next,
                scte35_cmd,
                scte35_out,
                scte35_in,
                extra_attributes,
            } => {
                write!(
                    f,
                    "#EXT-X-DATERANGE:ID=\"{}\",START-DATE=\"{}\"",
                    id, start_date
                )?;
                if let Some(cue) = cue {
                    write!(f, ",CUE=\"{}\"", cue)?;
                }
                if let Some(class) = class {
                    write!(f, ",CLASS=\"{}\"", class)?;
                }
                if let Some(end_date) = end_date {
                    write!(f, ",END-DATE=\"{}\"", end_date)?;
                }
                if let Some(duration) = duration {
                    write!(f, ",DURATION={}", duration)?;
                }
                if let Some(planned_duration) = planned_duration {
                    write!(f, ",PLANNED-DURATION={}", planned_duration)?;
                }
                if let Some(end_on_next) = end_on_next {
                    write!(f, ",END-ON-NEXT={}", yes_no(*end_on_next))?;
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
                for (name, value) in extra_attributes {
                    write!(f, ",{}={}", name, value)?;
                }
                Ok(())
            }
            Tag::ExtXBitrate(bitrate) => {
                write!(f, "#EXT-X-BITRATE:{}", bitrate)
            }
            Tag::ExtXIndependentSegments => write!(f, "#EXT-X-INDEPENDENT-SEGMENTS"),
            Tag::ExtXStart {
                time_offset,
                precise,
            } => {
                write!(f, "#EXT-X-START:TIME-OFFSET={}", time_offset)?;
                if let Some(precise) = precise {
                    write!(f, ",PRECISE={}", yes_no(*precise))?;
                }
                Ok(())
            }
            Tag::ExtXServerControl {
                can_skip_until,
                can_skip_dateranges,
                hold_back,
                part_hold_back,
                can_block_reload,
            } => {
                let mut attributes = Vec::new();
                if let Some(can_skip_until) = can_skip_until {
                    attributes.push(format!("CAN-SKIP-UNTIL={}", can_skip_until));
                }
                if let Some(can_skip_dateranges) = can_skip_dateranges {
                    attributes.push(format!(
                        "CAN-SKIP-DATERANGES={}",
                        yes_no(*can_skip_dateranges)
                    ));
                }
                if let Some(hold_back) = hold_back {
                    attributes.push(format!("HOLD-BACK={}", hold_back));
                }
                if let Some(part_hold_back) = part_hold_back {
                    attributes.push(format!("PART-HOLD-BACK={}", part_hold_back));
                }
                if let Some(can_block_reload) = can_block_reload {
                    attributes.push(format!("CAN-BLOCK-RELOAD={}", yes_no(*can_block_reload)));
                }
                write!(f, "#EXT-X-SERVER-CONTROL:{}", attributes.join(","))
            }
            Tag::ExtXPartInf {
                part_target_duration,
            } => {
                write!(f, "#EXT-X-PART-INF:PART-TARGET={}", part_target_duration)
            }
            Tag::ExtXPreloadHint {
                type_,
                uri,
                byterange_start,
                byterange_length,
            } => {
                write!(f, "#EXT-X-PRELOAD-HINT:TYPE={},URI=\"{}\"", type_, uri)?;
                if let Some(byterange_start) = byterange_start {
                    write!(f, ",BYTERANGE-START={}", byterange_start)?;
                }
                if let Some(byterange_length) = byterange_length {
                    write!(f, ",BYTERANGE-LENGTH={}", byterange_length)?;
                }
                Ok(())
            }
            Tag::ExtXRenditionReport {
                uri,
                last_msn,
                last_part,
            } => {
                write!(f, "#EXT-X-RENDITION-REPORT:URI=\"{}\"", uri)?;
                if let Some(last_msn) = last_msn {
                    write!(f, ",LAST-MSN={}", last_msn)?;
                }
                if let Some(last_part) = last_part {
                    write!(f, ",LAST-PART={}", last_part)?;
                }
                Ok(())
            }
            Tag::ExtXPart {
                uri,
                duration,
                independent,
                byterange,
                gap,
            } => {
                write!(f, "#EXT-X-PART:DURATION={},URI=\"{}\"", duration, uri)?;
                if let Some(independent) = independent {
                    write!(f, ",INDEPENDENT={}", yes_no(*independent))?;
                }
                if let Some(byterange) = byterange {
                    write!(f, ",BYTERANGE=\"{}\"", byterange)?;
                }
                if let Some(gap) = gap {
                    write!(f, ",GAP={}", yes_no(*gap))?;
                }
                Ok(())
            }
            Tag::ExtXSkip {
                skipped_segments,
                recently_removed_dateranges,
            } => {
                write!(f, "#EXT-X-SKIP:SKIPPED-SEGMENTS={}", skipped_segments)?;
                if let Some(recently_removed_dateranges) = recently_removed_dateranges {
                    write!(
                        f,
                        ",RECENTLY-REMOVED-DATERANGES=\"{}\"",
                        recently_removed_dateranges
                    )?;
                }
                Ok(())
            }
            Tag::ExtXDiscontinuity => write!(f, "#EXT-X-DISCONTINUITY"),
            Tag::ExtXSessionData {
                id,
                value,
                uri,
                language,
            } => {
                write!(f, "#EXT-X-SESSION-DATA:DATA-ID=\"{}\"", id)?;
                if let Some(value) = value {
                    write!(f, ",VALUE=\"{}\"", value)?;
                }
                if let Some(uri) = uri {
                    write!(f, ",URI=\"{}\"", uri)?;
                }
                if let Some(language) = language {
                    write!(f, ",LANGUAGE=\"{}\"", language)?;
                }
                Ok(())
            }
            Tag::ExtXSessionKey(key) => write!(f, "#EXT-X-SESSION-KEY:{}", key),
            Tag::ExtXPlaylistType(playlist_type) => {
                write!(f, "#EXT-X-PLAYLIST-TYPE:{}", playlist_type)?;
                Ok(())
            }
            Tag::Uri(uri) => write!(f, "{}", uri),
            Tag::Unknown(tag) => write!(f, "#{}", tag),
        }
    }
}
