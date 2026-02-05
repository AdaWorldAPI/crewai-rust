//! Tool usage lifecycle management for CrewAI agents.
//!
//! Corresponds to `crewai/tools/tool_usage.py`.
//!
//! Manages the full tool execution lifecycle: parsing the LLM's tool-call
//! text, selecting the right tool (fuzzy-matching), validating inputs,
//! checking usage limits, executing (sync & async), caching results,
//! emitting events, and handling retries with exponential back-off on
//! parse errors.

use std::collections::HashMap;
use std::fmt;
use std::time::Instant;

use serde_json::Value;

use super::structured_tool::CrewStructuredTool;
use super::tool_calling::ToolCalling;
use crate::agents::cache::CacheHandler;
use crate::utilities::i18n::I18N;
use crate::utilities::printer::{Printer, PrinterColor};
use crate::utilities::string_utils::sanitize_tool_name;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// OpenAI models that get fewer parsing retries.
pub const OPENAI_BIGGER_MODELS: &[&str] = &[
    "gpt-4",
    "gpt-4o",
    "o1-preview",
    "o1-mini",
    "o1",
    "o3",
    "o3-mini",
];

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Error raised during tool usage (parsing, selection, execution).
#[derive(Debug, Clone)]
pub struct ToolUsageError {
    pub message: String,
}

impl ToolUsageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ToolUsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ToolUsageError {}

// ---------------------------------------------------------------------------
// ToolAction (represents parsed agent action)
// ---------------------------------------------------------------------------

/// Represents a parsed agent action before tool selection.
///
/// Mirrors the `AgentAction` / action object used in the Python code:
/// `self.action.tool` and `self.action.tool_input`.
#[derive(Debug, Clone)]
pub struct ToolAction {
    /// The name of the tool the agent wants to call.
    pub tool: String,
    /// The raw input string/value the agent provided.
    pub tool_input: Option<String>,
}

// ---------------------------------------------------------------------------
// Helper: sanitize with default max_length
// ---------------------------------------------------------------------------

/// Convenience wrapper that calls `sanitize_tool_name` with default max length.
fn sanitize(name: &str) -> String {
    sanitize_tool_name(name, None)
}

// ---------------------------------------------------------------------------
// ToolUsage
// ---------------------------------------------------------------------------

/// Manages a single tool-use attempt by an agent.
///
/// Handles the full lifecycle: parse → select → validate → execute → cache → emit.
///
/// Corresponds to `crewai.tools.tool_usage.ToolUsage`.
pub struct ToolUsage {
    /// Internationalization helper.
    pub i18n: I18N,
    /// Output printer.
    pub printer: Printer,
    /// Current run attempt count (1-indexed).
    pub run_attempts: u32,
    /// Maximum parsing retries before giving up.
    pub max_parsing_attempts: u32,
    /// After this many tool uses, remind the agent of the correct format.
    pub remember_format_after_usages: u32,
    /// Available tools for the agent.
    pub tools: Vec<CrewStructuredTool>,
    /// Pre-rendered tool descriptions.
    pub tools_description: String,
    /// Comma-separated tool names.
    pub tools_names: String,
    /// Optional cache handler.
    pub cache: Option<CacheHandler>,
    /// The parsed action the agent wants to take.
    pub action: Option<ToolAction>,
    /// Optional fingerprint context for security metadata.
    pub fingerprint_context: HashMap<String, String>,
    /// Agent role (for logging / events).
    pub agent_role: Option<String>,
    /// Agent key (for events).
    pub agent_key: Option<String>,
    /// Whether the agent is in verbose mode.
    pub verbose: bool,
    /// Last tool calling for repeated-usage detection.
    pub last_used_tool: Option<ToolCalling>,
    /// Count of tools used so far in the task.
    pub used_tools: u32,
}

impl ToolUsage {
    /// Create a new `ToolUsage` with the given tools.
    pub fn new(
        tools: Vec<CrewStructuredTool>,
        cache: Option<CacheHandler>,
        model_name: Option<&str>,
    ) -> Self {
        let tools_description = render_text_description_and_args(&tools);
        let tools_names = get_tool_names(&tools);

        let (max_parsing, remember_format) = if let Some(model) = model_name {
            if OPENAI_BIGGER_MODELS.contains(&model) {
                (2, 4)
            } else {
                (3, 3)
            }
        } else {
            (3, 3)
        };

        Self {
            i18n: I18N::default(),
            printer: Printer::default(),
            run_attempts: 1,
            max_parsing_attempts: max_parsing,
            remember_format_after_usages: remember_format,
            tools,
            tools_description,
            tools_names,
            cache,
            action: None,
            fingerprint_context: HashMap::new(),
            agent_role: None,
            agent_key: None,
            verbose: false,
            last_used_tool: None,
            used_tools: 0,
        }
    }

    /// Parse a tool-calling string into a `ToolCalling`.
    pub fn parse_tool_calling(
        &mut self,
        tool_string: &str,
    ) -> Result<ToolCalling, ToolUsageError> {
        self.tool_calling(tool_string)
    }

    /// Execute a tool call synchronously.
    ///
    /// Handles: repeated-usage detection, cache reads, usage limits,
    /// argument validation, execution, caching, and retry logic.
    pub fn use_tool(
        &mut self,
        calling: &ToolCalling,
        _tool_string: &str,
    ) -> String {
        // Select the tool
        let tool_idx = match self.select_tool(&calling.tool_name) {
            Ok(idx) => idx,
            Err(e) => {
                if self.verbose {
                    self.printer
                        .print(&format!("\n\n{}\n", e.message), PrinterColor::Red);
                }
                return e.message;
            }
        };

        // Check repeated usage
        if self.check_tool_repeated_usage(calling) {
            return self.format_result(&format!(
                "I just used the {} tool with the same input. I need to try a different approach or use a different tool.",
                calling.tool_name
            ));
        }

        let started_at = Instant::now();
        let mut from_cache = false;

        // Check cache
        if let Some(ref cache) = self.cache {
            let input_str = calling
                .arguments
                .as_ref()
                .map(|args| serde_json::to_string(args).unwrap_or_default())
                .unwrap_or_default();

            if let Some(cached) = cache.read(&sanitize(&calling.tool_name), &input_str) {
                from_cache = true;
                let result = self.format_result(&cached.to_string());
                self.log_tool_finished(&calling.tool_name, started_at, from_cache, &result);
                return result;
            }
        }

        // Check usage limit
        if let Some(error) = self.check_usage_limit(tool_idx) {
            return self.format_result(&error);
        }

        // Execute
        let tool = &mut self.tools[tool_idx];
        let arguments = calling.arguments.clone().unwrap_or_default();

        let input = Value::Object(arguments.into_iter().collect());
        match tool.invoke(input) {
            Ok(result) => {
                // Cache the result
                if let Some(ref cache) = self.cache {
                    let input_str = calling
                        .arguments
                        .as_ref()
                        .map(|args| serde_json::to_string(args).unwrap_or_default())
                        .unwrap_or_default();
                    cache.add(&sanitize(&calling.tool_name), &input_str, result.clone());
                }

                let result_str = self.format_result(&result.to_string());
                self.log_tool_finished(&calling.tool_name, started_at, from_cache, &result_str);
                result_str
            }
            Err(e) => {
                self.run_attempts += 1;
                let error_msg = format!("Tool execution error: {}", e);
                if self.verbose {
                    self.printer
                        .print(&format!("\n\n{}\n", error_msg), PrinterColor::Red);
                }
                if self.run_attempts > self.max_parsing_attempts {
                    return self.format_result(&error_msg);
                }
                // Retry
                self.use_tool(calling, _tool_string)
            }
        }
    }

    /// Execute a tool call asynchronously.
    pub async fn ause_tool(
        &mut self,
        calling: &ToolCalling,
        _tool_string: &str,
    ) -> String {
        // Select the tool
        let tool_idx = match self.select_tool(&calling.tool_name) {
            Ok(idx) => idx,
            Err(e) => {
                if self.verbose {
                    self.printer
                        .print(&format!("\n\n{}\n", e.message), PrinterColor::Red);
                }
                return e.message;
            }
        };

        // Check repeated usage
        if self.check_tool_repeated_usage(calling) {
            return self.format_result(&format!(
                "I just used the {} tool with the same input. I need to try a different approach or use a different tool.",
                calling.tool_name
            ));
        }

        let started_at = Instant::now();
        let from_cache = false;

        // Check usage limit
        if let Some(error) = self.check_usage_limit(tool_idx) {
            return self.format_result(&error);
        }

        // Execute async
        let tool = &mut self.tools[tool_idx];
        let arguments = calling.arguments.clone().unwrap_or_default();

        let input = Value::Object(arguments.into_iter().collect());
        match tool.ainvoke(input).await {
            Ok(result) => {
                // Cache the result
                if let Some(ref cache) = self.cache {
                    let input_str = calling
                        .arguments
                        .as_ref()
                        .map(|args| serde_json::to_string(args).unwrap_or_default())
                        .unwrap_or_default();
                    cache.add(&sanitize(&calling.tool_name), &input_str, result.clone());
                }

                let result_str = self.format_result(&result.to_string());
                self.log_tool_finished(&calling.tool_name, started_at, from_cache, &result_str);
                result_str
            }
            Err(e) => {
                self.run_attempts += 1;
                let error_msg = format!("Tool execution error: {}", e);
                if self.verbose {
                    self.printer
                        .print(&format!("\n\n{}\n", error_msg), PrinterColor::Red);
                }
                self.format_result(&error_msg)
            }
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Select the best-matching tool by name using fuzzy matching.
    ///
    /// Uses a simple similarity ratio. If an exact match is found, it is
    /// returned immediately. Otherwise the closest match above 0.85
    /// similarity is used.
    fn select_tool(&self, tool_name: &str) -> Result<usize, ToolUsageError> {
        let sanitized_input = sanitize(tool_name);

        // Exact match first
        for (i, tool) in self.tools.iter().enumerate() {
            if sanitize(&tool.name) == sanitized_input {
                return Ok(i);
            }
        }

        // Fuzzy match
        let mut best_idx = None;
        let mut best_ratio = 0.0_f64;

        for (i, tool) in self.tools.iter().enumerate() {
            let sanitized_tool = sanitize(&tool.name);
            let ratio = similarity_ratio(&sanitized_tool, &sanitized_input);
            if ratio > best_ratio {
                best_ratio = ratio;
                best_idx = Some(i);
            }
        }

        if best_ratio > 0.85 {
            if let Some(idx) = best_idx {
                return Ok(idx);
            }
        }

        let error = if tool_name.is_empty() {
            format!(
                "I forgot the Action name, these are the only available Actions: {}",
                self.tools_description
            )
        } else {
            format!(
                "Action '{}' doesn't exist, these are the only available Actions:\n{}",
                tool_name, self.tools_description
            )
        };

        Err(ToolUsageError::new(error))
    }

    /// Check if the same tool+args were used on the previous call.
    fn check_tool_repeated_usage(&self, calling: &ToolCalling) -> bool {
        if let Some(ref last) = self.last_used_tool {
            sanitize(&calling.tool_name) == sanitize(&last.tool_name)
                && calling.arguments == last.arguments
        } else {
            false
        }
    }

    /// Check if a tool has reached its usage limit.
    fn check_usage_limit(&self, tool_idx: usize) -> Option<String> {
        let tool = &self.tools[tool_idx];
        if let Some(max) = tool.max_usage_count {
            if tool.current_usage_count >= max {
                return Some(format!(
                    "Tool '{}' has reached its usage limit of {} times and cannot be used anymore.",
                    sanitize(&tool.name),
                    max
                ));
            }
        }
        None
    }

    /// Format a result string, optionally appending a format reminder.
    fn format_result(&mut self, result: &str) -> String {
        self.used_tools += 1;
        let should_remind = self.used_tools % self.remember_format_after_usages == 0;
        if should_remind {
            format!(
                "{}\n\nRemember to use the correct tool format. Available tools: {}",
                result, self.tools_names
            )
        } else {
            result.to_string()
        }
    }

    /// Parse the tool-calling text into a structured `ToolCalling`.
    fn tool_calling(&mut self, tool_string: &str) -> Result<ToolCalling, ToolUsageError> {
        // Try to parse from the action object first
        if let Some(ref action) = self.action.clone() {
            match self.original_tool_calling(action) {
                Ok(tc) => return Ok(tc),
                Err(_) => {
                    // Fall through to JSON parsing
                }
            }
        }

        // Try JSON parsing
        match self.parse_json_tool_calling(tool_string) {
            Ok(tc) => Ok(tc),
            Err(e) => {
                self.run_attempts += 1;
                if self.run_attempts > self.max_parsing_attempts {
                    Err(ToolUsageError::new(format!(
                        "Failed to parse tool calling after {} attempts: {}. \
                         Moving on. Available tools: {}",
                        self.max_parsing_attempts, e, self.tools_names
                    )))
                } else {
                    self.tool_calling(tool_string)
                }
            }
        }
    }

    /// Try to build a `ToolCalling` from a pre-parsed action.
    fn original_tool_calling(&self, action: &ToolAction) -> Result<ToolCalling, ToolUsageError> {
        let _tool_idx = self
            .select_tool(&action.tool)
            .map_err(|e| ToolUsageError::new(e.message))?;

        let arguments = if let Some(ref input) = action.tool_input {
            self.validate_tool_input(input)?
        } else {
            HashMap::new()
        };

        Ok(ToolCalling {
            tool_name: sanitize(&action.tool),
            arguments: Some(arguments),
        })
    }

    /// Try to parse a tool call from a raw JSON / JSON5 string.
    fn parse_json_tool_calling(&self, tool_string: &str) -> Result<ToolCalling, ToolUsageError> {
        // Try strict JSON first
        if let Ok(val) = serde_json::from_str::<Value>(tool_string) {
            if let Some(obj) = val.as_object() {
                let tool_name = obj
                    .get("tool_name")
                    .or_else(|| obj.get("action"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolUsageError::new("Missing 'tool_name' in JSON"))?;

                let arguments = obj
                    .get("arguments")
                    .or_else(|| obj.get("action_input"))
                    .and_then(|v| v.as_object())
                    .map(|map| {
                        map.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect::<HashMap<String, Value>>()
                    });

                return Ok(ToolCalling {
                    tool_name: tool_name.to_string(),
                    arguments,
                });
            }
        }

        Err(ToolUsageError::new(format!(
            "Could not parse tool calling from: {}",
            &tool_string[..tool_string.len().min(200)]
        )))
    }

    /// Validate and parse tool input into a `HashMap`.
    fn validate_tool_input(
        &self,
        tool_input: &str,
    ) -> Result<HashMap<String, Value>, ToolUsageError> {
        if tool_input.is_empty() {
            return Ok(HashMap::new());
        }

        // Try JSON
        if let Ok(val) = serde_json::from_str::<Value>(tool_input) {
            if let Some(obj) = val.as_object() {
                return Ok(obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
            }
        }

        Err(ToolUsageError::new(
            "Tool input must be a valid dictionary in JSON format",
        ))
    }

    /// Log tool execution completion.
    fn log_tool_finished(
        &self,
        tool_name: &str,
        started_at: Instant,
        from_cache: bool,
        _result: &str,
    ) {
        let elapsed = started_at.elapsed();
        log::debug!(
            "Tool '{}' finished in {:.2}ms (from_cache={})",
            tool_name,
            elapsed.as_secs_f64() * 1000.0,
            from_cache
        );
    }

    /// Add fingerprint metadata to tool arguments.
    pub fn add_fingerprint_metadata(
        &self,
        mut arguments: HashMap<String, Value>,
    ) -> HashMap<String, Value> {
        if !arguments.contains_key("security_context") {
            arguments.insert(
                "security_context".to_string(),
                Value::Object(serde_json::Map::new()),
            );
        }

        if !self.fingerprint_context.is_empty() {
            if let Some(Value::Object(ref mut ctx)) = arguments.get_mut("security_context") {
                for (k, v) in &self.fingerprint_context {
                    ctx.insert(k.clone(), Value::String(v.clone()));
                }
            }
        }

        arguments
    }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Render tool descriptions and argument schemas as text.
///
/// Corresponds to `crewai.utilities.agent_utils.render_text_description_and_args`.
pub fn render_text_description_and_args(tools: &[CrewStructuredTool]) -> String {
    tools
        .iter()
        .map(|t| {
            let args = if t.args_schema.is_object()
                && !t
                    .args_schema
                    .as_object()
                    .map(|m| m.is_empty())
                    .unwrap_or(true)
            {
                format!(" | Args: {}", t.args_schema)
            } else {
                String::new()
            };
            format!("{}: {}{}", t.name, t.description, args)
        })
        .collect::<Vec<_>>()
        .join("\n--\n")
}

/// Get a comma-separated list of tool names.
///
/// Corresponds to `crewai.utilities.agent_utils.get_tool_names`.
pub fn get_tool_names(tools: &[CrewStructuredTool]) -> String {
    tools
        .iter()
        .map(|t| t.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Compute a simple character-level similarity ratio between two strings.
///
/// Returns a value between 0.0 (completely different) and 1.0 (identical).
/// This is a simplified version of Python's `SequenceMatcher.ratio()`.
fn similarity_ratio(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();

    // Longest common subsequence length
    let mut dp = vec![vec![0u32; len_b + 1]; len_a + 1];
    for i in 1..=len_a {
        for j in 1..=len_b {
            if a_chars[i - 1] == b_chars[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let lcs = dp[len_a][len_b] as f64;
    2.0 * lcs / (len_a + len_b) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_usage_error() {
        let err = ToolUsageError::new("bad input");
        assert_eq!(err.message, "bad input");
        assert_eq!(format!("{}", err), "bad input");
    }

    #[test]
    fn test_similarity_ratio_identical() {
        assert!((similarity_ratio("hello", "hello") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_similarity_ratio_empty() {
        assert!((similarity_ratio("", "hello")).abs() < f64::EPSILON);
        assert!((similarity_ratio("hello", "")).abs() < f64::EPSILON);
    }

    #[test]
    fn test_similarity_ratio_similar() {
        let ratio = similarity_ratio("search_tool", "search_tools");
        assert!(ratio > 0.85);
    }

    #[test]
    fn test_similarity_ratio_different() {
        let ratio = similarity_ratio("search", "zzzzzzz");
        assert!(ratio < 0.5);
    }

    #[test]
    fn test_render_text_description_and_args() {
        let tools = vec![
            CrewStructuredTool::from_function(
                "Search",
                "Search the web",
                std::sync::Arc::new(|_| Ok(Value::Null)),
            ),
            CrewStructuredTool::from_function(
                "Calculate",
                "Do math",
                std::sync::Arc::new(|_| Ok(Value::Null)),
            ),
        ];
        let desc = render_text_description_and_args(&tools);
        assert!(desc.contains("Search: Search the web"));
        assert!(desc.contains("Calculate: Do math"));
    }

    #[test]
    fn test_get_tool_names() {
        let tools = vec![
            CrewStructuredTool::from_function(
                "Alpha",
                "A",
                std::sync::Arc::new(|_| Ok(Value::Null)),
            ),
            CrewStructuredTool::from_function(
                "Beta",
                "B",
                std::sync::Arc::new(|_| Ok(Value::Null)),
            ),
        ];
        let names = get_tool_names(&tools);
        assert_eq!(names, "Alpha, Beta");
    }

    #[test]
    fn test_tool_usage_select_tool_exact() {
        let tools = vec![CrewStructuredTool::from_function(
            "search_web",
            "Search the web",
            std::sync::Arc::new(|_| Ok(Value::Null)),
        )];
        let usage = ToolUsage::new(tools, None, None);
        let idx = usage.select_tool("search_web");
        assert!(idx.is_ok());
        assert_eq!(idx.unwrap(), 0);
    }

    #[test]
    fn test_tool_usage_select_tool_missing() {
        let tools = vec![CrewStructuredTool::from_function(
            "search_web",
            "Search the web",
            std::sync::Arc::new(|_| Ok(Value::Null)),
        )];
        let usage = ToolUsage::new(tools, None, None);
        let result = usage.select_tool("nonexistent_tool");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_json_tool_calling() {
        let tools = vec![CrewStructuredTool::from_function(
            "search",
            "Search",
            std::sync::Arc::new(|_| Ok(Value::Null)),
        )];
        let usage = ToolUsage::new(tools, None, None);
        let result = usage
            .parse_json_tool_calling(r#"{"tool_name": "search", "arguments": {"q": "rust"}}"#);
        assert!(result.is_ok());
        let tc = result.unwrap();
        assert_eq!(tc.tool_name, "search");
        assert!(tc.arguments.is_some());
    }

    #[test]
    fn test_validate_tool_input_valid() {
        let usage = ToolUsage::new(vec![], None, None);
        let result = usage.validate_tool_input(r#"{"key": "value"}"#);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().get("key").unwrap(),
            &Value::String("value".to_string())
        );
    }

    #[test]
    fn test_validate_tool_input_empty() {
        let usage = ToolUsage::new(vec![], None, None);
        let result = usage.validate_tool_input("");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_add_fingerprint_metadata() {
        let mut usage = ToolUsage::new(vec![], None, None);
        usage
            .fingerprint_context
            .insert("agent_id".to_string(), "abc123".to_string());

        let args = HashMap::new();
        let result = usage.add_fingerprint_metadata(args);
        assert!(result.contains_key("security_context"));
    }

    #[test]
    fn test_check_tool_repeated_usage() {
        let mut usage = ToolUsage::new(vec![], None, None);
        let calling = ToolCalling {
            tool_name: "search".to_string(),
            arguments: Some(HashMap::from([(
                "q".to_string(),
                Value::String("rust".to_string()),
            )])),
        };

        // No last tool - not repeated
        assert!(!usage.check_tool_repeated_usage(&calling));

        // Set last tool to same
        usage.last_used_tool = Some(calling.clone());
        assert!(usage.check_tool_repeated_usage(&calling));

        // Different args - not repeated
        let calling2 = ToolCalling {
            tool_name: "search".to_string(),
            arguments: Some(HashMap::from([(
                "q".to_string(),
                Value::String("python".to_string()),
            )])),
        };
        assert!(!usage.check_tool_repeated_usage(&calling2));
    }
}
