//! The ISO 8601 date-time HLS uses for `EXT-X-PROGRAM-DATE-TIME`.

use std::fmt;
use std::str::FromStr;

/// A calendar date and time of day with an optional UTC offset, such as
/// `2010-02-19T14:54:23.031+08:00`.
///
/// Parsing accepts the forms HLS playlists use in practice: `HH:MM` or
/// `HH:MM:SS` with an optional fraction of a second (the clock may be `HH:MM`
/// only without a fraction), and a zone of `Z`, `+HH`, `+HHMM` or `+HH:MM`. A
/// missing zone is kept as `None` rather than guessed. A leap second (`:60`) is
/// accepted.
///
/// Writing is normalized, except that the number of fraction digits is kept so
/// `.000` survives a round trip: the seconds are always present, and a zero
/// offset is written as `Z`. Equality and hashing compare the instant's fields
/// and offset, not the number of fraction digits.
#[derive(Debug, Clone, Copy)]
pub struct DateTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    nanosecond: u32,
    /// How many digits of the fraction the text had (0 to 9).
    fraction_digits: u8,
    offset_minutes: Option<i16>,
}

impl PartialEq for DateTime {
    fn eq(&self, other: &DateTime) -> bool {
        self.fields() == other.fields()
    }
}

impl Eq for DateTime {}

impl std::hash::Hash for DateTime {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.fields().hash(state);
    }
}

/// The error returned when a string is not a valid [`DateTime`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ParseDateTimeError;

impl fmt::Display for ParseDateTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid ISO 8601 date-time")
    }
}

impl std::error::Error for ParseDateTimeError {}

impl DateTime {
    /// Builds a date-time, returning `None` when a field is out of range.
    ///
    /// `year` is 0 to 9999, `second` is 0 to 60 (60 is a leap second),
    /// `nanosecond` is below one billion, and `offset_minutes` is the offset
    /// from UTC in minutes (within ±23:59), or `None` when there is no zone.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
        offset_minutes: Option<i16>,
    ) -> Option<DateTime> {
        let valid = year <= 9999
            && (1..=12).contains(&month)
            && day >= 1
            && day <= days_in_month(year, month)
            && hour <= 23
            && minute <= 59
            && second <= 60
            && nanosecond < 1_000_000_000
            && offset_minutes.is_none_or(|offset| offset.abs() <= 23 * 60 + 59);
        let fraction_digits = match nanosecond {
            0 => 0,
            n if n % 1_000_000 == 0 => 3,
            n if n % 1_000 == 0 => 6,
            _ => 9,
        };
        valid.then_some(DateTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanosecond,
            fraction_digits,
            offset_minutes,
        })
    }

    fn fields(&self) -> (u16, u8, u8, u8, u8, u8, u32, Option<i16>) {
        (
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
            self.nanosecond,
            self.offset_minutes,
        )
    }

    /// Builds the UTC date-time for a Unix timestamp, or `None` when it falls
    /// outside the years 0 to 9999 or `nanosecond` is a second or more.
    pub fn from_unix_timestamp(seconds: i64, nanosecond: u32) -> Option<DateTime> {
        let days = seconds.div_euclid(86_400);
        let rest = seconds.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        DateTime::new(
            u16::try_from(year).ok()?,
            month,
            day,
            (rest / 3600) as u8,
            (rest % 3600 / 60) as u8,
            (rest % 60) as u8,
            nanosecond,
            Some(0),
        )
    }

    pub fn year(&self) -> u16 {
        self.year
    }

    pub fn month(&self) -> u8 {
        self.month
    }

    pub fn day(&self) -> u8 {
        self.day
    }

    pub fn hour(&self) -> u8 {
        self.hour
    }

    pub fn minute(&self) -> u8 {
        self.minute
    }

    /// The second, 0 to 60 (60 is a leap second).
    pub fn second(&self) -> u8 {
        self.second
    }

    /// The fraction of the second, in nanoseconds.
    pub fn nanosecond(&self) -> u32 {
        self.nanosecond
    }

    /// The offset from UTC in minutes, or `None` when the text had no zone.
    pub fn offset_minutes(&self) -> Option<i16> {
        self.offset_minutes
    }

    /// Seconds since the Unix epoch, ignoring the fraction. A date-time with no
    /// zone is read as UTC, and a leap second counts as the second after `:59`.
    pub fn unix_timestamp(&self) -> i64 {
        let days = days_from_civil(i64::from(self.year), self.month, self.day);
        days * 86_400
            + i64::from(self.hour) * 3600
            + i64::from(self.minute) * 60
            + i64::from(self.second)
            - i64::from(self.offset_minutes.unwrap_or(0)) * 60
    }
}

impl DateTime {
    /// Nanoseconds since the Unix epoch, for comparing two date-times. A date-time
    /// with no zone is read as UTC.
    pub fn unix_timestamp_nanos(&self) -> i128 {
        i128::from(self.unix_timestamp()) * 1_000_000_000 + i128::from(self.nanosecond)
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )?;
        if self.fraction_digits > 0 {
            let digits = format!("{:09}", self.nanosecond);
            write!(f, ".{}", &digits[..usize::from(self.fraction_digits)])?;
        }
        match self.offset_minutes {
            None => Ok(()),
            Some(0) => write!(f, "Z"),
            Some(offset) => write!(
                f,
                "{}{:02}:{:02}",
                if offset < 0 { '-' } else { '+' },
                offset.abs() / 60,
                offset.abs() % 60
            ),
        }
    }
}

impl FromStr for DateTime {
    type Err = ParseDateTimeError;

    fn from_str(value: &str) -> Result<DateTime, ParseDateTimeError> {
        parse(value).ok_or(ParseDateTimeError)
    }
}

/// Parses exactly `digits` ASCII digits.
fn number(value: &str, digits: usize) -> Option<u32> {
    if value.len() == digits && value.bytes().all(|byte| byte.is_ascii_digit()) {
        value.parse().ok()
    } else {
        None
    }
}

fn parse(value: &str) -> Option<DateTime> {
    if !value.is_ascii() {
        return None;
    }
    let (date, time) = value.split_once('T')?;
    let mut date = date.split('-');
    let year = number(date.next()?, 4)?;
    let month = number(date.next()?, 2)?;
    let day = number(date.next()?, 2)?;
    if date.next().is_some() {
        return None;
    }

    let zone_start = time.find(['Z', '+', '-']).unwrap_or(time.len());
    let (clock, zone) = time.split_at(zone_start);
    let (clock, fraction) = match clock.split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (clock, None),
    };

    let mut parts = clock.split(':');
    let hour = number(parts.next()?, 2)?;
    let minute = number(parts.next()?, 2)?;
    let second = match parts.next() {
        Some(second) => number(second, 2)?,
        None if fraction.is_none() => 0,
        None => return None,
    };
    if parts.next().is_some() {
        return None;
    }

    let nanosecond = match fraction {
        None => 0,
        Some(fraction) => {
            if fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            // Digits past nanoseconds are dropped.
            let digits = &fraction[..fraction.len().min(9)];
            digits.parse::<u32>().ok()? * 10u32.pow(9 - digits.len() as u32)
        }
    };

    let offset_minutes = match zone.as_bytes() {
        [] => None,
        [b'Z'] => Some(0),
        [sign @ (b'+' | b'-'), ..] => {
            let offset = &zone[1..];
            let (hours, minutes) = match offset.len() {
                2 => (number(offset, 2)?, 0),
                4 => (number(&offset[..2], 2)?, number(&offset[2..], 2)?),
                5 if offset.as_bytes()[2] == b':' => {
                    (number(&offset[..2], 2)?, number(&offset[3..], 2)?)
                }
                _ => return None,
            };
            let magnitude = i16::try_from(hours * 60 + minutes).ok()?;
            Some(if *sign == b'-' { -magnitude } else { magnitude })
        }
        _ => return None,
    };

    let mut date_time = DateTime::new(
        u16::try_from(year).ok()?,
        month as u8,
        day as u8,
        hour as u8,
        minute as u8,
        second as u8,
        nanosecond,
        offset_minutes,
    )?;
    date_time.fraction_digits = fraction.map_or(0, |fraction| fraction.len().min(9) as u8);
    Some(date_time)
}

fn days_in_month(year: u16, month: u8) -> u8 {
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 for a proleptic Gregorian date.
fn days_from_civil(year: i64, month: u8, day: u8) -> i64 {
    let month = i64::from(month);
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let day_of_year =
        (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The proleptic Gregorian `(year, month, day)` for days since 1970-01-01.
fn civil_from_days(days: i64) -> (i64, u8, u8) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u8;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u8;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}
