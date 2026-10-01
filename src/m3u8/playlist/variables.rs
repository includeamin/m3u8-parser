//! Variable substitution for EXT-X-DEFINE (draft-pantos-hls-rfc8216bis section 4.3).

use super::Playlist;
use crate::m3u8::error::SubstitutionError;
use crate::m3u8::parser::{attribute, parse_attribute_list};
use crate::m3u8::tags::Tag;
use std::collections::HashMap;

impl Playlist {
    /// Returns the variables declared by this playlist's EXT-X-DEFINE tags.
    ///
    /// `imports` holds the variables of the Multivariant Playlist, used by
    /// `IMPORT`. `query` is the query string of this playlist's URI without the
    /// leading `?`, used by `QUERYPARAM`; its values are percent-decoded.
    pub fn variables(
        &self,
        imports: &HashMap<String, String>,
        query: Option<&str>,
    ) -> Result<HashMap<String, String>, SubstitutionError> {
        let mut variables = HashMap::new();

        for tag in &self.tags {
            let Tag::ExtXDefine(definition) = tag else {
                continue;
            };
            let invalid = || SubstitutionError::InvalidDefine(definition.clone());
            let attributes = parse_attribute_list(definition).map_err(|_| invalid())?;
            let get = |name| attribute(&attributes, name).map(str::to_owned);

            let (name, value) = match (get("NAME"), get("VALUE"), get("IMPORT"), get("QUERYPARAM"))
            {
                (Some(name), Some(value), None, None) => (name, value),
                (None, None, Some(name), None) => {
                    let value = imports
                        .get(&name)
                        .cloned()
                        .ok_or_else(|| SubstitutionError::MissingImport(name.clone()))?;
                    (name, value)
                }
                (None, None, None, Some(name)) => {
                    let value = query
                        .and_then(|query| query_parameter(query, &name))
                        .ok_or_else(|| SubstitutionError::MissingQueryParameter(name.clone()))?;
                    (name, value)
                }
                _ => return Err(invalid()),
            };

            let valid_name = !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
            if !valid_name {
                return Err(invalid());
            }
            if variables.contains_key(&name) {
                return Err(SubstitutionError::DuplicateVariable(name));
            }
            variables.insert(name, value);
        }

        Ok(variables)
    }

    /// Returns a copy of the playlist with every `{$name}` reference replaced by
    /// its value. EXT-X-DEFINE tags are kept unchanged.
    ///
    /// See [`Playlist::variables`] for the meaning of `imports` and `query`.
    pub fn substitute_variables(
        &self,
        imports: &HashMap<String, String>,
        query: Option<&str>,
    ) -> Result<Playlist, SubstitutionError> {
        let variables = self.variables(imports, query)?;
        let mut text = String::new();

        for tag in &self.tags {
            let line = tag.to_string();
            if matches!(tag, Tag::ExtXDefine(_)) {
                text.push_str(&line);
            } else {
                text.push_str(&substitute(&line, &variables)?);
            }
            text.push('\n');
        }

        text.parse().map_err(SubstitutionError::Parse)
    }
}

fn substitute(
    line: &str,
    variables: &HashMap<String, String>,
) -> Result<String, SubstitutionError> {
    let mut output = String::with_capacity(line.len());
    let mut rest = line;

    while let Some(start) = rest.find("{$") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            output.push_str(&rest[start..]);
            return Ok(output);
        };
        let name = &after[..end];
        let value = variables
            .get(name)
            .ok_or_else(|| SubstitutionError::UndefinedVariable(name.to_string()))?;
        output.push_str(value);
        rest = &after[end + 1..];
    }

    output.push_str(rest);
    Ok(output)
}

fn query_parameter(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        (percent_decode(key) == name).then(|| percent_decode(value))
    })
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        let escape = (bytes[index] == b'%')
            .then(|| value.get(index + 1..index + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escape {
            Some(byte) => {
                decoded.push(byte);
                index += 3;
            }
            None => {
                decoded.push(bytes[index]);
                index += 1;
            }
        }
    }

    String::from_utf8_lossy(&decoded).into_owned()
}
