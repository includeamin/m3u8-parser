mod lib_tests;
mod validation;

/// A module for handling M3U8 playlists, including parsing and generating playlist tags.
pub mod m3u8_parser {
    use crate::validation::ValidationError;
    use std::fs::File;
    use std::io;
    use std::io::BufRead;
    use std::io::BufReader;
    use std::io::Write;
    use std::path::Path;

    /// Represents different types of tags found in an M3U8 playlist.
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

    /// Represents a playlist containing multiple tags.
    #[derive(Debug, PartialEq)]
    pub struct Playlist {
        pub tags: Vec<Tag>,
    }

    impl Playlist {
        /// Creates a new `Playlist` by reading tags from a buffered reader.
        ///
        /// # Arguments
        ///
        /// * `reader` - A buffered reader providing lines of the playlist.
        ///
        /// # Returns
        ///
        /// A result containing a `Playlist` or an error message as a string.
        pub fn from_reader<R: BufRead>(reader: R) -> Result<Self, String> {
            let mut tags = Vec::new();
            for line in reader.lines() {
                let line = line.map_err(|e| e.to_string())?;
                if line.is_empty() {
                    continue;
                }
                if line.starts_with("#EXTM3U") {
                    tags.push(Tag::ExtM3U);
                } else if let Some(stripped) = line.strip_prefix("#EXT-X-VERSION:") {
                    let version = stripped.parse().unwrap();
                    tags.push(Tag::ExtXVersion(version));
                } else if let Some(stripped) = line.strip_prefix("#EXTINF:") {
                    let parts: Vec<&str> = stripped.splitn(2, ',').collect();
                    let duration = parts[0].parse().unwrap();
                    let title = if parts.len() > 1 && !parts[1].to_string().is_empty() {
                        Some(parts[1].to_string())
                    } else {
                        None
                    };
                    tags.push(Tag::ExtInf(duration, title));
                } else if let Some(stripped) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
                    let duration = stripped.parse().unwrap();
                    tags.push(Tag::ExtXTargetDuration(duration));
                } else if let Some(stripped) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
                    let sequence = stripped.parse().unwrap();
                    tags.push(Tag::ExtXMediaSequence(sequence));
                } else if let Some(stripped) = line.strip_prefix("#EXT-X-DISCONTINUITY-SEQUENCE:") {
                    let sequence = stripped.parse().unwrap();
                    tags.push(Tag::ExtXDiscontinuitySequence(sequence));
                } else if line.starts_with("#EXT-X-ENDLIST") {
                    tags.push(Tag::ExtXEndList);
                } else if let Some(stripped) = line.strip_prefix("#EXT-X-KEY:") {
                    let attributes = parse_attributes(stripped)?;
                    tags.push(Tag::ExtXKey {
                        method: attributes
                            .get("METHOD")
                            .ok_or("Missing METHOD attribute")?
                            .clone(),
                        uri: attributes.get("URI").cloned(),
                        iv: attributes.get("IV").cloned(),
                        keyformat: attributes.get("KEYFORMAT").cloned(),
                        keyformatversions: attributes.get("KEYFORMATVERSIONS").cloned(),
                    });
                } else if let Some(stripped) = line.strip_prefix("#EXT-X-MAP:") {
                    let attributes = parse_attributes(stripped)?;
                    tags.push(Tag::ExtXMap {
                        uri: attributes
                            .get("URI")
                            .ok_or("Missing URI attribute")?
                            .clone(),
                        byterange: attributes.get("BYTERANGE").cloned(),
                    });
                } else if let Some(stripped) = line.strip_prefix("#EXT-X-PROGRAM-DATE-TIME:") {
                    tags.push(Tag::ExtXProgramDateTime(stripped.to_string()));
                } else if let Some(stripped) = line.strip_prefix("#EXT-X-DATERANGE:") {
                    let attributes = parse_attributes(stripped)?;
                    tags.push(Tag::ExtXDateRange {
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
                    tags.push(Tag::Uri(line));
                }
            }
            Ok(Playlist { tags })
        }

        /// Creates a new `Playlist` by reading tags from a file.
        ///
        /// # Arguments
        ///
        /// * `path` - The path to the file containing the playlist.
        ///
        /// # Returns
        ///
        /// A result containing a `Playlist` or an error message as a string.
        pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
            let file = File::open(path).map_err(|e| e.to_string())?;
            let reader = BufReader::new(file);
            Self::from_reader(reader)
        }

        /// Writes the playlist to a file.
        ///
        /// # Arguments
        ///
        /// * `path` - The path where the playlist should be saved.
        ///
        /// # Returns
        ///
        /// A result indicating success or an error if the write fails.
        pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
            let mut file = File::create(path)?;
            for tag in &self.tags {
                writeln!(file, "{}", tag)?;
            }
            Ok(())
        }

        /// Validates the playlist according to RFC 8216.
        ///
        /// # Returns
        ///
        /// A result indicating success or a list of validation errors.
        pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
            let mut errors = Vec::new();

            // Ensure the playlist starts with #EXTM3U
            match self.tags.first() {
                Some(Tag::ExtM3U) => {}
                _ => errors.push(ValidationError::MissingExtM3U),
            }

            // Validate each tag according to its rules
            for tag in &self.tags {
                match tag {
                    Tag::ExtXVersion(version) => {
                        if *version < 1 || *version > 7 {
                            errors.push(ValidationError::InvalidVersion(*version));
                        }
                    }
                    Tag::ExtInf(duration, _) => {
                        if *duration <= 0.0 {
                            errors.push(ValidationError::InvalidDuration(*duration));
                        }
                    }
                    Tag::ExtXTargetDuration(duration) => {
                        if *duration == 0 {
                            errors.push(ValidationError::InvalidTargetDuration(*duration));
                        }
                    }
                    Tag::ExtXMediaSequence(sequence) => {
                        if *sequence == 0 {
                            errors.push(ValidationError::InvalidMediaSequence(*sequence));
                        }
                    }
                    Tag::ExtXKey { method, .. } => {
                        if method != "NONE" && method != "AES-128" && method != "SAMPLE-AES" {
                            errors.push(ValidationError::InvalidKeyMethod(method.clone()));
                        }
                    }
                    Tag::ExtXMap { uri, .. } => {
                        if uri.is_empty() {
                            errors.push(ValidationError::InvalidMapUri);
                        }
                    }
                    Tag::ExtXProgramDateTime(date_time) => {
                        if date_time.is_empty() {
                            errors.push(ValidationError::InvalidProgramDateTime);
                        }
                    }
                    Tag::ExtXDateRange {
                        id,
                        start_date,
                        end_date,
                        duration,
                        planned_duration,
                        ..
                    } => {
                        if id.is_empty() {
                            errors.push(ValidationError::InvalidDateRangeId);
                        }
                        if start_date.is_empty() {
                            errors.push(ValidationError::InvalidDateRangeStartDate);
                        }
                        if let Some(end_date) = end_date {
                            if end_date < start_date {
                                errors.push(ValidationError::InvalidDateRangeEndDate);
                            }
                        }
                        if let Some(duration) = duration {
                            if *duration < 0.0 {
                                errors.push(ValidationError::InvalidDateRangeDuration(*duration));
                            }
                        }
                        if let Some(planned_duration) = planned_duration {
                            if *planned_duration < 0.0 {
                                errors.push(ValidationError::InvalidDateRangePlannedDuration(
                                    *planned_duration,
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
    }

    /// A builder for creating a `Playlist` with a chained interface.
    pub struct PlaylistBuilder {
        tags: Vec<Tag>,
    }

    impl Default for PlaylistBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    impl PlaylistBuilder {
        /// Creates a new `PlaylistBuilder`.
        pub fn new() -> Self {
            Self { tags: Vec::new() }
        }

        /// Adds an `ExtM3U` tag.
        pub fn extm3u(mut self) -> Self {
            self.tags.push(Tag::ExtM3U);
            self
        }

        /// Adds an `ExtXVersion` tag.
        pub fn version(mut self, version: u8) -> Self {
            self.tags.push(Tag::ExtXVersion(version));
            self
        }

        /// Adds an `ExtInf` tag.
        pub fn extinf(mut self, duration: f32, title: Option<String>) -> Self {
            self.tags.push(Tag::ExtInf(duration, title));
            self
        }

        /// Adds an `ExtXTargetDuration` tag.
        pub fn target_duration(mut self, duration: u32) -> Self {
            self.tags.push(Tag::ExtXTargetDuration(duration));
            self
        }

        /// Adds an `ExtXMediaSequence` tag.
        pub fn media_sequence(mut self, sequence: u64) -> Self {
            self.tags.push(Tag::ExtXMediaSequence(sequence));
            self
        }

        /// Adds an `ExtXDiscontinuitySequence` tag.
        pub fn discontinuity_sequence(mut self, sequence: u32) -> Self {
            self.tags.push(Tag::ExtXDiscontinuitySequence(sequence));
            self
        }

        /// Adds an `ExtXEndList` tag.
        pub fn end_list(mut self) -> Self {
            self.tags.push(Tag::ExtXEndList);
            self
        }

        /// Adds an `ExtXKey` tag.
        pub fn key(
            mut self,
            method: String,
            uri: Option<String>,
            iv: Option<String>,
            keyformat: Option<String>,
            keyformatversions: Option<String>,
        ) -> Self {
            self.tags.push(Tag::ExtXKey {
                method,
                uri,
                iv,
                keyformat,
                keyformatversions,
            });
            self
        }

        /// Adds an `ExtXMap` tag.
        pub fn map(mut self, uri: String, byterange: Option<String>) -> Self {
            self.tags.push(Tag::ExtXMap { uri, byterange });
            self
        }

        /// Adds an `ExtXProgramDateTime` tag.
        pub fn program_date_time(mut self, date_time: String) -> Self {
            self.tags.push(Tag::ExtXProgramDateTime(date_time));
            self
        }

        /// Adds an `ExtXDateRange` tag.
        #[allow(clippy::too_many_arguments)]
        pub fn date_range(
            mut self,
            id: String,
            start_date: String,
            end_date: Option<String>,
            duration: Option<f32>,
            planned_duration: Option<f32>,
            scte35_cmd: Option<String>,
            scte35_out: Option<String>,
            scte35_in: Option<String>,
            end_on_next: Option<bool>,
        ) -> Self {
            self.tags.push(Tag::ExtXDateRange {
                id,
                start_date,
                end_date,
                duration,
                planned_duration,
                scte35_cmd,
                scte35_out,
                scte35_in,
                end_on_next,
            });
            self
        }

        /// Adds a `Uri` tag.
        pub fn uri(mut self, uri: String) -> Self {
            self.tags.push(Tag::Uri(uri));
            self
        }

        /// Builds the `Playlist`, validating it according to RFC 8216.
        ///
        /// # Returns
        ///
        /// A result containing a `Playlist` if valid, or a list of validation errors.
        pub fn build(self) -> Result<Playlist, Vec<ValidationError>> {
            let playlist = Playlist { tags: self.tags };
            match playlist.validate() {
                Ok(_) => Ok(playlist),
                Err(errors) => Err(errors),
            }
        }
    }

    /// Parses attributes from a given input string and returns a map of key-value pairs.
    ///
    /// # Arguments
    ///
    /// * `input` - A string containing the attributes to be parsed.
    ///
    /// # Returns
    ///
    /// A result containing a map of attributes or an error message as a string.
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

    impl std::fmt::Display for Tag {
        /// Formats the tag as a string for output.
        ///
        /// # Arguments
        ///
        /// * `f` - A mutable reference to a formatter.
        ///
        /// # Returns
        ///
        /// A result indicating success or failure of formatting.
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
}
