/// Represents different types of validation errors that can occur when processing an M3U8 playlist.
///
/// This enum captures specific validation issues that may arise when checking the conformity
/// of a playlist to the M3U8 specification. Each variant represents a distinct error, providing
/// context for what went wrong during validation.
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    /// Error indicating that the #EXTM3U tag is missing from the playlist.
    MissingExtM3U,

    /// Error indicating that the specified version is invalid.
    ///
    /// # Arguments
    ///
    /// * `u8` - The invalid version number that was encountered.
    InvalidVersion(u8),

    /// Error indicating that the duration specified is invalid.
    ///
    /// # Arguments
    ///
    /// * `f32` - The invalid duration value that was encountered.
    InvalidDuration(f32),

    /// Error indicating that the target duration specified is invalid.
    ///
    /// # Arguments
    ///
    /// * `u32` - The invalid target duration value that was encountered.
    InvalidTargetDuration(u64),

    /// Error indicating that an invalid key method was specified.
    ///
    /// # Arguments
    ///
    /// * `String` - The invalid key method that was encountered.
    InvalidKeyMethod(String),

    /// Error indicating that the URI specified in a map tag is invalid.
    InvalidMapUri,

    /// Error indicating that the program date and time specified is invalid.
    InvalidProgramDateTime,

    /// Error indicating that the ID specified in a date range is invalid.
    InvalidDateRangeId,

    /// Error indicating that the start date specified in a date range is invalid.
    InvalidDateRangeStartDate,

    /// Error indicating that the end date specified in a date range is invalid.
    InvalidDateRangeEndDate,

    /// Error indicating that the planned duration specified in a date range is invalid.
    ///
    /// # Arguments
    ///
    /// * `f32` - The invalid planned duration value that was encountered in the date range.
    InvalidDateRangePlannedDuration(f32),

    /// Error indicating that the specified byte range is invalid.
    ///
    /// # Arguments
    ///
    /// * `String` - The invalid byte range that was encountered.
    InvalidByteRange(String),

    /// Error indicating that a media tag is missing required fields.
    MissingMediaFields,

    /// Error indicating that a stream information tag is invalid.
    ///
    /// # Arguments
    ///
    /// * `String` - The invalid stream information encountered.
    InvalidStreamInf(String),

    /// Error indicating that an I-frame stream information tag is invalid.
    ///
    /// # Arguments
    ///
    /// * `String` - The invalid I-frame stream information encountered.
    InvalidIFrameStreamInf(String),

    /// Error indicating that a part tag is invalid.
    ///
    /// # Arguments
    ///
    /// * `String` - The invalid part information encountered.
    InvalidPartInfo(String),

    /// Error indicating that a preload hint URI is invalid.
    InvalidPreloadHintUri,

    /// Error indicating that a rendition report URI is invalid.
    InvalidRenditionReportUri,

    /// Error indicating that the server control information is invalid.
    InvalidServerControl,

    /// Error indicating that the specified start time offset is invalid.
    InvalidStartTimeOffset,

    /// Error indicating that a skip tag is invalid.
    ///
    /// # Arguments
    ///
    /// * `String` - The invalid skip tag information encountered.
    InvalidSkipTag(String),

    /// Error indicating that the specified bitrate is invalid.
    ///
    /// # Arguments
    ///
    /// * `u32` - The invalid bitrate value that was encountered.
    InvalidBitrate(u32),

    /// Error indicating that the specified start offset is invalid.
    InvalidStartOffset,
}
