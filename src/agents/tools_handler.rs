//! Tools handler for managing tool execution and caching.
//!
//! Corresponds to `crewai/agents/tools_handler.py`.
//!
//! Provides the `ToolsHandler` struct which manages tool result caching
//! and tracks the last used tool.

use serde_json::Value;

use super::cache::CacheHandler;
use crate::tools::tool_calling::ToolCalling;

/// Callback handler for tool usage.
///
/// Tracks the most recently used tool and optionally caches tool outputs
/// for retrieval on subsequent calls with the same arguments.
#[derive(Debug, Clone)]
pub struct ToolsHandler {
    /// Optional cache handler for storing tool outputs.
    pub cache: Option<CacheHandler>,
    /// The most recently used tool calling instance.
    pub last_used_tool: Option<ToolCalling>,
}

impl Default for ToolsHandler {
    fn default() -> Self {
        Self::new(None)
    }
}

impl ToolsHandler {
    /// Create a new `ToolsHandler` with an optional cache.
    pub fn new(cache: Option<CacheHandler>) -> Self {
        Self {
            cache,
            last_used_tool: None,
        }
    }

    /// Handle a tool use event.
    ///
    /// Records the tool calling instance and optionally caches the output.
    ///
    /// # Arguments
    ///
    /// * `calling` - The tool calling instance describing which tool was called.
    /// * `output` - The string output from the tool execution.
    /// * `should_cache` - Whether to cache the tool output.
    pub fn on_tool_use(
        &mut self,
        calling: &ToolCalling,
        output: &str,
        should_cache: bool,
    ) {
        self.last_used_tool = Some(calling.clone());

        if let Some(ref mut cache) = self.cache {
            // Don't cache the cache tool itself
            if should_cache && calling.tool_name != "CacheTools" {
                let input_str = match &calling.arguments {
                    Some(args) => {
                        serde_json::to_string(args).unwrap_or_default()
                    }
                    None => String::new(),
                };

                cache.add(
                    &calling.tool_name,
                    &input_str,
                    Value::String(output.to_string()),
                );
            }
        }
    }
}
