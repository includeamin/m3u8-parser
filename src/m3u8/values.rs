//! Typed values for HLS attributes that hold a closed set of names or a small
//! structured value.
//!
//! These types are re-exported from [`tags`](crate::m3u8::tags).

use std::fmt;
use std::str::FromStr;

/// Defines an enumerated-string type: known names plus `Other` for anything
/// else, so unknown values survive a round trip.
macro_rules! named_values {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, PartialEq, Eq, Clone, Hash)]
        pub enum $name {
            $($(#[$vmeta])* $variant,)+
            /// A value this crate does not know, kept as written.
            Other(String),
        }

        impl $name {
            /// The value as it appears in a playlist.
            pub fn as_str(&self) -> &str {
                match self {
                    $($name::$variant => $text,)+
                    $name::Other(other) => other,
                }
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                match value {
                    $($text => $name::$variant,)+
                    other => $name::Other(other.to_string()),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

named_values! {
    /// The `TYPE` of an `EXT-X-MEDIA` rendition.
    MediaType {
        Audio => "AUDIO",
        Video => "VIDEO",
        Subtitles => "SUBTITLES",
        ClosedCaptions => "CLOSED-CAPTIONS",
    }
}

named_values! {
    /// The `TYPE` of an `EXT-X-PRELOAD-HINT`.
    PreloadHintType {
        Part => "PART",
        Map => "MAP",
    }
}

named_values! {
    /// The `VIDEO-RANGE` of a stream.
    VideoRange {
        Sdr => "SDR",
        Hlg => "HLG",
        Pq => "PQ",
    }
}

named_values! {
    /// The `HDCP-LEVEL` a stream requires.
    HdcpLevel {
        None => "NONE",
        Type0 => "TYPE-0",
        Type1 => "TYPE-1",
    }
}

/// The `CLOSED-CAPTIONS` attribute of a variant: a rendition group, or `NONE`
/// when the variant has no closed captions.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ClosedCaptions {
    /// `CLOSED-CAPTIONS=NONE`.
    None,
    /// The `GROUP-ID` of the `TYPE=CLOSED-CAPTIONS` renditions.
    Group(String),
}

impl From<&str> for ClosedCaptions {
    fn from(value: &str) -> Self {
        if value == "NONE" {
            ClosedCaptions::None
        } else {
            ClosedCaptions::Group(value.to_string())
        }
    }
}

impl fmt::Display for ClosedCaptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClosedCaptions::None => f.write_str("NONE"),
            ClosedCaptions::Group(group) => f.write_str(group),
        }
    }
}

/// The `INSTREAM-ID` of a closed-captions rendition: `CC1` to `CC4`, or
/// `SERVICE1` to `SERVICE63`.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum InstreamId {
    /// `CC1` to `CC4`, holding the number.
    Cc(u8),
    /// `SERVICE1` to `SERVICE63`, holding the number.
    Service(u8),
    /// Any other value, kept as written. Validation rejects it.
    Other(String),
}

impl From<&str> for InstreamId {
    fn from(value: &str) -> Self {
        let numbered = |prefix: &str, max: u8| {
            let number: u8 = value.strip_prefix(prefix)?.parse().ok()?;
            // Rejects leading zeros and signs, so the text round-trips.
            ((1..=max).contains(&number) && value == format!("{prefix}{number}")).then_some(number)
        };
        if let Some(number) = numbered("CC", 4) {
            InstreamId::Cc(number)
        } else if let Some(number) = numbered("SERVICE", 63) {
            InstreamId::Service(number)
        } else {
            InstreamId::Other(value.to_string())
        }
    }
}

impl fmt::Display for InstreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstreamId::Cc(number) => write!(f, "CC{number}"),
            InstreamId::Service(number) => write!(f, "SERVICE{number}"),
            InstreamId::Other(other) => f.write_str(other),
        }
    }
}

impl InstreamId {
    /// Whether this is `CC1` to `CC4` or `SERVICE1` to `SERVICE63`.
    pub fn is_valid(&self) -> bool {
        !matches!(self, InstreamId::Other(_))
    }
}

/// The `CHANNELS` of an audio rendition: a channel count, optionally followed by
/// `/`-separated parameters such as `JOC`, as in `6/JOC`.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Channels {
    /// The number of audio channels.
    pub count: u32,
    /// The parameters after the count, kept as written.
    pub parameters: Vec<String>,
}

impl FromStr for Channels {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, ()> {
        let mut parts = value.split('/');
        let count = parts.next().ok_or(())?;
        if count.is_empty() || !count.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(());
        }
        Ok(Channels {
            count: count.parse().map_err(|_| ())?,
            parameters: parts.map(str::to_string).collect(),
        })
    }
}

impl fmt::Display for Channels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.count)?;
        for parameter in &self.parameters {
            write!(f, "/{parameter}")?;
        }
        Ok(())
    }
}

/// The `CHARACTERISTICS` of a rendition: a comma-separated list of Uniform Type
/// Identifiers such as `public.accessibility.describes-video`.
///
/// Parsing trims whitespace around each entry; writing joins them with a bare
/// comma.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Default)]
pub struct Characteristics(Vec<String>);

impl Characteristics {
    /// The identifiers in playlist order.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }

    /// The number of identifiers.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no identifiers.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether `characteristic` is listed.
    pub fn contains(&self, characteristic: &str) -> bool {
        self.iter().any(|own| own == characteristic)
    }
}

impl<S: Into<String>> FromIterator<S> for Characteristics {
    fn from_iter<I: IntoIterator<Item = S>>(items: I) -> Self {
        Characteristics(items.into_iter().map(Into::into).collect())
    }
}

impl From<&str> for Characteristics {
    fn from(value: &str) -> Self {
        value.split(',').map(str::trim).collect()
    }
}

impl fmt::Display for Characteristics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.join(","))
    }
}

/// A hexadecimal sequence such as `0xFC002F`, used by the `SCTE35-*` attributes.
///
/// Parsing accepts a `0x` or `0X` prefix and an even, non-zero number of hex
/// digits in either case; writing uses `0x` and upper-case digits.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct HexSequence(Vec<u8>);

impl HexSequence {
    /// The decoded bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for HexSequence {
    fn from(bytes: Vec<u8>) -> Self {
        HexSequence(bytes)
    }
}

impl FromStr for HexSequence {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, ()> {
        let digits = value
            .strip_prefix("0x")
            .or_else(|| value.strip_prefix("0X"))
            .ok_or(())?;
        if digits.is_empty()
            || digits.len() % 2 != 0
            || !digits.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(());
        }
        let bytes = digits
            .as_bytes()
            .chunks(2)
            .map(|pair| {
                u8::from_str_radix(std::str::from_utf8(pair).map_err(|_| ())?, 16).map_err(|_| ())
            })
            .collect::<Result<_, _>>()?;
        Ok(HexSequence(bytes))
    }
}

impl fmt::Display for HexSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("0x")?;
        for byte in &self.0 {
            write!(f, "{byte:02X}")?;
        }
        Ok(())
    }
}

/// An RFC 5646 language tag, such as `en` or `zh-Hant-TW`, from the `LANGUAGE`
/// attribute.
///
/// Any text is kept as written so playlists round-trip; [`is_valid`](Self::is_valid)
/// checks the tag's shape, and validation reports invalid tags.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct LanguageTag(String);

impl LanguageTag {
    /// The tag as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The `-`-separated subtags.
    pub fn subtags(&self) -> impl Iterator<Item = &str> {
        self.0.split('-')
    }

    /// The first subtag, which is the primary language such as `en`.
    pub fn primary(&self) -> &str {
        self.subtags().next().unwrap_or_default()
    }

    /// Whether every subtag is 1 to 8 ASCII letters or digits and the primary
    /// subtag is made of letters. This checks the tag's shape, not whether the
    /// subtags are registered.
    pub fn is_valid(&self) -> bool {
        self.primary()
            .bytes()
            .all(|byte| byte.is_ascii_alphabetic())
            && self.subtags().all(|subtag| {
                (1..=8).contains(&subtag.len())
                    && subtag.bytes().all(|byte| byte.is_ascii_alphanumeric())
            })
    }
}

impl From<&str> for LanguageTag {
    fn from(value: &str) -> Self {
        LanguageTag(value.to_string())
    }
}

impl fmt::Display for LanguageTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An `EXT-X-DEFINE` declaration of a playlist variable.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Define {
    /// `NAME` and `VALUE`: defines a variable in place.
    Value {
        /// The variable name.
        name: String,
        /// The variable's value.
        value: String,
    },
    /// `IMPORT`: takes the variable of that name from the Multivariant Playlist.
    Import(String),
    /// `QUERYPARAM`: takes the value of that query parameter of the playlist's URI.
    QueryParam(String),
}

impl Define {
    /// The name of the variable this tag defines.
    pub fn name(&self) -> &str {
        match self {
            Define::Value { name, .. } | Define::Import(name) | Define::QueryParam(name) => name,
        }
    }
}

impl fmt::Display for Define {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Define::Value { name, value } => write!(f, "NAME=\"{name}\",VALUE=\"{value}\""),
            Define::Import(name) => write!(f, "IMPORT=\"{name}\""),
            Define::QueryParam(name) => write!(f, "QUERYPARAM=\"{name}\""),
        }
    }
}
