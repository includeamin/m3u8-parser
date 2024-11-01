/// Represents different types of validation errors.
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    MissingExtM3U,
    InvalidVersion(u8),
    InvalidDuration(f32),
    InvalidTargetDuration(u32),
    InvalidMediaSequence(u64),
    InvalidKeyMethod(String),
    InvalidMapUri,
    InvalidProgramDateTime,
    InvalidDateRangeId,
    InvalidDateRangeStartDate,
    InvalidDateRangeEndDate,
    InvalidDateRangeDuration(f32),
    InvalidDateRangePlannedDuration(f32),
}
