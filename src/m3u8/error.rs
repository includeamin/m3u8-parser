//! Error types returned when reading M3U8 playlists.

use std::fmt;
use std::io;

/// An error encountered while reading a playlist.
#[derive(Debug)]
#[non_exhaustive]
pub enum ParseError {
    /// The playlist could not be read from its source.
    Io(io::Error),
    /// A line of the playlist is malformed.
    Syntax {
        /// The 1-based line number where the error was found.
        line: usize,
        kind: SyntaxError,
    },
}

impl ParseError {
    /// Returns the 1-based line number for syntax errors.
    pub fn line(&self) -> Option<usize> {
        match self {
            ParseError::Syntax { line, .. } => Some(*line),
            ParseError::Io(_) => None,
        }
    }

    /// Returns the kind of syntax error, if this is a syntax error.
    pub fn syntax(&self) -> Option<&SyntaxError> {
        match self {
            ParseError::Syntax { kind, .. } => Some(kind),
            ParseError::Io(_) => None,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Io(error) => write!(f, "failed to read playlist: {error}"),
            ParseError::Syntax { line, kind } => write!(f, "line {line}: {kind}"),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Io(error) => Some(error),
            ParseError::Syntax { kind, .. } => Some(kind),
        }
    }
}

impl From<io::Error> for ParseError {
    fn from(error: io::Error) -> Self {
        ParseError::Io(error)
    }
}

/// The reason a playlist line could not be parsed.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SyntaxError {
    /// An `#EXTINF` tag was not followed by a media segment URI.
    MissingUriAfterExtInf,
    /// An `#EXTINF` tag is malformed.
    InvalidExtInf(String),
    /// An attribute list could not be split into `NAME=VALUE` pairs.
    MalformedAttributeList { reason: &'static str, input: String },
    /// A required attribute is absent.
    MissingAttribute {
        tag: &'static str,
        attribute: &'static str,
    },
    /// An attribute value does not have the type the tag requires.
    InvalidAttributeValue {
        tag: &'static str,
        attribute: &'static str,
        value: String,
    },
    /// A tag's value does not have the type the tag requires.
    InvalidTagValue { tag: &'static str, value: String },
    /// A tag's attributes are present in a combination the specification forbids.
    ConflictingAttributes {
        tag: &'static str,
        reason: &'static str,
    },
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyntaxError::MissingUriAfterExtInf => write!(f, "missing URI after EXTINF"),
            SyntaxError::InvalidExtInf(value) => write!(f, "invalid EXTINF: {value}"),
            SyntaxError::MalformedAttributeList { reason, input } => {
                write!(f, "malformed attribute list ({reason}): {input}")
            }
            SyntaxError::MissingAttribute { tag, attribute } => {
                write!(f, "{tag} requires {attribute}")
            }
            SyntaxError::InvalidAttributeValue {
                tag,
                attribute,
                value,
            } => write!(f, "invalid {tag} {attribute}: {value}"),
            SyntaxError::InvalidTagValue { tag, value } => write!(f, "invalid {tag}: {value}"),
            SyntaxError::ConflictingAttributes { tag, reason } => write!(f, "{tag}: {reason}"),
        }
    }
}

impl std::error::Error for SyntaxError {}
