//! Provider utility functions shared across LLM providers.
//!
//! Corresponds to `crewai/llms/providers/utils/common.py`.
//!
//! This module provides common helper functions for tool/function name
//! validation, tool info extraction, and function name sanitization that
//! are used by all native SDK provider implementations.

use regex::Regex;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Function name validation
// ---------------------------------------------------------------------------

/// Validate a function name according to common LLM provider requirements.
///
/// Rules:
/// - Must not be empty
/// - Must start with a letter or underscore
/// - Must be <= 64 characters
/// - Must contain only lowercase letters, numbers, and underscores
///
/// Corresponds to `validate_function_name` in Python.
///
/// # Arguments
///
/// * `name` - The function name to validate.
/// * `provider` - The provider name for error messages.
///
/// # Returns
///
/// The validated function name (unchanged if valid).
///
/// # Errors
///
/// Returns an error if the function name is invalid.
pub fn validate_function_name(name: &str, provider: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err(format!("{} function name cannot be empty", provider));
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err(format!(
            "{} function name '{}' must start with a letter or underscore",
            provider, name
        ));
    }

    if name.len() > 64 {
        return Err(format!(
            "{} function name '{}' exceeds 64 character limit",
            provider, name
        ));
    }

    let valid_pattern = Regex::new(r"^[a-z_][a-z0-9_]*$").unwrap();
    if !valid_pattern.is_match(name) {
        return Err(format!(
            "{} function name '{}' contains invalid characters. \
             Only lowercase letters, numbers, and underscores allowed",
            provider, name
        ));
    }

    Ok(name.to_string())
}

// ---------------------------------------------------------------------------
// Tool info extraction
// ---------------------------------------------------------------------------

/// Extract tool information from various schema formats.
///
/// Handles both OpenAI/standard format and direct format:
/// - OpenAI format: `{"type": "function", "function": {"name": "...", ...}}`
/// - Direct format: `{"name": "...", "description": "...", ...}`
///
/// Corresponds to `extract_tool_info` in Python.
///
/// # Arguments
///
/// * `tool` - Tool value in any supported format.
///
/// # Returns
///
/// Tuple of (name, description, parameters).
pub fn extract_tool_info(tool: &Value) -> Result<(String, String, Value), String> {
    let obj = tool
        .as_object()
        .ok_or_else(|| "Tool must be a JSON object".to_string())?;

    // Handle nested function schema format (OpenAI/standard)
    if let Some(function_info) = obj.get("function") {
        let function_obj = function_info
            .as_object()
            .ok_or_else(|| "Tool function must be a JSON object".to_string())?;

        let name = function_obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let description = function_obj
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let parameters = function_obj
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

        return Ok((name, description, parameters));
    }

    // Direct format
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let description = obj
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let parameters = obj
        .get("parameters")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

    Ok((name, description, parameters))
}

// ---------------------------------------------------------------------------
// Function name sanitization
// ---------------------------------------------------------------------------

/// Sanitize a function name for LLM provider compatibility.
///
/// Converts to lowercase, replaces invalid characters with underscores,
/// ensures it starts with a letter/underscore, and truncates to 64 chars.
///
/// Corresponds to `sanitize_function_name` in Python.
///
/// # Arguments
///
/// * `name` - Original function name.
///
/// # Returns
///
/// Sanitized function name (lowercase, a-z0-9_ only, max 64 chars).
pub fn sanitize_function_name(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }

    // Convert to lowercase
    let mut sanitized: String = name.to_lowercase();

    // Replace invalid characters with underscores
    let replace_re = Regex::new(r"[^a-z0-9_]").unwrap();
    sanitized = replace_re.replace_all(&sanitized, "_").to_string();

    // Ensure starts with letter or underscore
    if let Some(first) = sanitized.chars().next() {
        if !first.is_ascii_alphabetic() && first != '_' {
            sanitized = format!("_{}", sanitized);
        }
    }

    // Remove consecutive underscores
    let dedup_re = Regex::new(r"_+").unwrap();
    sanitized = dedup_re.replace_all(&sanitized, "_").to_string();

    // Remove trailing underscore
    sanitized = sanitized.trim_end_matches('_').to_string();

    // Truncate to 64 characters
    if sanitized.len() > 64 {
        sanitized.truncate(64);
    }

    sanitized
}

// ---------------------------------------------------------------------------
// Tool conversion logging
// ---------------------------------------------------------------------------

/// Log tool conversion for debugging.
///
/// Corresponds to `log_tool_conversion` in Python.
pub fn log_tool_conversion(tool: &Value, provider: &str) {
    match extract_tool_info(tool) {
        Ok((name, description, parameters)) => {
            let desc_preview = if description.len() > 50 {
                format!("{}...", &description[..50])
            } else {
                description.clone()
            };
            log::debug!(
                "{}: Converting tool '{}' (desc: {})",
                provider,
                name,
                desc_preview
            );
            log::debug!("{}: Tool parameters: {:?}", provider, parameters);
        }
        Err(e) => {
            log::error!("{}: Error extracting tool info: {}", provider, e);
            log::error!("{}: Tool structure: {:?}", provider, tool);
        }
    }
}

// ---------------------------------------------------------------------------
// Safe tool conversion
// ---------------------------------------------------------------------------

/// Safely extract and validate tool information.
///
/// Combines extraction, sanitization, validation, and logging for robust
/// tool conversion.
///
/// Corresponds to `safe_tool_conversion` in Python.
///
/// # Arguments
///
/// * `tool` - Tool value to convert.
/// * `provider` - Provider name for error messages and logging.
///
/// # Returns
///
/// Tuple of (validated_name, description, parameters).
pub fn safe_tool_conversion(
    tool: &Value,
    provider: &str,
) -> Result<(String, String, Value), String> {
    log_tool_conversion(tool, provider);

    let (name, description, parameters) = extract_tool_info(tool)?;

    // Sanitize name before validation
    let sanitized_name = sanitize_function_name(&name);

    let validated_name = validate_function_name(&sanitized_name, provider)?;

    log::info!(
        "{}: Successfully validated tool '{}'",
        provider,
        validated_name
    );

    Ok((validated_name, description, parameters))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_function_name_valid() {
        assert!(validate_function_name("search_web", "test").is_ok());
        assert!(validate_function_name("_private_fn", "test").is_ok());
        assert!(validate_function_name("tool123", "test").is_ok());
    }

    #[test]
    fn test_validate_function_name_invalid() {
        assert!(validate_function_name("", "test").is_err());
        assert!(validate_function_name("123start", "test").is_err());
        assert!(validate_function_name("UPPERCASE", "test").is_err());
        assert!(validate_function_name("has-dashes", "test").is_err());
        assert!(validate_function_name("has spaces", "test").is_err());

        // Too long
        let long_name = "a".repeat(65);
        assert!(validate_function_name(&long_name, "test").is_err());
    }

    #[test]
    fn test_sanitize_function_name() {
        assert_eq!(sanitize_function_name("search_web"), "search_web");
        assert_eq!(sanitize_function_name("Search-Web"), "search_web");
        assert_eq!(sanitize_function_name("My Tool!"), "my_tool");
        assert_eq!(sanitize_function_name("123start"), "_123start");
        assert_eq!(sanitize_function_name(""), "");
    }

    #[test]
    fn test_extract_tool_info_openai_format() {
        let tool = serde_json::json!({
            "type": "function",
            "function": {
                "name": "search",
                "description": "Search the web",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    }
                }
            }
        });

        let (name, desc, params) = extract_tool_info(&tool).unwrap();
        assert_eq!(name, "search");
        assert_eq!(desc, "Search the web");
        assert!(params.get("properties").is_some());
    }

    #[test]
    fn test_extract_tool_info_direct_format() {
        let tool = serde_json::json!({
            "name": "calculator",
            "description": "Do math",
            "parameters": {}
        });

        let (name, desc, _params) = extract_tool_info(&tool).unwrap();
        assert_eq!(name, "calculator");
        assert_eq!(desc, "Do math");
    }

    #[test]
    fn test_extract_tool_info_invalid() {
        let tool = Value::String("not an object".to_string());
        assert!(extract_tool_info(&tool).is_err());
    }

    #[test]
    fn test_safe_tool_conversion() {
        let tool = serde_json::json!({
            "type": "function",
            "function": {
                "name": "search_web",
                "description": "Search the web for information",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    }
                }
            }
        });

        let (name, desc, params) = safe_tool_conversion(&tool, "test").unwrap();
        assert_eq!(name, "search_web");
        assert_eq!(desc, "Search the web for information");
        assert!(params.get("properties").is_some());
    }
}
