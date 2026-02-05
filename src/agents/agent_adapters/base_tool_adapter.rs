//! Base tool adapter trait.
//!
//! Corresponds to `crewai/agents/agent_adapters/base_tool_adapter.py`.
//!
//! Defines the common interface that all tool adapters must implement.
//! Tool adapters are responsible for converting CrewAI tools to the
//! format expected by different agent frameworks.

use std::any::Any;
use std::fmt;

/// Abstract base trait for all tool adapters in CrewAI.
///
/// Defines the common interface for adapting CrewAI tools to different
/// frameworks and platforms. Concrete implementations handle the conversion
/// of `BaseTool` instances into framework-specific tool representations.
pub trait BaseToolAdapter: Send + Sync + fmt::Debug {
    /// Configure and convert tools for the specific implementation.
    ///
    /// Takes a list of tool objects (type-erased) and converts them to
    /// the format expected by the target framework.
    ///
    /// # Arguments
    ///
    /// * `tools` - List of tool objects to be configured and converted.
    fn configure_tools(
        &mut self,
        tools: Vec<Box<dyn Any + Send + Sync>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Return all converted tools.
    fn tools(&self) -> &[Box<dyn Any + Send + Sync>];

    /// Sanitize a tool name for API compatibility.
    ///
    /// Default implementation replaces spaces with underscores and removes
    /// special characters.
    fn sanitize_tool_name(tool_name: &str) -> String
    where
        Self: Sized,
    {
        sanitize_tool_name(tool_name)
    }
}

/// Sanitize a tool name for API compatibility.
///
/// Replaces spaces with underscores and removes special characters to ensure
/// the tool name is compatible with various APIs.
pub fn sanitize_tool_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else if c == ' ' {
                '_'
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase()
}
