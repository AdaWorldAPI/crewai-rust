//! xAI / Grok native completion provider.
//!
//! Provides direct integration with the xAI API, which is OpenAI-compatible
//! at `https://api.x.ai/v1`. Supports Grok models (grok-3, grok-3-mini,
//! grok-2, grok-3-fast) with native function calling, streaming, and
//! live search grounding.
//!
//! # Features
//!
//! - OpenAI-compatible Chat Completions API via `reqwest`
//! - Retry with exponential backoff on 429/5xx
//! - Native tool use (function calling)
//! - Live search grounding (xAI-specific)
//! - Deferred reasoning support (grok-3)
//! - Token usage tracking
//!
//! # Environment Variables
//!
//! - `XAI_API_KEY` — xAI API key (required)
//! - `XAI_BASE_URL` — Custom base URL (defaults to `https://api.x.ai/v1`)

use std::any::Any;
use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::llms::base_llm::{BaseLLM, BaseLLMState, LLMMessage};
use crate::types::usage_metrics::UsageMetrics;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default xAI API base URL.
pub const XAI_DEFAULT_BASE_URL: &str = "https://api.x.ai/v1";

// ---------------------------------------------------------------------------
// XAICompletion provider
// ---------------------------------------------------------------------------

/// xAI / Grok native completion implementation.
///
/// Provides direct integration with the xAI API via `reqwest`. The xAI API
/// is OpenAI-compatible, so this provider reuses the Chat Completions
/// request/response format with xAI-specific extensions.
///
/// # Supported Models
///
/// - `grok-3` — Flagship reasoning model (131k context)
/// - `grok-3-mini` — Lightweight reasoning model (131k context)
/// - `grok-3-fast` — Low-latency model (131k context)
/// - `grok-2` — Previous generation (131k context)
/// - `grok-2-vision` — Multimodal with vision (32k context)
///
/// # Example
///
/// ```ignore
/// let provider = XAICompletion::new("grok-3-mini", None, None);
/// let messages = vec![/* ... */];
/// let response = provider.call(messages, None, None)?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XAICompletion {
    /// Shared base LLM state.
    #[serde(flatten)]
    pub state: BaseLLMState,

    /// Request timeout in seconds.
    pub timeout: Option<f64>,
    /// Maximum number of retries.
    pub max_retries: u32,
    /// Nucleus sampling parameter.
    pub top_p: Option<f64>,
    /// Frequency penalty (-2 to 2).
    pub frequency_penalty: Option<f64>,
    /// Presence penalty (-2 to 2).
    pub presence_penalty: Option<f64>,
    /// Maximum tokens in response.
    pub max_tokens: Option<u32>,
    /// Seed for deterministic generation.
    pub seed: Option<i64>,
    /// Whether to use streaming responses.
    pub stream: bool,
    /// Response format (e.g., `{"type": "json_object"}`).
    pub response_format: Option<Value>,
    /// Reasoning effort for grok-3 (low/medium/high).
    pub reasoning_effort: Option<String>,
    /// Enable live search grounding (xAI-specific).
    pub search: Option<bool>,
}

impl XAICompletion {
    /// Create a new xAI completion provider.
    ///
    /// # Arguments
    ///
    /// * `model` - xAI model name (e.g., "grok-3-mini", "grok-3").
    /// * `api_key` - Optional API key (defaults to XAI_API_KEY env var).
    /// * `base_url` - Optional custom base URL (defaults to `https://api.x.ai/v1`).
    pub fn new(
        model: impl Into<String>,
        api_key: Option<String>,
        base_url: Option<String>,
    ) -> Self {
        let api_key = api_key.or_else(|| std::env::var("XAI_API_KEY").ok());

        let mut state = BaseLLMState::new(model);
        state.api_key = api_key;
        state.base_url = base_url;
        state.provider = "xai".to_string();

        Self {
            state,
            timeout: None,
            max_retries: 2,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            max_tokens: None,
            seed: None,
            stream: false,
            response_format: None,
            reasoning_effort: None,
            search: None,
        }
    }

    /// Get the API base URL.
    pub fn api_base_url(&self) -> String {
        self.state
            .base_url
            .clone()
            .unwrap_or_else(|| XAI_DEFAULT_BASE_URL.to_string())
    }

    /// Check if the model is a reasoning model (supports reasoning_effort).
    pub fn is_reasoning_model(&self) -> bool {
        let m = self.state.model.to_lowercase();
        m.contains("grok-3") && !m.contains("fast")
    }

    /// Build the request body for the xAI Chat Completions API.
    ///
    /// The xAI API is OpenAI-compatible with additional parameters:
    /// - `search`: enable live web search grounding
    /// - `reasoning_effort`: control thinking depth for grok-3
    pub fn build_request_body(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[Value]>,
    ) -> Value {
        let mut body = serde_json::json!({
            "model": self.state.model,
            "messages": messages,
        });

        if let Some(temp) = self.state.temperature {
            // Reasoning models don't support temperature
            if !self.is_reasoning_model() {
                body["temperature"] = serde_json::json!(temp);
            }
        }

        if let Some(max_tokens) = self.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }

        if let Some(top_p) = self.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }

        if let Some(freq_pen) = self.frequency_penalty {
            body["frequency_penalty"] = serde_json::json!(freq_pen);
        }

        if let Some(pres_pen) = self.presence_penalty {
            body["presence_penalty"] = serde_json::json!(pres_pen);
        }

        if !self.state.stop.is_empty() {
            body["stop"] = serde_json::json!(self.state.stop);
        }

        if let Some(ref format) = self.response_format {
            body["response_format"] = format.clone();
        }

        if let Some(seed) = self.seed {
            body["seed"] = serde_json::json!(seed);
        }

        if let Some(ref effort) = self.reasoning_effort {
            if self.is_reasoning_model() {
                body["reasoning_effort"] = serde_json::json!(effort);
            }
        }

        if self.stream {
            body["stream"] = serde_json::json!(true);
        }

        // xAI-specific: live search grounding
        if let Some(search) = self.search {
            if search {
                body["search"] = serde_json::json!(true);
            }
        }

        if let Some(tools) = tools {
            if !tools.is_empty() {
                body["tools"] = serde_json::json!(tools);
                body["tool_choice"] = serde_json::json!("auto");
            }
        }

        body
    }

    /// Parse a Chat Completions API response (OpenAI-compatible format).
    fn parse_response(
        &self,
        response: &Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let choice = response
            .get("choices")
            .and_then(|c| c.get(0))
            .ok_or("No choices in xAI response")?;

        let message = choice
            .get("message")
            .ok_or("No message in xAI choice")?;

        // Check for tool calls
        if let Some(tool_calls) = message.get("tool_calls") {
            if tool_calls.is_array() && !tool_calls.as_array().unwrap().is_empty() {
                return Ok(message.clone());
            }
        }

        // Extract text content
        let content = message
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("");

        let final_content = self.state.apply_stop_words(content);

        // Log token usage
        if let Some(usage) = response.get("usage") {
            log::debug!(
                "xAI token usage: prompt={}, completion={}, total={}",
                usage.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
                usage.get("completion_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
                usage.get("total_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
            );
        }

        Ok(Value::String(final_content))
    }
}

#[async_trait]
impl BaseLLM for XAICompletion {
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
        "xai"
    }

    fn supports_function_calling(&self) -> bool {
        true
    }

    fn supports_multimodal(&self) -> bool {
        let lower = self.state.model.to_lowercase();
        lower.contains("vision")
    }

    fn supports_stop_words(&self) -> bool {
        self.state.has_stop_words()
    }

    fn get_context_window_size(&self) -> usize {
        let model = &self.state.model;
        if model.contains("grok-2-vision") {
            32_768
        } else {
            // grok-3, grok-3-mini, grok-3-fast, grok-2 all have 131k
            131_072
        }
    }

    fn call(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "XAICompletion.call: model={}, messages={}, tools={:?}",
            self.state.model,
            messages.len(),
            tools.as_ref().map(|t| t.len()),
        );

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
            "XAICompletion.acall: model={}, messages={}",
            self.state.model,
            messages.len(),
        );

        // Validate API key
        let api_key = self.state.api_key.as_ref().ok_or_else(|| {
            "xAI API key not set. Set XAI_API_KEY environment variable or pass api_key to constructor."
        })?;

        // Build request body
        let tools_slice = tools.as_deref();
        let body = self.build_request_body(&messages, tools_slice);

        // Endpoint: POST /chat/completions (OpenAI-compatible)
        let base_url = self.api_base_url();
        let endpoint = format!("{}/chat/completions", base_url);

        // Build HTTP client
        let timeout_secs = self.timeout.unwrap_or(120.0);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs_f64(timeout_secs))
            .build()?;

        // Retry loop with exponential backoff
        let mut last_error: Option<Box<dyn std::error::Error + Send + Sync>> = None;
        let mut retry_delay = std::time::Duration::from_secs(1);

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                log::warn!(
                    "xAI API retry attempt {} after {:?}",
                    attempt,
                    retry_delay
                );
                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2;
            }

            let request = client
                .post(&endpoint)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", api_key));

            let response = match request.json(&body).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = Some(Box::new(e));
                    continue;
                }
            };

            let status = response.status();

            // Rate limiting
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                last_error = Some("Rate limited by xAI API (429)".into());
                continue;
            }

            // Server errors
            if status.is_server_error() {
                last_error =
                    Some(format!("xAI API server error: {}", status).into());
                continue;
            }

            let response_text = match response.text().await {
                Ok(text) => text,
                Err(e) => {
                    last_error = Some(Box::new(e));
                    continue;
                }
            };

            // Client errors — don't retry
            if status.is_client_error() {
                return Err(format!(
                    "xAI API error ({}): {}",
                    status, response_text
                )
                .into());
            }

            // Parse JSON
            let response_json: Value = match serde_json::from_str(&response_text) {
                Ok(json) => json,
                Err(e) => {
                    return Err(format!(
                        "Failed to parse xAI response: {} - Body: {}",
                        e,
                        &response_text[..response_text.len().min(500)]
                    )
                    .into());
                }
            };

            // Check for error in response body
            if let Some(err) = response_json.get("error") {
                let msg = err
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown xAI API error");
                return Err(format!("xAI API error: {}", msg).into());
            }

            let result = self.parse_response(&response_json)?;
            return Ok(result);
        }

        Err(last_error
            .unwrap_or_else(|| "xAI API call failed after all retries".into()))
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
    fn test_xai_new() {
        let provider = XAICompletion::new("grok-3-mini", None, None);
        assert_eq!(provider.state.model, "grok-3-mini");
        assert_eq!(provider.state.provider, "xai");
        assert_eq!(provider.max_retries, 2);
    }

    #[test]
    fn test_api_base_url_default() {
        let provider = XAICompletion::new("grok-3-mini", None, None);
        assert_eq!(provider.api_base_url(), "https://api.x.ai/v1");
    }

    #[test]
    fn test_api_base_url_custom() {
        let provider = XAICompletion::new(
            "grok-3",
            None,
            Some("https://custom.api.com/v1".to_string()),
        );
        assert_eq!(provider.api_base_url(), "https://custom.api.com/v1");
    }

    #[test]
    fn test_is_reasoning_model() {
        let grok3 = XAICompletion::new("grok-3", None, None);
        assert!(grok3.is_reasoning_model());

        let grok3_mini = XAICompletion::new("grok-3-mini", None, None);
        assert!(grok3_mini.is_reasoning_model());

        let grok3_fast = XAICompletion::new("grok-3-fast", None, None);
        assert!(!grok3_fast.is_reasoning_model());

        let grok2 = XAICompletion::new("grok-2", None, None);
        assert!(!grok2.is_reasoning_model());
    }

    #[test]
    fn test_context_window() {
        let grok3 = XAICompletion::new("grok-3", None, None);
        assert_eq!(grok3.get_context_window_size(), 131_072);

        let vision = XAICompletion::new("grok-2-vision", None, None);
        assert_eq!(vision.get_context_window_size(), 32_768);
    }

    #[test]
    fn test_supports_multimodal() {
        let grok3 = XAICompletion::new("grok-3", None, None);
        assert!(!grok3.supports_multimodal());

        let vision = XAICompletion::new("grok-2-vision", None, None);
        assert!(vision.supports_multimodal());
    }

    #[test]
    fn test_build_request_body_basic() {
        let provider = XAICompletion::new("grok-3-mini", None, None);

        let mut msg = HashMap::new();
        msg.insert("role".to_string(), Value::String("user".to_string()));
        msg.insert(
            "content".to_string(),
            Value::String("Hello".to_string()),
        );
        let messages = vec![msg];

        let body = provider.build_request_body(&messages, None);
        assert_eq!(body["model"], "grok-3-mini");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        // No tools, no temperature set
        assert!(body.get("tools").is_none());
    }

    #[test]
    fn test_build_request_body_with_search() {
        let mut provider = XAICompletion::new("grok-3-mini", None, None);
        provider.search = Some(true);

        let messages: Vec<LLMMessage> = vec![];
        let body = provider.build_request_body(&messages, None);
        assert_eq!(body["search"], true);
    }

    #[test]
    fn test_build_request_body_reasoning() {
        let mut provider = XAICompletion::new("grok-3", None, None);
        provider.state.temperature = Some(0.5);
        provider.reasoning_effort = Some("high".to_string());

        let messages: Vec<LLMMessage> = vec![];
        let body = provider.build_request_body(&messages, None);

        // Reasoning models don't use temperature
        assert!(body.get("temperature").is_none());
        // But do use reasoning_effort
        assert_eq!(body["reasoning_effort"], "high");
    }

    #[test]
    fn test_parse_response_text() {
        let provider = XAICompletion::new("grok-3-mini", None, None);

        let response = serde_json::json!({
            "id": "chatcmpl-abc",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! I'm Grok."
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15,
            }
        });

        let result = provider.parse_response(&response).unwrap();
        assert_eq!(result.as_str().unwrap(), "Hello! I'm Grok.");
    }

    #[test]
    fn test_parse_response_tool_calls() {
        let provider = XAICompletion::new("grok-3-mini", None, None);

        let response = serde_json::json!({
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "search",
                            "arguments": "{\"query\": \"weather\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });

        let result = provider.parse_response(&response).unwrap();
        assert!(result.get("tool_calls").is_some());
    }

    /// Integration test — requires XAI_API_KEY.
    #[tokio::test]
    #[ignore]
    async fn test_xai_real_call() {
        let provider = XAICompletion::new("grok-3-mini", None, None);
        let mut msg = HashMap::new();
        msg.insert("role".to_string(), Value::String("user".to_string()));
        msg.insert(
            "content".to_string(),
            Value::String("Say hello in exactly 3 words.".to_string()),
        );
        let result = provider.acall(vec![msg], None, None).await;
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }
}
