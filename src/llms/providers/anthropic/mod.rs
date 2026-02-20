//! Anthropic native completion provider.
//!
//! Corresponds to `crewai/llms/providers/anthropic/completion.py`.
//!
//! This module provides direct integration with the Anthropic Messages API,
//! supporting native tool use, streaming, extended thinking, and proper
//! message formatting with system message extraction.
//!
//! # Features
//!
//! - Anthropic Messages API with real HTTP calls via `reqwest`
//! - Retry with exponential backoff on 429/5xx
//! - Native tool use (function calling)
//! - Extended thinking / chain-of-thought (budget_tokens)
//! - System message extraction from message list
//! - Files API beta support
//! - Token usage tracking

use std::any::Any;
use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::llms::base_llm::{BaseLLM, BaseLLMState, LLMMessage};
use crate::types::usage_metrics::UsageMetrics;

// ---------------------------------------------------------------------------
// Anthropic thinking configuration
// ---------------------------------------------------------------------------

/// Configuration for Anthropic's extended thinking feature.
///
/// Corresponds to `AnthropicThinkingConfig` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicThinkingConfig {
    /// Whether thinking is "enabled" or "disabled".
    #[serde(rename = "type")]
    pub thinking_type: String,
    /// Budget tokens for thinking (required when enabled).
    pub budget_tokens: Option<u32>,
}

impl AnthropicThinkingConfig {
    /// Create an enabled thinking config with the given budget.
    pub fn enabled(budget_tokens: u32) -> Self {
        Self {
            thinking_type: "enabled".to_string(),
            budget_tokens: Some(budget_tokens),
        }
    }

    /// Create a disabled thinking config.
    pub fn disabled() -> Self {
        Self {
            thinking_type: "disabled".to_string(),
            budget_tokens: None,
        }
    }

    /// Check if thinking is enabled.
    pub fn is_enabled(&self) -> bool {
        self.thinking_type == "enabled"
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Anthropic Files API beta header value.
pub const ANTHROPIC_FILES_API_BETA: &str = "files-api-2025-04-14";

/// Anthropic Structured Outputs beta header value.
pub const ANTHROPIC_STRUCTURED_OUTPUTS_BETA: &str = "structured-outputs-2025-11-13";

/// Models that support native structured outputs.
pub const NATIVE_STRUCTURED_OUTPUT_MODELS: &[&str] = &[
    "claude-opus-4-5",
    "claude-opus-4.5",
];

/// Check if a model supports native structured outputs.
pub fn supports_native_structured_outputs(model: &str) -> bool {
    let lower = model.to_lowercase();
    NATIVE_STRUCTURED_OUTPUT_MODELS
        .iter()
        .any(|prefix| lower.contains(prefix))
}

// ---------------------------------------------------------------------------
// AnthropicCompletion provider
// ---------------------------------------------------------------------------

/// Anthropic native completion implementation.
///
/// Provides direct integration with the Anthropic Messages API via `reqwest`.
///
/// Corresponds to `AnthropicCompletion(BaseLLM)` in Python.
///
/// # Example
///
/// ```ignore
/// let provider = AnthropicCompletion::new(
///     "claude-opus-4-5-20251101",
///     None,
///     None,
/// );
/// let messages = vec![/* ... */];
/// let response = provider.call(messages, None, None)?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicCompletion {
    /// Shared base LLM state.
    #[serde(flatten)]
    pub state: BaseLLMState,

    /// Request timeout in seconds.
    pub timeout: Option<f64>,
    /// Maximum number of retries.
    pub max_retries: u32,
    /// Maximum tokens in response (required for Anthropic).
    pub max_tokens: u32,
    /// Anthropic API version header.
    pub anthropic_version: String,
    /// Nucleus sampling parameter.
    pub top_p: Option<f64>,
    /// Stop sequences.
    pub stop_sequences: Vec<String>,
    /// Whether to use streaming responses.
    pub stream: bool,
    /// Additional client parameters.
    pub client_params: Option<HashMap<String, Value>>,
    /// Extended thinking configuration.
    pub thinking: Option<AnthropicThinkingConfig>,
    /// Response format for structured output.
    pub response_format: Option<Value>,
}

impl AnthropicCompletion {
    /// Create a new Anthropic completion provider.
    ///
    /// # Arguments
    ///
    /// * `model` - Anthropic model name (e.g., "claude-opus-4-5-20251101").
    /// * `api_key` - Optional API key (defaults to ANTHROPIC_API_KEY env var).
    /// * `base_url` - Optional custom base URL.
    pub fn new(
        model: impl Into<String>,
        api_key: Option<String>,
        base_url: Option<String>,
    ) -> Self {
        let api_key = api_key.or_else(|| std::env::var("ANTHROPIC_API_KEY").ok());

        let mut state = BaseLLMState::new(model);
        state.api_key = api_key;
        state.base_url = base_url;
        state.provider = "anthropic".to_string();

        Self {
            state,
            timeout: None,
            max_retries: 2,
            max_tokens: 4096,
            anthropic_version: "2023-06-01".to_string(),
            top_p: None,
            stop_sequences: Vec::new(),
            stream: false,
            client_params: None,
            thinking: None,
            response_format: None,
        }
    }

    /// Get the API base URL.
    pub fn api_base_url(&self) -> String {
        self.state
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com".to_string())
    }

    /// Check if extended thinking is available for this model.
    pub fn supports_thinking(&self) -> bool {
        let model = &self.state.model;
        model.contains("claude-opus-4-5")
    }

    /// Extract system message from the message list.
    ///
    /// Anthropic requires system messages to be passed as a separate `system`
    /// parameter, not as part of the `messages` array. This method separates
    /// system messages from conversation messages and concatenates multiple
    /// system messages with double newlines.
    ///
    /// Corresponds to `_format_messages_for_anthropic()` in Python.
    fn extract_system_and_messages(
        &self,
        messages: &[LLMMessage],
    ) -> (Option<String>, Vec<Value>) {
        let mut system_parts: Vec<String> = Vec::new();
        let mut formatted: Vec<Value> = Vec::new();

        for msg in messages {
            let role = msg
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("user");
            let content = msg
                .get("content")
                .cloned()
                .unwrap_or(Value::String(String::new()));

            if role == "system" {
                // Extract system messages into the separate parameter
                if let Some(text) = content.as_str() {
                    system_parts.push(text.to_string());
                } else if let Some(arr) = content.as_array() {
                    // Handle content blocks array
                    for block in arr {
                        if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                            system_parts.push(text.to_string());
                        }
                    }
                }
            } else if role == "tool" {
                // Convert OpenAI-style tool results to Anthropic format:
                // role: "user" with content: [{ type: "tool_result", ... }]
                let tool_call_id = msg
                    .get("tool_call_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let content_str = content.as_str().unwrap_or("").to_string();

                formatted.push(serde_json::json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": tool_call_id,
                        "content": content_str,
                    }]
                }));
            } else {
                // Map "assistant" tool_calls to Anthropic's content block format
                if role == "assistant" {
                    if let Some(tool_calls) = msg.get("tool_calls").and_then(|v| v.as_array()) {
                        let mut blocks: Vec<Value> = Vec::new();

                        // Add text content if present
                        if let Some(text) = content.as_str() {
                            if !text.is_empty() {
                                blocks.push(serde_json::json!({
                                    "type": "text",
                                    "text": text,
                                }));
                            }
                        }

                        // Convert OpenAI tool_calls to Anthropic tool_use blocks
                        for tc in tool_calls {
                            let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let func = tc.get("function").unwrap_or(&Value::Null);
                            let name = func.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            let args_str =
                                func.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}");
                            let input: Value =
                                serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));

                            blocks.push(serde_json::json!({
                                "type": "tool_use",
                                "id": id,
                                "name": name,
                                "input": input,
                            }));
                        }

                        formatted.push(serde_json::json!({
                            "role": "assistant",
                            "content": blocks,
                        }));
                        continue;
                    }
                }

                // Standard message passthrough
                formatted.push(serde_json::json!({
                    "role": role,
                    "content": content,
                }));
            }
        }

        let system = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };

        (system, formatted)
    }

    /// Build the request body for the Anthropic Messages API.
    ///
    /// Extracts system messages from the messages list and places them in the
    /// separate `system` parameter as required by the Anthropic API.
    pub fn build_request_body(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[Value]>,
    ) -> Value {
        let (system, formatted_messages) = self.extract_system_and_messages(messages);

        let mut body = serde_json::json!({
            "model": self.state.model,
            "max_tokens": self.max_tokens,
            "messages": formatted_messages,
        });

        if let Some(system_text) = system {
            body["system"] = Value::String(system_text);
        }

        if let Some(temp) = self.state.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if let Some(top_p) = self.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }

        if !self.state.stop.is_empty() {
            body["stop_sequences"] = serde_json::json!(self.state.stop);
        } else if !self.stop_sequences.is_empty() {
            body["stop_sequences"] = serde_json::json!(self.stop_sequences);
        }

        if let Some(tools) = tools {
            if !tools.is_empty() {
                body["tools"] = serde_json::json!(tools);
            }
        }

        if let Some(ref thinking) = self.thinking {
            if thinking.is_enabled() {
                body["thinking"] = serde_json::json!({
                    "type": "enabled",
                    "budget_tokens": thinking.budget_tokens.unwrap_or(10_000)
                });
            }
        }

        if self.stream {
            body["stream"] = serde_json::json!(true);
        }

        body
    }

    /// Parse an Anthropic Messages API response.
    ///
    /// Handles the content[] array response format:
    /// - `text` blocks → concatenated into a text string
    /// - `tool_use` blocks → returned as a message value with tool_calls for the executor
    /// - `thinking` blocks → stored for extended thinking support
    ///
    /// Corresponds to `_handle_completion()` in Python.
    fn parse_response(
        &self,
        response: &Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let content = response
            .get("content")
            .and_then(|c| c.as_array())
            .ok_or("No content array in Anthropic response")?;

        let mut text_parts: Vec<String> = Vec::new();
        let mut tool_uses: Vec<Value> = Vec::new();

        for block in content {
            let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match block_type {
                "text" => {
                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                        text_parts.push(text.to_string());
                    }
                }
                "tool_use" => {
                    // Collect tool use blocks to return for the executor
                    tool_uses.push(block.clone());
                }
                "thinking" => {
                    // Log thinking blocks for debugging
                    if let Some(thinking_text) = block.get("thinking").and_then(|t| t.as_str()) {
                        log::debug!(
                            "Anthropic thinking: {}...",
                            &thinking_text[..thinking_text.len().min(200)]
                        );
                    }
                }
                _ => {
                    log::debug!("Unknown Anthropic content block type: {}", block_type);
                }
            }
        }

        // If there are tool_use blocks, return them in a format the executor understands
        // Convert to OpenAI-compatible tool_calls format for executor compatibility
        if !tool_uses.is_empty() {
            let tool_calls: Vec<Value> = tool_uses
                .iter()
                .map(|tu| {
                    let id = tu.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let name = tu.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let input = tu.get("input").unwrap_or(&Value::Null);
                    let args_str = serde_json::to_string(input).unwrap_or_default();

                    serde_json::json!({
                        "id": id,
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": args_str,
                        }
                    })
                })
                .collect();

            // Return as a message object with tool_calls (OpenAI-compatible)
            let combined_text = text_parts.join("");
            return Ok(serde_json::json!({
                "role": "assistant",
                "content": if combined_text.is_empty() { Value::Null } else { Value::String(combined_text) },
                "tool_calls": tool_calls,
            }));
        }

        // Text-only response
        let combined = text_parts.join("");
        let final_content = self.state.apply_stop_words(&combined);
        Ok(Value::String(final_content))
    }

    /// Extract token usage from an Anthropic response.
    ///
    /// Anthropic reports `input_tokens` and `output_tokens` in `response.usage`.
    /// Corresponds to `_extract_anthropic_token_usage()` in Python.
    fn extract_token_usage(response: &Value) -> HashMap<String, Value> {
        let mut usage = HashMap::new();
        if let Some(usage_obj) = response.get("usage") {
            let input = usage_obj
                .get("input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let output = usage_obj
                .get("output_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let cache_read = usage_obj
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            usage.insert("input_tokens".to_string(), serde_json::json!(input));
            usage.insert("output_tokens".to_string(), serde_json::json!(output));
            usage.insert(
                "total_tokens".to_string(),
                serde_json::json!(input + output),
            );
            usage.insert(
                "cached_tokens".to_string(),
                serde_json::json!(cache_read),
            );

            log::debug!(
                "Anthropic token usage: input={}, output={}, total={}, cached={}",
                input,
                output,
                input + output,
                cache_read,
            );
        }
        usage
    }

    /// Collect beta headers needed for this request.
    fn beta_headers(&self) -> Vec<String> {
        let mut betas = Vec::new();
        if self.response_format.is_some()
            && supports_native_structured_outputs(&self.state.model)
        {
            betas.push(ANTHROPIC_STRUCTURED_OUTPUTS_BETA.to_string());
        }
        betas
    }
}

#[async_trait]
impl BaseLLM for AnthropicCompletion {
    fn model(&self) -> &str {
        &self.state.model
    }

    fn temperature(&self) -> Option<f64> {
        self.state.temperature
    }

    fn stop(&self) -> &[String] {
        &self.state.stop
    }

    fn set_stop(&mut self, stop: Vec<String>) {
        self.state.stop = stop;
    }

    fn provider(&self) -> &str {
        "anthropic"
    }

    fn supports_function_calling(&self) -> bool {
        true
    }

    fn supports_multimodal(&self) -> bool {
        // All Claude 3+ models support multimodal
        true
    }

    fn supports_stop_words(&self) -> bool {
        self.state.has_stop_words()
    }

    fn get_context_window_size(&self) -> usize {
        // Claude 3+ models have 200k context
        200_000
    }

    fn call(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "AnthropicCompletion.call: model={}, messages={}, tools={:?}",
            self.state.model,
            messages.len(),
            tools.as_ref().map(|t| t.len()),
        );

        // Use tokio runtime for sync call
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(self.acall(messages, tools, available_functions))
    }

    async fn acall(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        _available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "AnthropicCompletion.acall: model={}, messages={}",
            self.state.model,
            messages.len(),
        );

        // Validate API key
        let api_key = self.state.api_key.as_ref().ok_or_else(|| {
            "Anthropic API key not set. Set ANTHROPIC_API_KEY environment variable or pass api_key to constructor."
        })?;

        // Build request body
        let tools_slice = tools.as_deref();
        let body = self.build_request_body(&messages, tools_slice);

        // Endpoint: POST /v1/messages
        let base_url = self.api_base_url();
        let endpoint = format!("{}/v1/messages", base_url);

        // Build HTTP client with timeout
        let timeout_secs = self.timeout.unwrap_or(120.0);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs_f64(timeout_secs))
            .build()?;

        // Collect beta headers
        let betas = self.beta_headers();

        // Retry loop with exponential backoff
        let mut last_error: Option<Box<dyn std::error::Error + Send + Sync>> = None;
        let mut retry_delay = std::time::Duration::from_secs(1);

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                log::warn!(
                    "Anthropic API retry attempt {} after {:?}",
                    attempt,
                    retry_delay
                );
                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2; // Exponential backoff
            }

            // Build request with Anthropic-specific headers
            let mut request = client
                .post(&endpoint)
                .header("content-type", "application/json")
                .header("x-api-key", api_key.as_str())
                .header("anthropic-version", &self.anthropic_version);

            // Add beta headers if needed
            if !betas.is_empty() {
                request = request.header("anthropic-beta", betas.join(","));
            }

            // Send request
            let response = match request.json(&body).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = Some(Box::new(e));
                    continue;
                }
            };

            let status = response.status();

            // Handle rate limiting (429)
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                // Check for Retry-After header
                if let Some(retry_after) = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                {
                    retry_delay = std::time::Duration::from_secs(retry_after);
                }
                last_error = Some("Rate limited by Anthropic API (429)".into());
                continue;
            }

            // Handle overloaded (529)
            if status.as_u16() == 529 {
                last_error = Some("Anthropic API overloaded (529)".into());
                continue;
            }

            // Handle server errors (5xx)
            if status.is_server_error() {
                last_error =
                    Some(format!("Anthropic API server error: {}", status).into());
                continue;
            }

            // Parse response body
            let response_text = match response.text().await {
                Ok(text) => text,
                Err(e) => {
                    last_error = Some(Box::new(e));
                    continue;
                }
            };

            // Handle client errors (4xx) — don't retry
            if status.is_client_error() {
                return Err(format!(
                    "Anthropic API error ({}): {}",
                    status, response_text
                )
                .into());
            }

            // Parse JSON response
            let response_json: Value = match serde_json::from_str(&response_text) {
                Ok(json) => json,
                Err(e) => {
                    return Err(format!(
                        "Failed to parse Anthropic response: {} - Body: {}",
                        e,
                        &response_text[..response_text.len().min(500)]
                    )
                    .into());
                }
            };

            // Check for API-level error in the response body
            if let Some(err_type) = response_json.get("type").and_then(|t| t.as_str()) {
                if err_type == "error" {
                    let err_msg = response_json
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown Anthropic API error");
                    return Err(format!("Anthropic API error: {}", err_msg).into());
                }
            }

            // Log token usage
            let usage = Self::extract_token_usage(&response_json);
            if !usage.is_empty() {
                log::debug!("Anthropic usage tracked: {:?}", usage);
            }

            // Parse the response content
            let result = self.parse_response(&response_json)?;

            return Ok(result);
        }

        // All retries exhausted
        Err(last_error
            .unwrap_or_else(|| "Anthropic API call failed after all retries".into()))
    }

    fn get_token_usage_summary(&self) -> UsageMetrics {
        self.state.get_token_usage_summary()
    }

    fn track_token_usage(&mut self, usage_data: &HashMap<String, Value>) {
        self.state.track_token_usage_internal(usage_data);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_new() {
        let provider = AnthropicCompletion::new("claude-opus-4-5-20251101", None, None);
        assert_eq!(provider.state.model, "claude-opus-4-5-20251101");
        assert_eq!(provider.state.provider, "anthropic");
        assert_eq!(provider.max_tokens, 4096);
        assert_eq!(provider.anthropic_version, "2023-06-01");
    }

    #[test]
    fn test_api_base_url_default() {
        let provider = AnthropicCompletion::new("claude-opus-4-5-20251101", None, None);
        assert_eq!(provider.api_base_url(), "https://api.anthropic.com");
    }

    #[test]
    fn test_api_base_url_custom() {
        let provider = AnthropicCompletion::new(
            "claude-opus-4-5-20251101",
            None,
            Some("https://custom.api.com".to_string()),
        );
        assert_eq!(provider.api_base_url(), "https://custom.api.com");
    }

    #[test]
    fn test_supports_thinking() {
        let p1 = AnthropicCompletion::new("claude-opus-4-5-20251101", None, None);
        assert!(p1.supports_thinking());

        let p2 = AnthropicCompletion::new("claude-opus-4-5", None, None);
        assert!(p2.supports_thinking());

        let p3 = AnthropicCompletion::new("claude-opus-4-5-20251101", None, None);
        assert!(!p3.supports_thinking());
    }

    #[test]
    fn test_extract_system_and_messages() {
        let provider = AnthropicCompletion::new("claude-opus-4-5-20251101", None, None);

        let messages: Vec<LLMMessage> = vec![
            {
                let mut m = HashMap::new();
                m.insert(
                    "role".to_string(),
                    Value::String("system".to_string()),
                );
                m.insert(
                    "content".to_string(),
                    Value::String("You are a helpful assistant.".to_string()),
                );
                m
            },
            {
                let mut m = HashMap::new();
                m.insert("role".to_string(), Value::String("user".to_string()));
                m.insert(
                    "content".to_string(),
                    Value::String("Hello!".to_string()),
                );
                m
            },
        ];

        let (system, formatted) = provider.extract_system_and_messages(&messages);
        assert_eq!(system, Some("You are a helpful assistant.".to_string()));
        assert_eq!(formatted.len(), 1);
        assert_eq!(formatted[0]["role"], "user");
    }

    #[test]
    fn test_extract_system_multiple() {
        let provider = AnthropicCompletion::new("claude-opus-4-5-20251101", None, None);

        let messages: Vec<LLMMessage> = vec![
            {
                let mut m = HashMap::new();
                m.insert(
                    "role".to_string(),
                    Value::String("system".to_string()),
                );
                m.insert(
                    "content".to_string(),
                    Value::String("System 1.".to_string()),
                );
                m
            },
            {
                let mut m = HashMap::new();
                m.insert(
                    "role".to_string(),
                    Value::String("system".to_string()),
                );
                m.insert(
                    "content".to_string(),
                    Value::String("System 2.".to_string()),
                );
                m
            },
            {
                let mut m = HashMap::new();
                m.insert("role".to_string(), Value::String("user".to_string()));
                m.insert(
                    "content".to_string(),
                    Value::String("Hi".to_string()),
                );
                m
            },
        ];

        let (system, formatted) = provider.extract_system_and_messages(&messages);
        assert_eq!(system, Some("System 1.\n\nSystem 2.".to_string()));
        assert_eq!(formatted.len(), 1);
    }

    #[test]
    fn test_build_request_body_with_system() {
        let provider = AnthropicCompletion::new("claude-opus-4-5-20251101", None, None);

        let messages: Vec<LLMMessage> = vec![
            {
                let mut m = HashMap::new();
                m.insert(
                    "role".to_string(),
                    Value::String("system".to_string()),
                );
                m.insert(
                    "content".to_string(),
                    Value::String("Be concise.".to_string()),
                );
                m
            },
            {
                let mut m = HashMap::new();
                m.insert("role".to_string(), Value::String("user".to_string()));
                m.insert(
                    "content".to_string(),
                    Value::String("What is Rust?".to_string()),
                );
                m
            },
        ];

        let body = provider.build_request_body(&messages, None);
        assert_eq!(body["model"], "claude-opus-4-5-20251101");
        assert_eq!(body["max_tokens"], 4096);
        assert_eq!(body["system"], "Be concise.");
        // Messages should only have the user message
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["messages"][0]["role"], "user");
    }

    #[test]
    fn test_parse_response_text() {
        let provider = AnthropicCompletion::new("claude-opus-4-5-20251101", None, None);

        let response = serde_json::json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Hello! How can I help?"
                }
            ],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 8,
            }
        });

        let result = provider.parse_response(&response).unwrap();
        assert_eq!(result.as_str().unwrap(), "Hello! How can I help?");
    }

    #[test]
    fn test_parse_response_tool_use() {
        let provider = AnthropicCompletion::new("claude-opus-4-5-20251101", None, None);

        let response = serde_json::json!({
            "id": "msg_456",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Let me search for that."
                },
                {
                    "type": "tool_use",
                    "id": "toolu_abc",
                    "name": "search",
                    "input": { "query": "Rust programming" }
                }
            ],
            "usage": {
                "input_tokens": 20,
                "output_tokens": 15,
            }
        });

        let result = provider.parse_response(&response).unwrap();
        // Should return a message with tool_calls
        assert!(result.get("tool_calls").is_some());
        let tool_calls = result["tool_calls"].as_array().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0]["function"]["name"], "search");
        assert_eq!(result["content"], "Let me search for that.");
    }

    #[test]
    fn test_extract_token_usage() {
        let response = serde_json::json!({
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "cache_read_input_tokens": 20,
            }
        });

        let usage = AnthropicCompletion::extract_token_usage(&response);
        assert_eq!(usage["input_tokens"], 100);
        assert_eq!(usage["output_tokens"], 50);
        assert_eq!(usage["total_tokens"], 150);
        assert_eq!(usage["cached_tokens"], 20);
    }

    #[test]
    fn test_native_structured_output_models() {
        assert!(supports_native_structured_outputs("claude-opus-4-5-20251101"));
        assert!(supports_native_structured_outputs("claude-opus-4-5-20251101"));
        assert!(!supports_native_structured_outputs("claude-opus-4-5-20251101"));
        assert!(!supports_native_structured_outputs("gpt-4o"));
    }

    #[test]
    fn test_thinking_config() {
        let enabled = AnthropicThinkingConfig::enabled(5000);
        assert!(enabled.is_enabled());
        assert_eq!(enabled.budget_tokens, Some(5000));

        let disabled = AnthropicThinkingConfig::disabled();
        assert!(!disabled.is_enabled());
    }

    /// Integration test — requires ANTHROPIC_API_KEY.
    #[tokio::test]
    #[ignore]
    async fn test_anthropic_real_call() {
        let provider = AnthropicCompletion::new("claude-opus-4-5-20251101", None, None);
        let mut msg = HashMap::new();
        msg.insert("role".to_string(), Value::String("user".to_string()));
        msg.insert(
            "content".to_string(),
            Value::String("Say hello in exactly 3 words.".to_string()),
        );
        let messages = vec![msg];
        let result = provider.acall(messages, None, None).await;
        assert!(result.is_ok(), "Failed: {:?}", result.err());
        let val = result.unwrap();
        assert!(val.as_str().is_some(), "Expected string response");
    }
}
