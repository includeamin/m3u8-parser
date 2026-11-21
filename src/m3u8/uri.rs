//! URI reference resolution as defined by RFC 3986 section 5.

use crate::m3u8::error::UriError;

/// Resolves `reference` against `base`, following RFC 3986 section 5.2.
///
/// `base` must be an absolute URI (it needs a scheme) without whitespace,
/// control characters, or double quotes. Resolution is purely textual: nothing
/// is normalised beyond removing `.` and `..` path segments, and `{$name}`
/// variable references are not understood, so call
/// [`Playlist::substitute_variables`](crate::m3u8::playlist::Playlist::substitute_variables)
/// first.
///
/// ```
/// use m3u8_parser::m3u8::uri::resolve_uri;
///
/// let resolved = resolve_uri("https://cdn.example.com/video/master.m3u8", "../audio/en.m3u8");
/// assert_eq!(resolved.unwrap(), "https://cdn.example.com/audio/en.m3u8");
/// ```
pub fn resolve_uri(base: &str, reference: &str) -> Result<String, UriError> {
    let base_parts = split_base(base)?;
    let reference = Parts::parse(reference);

    let (scheme, authority, path, query);
    if let Some(reference_scheme) = reference.scheme {
        scheme = reference_scheme;
        authority = reference.authority;
        path = remove_dot_segments(reference.path);
        query = reference.query;
    } else {
        scheme = base_parts.scheme.unwrap_or_default();
        if reference.authority.is_some() {
            authority = reference.authority;
            path = remove_dot_segments(reference.path);
            query = reference.query;
        } else {
            authority = base_parts.authority;
            if reference.path.is_empty() {
                path = base_parts.path.to_string();
                query = reference.query.or(base_parts.query);
            } else {
                path = if reference.path.starts_with('/') {
                    remove_dot_segments(reference.path)
                } else {
                    remove_dot_segments(&merge(&base_parts, reference.path))
                };
                query = reference.query;
            }
        }
    }

    let mut resolved = format!("{scheme}:");
    if let Some(authority) = authority {
        resolved.push_str("//");
        resolved.push_str(authority);
    }
    resolved.push_str(&path);
    if let Some(query) = query {
        resolved.push('?');
        resolved.push_str(query);
    }
    if let Some(fragment) = reference.fragment {
        resolved.push('#');
        resolved.push_str(fragment);
    }
    Ok(resolved)
}

/// Checks that `base` can be used as a base URI.
pub(crate) fn validate_base(base: &str) -> Result<(), UriError> {
    split_base(base).map(|_| ())
}

fn split_base(base: &str) -> Result<Parts<'_>, UriError> {
    let parts = Parts::parse(base);
    let unusable = parts.scheme.is_none()
        || base
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '"');
    if unusable {
        Err(UriError::InvalidBase(base.to_string()))
    } else {
        Ok(parts)
    }
}

/// The five components of a URI reference (RFC 3986 appendix B).
struct Parts<'a> {
    scheme: Option<&'a str>,
    authority: Option<&'a str>,
    path: &'a str,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
}

impl<'a> Parts<'a> {
    fn parse(uri: &'a str) -> Self {
        let (rest, fragment) = match uri.split_once('#') {
            Some((rest, fragment)) => (rest, Some(fragment)),
            None => (uri, None),
        };
        let (rest, query) = match rest.split_once('?') {
            Some((rest, query)) => (rest, Some(query)),
            None => (rest, None),
        };
        let (scheme, rest) = match rest.split_once(':') {
            Some((scheme, after)) if is_scheme(scheme) => (Some(scheme), after),
            _ => (None, rest),
        };
        let (authority, path) = match rest.strip_prefix("//") {
            Some(after) => match after.find('/') {
                Some(end) => (Some(&after[..end]), &after[end..]),
                None => (Some(after), ""),
            },
            None => (None, rest),
        };
        Parts {
            scheme,
            authority,
            path,
            query,
            fragment,
        }
    }
}

fn is_scheme(candidate: &str) -> bool {
    let mut chars = candidate.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// RFC 3986 section 5.2.3.
fn merge(base: &Parts<'_>, reference_path: &str) -> String {
    if base.authority.is_some() && base.path.is_empty() {
        format!("/{reference_path}")
    } else {
        match base.path.rfind('/') {
            Some(index) => format!("{}{reference_path}", &base.path[..=index]),
            None => reference_path.to_string(),
        }
    }
}

/// RFC 3986 section 5.2.4.
fn remove_dot_segments(path: &str) -> String {
    let mut input = path;
    let mut output: Vec<&str> = Vec::new();

    while !input.is_empty() {
        if let Some(rest) = input.strip_prefix("../") {
            input = rest;
        } else if let Some(rest) = input.strip_prefix("./") {
            input = rest;
        } else if input.starts_with("/./") {
            input = &input[2..];
        } else if input == "/." {
            input = "/";
        } else if input.starts_with("/../") {
            input = &input[3..];
            output.pop();
        } else if input == "/.." {
            input = "/";
            output.pop();
        } else if input == "." || input == ".." {
            input = "";
        } else {
            let start = usize::from(input.starts_with('/'));
            let end = input[start..].find('/').map_or(input.len(), |i| i + start);
            output.push(&input[..end]);
            input = &input[end..];
        }
    }

    output.concat()
}
