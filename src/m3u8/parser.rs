//! Attribute-list parsing shared by the playlist reader.

use crate::m3u8::error::SyntaxError;
use crate::m3u8::tags::AttributeValue;
use std::collections::HashMap;

/// Parses an attribute list into a map of attribute names to unquoted values.
///
/// Quoted values may contain commas. If an attribute name repeats, the last
/// value wins.
///
/// # Example
///
/// ```
/// use m3u8_parser::m3u8::parser::parse_attributes;
/// let input = r#"METHOD=AES-128,URI="https://example.com/key",KEYFORMATVERSIONS="1/2""#;
/// let attributes = parse_attributes(input).expect("Failed to parse attributes");
/// assert_eq!(attributes.get("METHOD"), Some(&"AES-128".to_string()));
/// assert_eq!(attributes.get("URI"), Some(&"https://example.com/key".to_string()));
/// ```
pub fn parse_attributes(input: &str) -> Result<HashMap<String, String>, SyntaxError> {
    Ok(parse_attribute_list(input)?
        .into_iter()
        .map(|attribute| (attribute.name, attribute.value))
        .collect())
}

pub(crate) struct Attribute {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) quoted: bool,
}

impl Attribute {
    pub(crate) fn into_value(self) -> AttributeValue {
        if self.quoted {
            AttributeValue::Quoted(self.value)
        } else {
            AttributeValue::Unquoted(self.value)
        }
    }
}

pub(crate) fn parse_attribute_list(input: &str) -> Result<Vec<Attribute>, SyntaxError> {
    let malformed = |reason| SyntaxError::MalformedAttributeList {
        reason,
        input: input.to_string(),
    };
    let mut attributes = Vec::new();
    let mut remaining = input.trim();

    while !remaining.is_empty() {
        let Some(equals_index) = remaining.find('=') else {
            return Err(malformed("missing '='"));
        };
        let name = remaining[..equals_index].trim();
        if name.is_empty() || name.contains(',') {
            return Err(malformed("invalid attribute name"));
        }

        let value_start = remaining[equals_index + 1..].trim_start();
        let quoted = value_start.starts_with('"');
        let (value, after_value) = if let Some(quoted) = value_start.strip_prefix('"') {
            let Some(end) = quoted.find('"') else {
                return Err(malformed("unterminated quoted string"));
            };
            (&quoted[..end], &quoted[end + 1..])
        } else {
            let end = value_start.find(',').unwrap_or(value_start.len());
            (value_start[..end].trim_end(), &value_start[end..])
        };
        attributes.push(Attribute {
            name: name.to_string(),
            value: value.to_string(),
            quoted,
        });

        let after_value = after_value.trim_start();
        if after_value.is_empty() {
            break;
        }
        let Some(next) = after_value.strip_prefix(',') else {
            return Err(malformed("missing ',' between attributes"));
        };
        remaining = next.trim_start();
    }

    Ok(attributes)
}

pub(crate) fn attribute<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|attribute| attribute.name == name)
        .map(|attribute| attribute.value.as_str())
}

pub(crate) fn optional_string(attributes: &[Attribute], name: &str) -> Option<String> {
    attribute(attributes, name).map(str::to_owned)
}

pub(crate) fn required_string(
    attributes: &[Attribute],
    name: &'static str,
    tag: &'static str,
) -> Result<String, SyntaxError> {
    optional_string(attributes, name).ok_or(SyntaxError::MissingAttribute {
        tag,
        attribute: name,
    })
}

pub(crate) fn optional_boolean(
    attributes: &[Attribute],
    name: &'static str,
    tag: &'static str,
) -> Result<Option<bool>, SyntaxError> {
    attribute(attributes, name)
        .map(|value| match value {
            "YES" => Ok(true),
            "NO" => Ok(false),
            _ => Err(SyntaxError::InvalidAttributeValue {
                tag,
                attribute: name,
                value: value.to_string(),
            }),
        })
        .transpose()
}

pub(crate) fn optional_number<T: std::str::FromStr>(
    attributes: &[Attribute],
    name: &'static str,
    tag: &'static str,
) -> Result<Option<T>, SyntaxError> {
    attribute(attributes, name)
        .map(|value| {
            value
                .parse()
                .map_err(|_| SyntaxError::InvalidAttributeValue {
                    tag,
                    attribute: name,
                    value: value.to_string(),
                })
        })
        .transpose()
}

pub(crate) fn required_number<T: std::str::FromStr>(
    attributes: &[Attribute],
    name: &'static str,
    tag: &'static str,
) -> Result<T, SyntaxError> {
    optional_number(attributes, name, tag)?.ok_or(SyntaxError::MissingAttribute {
        tag,
        attribute: name,
    })
}
