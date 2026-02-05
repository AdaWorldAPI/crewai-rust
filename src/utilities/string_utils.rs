//! String utility functions.
//!
//! Corresponds to `crewai/utilities/string_utils.py`.

use std::collections::HashMap;

use regex::Regex;
use once_cell::sync::Lazy;

static VARIABLE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{([A-Za-z_][A-Za-z0-9_\-]*)\}").unwrap());
static QUOTE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r#"['"]+"#).unwrap());
static CAMEL_LOWER_UPPER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([a-z])([A-Z])").unwrap());
static CAMEL_UPPER_LOWER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([A-Z]+)([A-Z][a-z])").unwrap());
static DISALLOWED_CHARS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[^a-zA-Z0-9]+").unwrap());
static DUPLICATE_UNDERSCORE: Lazy<Regex> = Lazy::new(|| Regex::new(r"_+").unwrap());

const MAX_TOOL_NAME_LENGTH: usize = 64;

/// Sanitize a tool name for LLM provider compatibility.
///
/// Normalizes Unicode, splits camelCase, lowercases, replaces invalid characters
/// with underscores, and truncates to `max_length`.
/// Conforms to OpenAI/Bedrock requirements (lowercase, a-z0-9_ only, max 64 chars).
///
/// # Arguments
/// * `name` - Original tool name.
/// * `max_length` - Maximum allowed length (default 64).
pub fn sanitize_tool_name(name: &str, max_length: Option<usize>) -> String {
    let max_len = max_length.unwrap_or(MAX_TOOL_NAME_LENGTH);

    // Basic ASCII normalization (drop non-ASCII)
    let ascii_name: String = name.chars().filter(|c| c.is_ascii()).collect();

    // Split camelCase
    let step1 = CAMEL_UPPER_LOWER.replace_all(&ascii_name, "${1}_${2}");
    let step2 = CAMEL_LOWER_UPPER.replace_all(&step1, "${1}_${2}");

    // Lowercase
    let lowered = step2.to_lowercase();

    // Remove quotes
    let no_quotes = QUOTE_PATTERN.replace_all(&lowered, "");

    // Replace disallowed characters with underscore
    let replaced = DISALLOWED_CHARS.replace_all(&no_quotes, "_");

    // Collapse duplicate underscores
    let collapsed = DUPLICATE_UNDERSCORE.replace_all(&replaced, "_");

    // Strip leading/trailing underscores
    let stripped = collapsed.trim_matches('_').to_string();

    // Truncate
    if stripped.len() > max_len {
        stripped[..max_len].trim_end_matches('_').to_string()
    } else {
        stripped
    }
}

/// Interpolate placeholders (e.g., `{key}`) in a string while leaving JSON untouched.
///
/// Only interpolates placeholders that follow the pattern `{variable_name}` where
/// `variable_name` starts with a letter/underscore and contains only alphanumeric chars,
/// underscores, and hyphens.
///
/// # Arguments
/// * `input_string` - The string containing template variables.
/// * `inputs` - Dictionary mapping template variables to their values.
///
/// # Errors
/// Returns an error if a template variable is not found in `inputs`.
pub fn interpolate_only(
    input_string: Option<&str>,
    inputs: &HashMap<String, String>,
) -> Result<String, String> {
    let input = match input_string {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(String::new()),
    };

    if !input.contains('{') && !input.contains('}') {
        return Ok(input.to_string());
    }

    if inputs.is_empty() {
        return Err("Inputs dictionary cannot be empty when interpolating variables".to_string());
    }

    let variables: Vec<String> = VARIABLE_PATTERN
        .captures_iter(input)
        .map(|cap| cap[1].to_string())
        .collect();

    // Check for missing variables
    let missing: Vec<&String> = variables.iter().filter(|v| !inputs.contains_key(*v)).collect();
    if !missing.is_empty() {
        return Err(format!(
            "Template variable '{}' not found in inputs dictionary",
            missing[0]
        ));
    }

    let mut result = input.to_string();
    for var in &variables {
        if let Some(value) = inputs.get(var) {
            let placeholder = format!("{{{}}}", var);
            result = result.replace(&placeholder, value);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_tool_name_camel_case() {
        assert_eq!(sanitize_tool_name("MyToolName", None), "my_tool_name");
    }

    #[test]
    fn test_sanitize_tool_name_special_chars() {
        assert_eq!(sanitize_tool_name("hello world!", None), "hello_world");
    }

    #[test]
    fn test_interpolate_only_basic() {
        let mut inputs = HashMap::new();
        inputs.insert("name".to_string(), "Alice".to_string());
        let result = interpolate_only(Some("Hello {name}!"), &inputs).unwrap();
        assert_eq!(result, "Hello Alice!");
    }

    #[test]
    fn test_interpolate_only_missing_var() {
        let inputs = HashMap::new();
        let result = interpolate_only(Some("Hello {name}!"), &inputs);
        assert!(result.is_err());
    }
}
