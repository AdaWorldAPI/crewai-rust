//! Tools system for CrewAI agents.
//!
//! Corresponds to `crewai/tools/` Python package.
//!
//! This module provides the tools infrastructure including base tool traits,
//! structured tools, tool calling, tool usage lifecycle, cache tools,
//! agent tools, and MCP tool wrappers.

pub mod agent_tools;
pub mod base_tool;
pub mod cache_tools;
#[cfg(feature = "chess")]
pub mod chess;
pub mod mcp_native_tool;
pub mod mcp_tool_wrapper;
pub mod structured_tool;
pub mod tool_calling;
pub mod tool_types;
pub mod tool_usage;

// Re-exports for convenience
pub use base_tool::{BaseTool, EnvVar, Tool};
pub use cache_tools::CacheTools;
pub use structured_tool::CrewStructuredTool;
pub use tool_calling::ToolCalling;
pub use tool_types::ToolResult;
pub use tool_usage::{ToolUsage, ToolUsageError};
