//! Cache tools for CrewAI agents.
//!
//! Corresponds to `crewai/tools/cache_tools/cache_tools.py`.
//!
//! Provides a `CacheTools` struct that creates a structured tool for
//! reading directly from the agent's tool-result cache.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use super::structured_tool::CrewStructuredTool;
use crate::agents::cache::CacheHandler;
use crate::utilities::string_utils::sanitize_tool_name;

/// Default tools to hit the cache.
///
/// Creates a `CrewStructuredTool` that reads previously cached tool results.
/// The cache key format is `"tool:{tool_name}|input:{input}"`.
///
/// Corresponds to `crewai.tools.cache_tools.cache_tools.CacheTools`.
#[derive(Debug, Clone)]
pub struct CacheTools {
    /// Display name for the cache tool.
    pub name: String,
    /// The cache handler to read from.
    pub cache_handler: CacheHandler,
}

impl CacheTools {
    /// Create a new `CacheTools` with the given cache handler.
    pub fn new(cache_handler: CacheHandler) -> Self {
        Self {
            name: "Hit Cache".to_string(),
            cache_handler,
        }
    }

    /// Create the structured tool that reads from the cache.
    ///
    /// The tool accepts a `key` argument in the format
    /// `"tool:{tool_name}|input:{input}"` and returns the cached result
    /// if found.
    pub fn tool(&self) -> CrewStructuredTool {
        let cache = self.cache_handler.clone();
        let name = sanitize_tool_name(&self.name, None);

        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "Cache key in format 'tool:{tool_name}|input:{input}'"
                }
            },
            "required": ["key"]
        });

        let mut tool = CrewStructuredTool::new(
            name,
            "Reads directly from the cache",
            schema,
            Arc::new(move |args: HashMap<String, Value>| {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'key' argument")?;

                let (tool_name, input) = parse_cache_key(key)?;
                match cache.read(&tool_name, &input) {
                    Some(value) => Ok(value),
                    None => Ok(Value::Null),
                }
            }),
        );

        tool.result_as_answer = false;
        tool
    }
}

impl Default for CacheTools {
    fn default() -> Self {
        Self::new(CacheHandler::new())
    }
}

/// Parse a cache key in the format `"tool:{name}|input:{input}"`.
fn parse_cache_key(key: &str) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let parts: Vec<&str> = key.splitn(2, "tool:").collect();
    if parts.len() < 2 {
        return Err(format!("Invalid cache key format: {}", key).into());
    }

    let remainder = parts[1];
    let tool_input: Vec<&str> = remainder.splitn(2, "|input:").collect();
    if tool_input.len() < 2 {
        return Err(format!("Invalid cache key format (missing |input:): {}", key).into());
    }

    let tool_name = tool_input[0].trim().to_string();
    let input = tool_input[1].trim().to_string();
    Ok((tool_name, input))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cache_key() {
        let (tool, input) = parse_cache_key("tool:search|input:query=rust").unwrap();
        assert_eq!(tool, "search");
        assert_eq!(input, "query=rust");
    }

    #[test]
    fn test_parse_cache_key_invalid() {
        assert!(parse_cache_key("invalid_key").is_err());
        assert!(parse_cache_key("tool:search_no_input").is_err());
    }

    #[test]
    fn test_cache_tools_default() {
        let ct = CacheTools::default();
        assert_eq!(ct.name, "Hit Cache");
    }

    #[test]
    fn test_cache_tools_creates_tool() {
        let handler = CacheHandler::new();
        handler.add("search", "query=rust", Value::String("Found results".to_string()));

        let ct = CacheTools::new(handler);
        let mut tool = ct.tool();

        // The tool should be callable
        let input = serde_json::json!({"key": "tool:search|input:query=rust"});
        let result = tool.invoke(input).unwrap();
        assert_eq!(result, Value::String("Found results".to_string()));
    }

    #[test]
    fn test_cache_tools_cache_miss() {
        let ct = CacheTools::default();
        let mut tool = ct.tool();

        let input = serde_json::json!({"key": "tool:missing|input:nothing"});
        let result = tool.invoke(input).unwrap();
        assert_eq!(result, Value::Null);
    }
}
