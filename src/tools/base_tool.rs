//! Base tool definitions for CrewAI.
//!
//! Corresponds to `crewai/tools/base_tool.py`.
//!
//! Provides the core tool abstractions including `EnvVar`, the `BaseTool` trait,
//! and the concrete `Tool` struct that wraps a callable function.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::structured_tool::CrewStructuredTool;

// ---------------------------------------------------------------------------
// EnvVar
// ---------------------------------------------------------------------------

/// Environment variable definition used by a tool.
///
/// Describes an environment variable that a tool requires or optionally uses,
/// along with its description and default value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    /// Name of the environment variable.
    pub name: String,
    /// Human-readable description of the environment variable.
    pub description: String,
    /// Whether the environment variable is required.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Default value if the environment variable is not set.
    #[serde(default)]
    pub default: Option<String>,
}

fn default_true() -> bool {
    true
}

impl EnvVar {
    /// Create a new required environment variable.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            required: true,
            default: None,
        }
    }

    /// Create a new optional environment variable with a default value.
    pub fn with_default(
        name: impl Into<String>,
        description: impl Into<String>,
        default: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            required: false,
            default: Some(default.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// ToolUsageLimitExceededError
// ---------------------------------------------------------------------------

/// Error raised when a tool has reached its maximum usage limit.
#[derive(Debug, Clone)]
pub struct ToolUsageLimitExceededError {
    pub message: String,
}

impl fmt::Display for ToolUsageLimitExceededError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ToolUsageLimitExceededError {}

// ---------------------------------------------------------------------------
// BaseTool trait
// ---------------------------------------------------------------------------

/// Abstract base trait for all tools compatible with CrewAI.
///
/// This mirrors Python's `BaseTool` ABC. Implementors must provide `name`,
/// `description`, and `run`. The trait provides default implementations
/// for usage tracking, async execution, and conversion to structured tools.
#[async_trait]
pub trait BaseTool: Send + Sync + fmt::Debug {
    /// The unique name of the tool that clearly communicates its purpose.
    fn name(&self) -> &str;

    /// Description used to tell the model how/when/why to use the tool.
    fn description(&self) -> &str;

    /// JSON schema for the arguments that the tool accepts.
    /// Returns a `serde_json::Value` representing the JSON Schema object.
    fn args_schema(&self) -> Value {
        Value::Object(serde_json::Map::new())
    }

    /// List of environment variables used by the tool.
    fn env_vars(&self) -> &[EnvVar] {
        &[]
    }

    /// Whether the tool result should be the final agent answer.
    fn result_as_answer(&self) -> bool {
        false
    }

    /// Maximum number of times this tool can be used. `None` means unlimited.
    fn max_usage_count(&self) -> Option<u32> {
        None
    }

    /// Current number of times this tool has been used.
    fn current_usage_count(&self) -> u32;

    /// Increment the current usage count.
    fn increment_usage_count(&mut self);

    /// Reset the current usage count to zero.
    fn reset_usage_count(&mut self);

    /// Check whether the tool has reached its maximum usage count.
    fn has_reached_max_usage_count(&self) -> bool {
        match self.max_usage_count() {
            Some(max) => self.current_usage_count() >= max,
            None => false,
        }
    }

    /// Cache function that determines if the tool result should be cached.
    /// Returns `true` if the result should be cached.
    fn should_cache(&self, _args: &Value, _result: &Value) -> bool {
        true
    }

    /// Synchronous execution of the tool.
    ///
    /// Subclasses must implement this method for synchronous execution.
    fn run(
        &mut self,
        args: HashMap<String, Value>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;

    /// Asynchronous execution of the tool.
    ///
    /// Default implementation calls `run` synchronously. Override for true
    /// async support.
    async fn arun(
        &mut self,
        args: HashMap<String, Value>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.run(args)
    }

    /// Convert this tool into a `CrewStructuredTool`.
    fn to_structured_tool(&self) -> CrewStructuredTool
    where
        Self: Sized + Clone + 'static,
    {
        CrewStructuredTool {
            name: self.name().to_string(),
            description: self.description().to_string(),
            args_schema: self.args_schema(),
            func: None,
            result_as_answer: self.result_as_answer(),
            max_usage_count: self.max_usage_count(),
            current_usage_count: self.current_usage_count(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool struct (wraps a callable function)
// ---------------------------------------------------------------------------

/// Type alias for a boxed synchronous tool function.
pub type ToolFn =
    Arc<dyn Fn(HashMap<String, Value>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> + Send + Sync>;

/// Concrete tool that wraps a callable function.
///
/// This corresponds to Python's `Tool` class which is generic over `P` (params)
/// and `R` (return type). In Rust we use a boxed function pointer.
#[derive(Clone)]
pub struct Tool {
    /// The unique name of the tool.
    tool_name: String,
    /// Description of the tool's purpose.
    tool_description: String,
    /// JSON Schema for the arguments the tool accepts.
    tool_args_schema: Value,
    /// Environment variables used by the tool.
    tool_env_vars: Vec<EnvVar>,
    /// The wrapped function.
    pub func: ToolFn,
    /// Whether the tool result should be the final agent answer.
    tool_result_as_answer: bool,
    /// Maximum usage count (None = unlimited).
    tool_max_usage_count: Option<u32>,
    /// Current usage count.
    tool_current_usage_count: u32,
}

impl fmt::Debug for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tool")
            .field("name", &self.tool_name)
            .field("description", &self.tool_description)
            .field("result_as_answer", &self.tool_result_as_answer)
            .field("max_usage_count", &self.tool_max_usage_count)
            .field("current_usage_count", &self.tool_current_usage_count)
            .finish()
    }
}

impl Tool {
    /// Create a new Tool wrapping the given function.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        func: ToolFn,
    ) -> Self {
        Self {
            tool_name: name.into(),
            tool_description: description.into(),
            tool_args_schema: Value::Object(serde_json::Map::new()),
            tool_env_vars: Vec::new(),
            func,
            tool_result_as_answer: false,
            tool_max_usage_count: None,
            tool_current_usage_count: 0,
        }
    }

    /// Builder method to set the args schema.
    pub fn with_args_schema(mut self, schema: Value) -> Self {
        self.tool_args_schema = schema;
        self
    }

    /// Builder method to set environment variables.
    pub fn with_env_vars(mut self, env_vars: Vec<EnvVar>) -> Self {
        self.tool_env_vars = env_vars;
        self
    }

    /// Builder method to mark result as the final answer.
    pub fn with_result_as_answer(mut self, result_as_answer: bool) -> Self {
        self.tool_result_as_answer = result_as_answer;
        self
    }

    /// Builder method to set the maximum usage count.
    pub fn with_max_usage_count(mut self, max_usage_count: Option<u32>) -> Self {
        if let Some(count) = max_usage_count {
            assert!(count > 0, "max_usage_count must be a positive integer");
        }
        self.tool_max_usage_count = max_usage_count;
        self
    }
}

#[async_trait]
impl BaseTool for Tool {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.tool_description
    }

    fn args_schema(&self) -> Value {
        self.tool_args_schema.clone()
    }

    fn env_vars(&self) -> &[EnvVar] {
        &self.tool_env_vars
    }

    fn result_as_answer(&self) -> bool {
        self.tool_result_as_answer
    }

    fn max_usage_count(&self) -> Option<u32> {
        self.tool_max_usage_count
    }

    fn current_usage_count(&self) -> u32 {
        self.tool_current_usage_count
    }

    fn increment_usage_count(&mut self) {
        self.tool_current_usage_count += 1;
    }

    fn reset_usage_count(&mut self) {
        self.tool_current_usage_count = 0;
    }

    fn run(
        &mut self,
        args: HashMap<String, Value>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let result = (self.func)(args)?;
        self.tool_current_usage_count += 1;
        Ok(result)
    }
}

/// Convert a list of `BaseTool` trait objects into `CrewStructuredTool` instances.
pub fn to_structured_tools(tools: &[Box<dyn BaseTool>]) -> Vec<CrewStructuredTool> {
    tools
        .iter()
        .map(|t| CrewStructuredTool {
            name: t.name().to_string(),
            description: t.description().to_string(),
            args_schema: t.args_schema(),
            func: None,
            result_as_answer: t.result_as_answer(),
            max_usage_count: t.max_usage_count(),
            current_usage_count: t.current_usage_count(),
        })
        .collect()
}
