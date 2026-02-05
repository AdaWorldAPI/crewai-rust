//! Tools system for CrewAI agents.
//!
//! Corresponds to `crewai/tools/` Python package.
//!
//! This module provides the tools infrastructure including base tool traits,
//! structured tools, tool calling, agent tools, and MCP tool wrappers.

pub mod agent_tools;
pub mod base_tool;
pub mod mcp_native_tool;
pub mod mcp_tool_wrapper;
pub mod structured_tool;
pub mod tool_calling;

// Re-exports for convenience
pub use base_tool::{BaseTool, EnvVar, Tool};
pub use structured_tool::CrewStructuredTool;
pub use tool_calling::ToolCalling;
