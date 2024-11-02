/// Parses attributes from a given input string and returns a map of key-value pairs.
///
/// This function processes an input string formatted as key-value pairs separated by commas.
/// Each key-value pair is expected to be in the format `key="value"`. The function will trim
/// the quotes around the values and construct a `HashMap` containing the parsed attributes.
///
/// # Arguments
///
/// * `input` - A string containing the attributes to be parsed. The expected format is
///   `key1="value1",key2="value2",...`.
///
/// # Returns
///
/// A result containing a `HashMap<String, String>` of attributes if parsing is successful,
/// or an error message as a string if an error occurs during parsing.
///
/// # Example
///
/// ```
/// use m3u8_parser::m3u8::parser::parse_attributes;
/// let input = r#"METHOD="AES-128",URI="https://example.com/key",IV="1234567890abcdef""#;
/// let attributes = parse_attributes(input).expect("Failed to parse attributes");
/// assert_eq!(attributes.get("METHOD"), Some(&"AES-128".to_string()));
/// assert_eq!(attributes.get("URI"), Some(&"https://example.com/key".to_string()));
/// ```
///
pub fn parse_attributes(input: &str) -> Result<std::collections::HashMap<String, String>, String> {
    let mut attributes = std::collections::HashMap::new();
    for part in input.split(',') {
        let parts: Vec<&str> = part.splitn(2, '=').collect();
        if parts.len() == 2 {
            attributes.insert(parts[0].to_string(), parts[1].trim_matches('"').to_string());
        }
    }
    Ok(attributes)
}
