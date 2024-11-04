/// Represents different types of tags found in an M3U8 playlist.
///
/// Each variant corresponds to a specific type of tag defined in the M3U8 specification.
/// This enum allows for easy manipulation and representation of these tags in a playlist.
#[derive(Debug, PartialEq, Clone)]
pub enum Tag {
    /// Indicates the start of an M3U8 file.
    ExtM3U,
    /// Specifies the version of the M3U8 playlist.
    ExtXVersion(u8),
    /// The EXT-X-PLAYLIST-TYPE tag provides mutability information about the
    //    Media Playlist file.  It applies to the entire Media Playlist file.
    //    It is OPTIONAL.  Its format is:
    ExtXPlaylistType(String),
    /// Represents a media segment with a duration and an optional title.
    ExtInf(f32, Option<String>),
    /// Indicates the target duration for media segments.
    ExtXTargetDuration(u64),
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
    /// Represents a byte range.
    ExtXByteRange(String),
    /// Defines a custom tag with a specific value.
    ExtXDefine(String),
    /// Represents media information.
    ExtXMedia {
        type_: String,
        group_id: String,
        name: Option<String>,
        uri: Option<String>,
        default: Option<bool>,
        autoplay: Option<bool>,
        characteristics: Option<String>,
        language: Option<String>,
    },
    /// Represents stream information.
    ExtXStreamInf {
        bandwidth: u32,
        codecs: Option<String>,
        resolution: Option<String>,
        frame_rate: Option<f32>,
        audio: Option<String>,
        video: Option<String>,
        subtitle: Option<String>,
        closed_captions: Option<String>,
    },
    /// Represents an I-frame stream information.
    ExtXIFrameStreamInf {
        bandwidth: u32,
        codecs: Option<String>,
        resolution: Option<String>,
        frame_rate: Option<f32>,
        uri: String,
    },
    /// Indicates a gap in the playlist.
    ExtXGap,
    /// Specifies the bitrate of the stream.
    ExtXBitrate(u32),
    /// Indicates that segments are independent.
    ExtXIndependentSegments,
    /// Specifies the start time offset.
    ExtXStart {
        time_offset: String,
        precise: Option<bool>,
    },
    /// Provides server control information.
    ExtXServerControl {
        can_play: Option<bool>,
        can_seek: Option<bool>,
        can_pause: Option<bool>,
        min_buffer_time: Option<f32>,
    },
    /// Represents part information.
    ExtXPartInf {
        part_target_duration: f32,
        part_hold_back: Option<f32>,
        part_number: Option<u64>,
    },
    /// Represents a preload hint.
    ExtXPreloadHint {
        uri: String,
        /// Optional byte range for the preload hint.
        byterange: Option<String>,
    },
    /// Represents a rendition report.
    ExtXRenditionReport { uri: String, bandwidth: u32 },
    /// Represents a part of a media segment.
    ExtXPart {
        uri: String,
        duration: Option<f32>,
        // additional fields if necessary
    },
    /// Indicates a skip in the playlist.
    ExtXSkip {
        uri: String,
        duration: Option<f32>,
        skipped_segments: u32,
        reason: Option<String>,
    },
    /// Indicates a discontinuity in the media stream.
    ExtXDiscontinuity,
    /// Represents session data for tracking and metadata.
    ExtXSessionData {
        id: String,
        value: String,
        // Optional fields for additional parameters
        language: Option<String>,
    },
    ExtXSessionKey {
        method: String,
        uri: Option<String>,
        iv: Option<String>,
    },
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
                autoplay,
                characteristics,
                language,
            } => {
                // Basic fields
                write!(f, "#EXT-X-MEDIA:TYPE={},GROUP-ID=\"{}\"", type_, group_id)?;

                // Required URI field
                if let Some(uri) = uri {
                    write!(f, ",URI=\"{}\"", uri)?;
                }

                // Optional fields
                if let Some(name) = name {
                    write!(f, ",NAME=\"{}\"", name)?;
                }
                if let Some(default) = default {
                    write!(f, ",DEFAULT={}", if *default { "YES" } else { "NO" })?;
                }
                if let Some(autoplay) = autoplay {
                    write!(f, ",AUTOPLAY={}", if *autoplay { "YES" } else { "NO" })?;
                }
                if let Some(characteristics) = characteristics {
                    write!(f, ",CHARACTERISTICS={}", characteristics)?;
                }
                if let Some(language) = language {
                    write!(f, ",LANGUAGE=\"{}\"", language)?;
                }

                Ok(())
            }
            Tag::ExtXStreamInf {
                bandwidth,
                codecs,
                resolution,
                frame_rate,
                audio,
                video,
                subtitle,
                closed_captions,
            } => {
                write!(f, "#EXT-X-STREAM-INF:BANDWIDTH={}", bandwidth)?;
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
                if let Some(closed_captions) = closed_captions {
                    write!(f, ",CLOSED-CAPTIONS=\"{}\"", closed_captions)?;
                }
                Ok(())
            }
            Tag::ExtXIFrameStreamInf {
                bandwidth,
                codecs,
                resolution,
                frame_rate,
                uri,
            } => {
                write!(f, "#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH={}", bandwidth)?;
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
                    write!(f, ",PRECISE={}", if *precise { "YES" } else { "NO" })?;
                }
                Ok(())
            }
            Tag::ExtXServerControl {
                can_play,
                can_seek,
                can_pause,
                min_buffer_time,
            } => {
                write!(f, "#EXT-X-SERVER-CONTROL")?;
                if let Some(can_play) = can_play {
                    write!(f, ",CAN-PLAY={}", if *can_play { "YES" } else { "NO" })?;
                }
                if let Some(can_seek) = can_seek {
                    write!(f, ",CAN-SEEK={}", if *can_seek { "YES" } else { "NO" })?;
                }
                if let Some(can_pause) = can_pause {
                    write!(f, ",CAN-PAUSE={}", if *can_pause { "YES" } else { "NO" })?;
                }
                if let Some(min_buffer_time) = min_buffer_time {
                    write!(f, ",MIN-BUFFER-TIME={}", min_buffer_time)?;
                }
                Ok(())
            }
            Tag::ExtXPartInf {
                part_target_duration,
                part_hold_back,
                part_number,
            } => {
                write!(f, "#EXT-X-PART-INF:PART-TARGET={}", part_target_duration)?;
                if let Some(part_hold_back) = part_hold_back {
                    write!(f, ",PART-HOLD-BACK={}", part_hold_back)?;
                }
                if let Some(part_number) = part_number {
                    write!(f, ",PART-NUMBER={}", part_number)?;
                }
                Ok(())
            }
            Tag::ExtXPreloadHint { uri, byterange } => {
                let mut result = format!("#EXT-X-PRELOAD-HINT:URI=\"{}\"", uri);
                if let Some(byterange) = byterange {
                    result.push_str(&format!(",BYTERANGE={}", byterange));
                }
                write!(f, "{}", result)
            }
            Tag::ExtXRenditionReport { uri, bandwidth } => {
                write!(
                    f,
                    "#EXT-X-RENDITION-REPORT:URI=\"{}\",BANDWIDTH={}",
                    uri, bandwidth
                )
            }
            Tag::ExtXPart { uri, duration } => {
                write!(f, "#EXT-X-PART:URI=\"{}\"", uri)?;
                if let Some(duration) = duration {
                    write!(f, ",DURATION={}", duration)?;
                }
                Ok(())
            }
            Tag::ExtXSkip {
                uri,
                duration,
                skipped_segments,
                reason,
            } => {
                let mut output = format!(
                    "#EXT-X-SKIP:URI=\"{}\",SKIPPED-SEGMENTS={}",
                    uri, skipped_segments
                );

                if let Some(duration) = duration {
                    output.push_str(&format!(",DURATION={}", duration));
                }

                if let Some(reason) = reason {
                    output.push_str(&format!(",REASON=\"{}\"", reason));
                }

                write!(f, "{}", output)
            }
            Tag::Uri(uri) => {
                write!(f, "#EXT-X-URI:{}", uri)
            }
            Tag::ExtXDiscontinuity => write!(f, "#EXT-X-DISCONTINUITY"),
            Tag::ExtXSessionData {
                id,
                value,
                language,
            } => {
                write!(f, "#EXT-X-SESSION-DATA:ID=\"{}\",VALUE=\"{}\"", id, value)?;
                if let Some(language) = language {
                    write!(f, ",LANGUAGE=\"{}\"", language)?;
                }
                Ok(())
            }
            Tag::ExtXSessionKey { method, uri, iv } => {
                write!(f, "#EXT-X-SESSION-KEY:METHOD={}", method)?;
                if let Some(uri) = uri {
                    write!(f, ",URI={}", uri)?;
                }
                if let Some(iv) = iv {
                    write!(f, ",IV={}", iv)?;
                }
                Ok(())
            }
            Tag::ExtXPlaylistType(playlist_type) => {
                write!(f, "#EXT-X-PLAYLIST-TYPE:{}", playlist_type)?;
                Ok(())
            }
        }
    }
}
