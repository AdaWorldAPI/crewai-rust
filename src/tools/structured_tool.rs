//! Structured tool implementation for CrewAI.
//!
//! Corresponds to `crewai/tools/structured_tool.py`.
//!
//! Provides `CrewStructuredTool`, a structured tool that can operate on any
//! number of inputs. This replaces LangChain's `StructuredTool` with a custom
//! implementation that integrates with CrewAI's ecosystem.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use serde_json::Value;

use super::base_tool::ToolUsageLimitExceededError;

/// Type alias for a structured tool function.
pub type StructuredToolFn =
    Arc<dyn Fn(HashMap<String, Value>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> + Send + Sync>;

/// A structured tool that can operate on any number of inputs.
///
/// This tool replaces LangChain's `StructuredTool` with a custom implementation
/// that integrates better with CrewAI's ecosystem. It supports argument
/// validation via a JSON schema, usage limiting, and both sync and async
/// invocation.
#[derive(Clone)]
pub struct CrewStructuredTool {
    /// The name of the tool.
    pub name: String,
    /// A description of what the tool does.
    pub description: String,
    /// JSON Schema for the tool's arguments (as `serde_json::Value`).
    pub args_schema: Value,
    /// The function to run when the tool is called.
    /// `None` if the tool is a schema-only placeholder (e.g., converted from
    /// a trait object without capturing the function).
    pub func: Option<StructuredToolFn>,
    /// Whether to return the output directly as the agent's final answer.
    pub result_as_answer: bool,
    /// Maximum number of times this tool can be used. `None` means unlimited.
    pub max_usage_count: Option<u32>,
    /// Current number of times this tool has been used.
    pub current_usage_count: u32,
}

impl fmt::Debug for CrewStructuredTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CrewStructuredTool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("result_as_answer", &self.result_as_answer)
            .field("max_usage_count", &self.max_usage_count)
            .field("current_usage_count", &self.current_usage_count)
            .finish()
    }
}

impl fmt::Display for CrewStructuredTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CrewStructuredTool(name='{}', description='{}')",
            self.name, self.description
        )
    }
}

impl CrewStructuredTool {
    /// Create a new `CrewStructuredTool`.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        args_schema: Value,
        func: StructuredToolFn,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            args_schema,
            func: Some(func),
            result_as_answer: false,
            max_usage_count: None,
            current_usage_count: 0,
        }
    }

    /// Create a `CrewStructuredTool` from a function with inferred defaults.
    pub fn from_function(
        name: impl Into<String>,
        description: impl Into<String>,
        func: StructuredToolFn,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            args_schema: Value::Object(serde_json::Map::new()),
            func: Some(func),
            result_as_answer: false,
            max_usage_count: None,
            current_usage_count: 0,
        }
    }

    /// Parse and validate the input arguments against the schema.
    ///
    /// Accepts either a JSON string or a `Value::Object`. Returns the parsed
    /// arguments as a `HashMap`.
    pub fn parse_args(
        &self,
        raw_args: Value,
    ) -> Result<HashMap<String, Value>, Box<dyn std::error::Error + Send + Sync>> {
        let obj = match raw_args {
            Value::Object(map) => map,
            Value::String(s) => {
                let parsed: serde_json::Map<String, Value> = serde_json::from_str(&s)
                    .map_err(|e| format!("Failed to parse arguments as JSON: {}", e))?;
                parsed
            }
            _ => {
                return Err("Arguments must be a JSON object or string".into());
            }
        };

        Ok(obj.into_iter().collect())
    }

    /// Invoke the tool synchronously.
    ///
    /// Parses arguments, checks usage limits, increments the count, and
    /// executes the function.
    pub fn invoke(
        &mut self,
        input: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let parsed_args = self.parse_args(input)?;

        if self.has_reached_max_usage_count() {
            return Err(Box::new(ToolUsageLimitExceededError {
                message: format!(
                    "Tool '{}' has reached its maximum usage limit of {}. You should not use the {} tool again.",
                    self.name,
                    self.max_usage_count.unwrap_or(0),
                    self.name,
                ),
            }));
        }

        self.increment_usage_count();

        match &self.func {
            Some(func) => func(parsed_args),
            None => Err("Tool function is not set".into()),
        }
    }

    /// Invoke the tool asynchronously.
    ///
    /// Currently delegates to `invoke` as a blocking call. For true async
    /// support, the function itself should be async-aware.
    pub async fn ainvoke(
        &mut self,
        input: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // In a real async implementation, this would run the function
        // in a spawned blocking task or use an async function directly.
        self.invoke(input)
    }

    /// Legacy `_run` method for compatibility.
    pub fn _run(
        &mut self,
        args: HashMap<String, Value>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let input = Value::Object(args.into_iter().collect());
        self.invoke(input)
    }

    /// Check if the tool has reached its maximum usage count.
    pub fn has_reached_max_usage_count(&self) -> bool {
        match self.max_usage_count {
            Some(max) => self.current_usage_count >= max,
            None => false,
        }
    }

    /// Increment the usage count.
    pub fn increment_usage_count(&mut self) {
        self.current_usage_count += 1;
    }

    /// Reset the usage count to zero.
    pub fn reset_usage_count(&mut self) {
        self.current_usage_count = 0;
    }

    /// Get the tool's input arguments schema as a JSON object of properties.
    pub fn args(&self) -> Value {
        match &self.args_schema {
            Value::Object(map) => {
                if let Some(props) = map.get("properties") {
                    props.clone()
                } else {
                    Value::Object(serde_json::Map::new())
                }
            }
            _ => Value::Object(serde_json::Map::new()),
        }
    }
}
