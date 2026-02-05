//! Base LLM abstract class for CrewAI.
//!
//! Corresponds to `crewai/llms/base_llm.py`.
//!
//! Provides the abstract base trait for all LLM implementations in CrewAI,
//! including common functionality for native SDK implementations such as
//! token usage tracking, message formatting, stop word handling, and
//! event emission helpers.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::types::usage_metrics::UsageMetrics;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default context window size in tokens.
pub const DEFAULT_CONTEXT_WINDOW_SIZE: usize = 4096;

/// Default support for stop words.
pub const DEFAULT_SUPPORTS_STOP_WORDS: bool = true;

// ---------------------------------------------------------------------------
// LLM Call Type
// ---------------------------------------------------------------------------

/// The type of LLM call being made.
///
/// Corresponds to `LLMCallType` enum in the Python events system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LLMCallType {
    /// A regular LLM completion call.
    LlmCall,
    /// A tool/function call.
    ToolCall,
}

// ---------------------------------------------------------------------------
// LLM Message type alias
// ---------------------------------------------------------------------------

/// A single message in an LLM conversation.
///
/// Corresponds to Python's `LLMMessage` TypedDict with `role` and `content`
/// keys, plus optional `files`, `tool_calls`, `tool_call_id`, etc.
pub type LLMMessage = HashMap<String, Value>;

// ---------------------------------------------------------------------------
// Call context management
// ---------------------------------------------------------------------------

/// Generate a unique call ID for an LLM call context.
///
/// Corresponds to `get_current_call_id()` in Python. In the Rust port we
/// generate a fresh UUID; task-local context management can be layered on
/// via `tokio::task_local!` when needed.
pub fn generate_call_id() -> String {
    Uuid::new_v4().to_string()
}

/// Monotonically increasing call counter for debugging.
static CALL_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Get the next call sequence number.
pub fn next_call_sequence() -> usize {
    CALL_COUNTER.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// BaseLLM trait
// ---------------------------------------------------------------------------

/// Abstract base trait for LLM implementations.
///
/// Defines the interface that all LLM implementations must follow. Users can
/// implement this trait to create custom LLM implementations that don't rely
/// on any specific provider's authentication mechanism.
///
/// Custom implementations should handle error cases gracefully, including
/// timeouts, authentication failures, and malformed responses.
///
/// Corresponds to the `BaseLLM` ABC in `crewai/llms/base_llm.py`.
#[async_trait]
pub trait BaseLLM: Send + Sync + fmt::Debug {
    // --- Required accessors ---

    /// Get the model identifier/name.
    fn model(&self) -> &str;

    /// Get the optional temperature setting.
    fn temperature(&self) -> Option<f64>;

    /// Get the stop sequences.
    fn stop(&self) -> &[String];

    /// Set the stop sequences.
    fn set_stop(&mut self, stop: Vec<String>);

    // --- Optional accessors with defaults ---

    /// Get the provider name.
    fn provider(&self) -> &str {
        "openai"
    }

    /// Whether this is a litellm-based implementation.
    fn is_litellm(&self) -> bool {
        false
    }

    // --- Core call methods ---

    /// Call the LLM with the given messages (synchronous).
    ///
    /// # Arguments
    ///
    /// * `messages` - Input messages for the LLM (list of message dicts).
    /// * `tools` - Optional list of tool schemas for function calling.
    /// * `available_functions` - Optional dict mapping function names to callables.
    ///
    /// # Returns
    ///
    /// Either a text response (`Value::String`) or the result of a tool call.
    fn call(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;

    /// Call the LLM with the given messages (asynchronous).
    ///
    /// Default implementation returns `NotImplemented`. Override for async support.
    async fn acall(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // Default: not implemented
        let _ = (messages, tools, available_functions);
        Err("Async call not implemented for this LLM".into())
    }

    // --- Capability queries ---

    /// Check if the LLM supports function calling.
    fn supports_function_calling(&self) -> bool {
        false
    }

    /// Check if the LLM supports stop words.
    ///
    /// Returns `true` by default; native providers may override.
    fn supports_stop_words(&self) -> bool {
        DEFAULT_SUPPORTS_STOP_WORDS
    }

    /// Get the context window size for the LLM.
    fn get_context_window_size(&self) -> usize {
        DEFAULT_CONTEXT_WINDOW_SIZE
    }

    /// Check if the LLM supports multimodal inputs (images, PDFs, audio, video).
    fn supports_multimodal(&self) -> bool {
        false
    }

    // --- Content formatting ---

    /// Format text as a content block for the LLM.
    ///
    /// Default implementation uses the OpenAI/Anthropic `{"type": "text", "text": ...}` format.
    fn format_text_content(&self, text: &str) -> Value {
        serde_json::json!({
            "type": "text",
            "text": text
        })
    }

    // --- Token usage ---

    /// Get a summary of token usage for this LLM instance.
    fn get_token_usage_summary(&self) -> UsageMetrics;

    /// Track token usage internally from API response data.
    fn track_token_usage(&mut self, usage_data: &HashMap<String, Value>);

    // --- Tool conversion ---

    /// Convert tools to a format that can be used for inference.
    ///
    /// Default implementation returns tools as-is. Providers may override
    /// to transform to their specific format.
    fn convert_tools_for_inference(&self, tools: Vec<Value>) -> Vec<Value> {
        tools
    }
}

// ---------------------------------------------------------------------------
// BaseLLMState - shared state for LLM implementations
// ---------------------------------------------------------------------------

/// Shared state for LLM implementations.
///
/// Provides common fields and helper methods that concrete LLM implementations
/// can embed and delegate to. This avoids trait object limitations while
/// sharing common functionality.
///
/// Corresponds to the instance variables of `BaseLLM.__init__` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseLLMState {
    /// The model identifier/name.
    pub model: String,
    /// Optional temperature setting for response generation.
    pub temperature: Option<f64>,
    /// Optional API key.
    pub api_key: Option<String>,
    /// Optional base URL for the API.
    pub base_url: Option<String>,
    /// Stop sequences that the LLM should use to stop generation.
    pub stop: Vec<String>,
    /// Provider name (e.g., "openai", "anthropic").
    pub provider: String,
    /// Whether to prefer file upload over inline base64.
    pub prefer_upload: bool,
    /// Additional provider-specific parameters.
    pub additional_params: HashMap<String, Value>,
    /// Internal token usage tracking.
    pub token_usage: TokenUsage,
}

/// Internal token usage counters.
///
/// Tracks cumulative token usage across all calls made by an LLM instance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub total_tokens: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub successful_requests: i64,
    pub cached_prompt_tokens: i64,
}

impl BaseLLMState {
    /// Create a new `BaseLLMState` with the given model name.
    ///
    /// # Panics
    ///
    /// Panics if `model` is empty.
    pub fn new(model: impl Into<String>) -> Self {
        let model = model.into();
        assert!(!model.is_empty(), "Model name is required and cannot be empty");

        Self {
            model,
            temperature: None,
            api_key: None,
            base_url: None,
            stop: Vec::new(),
            provider: "openai".to_string(),
            prefer_upload: false,
            additional_params: HashMap::new(),
            token_usage: TokenUsage::default(),
        }
    }

    /// Create a new `BaseLLMState` with full configuration.
    pub fn with_config(
        model: impl Into<String>,
        temperature: Option<f64>,
        api_key: Option<String>,
        base_url: Option<String>,
        provider: Option<String>,
        prefer_upload: bool,
    ) -> Self {
        let model = model.into();
        assert!(!model.is_empty(), "Model name is required and cannot be empty");

        Self {
            model,
            temperature,
            api_key,
            base_url,
            stop: Vec::new(),
            provider: provider.unwrap_or_else(|| "openai".to_string()),
            prefer_upload,
            additional_params: HashMap::new(),
            token_usage: TokenUsage::default(),
        }
    }

    // --- Stop word handling ---

    /// Apply stop words to truncate response content.
    ///
    /// Finds the earliest occurrence of any stop word and truncates the
    /// content at that point.
    ///
    /// Corresponds to `BaseLLM._apply_stop_words` in Python.
    pub fn apply_stop_words(&self, content: &str) -> String {
        if self.stop.is_empty() || content.is_empty() {
            return content.to_string();
        }

        let mut earliest_stop_pos = content.len();
        let mut found_stop_word: Option<&str> = None;

        for stop_word in &self.stop {
            if let Some(pos) = content.find(stop_word.as_str()) {
                if pos < earliest_stop_pos {
                    earliest_stop_pos = pos;
                    found_stop_word = Some(stop_word);
                }
            }
        }

        if let Some(word) = found_stop_word {
            log::debug!(
                "Applied stop word '{}' at position {}",
                word,
                earliest_stop_pos
            );
            content[..earliest_stop_pos].trim().to_string()
        } else {
            content.to_string()
        }
    }

    /// Check if stop words are configured for this instance.
    ///
    /// Corresponds to `BaseLLM._supports_stop_words_implementation`.
    pub fn has_stop_words(&self) -> bool {
        !self.stop.is_empty()
    }

    // --- Message formatting ---

    /// Format messages to standard list format.
    ///
    /// If already a list of message dicts, validates them. If you have a
    /// plain string, use `string_to_messages` first.
    ///
    /// Corresponds to `BaseLLM._format_messages`.
    pub fn format_messages(
        &self,
        messages: Vec<LLMMessage>,
    ) -> Result<Vec<LLMMessage>, String> {
        for (i, msg) in messages.iter().enumerate() {
            if !msg.contains_key("role") || !msg.contains_key("content") {
                return Err(format!(
                    "Message at index {} must have 'role' and 'content' keys",
                    i
                ));
            }
        }
        Ok(messages)
    }

    /// Format a string input as a single user message.
    pub fn string_to_messages(text: &str) -> Vec<LLMMessage> {
        let mut msg = HashMap::new();
        msg.insert("role".to_string(), Value::String("user".to_string()));
        msg.insert("content".to_string(), Value::String(text.to_string()));
        vec![msg]
    }

    // --- Token usage tracking ---

    /// Track token usage from API response data.
    ///
    /// Extracts tokens in a provider-agnostic way, supporting OpenAI,
    /// Anthropic, Gemini, and Bedrock field names.
    ///
    /// Corresponds to `BaseLLM._track_token_usage_internal`.
    pub fn track_token_usage_internal(&mut self, usage_data: &HashMap<String, Value>) {
        let prompt_tokens = usage_data
            .get("prompt_tokens")
            .or_else(|| usage_data.get("prompt_token_count"))
            .or_else(|| usage_data.get("input_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let completion_tokens = usage_data
            .get("completion_tokens")
            .or_else(|| usage_data.get("candidates_token_count"))
            .or_else(|| usage_data.get("output_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let cached_tokens = usage_data
            .get("cached_tokens")
            .or_else(|| usage_data.get("cached_prompt_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        self.token_usage.prompt_tokens += prompt_tokens;
        self.token_usage.completion_tokens += completion_tokens;
        self.token_usage.total_tokens += prompt_tokens + completion_tokens;
        self.token_usage.successful_requests += 1;
        self.token_usage.cached_prompt_tokens += cached_tokens;
    }

    /// Get summary of token usage as `UsageMetrics`.
    pub fn get_token_usage_summary(&self) -> UsageMetrics {
        UsageMetrics {
            total_tokens: self.token_usage.total_tokens,
            prompt_tokens: self.token_usage.prompt_tokens,
            cached_prompt_tokens: self.token_usage.cached_prompt_tokens,
            completion_tokens: self.token_usage.completion_tokens,
            successful_requests: self.token_usage.successful_requests,
        }
    }

    // --- Provider utilities ---

    /// Extract provider from model string (e.g., "openai/gpt-4" -> "openai").
    ///
    /// Corresponds to `BaseLLM._extract_provider`.
    pub fn extract_provider(model: &str) -> String {
        if let Some(idx) = model.find('/') {
            model[..idx].to_string()
        } else {
            "openai".to_string()
        }
    }

    /// Validate and parse structured output from a response string.
    ///
    /// Tries to parse the response as JSON directly, and falls back to
    /// extracting the first JSON object found via regex.
    ///
    /// Corresponds to `BaseLLM._validate_structured_output`.
    pub fn validate_structured_output(
        response: &str,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // Try to parse as JSON first
        let trimmed = response.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
                return Ok(val);
            }
        }

        // Try to extract JSON from the response using regex
        let json_re = Regex::new(r"\{.*\}").unwrap();
        if let Some(m) = json_re.find(response) {
            if let Ok(val) = serde_json::from_str::<Value>(m.as_str()) {
                return Ok(val);
            }
        }

        Err("No JSON found in response".into())
    }
}

// ---------------------------------------------------------------------------
// Event emission stubs
// ---------------------------------------------------------------------------

/// Stub for emitting LLM call started events.
///
/// In a full implementation, this would publish to the CrewAI event bus.
/// Corresponds to `BaseLLM._emit_call_started_event`.
pub fn emit_call_started_event(
    model: &str,
    messages: &[LLMMessage],
    call_id: &str,
) {
    log::debug!(
        "LLM call started: model={}, call_id={}, messages={}",
        model,
        call_id,
        messages.len()
    );
}

/// Stub for emitting LLM call completed events.
///
/// Corresponds to `BaseLLM._emit_call_completed_event`.
pub fn emit_call_completed_event(
    model: &str,
    call_type: LLMCallType,
    call_id: &str,
) {
    log::debug!(
        "LLM call completed: model={}, type={:?}, call_id={}",
        model,
        call_type,
        call_id
    );
}

/// Stub for emitting LLM call failed events.
///
/// Corresponds to `BaseLLM._emit_call_failed_event`.
pub fn emit_call_failed_event(model: &str, error: &str, call_id: &str) {
    log::warn!(
        "LLM call failed: model={}, error={}, call_id={}",
        model,
        error,
        call_id
    );
}

/// Stub for emitting stream chunk events.
///
/// Corresponds to `BaseLLM._emit_stream_chunk_event`.
pub fn emit_stream_chunk_event(chunk: &str, call_id: &str) {
    log::trace!(
        "LLM stream chunk: call_id={}, chunk_len={}",
        call_id,
        chunk.len()
    );
}

// ---------------------------------------------------------------------------
// Hook invocation stubs
// ---------------------------------------------------------------------------

/// Stub: invoke before-LLM-call hooks.
///
/// Returns `true` if the call should proceed, `false` if blocked by a hook.
/// Corresponds to `BaseLLM._invoke_before_llm_call_hooks`.
pub fn invoke_before_llm_call_hooks(
    _messages: &[LLMMessage],
    _model: &str,
) -> bool {
    // Stub: always allow
    true
}

/// Stub: invoke after-LLM-call hooks.
///
/// Returns the (potentially modified) response string.
/// Corresponds to `BaseLLM._invoke_after_llm_call_hooks`.
pub fn invoke_after_llm_call_hooks(
    _messages: &[LLMMessage],
    response: String,
    _model: &str,
) -> String {
    // Stub: return response unchanged
    response
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_llm_state_new() {
        let state = BaseLLMState::new("gpt-4o");
        assert_eq!(state.model, "gpt-4o");
        assert_eq!(state.provider, "openai");
        assert!(state.stop.is_empty());
    }

    #[test]
    #[should_panic(expected = "Model name is required")]
    fn test_base_llm_state_empty_model() {
        BaseLLMState::new("");
    }

    #[test]
    fn test_apply_stop_words() {
        let mut state = BaseLLMState::new("test-model");
        state.stop = vec!["Observation:".to_string(), "Final Answer:".to_string()];

        let content = "I need to search.\n\nAction: search\nObservation: Found results";
        let result = state.apply_stop_words(content);
        assert_eq!(result, "I need to search.\n\nAction: search");
    }

    #[test]
    fn test_apply_stop_words_no_match() {
        let mut state = BaseLLMState::new("test-model");
        state.stop = vec!["STOP".to_string()];

        let content = "No stop word here";
        let result = state.apply_stop_words(content);
        assert_eq!(result, "No stop word here");
    }

    #[test]
    fn test_apply_stop_words_empty() {
        let state = BaseLLMState::new("test-model");
        let result = state.apply_stop_words("some content");
        assert_eq!(result, "some content");
    }

    #[test]
    fn test_extract_provider() {
        assert_eq!(BaseLLMState::extract_provider("openai/gpt-4"), "openai");
        assert_eq!(
            BaseLLMState::extract_provider("anthropic/claude-3"),
            "anthropic"
        );
        assert_eq!(BaseLLMState::extract_provider("gpt-4"), "openai");
    }

    #[test]
    fn test_validate_structured_output() {
        let json_str = r#"{"key": "value"}"#;
        let result = BaseLLMState::validate_structured_output(json_str);
        assert!(result.is_ok());

        let mixed = "Some text before {\"key\": \"value\"} and after";
        let result = BaseLLMState::validate_structured_output(mixed);
        assert!(result.is_ok());

        let no_json = "No JSON here";
        let result = BaseLLMState::validate_structured_output(no_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_to_messages() {
        let messages = BaseLLMState::string_to_messages("Hello!");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"], "Hello!");
    }

    #[test]
    fn test_format_messages_valid() {
        let state = BaseLLMState::new("test");
        let mut msg = HashMap::new();
        msg.insert("role".to_string(), Value::String("user".to_string()));
        msg.insert("content".to_string(), Value::String("hi".to_string()));
        let result = state.format_messages(vec![msg]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_messages_invalid() {
        let state = BaseLLMState::new("test");
        let mut msg = HashMap::new();
        msg.insert("role".to_string(), Value::String("user".to_string()));
        // Missing "content"
        let result = state.format_messages(vec![msg]);
        assert!(result.is_err());
    }

    #[test]
    fn test_token_usage_tracking() {
        let mut state = BaseLLMState::new("test");
        let mut usage = HashMap::new();
        usage.insert("prompt_tokens".to_string(), serde_json::json!(100));
        usage.insert("completion_tokens".to_string(), serde_json::json!(50));
        usage.insert("cached_tokens".to_string(), serde_json::json!(10));

        state.track_token_usage_internal(&usage);

        assert_eq!(state.token_usage.prompt_tokens, 100);
        assert_eq!(state.token_usage.completion_tokens, 50);
        assert_eq!(state.token_usage.total_tokens, 150);
        assert_eq!(state.token_usage.successful_requests, 1);
        assert_eq!(state.token_usage.cached_prompt_tokens, 10);

        // Track again to test accumulation
        state.track_token_usage_internal(&usage);
        assert_eq!(state.token_usage.total_tokens, 300);
        assert_eq!(state.token_usage.successful_requests, 2);
    }

    #[test]
    fn test_generate_call_id() {
        let id1 = generate_call_id();
        let id2 = generate_call_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID format
    }
}
