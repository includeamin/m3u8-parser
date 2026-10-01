/// Represents different types of validation errors that can occur when processing an M3U8 playlist.
///
/// This enum captures specific validation issues that may arise when checking the conformity
/// of a playlist to the M3U8 specification. Each variant represents a distinct error, providing
/// context for what went wrong during validation.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum ValidationError {
    /// Error indicating that the #EXTM3U tag is missing from the playlist.
    MissingExtM3U,
    /// Error indicating that #EXTM3U is not the first tag in the playlist.
    ExtM3UNotFirst,
    /// Error indicating that a tag that may occur only once is duplicated.
    DuplicateTag(String),
    /// Error indicating that Master Playlist and Media Playlist tags are mixed.
    MixedPlaylistTypes,
    /// Error indicating that a Media Playlist is missing EXT-X-TARGETDURATION.
    MissingTargetDuration,
    /// Error indicating that an EXT-X-STREAM-INF tag is not followed by a URI.
    MissingVariantUri,
    /// Error indicating that a stream rendition-group reference cannot be resolved.
    UnresolvedRenditionGroup { attribute: String, group_id: String },
    /// Error indicating that a tag requires a newer EXT-X-VERSION.
    InsufficientVersion {
        tag: String,
        required: u8,
        actual: u8,
    },
    /// Error indicating that a tag was removed in the playlist's protocol version.
    RemovedTag { tag: String, removed_in: u8 },
    /// Error indicating that a media segment exceeds EXT-X-TARGETDURATION.
    SegmentDurationExceedsTarget { duration: f64, target_duration: u64 },

    /// Error indicating that the specified version is outside 1 through 12,
    /// the versions defined by draft-pantos-hls-rfc8216bis.
    ///
    /// # Arguments
    ///
    /// * `u8` - The invalid version number that was encountered.
    InvalidVersion(u8),

    /// Error indicating that the duration specified is invalid.
    ///
    /// # Arguments
    ///
    /// * `f64` - The invalid duration value that was encountered.
    InvalidDuration(f64),

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
    /// Error indicating that key attributes are inconsistent with the selected method.
    InvalidKeyAttributes(String),

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
    /// * `f64` - The invalid planned duration value that was encountered in the date range.
    InvalidDateRangePlannedDuration(f64),
    /// Error indicating incompatible EXT-X-DATERANGE attributes.
    InvalidDateRangeAttributes(String),

    /// Error indicating that a media tag is missing required fields.
    MissingMediaFields,

    /// Error indicating that EXT-X-MEDIA attributes are inconsistent with its TYPE.
    InvalidMediaAttributes(String),

    /// Error indicating that a preload hint URI is invalid.
    InvalidPreloadHintUri,

    /// Error indicating that a rendition report URI is invalid.
    InvalidRenditionReportUri,

    /// Error indicating that a segment's EXT-X-BYTERANGE has no offset but does not
    /// follow a sub-range of the same resource. Holds the segment URI.
    InvalidByteRange(String),
}
