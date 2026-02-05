//! Tool filtering support for MCP servers.
//!
//! Port of crewai/mcp/filters.py

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Context for dynamic tool filtering.
///
/// This context is passed to dynamic tool filters to provide
/// information about the agent, run context, and server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFilterContext {
    /// The agent requesting tools (serialized as JSON).
    pub agent: Value,
    /// Name of the MCP server.
    pub server_name: String,
    /// Optional run context for additional filtering logic.
    pub run_context: Option<Value>,
}

impl ToolFilterContext {
    /// Create a new ToolFilterContext.
    pub fn new(agent: Value, server_name: String, run_context: Option<Value>) -> Self {
        Self {
            agent,
            server_name,
            run_context,
        }
    }
}

/// Type alias for tool filter functions.
///
/// A tool filter is a function that takes a tool definition and returns
/// whether the tool should be included (true) or excluded (false).
pub type ToolFilter = Box<dyn Fn(&Value) -> bool + Send + Sync>;

/// Type alias for context-aware dynamic tool filter functions.
///
/// A dynamic tool filter takes both a context and a tool definition.
pub type DynamicToolFilter =
    Box<dyn Fn(&ToolFilterContext, &Value) -> bool + Send + Sync>;

/// Static tool filter with allow/block lists.
///
/// Provides simple allow/block list filtering based on tool names.
/// Useful for restricting which tools are available from an MCP server.
#[derive(Debug, Clone)]
pub struct StaticToolFilter {
    /// Set of allowed tool names. If empty, all tools are allowed (unless blocked).
    pub allowed_tool_names: HashSet<String>,
    /// Set of blocked tool names. Blocked tools take precedence over allowed tools.
    pub blocked_tool_names: HashSet<String>,
}

impl StaticToolFilter {
    /// Create a new StaticToolFilter.
    ///
    /// # Arguments
    /// * `allowed_tool_names` - Optional list of tool names to allow.
    ///   If None, all tools are allowed (unless blocked).
    /// * `blocked_tool_names` - Optional list of tool names to block.
    ///   Blocked tools take precedence over allowed tools.
    pub fn new(
        allowed_tool_names: Option<Vec<String>>,
        blocked_tool_names: Option<Vec<String>>,
    ) -> Self {
        Self {
            allowed_tool_names: allowed_tool_names
                .unwrap_or_default()
                .into_iter()
                .collect(),
            blocked_tool_names: blocked_tool_names
                .unwrap_or_default()
                .into_iter()
                .collect(),
        }
    }

    /// Filter a tool based on allow/block lists.
    ///
    /// # Arguments
    /// * `tool` - Tool definition JSON with at least a "name" key.
    ///
    /// # Returns
    /// `true` if tool should be included, `false` otherwise.
    pub fn filter(&self, tool: &Value) -> bool {
        let tool_name = tool
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("");

        // Blocked tools take precedence
        if !self.blocked_tool_names.is_empty()
            && self.blocked_tool_names.contains(tool_name)
        {
            return false;
        }

        // If allow list exists, tool must be in it
        if !self.allowed_tool_names.is_empty() {
            return self.allowed_tool_names.contains(tool_name);
        }

        // No restrictions -- allow all
        true
    }

    /// Convert this static filter into a boxed ToolFilter function.
    pub fn into_tool_filter(self) -> ToolFilter {
        Box::new(move |tool: &Value| self.filter(tool))
    }
}

/// Create a static tool filter function.
///
/// Convenience function for creating static tool filters with allow/block lists.
///
/// # Arguments
/// * `allowed_tool_names` - Optional list of tool names to allow.
/// * `blocked_tool_names` - Optional list of tool names to block.
///
/// # Returns
/// A boxed ToolFilter function.
pub fn create_static_tool_filter(
    allowed_tool_names: Option<Vec<String>>,
    blocked_tool_names: Option<Vec<String>>,
) -> ToolFilter {
    StaticToolFilter::new(allowed_tool_names, blocked_tool_names).into_tool_filter()
}

/// Create a dynamic tool filter function.
///
/// Wraps a dynamic filter function that has access to the tool filter context.
///
/// # Arguments
/// * `filter_func` - Function that takes (context, tool) and returns bool.
///
/// # Returns
/// A boxed DynamicToolFilter function.
pub fn create_dynamic_tool_filter<F>(filter_func: F) -> DynamicToolFilter
where
    F: Fn(&ToolFilterContext, &Value) -> bool + Send + Sync + 'static,
{
    Box::new(filter_func)
}
