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
//! - Anthropic Messages API
//! - Streaming support
//! - Native tool use (function calling)
//! - Extended thinking / chain-of-thought (budget_tokens)
//! - Structured output via tool-based approach and native beta
//! - System message extraction from message list
//! - Files API beta support
//! - HTTP interceptor support
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
    "claude-sonnet-4-5",
    "claude-sonnet-4.5",
    "claude-opus-4-5",
    "claude-opus-4.5",
    "claude-opus-4-1",
    "claude-opus-4.1",
    "claude-haiku-4-5",
    "claude-haiku-4.5",
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
/// Provides direct integration with the Anthropic Messages API.
///
/// Corresponds to `AnthropicCompletion(BaseLLM)` in Python.
///
/// # Example
///
/// ```ignore
/// let provider = AnthropicCompletion::new(
///     "claude-3-5-sonnet-20241022",
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
    /// * `model` - Anthropic model name (e.g., "claude-3-5-sonnet-20241022").
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

    /// Check if extended thinking is available for this model.
    pub fn supports_thinking(&self) -> bool {
        let model = &self.state.model;
        model.contains("claude-3-5")
            || model.contains("claude-3-7")
            || model.contains("claude-sonnet-4")
            || model.contains("claude-opus-4")
            || model.contains("claude-haiku-4")
    }

    /// Build the request body for the Anthropic Messages API.
    pub fn build_request_body(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[Value]>,
    ) -> Value {
        let mut body = serde_json::json!({
            "model": self.state.model,
            "max_tokens": self.max_tokens,
            "messages": messages,
        });

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
        _available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "AnthropicCompletion.call: model={}, messages={}, tools={:?}",
            self.state.model,
            messages.len(),
            tools.as_ref().map(|t| t.len()),
        );

        Err("AnthropicCompletion.call is a stub - HTTP client not yet implemented".into())
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
        let _ = tools;

        Err(
            "AnthropicCompletion.acall is a stub - async HTTP client not yet implemented"
                .into(),
        )
    }

    fn get_token_usage_summary(&self) -> UsageMetrics {
        self.state.get_token_usage_summary()
    }

    fn track_token_usage(&mut self, usage_data: &HashMap<String, Value>) {
        self.state.track_token_usage_internal(usage_data);
    }
}
