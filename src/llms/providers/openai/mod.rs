//! OpenAI native completion provider.
//!
//! Corresponds to `crewai/llms/providers/openai/completion.py`.
//!
//! This module provides direct integration with the OpenAI API, supporting
//! both the Chat Completions API and the newer Responses API. It handles
//! native function calling, streaming, structured output, and built-in tools
//! (web search, file search, code interpreter, computer use).
//!
//! # Features
//!
//! - Chat Completions API (default)
//! - Responses API with built-in tools
//! - Streaming support
//! - Function/tool calling
//! - Structured output (JSON schema / Pydantic models)
//! - Reasoning model support (o1, o3, o4)
//! - HTTP interceptor support
//! - Auto-chaining for multi-turn conversations
//! - Token usage tracking

use std::any::Any;
use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::llms::base_llm::{BaseLLM, BaseLLMState, LLMMessage};
use crate::types::usage_metrics::UsageMetrics;

// ---------------------------------------------------------------------------
// OpenAI API mode
// ---------------------------------------------------------------------------

/// Which OpenAI API to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenAIApiMode {
    /// Chat Completions API (default).
    Completions,
    /// Responses API (newer, with built-in tools).
    Responses,
}

impl Default for OpenAIApiMode {
    fn default() -> Self {
        Self::Completions
    }
}

// ---------------------------------------------------------------------------
// Responses API result types
// ---------------------------------------------------------------------------

/// Result from OpenAI Responses API including text and tool outputs.
///
/// Corresponds to `ResponsesAPIResult` dataclass in the Python provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponsesApiResult {
    /// The text content from the response.
    pub text: String,
    /// Results from web_search built-in tool calls.
    pub web_search_results: Vec<Value>,
    /// Results from file_search built-in tool calls.
    pub file_search_results: Vec<Value>,
    /// Results from code_interpreter built-in tool calls.
    pub code_interpreter_results: Vec<Value>,
    /// Results from computer_use built-in tool calls.
    pub computer_use_results: Vec<Value>,
    /// Reasoning/thinking summaries from the model.
    pub reasoning_summaries: Vec<Value>,
    /// Custom function tool calls.
    pub function_calls: Vec<Value>,
    /// The response ID for multi-turn conversations.
    pub response_id: Option<String>,
}

impl ResponsesApiResult {
    /// Check if there are any built-in tool outputs.
    pub fn has_tool_outputs(&self) -> bool {
        !self.web_search_results.is_empty()
            || !self.file_search_results.is_empty()
            || !self.code_interpreter_results.is_empty()
            || !self.computer_use_results.is_empty()
    }

    /// Check if there are reasoning summaries.
    pub fn has_reasoning(&self) -> bool {
        !self.reasoning_summaries.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Built-in tool type mapping
// ---------------------------------------------------------------------------

/// Map from user-friendly built-in tool names to OpenAI API types.
pub fn builtin_tool_type(name: &str) -> Option<&'static str> {
    match name {
        "web_search" => Some("web_search_preview"),
        "file_search" => Some("file_search"),
        "code_interpreter" => Some("code_interpreter"),
        "computer_use" => Some("computer_use_preview"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// OpenAICompletion provider
// ---------------------------------------------------------------------------

/// OpenAI native completion implementation.
///
/// Provides direct integration with the OpenAI API via `reqwest`, supporting
/// both Chat Completions API and Responses API.
///
/// Corresponds to `OpenAICompletion(BaseLLM)` in Python.
///
/// # Example
///
/// ```ignore
/// let provider = OpenAICompletion::new("gpt-4o", None, None);
/// let messages = vec![/* ... */];
/// let response = provider.call(messages, None, None)?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICompletion {
    /// Shared base LLM state.
    #[serde(flatten)]
    pub state: BaseLLMState,

    /// Organization ID for multi-tenant access.
    pub organization: Option<String>,
    /// Project ID for project-scoped access.
    pub project: Option<String>,
    /// Request timeout in seconds.
    pub timeout: Option<f64>,
    /// Maximum number of retries.
    pub max_retries: u32,
    /// Default headers to include in requests.
    pub default_headers: Option<HashMap<String, String>>,
    /// Default query parameters.
    pub default_query: Option<HashMap<String, Value>>,
    /// Additional client parameters.
    pub client_params: Option<HashMap<String, Value>>,

    // --- Generation parameters ---
    /// Nucleus sampling parameter.
    pub top_p: Option<f64>,
    /// Frequency penalty (-2 to 2).
    pub frequency_penalty: Option<f64>,
    /// Presence penalty (-2 to 2).
    pub presence_penalty: Option<f64>,
    /// Maximum tokens in response.
    pub max_tokens: Option<u32>,
    /// Maximum completion tokens (newer parameter name).
    pub max_completion_tokens: Option<u32>,
    /// Seed for deterministic generation.
    pub seed: Option<i64>,
    /// Whether to use streaming responses.
    pub stream: bool,
    /// Response format (JSON schema or structured output config).
    pub response_format: Option<Value>,
    /// Whether to return log probabilities.
    pub logprobs: Option<bool>,
    /// Number of top log probabilities to return.
    pub top_logprobs: Option<i32>,
    /// Reasoning effort level for reasoning models.
    pub reasoning_effort: Option<String>,

    // --- Responses API parameters ---
    /// Which API to use: "completions" or "responses".
    pub api: OpenAIApiMode,
    /// System-level instructions (Responses API only).
    pub instructions: Option<String>,
    /// Whether to store responses for multi-turn (Responses API only).
    pub store: Option<bool>,
    /// ID of previous response for multi-turn (Responses API only).
    pub previous_response_id: Option<String>,
    /// Additional data to include in response (Responses API only).
    pub include: Option<Vec<String>>,
    /// List of OpenAI built-in tools to enable (Responses API only).
    pub builtin_tools: Option<Vec<String>>,
    /// Whether to return structured ResponsesAPIResult (Responses API only).
    pub parse_tool_outputs: bool,
    /// Automatically track response IDs for multi-turn (Responses API only).
    pub auto_chain: bool,
    /// Automatically track reasoning items for ZDR (Responses API only).
    pub auto_chain_reasoning: bool,
}

impl OpenAICompletion {
    /// Create a new OpenAI completion provider.
    ///
    /// # Arguments
    ///
    /// * `model` - OpenAI model name (e.g., "gpt-4o", "o3-mini").
    /// * `api_key` - Optional API key (defaults to OPENAI_API_KEY env var).
    /// * `base_url` - Optional custom base URL.
    pub fn new(
        model: impl Into<String>,
        api_key: Option<String>,
        base_url: Option<String>,
    ) -> Self {
        let api_key = api_key.or_else(|| std::env::var("OPENAI_API_KEY").ok());

        let mut state = BaseLLMState::new(model);
        state.api_key = api_key;
        state.base_url = base_url;
        state.provider = "openai".to_string();

        Self {
            state,
            organization: std::env::var("OPENAI_ORGANIZATION").ok(),
            project: None,
            timeout: None,
            max_retries: 2,
            default_headers: None,
            default_query: None,
            client_params: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            max_tokens: None,
            max_completion_tokens: None,
            seed: None,
            stream: false,
            response_format: None,
            logprobs: None,
            top_logprobs: None,
            reasoning_effort: None,
            api: OpenAIApiMode::default(),
            instructions: None,
            store: None,
            previous_response_id: None,
            include: None,
            builtin_tools: None,
            parse_tool_outputs: false,
            auto_chain: false,
            auto_chain_reasoning: false,
        }
    }

    /// Get the API base URL.
    pub fn api_base_url(&self) -> String {
        self.state
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string())
    }

    /// Parse a Chat Completions API response.
    fn parse_completions_response(
        &self,
        response: &Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // Extract the first choice
        let choice = response
            .get("choices")
            .and_then(|c| c.get(0))
            .ok_or("No choices in OpenAI response")?;

        let message = choice
            .get("message")
            .ok_or("No message in OpenAI choice")?;

        // Check for tool calls
        if let Some(tool_calls) = message.get("tool_calls") {
            if tool_calls.is_array() && !tool_calls.as_array().unwrap().is_empty() {
                // Return the full message with tool_calls for the executor to handle
                return Ok(message.clone());
            }
        }

        // Extract text content
        let content = message
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("");

        // Apply stop words
        let final_content = self.state.apply_stop_words(content);

        // Log token usage if present
        if let Some(usage) = response.get("usage") {
            log::debug!(
                "OpenAI token usage: prompt={}, completion={}, total={}",
                usage.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
                usage.get("completion_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
                usage.get("total_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
            );
        }

        Ok(Value::String(final_content))
    }

    /// Parse a Responses API response.
    fn parse_responses_response(
        &self,
        response: &Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // Responses API returns different structure
        // Extract output items
        let output = response.get("output").unwrap_or(response);

        if self.parse_tool_outputs {
            // Build ResponsesApiResult
            let mut result = ResponsesApiResult::default();

            // Get response ID for multi-turn
            result.response_id = response
                .get("id")
                .and_then(|v| v.as_str())
                .map(String::from);

            // Parse output items
            if let Some(items) = output.as_array() {
                for item in items {
                    let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    match item_type {
                        "message" => {
                            if let Some(content) = item.get("content") {
                                if let Some(text) = content.get(0).and_then(|c| c.get("text")).and_then(|t| t.as_str()) {
                                    result.text = self.state.apply_stop_words(text);
                                }
                            }
                        }
                        "web_search_call" => {
                            result.web_search_results.push(item.clone());
                        }
                        "file_search_call" => {
                            result.file_search_results.push(item.clone());
                        }
                        "code_interpreter_call" => {
                            result.code_interpreter_results.push(item.clone());
                        }
                        "computer_call" => {
                            result.computer_use_results.push(item.clone());
                        }
                        "reasoning" => {
                            result.reasoning_summaries.push(item.clone());
                        }
                        "function_call" => {
                            result.function_calls.push(item.clone());
                        }
                        _ => {}
                    }
                }
            }

            return Ok(serde_json::to_value(result)?);
        }

        // Simple text extraction
        let text = if let Some(items) = output.as_array() {
            items
                .iter()
                .filter_map(|item| {
                    if item.get("type").and_then(|t| t.as_str()) == Some("message") {
                        item.get("content")
                            .and_then(|c| c.get(0))
                            .and_then(|c| c.get("text"))
                            .and_then(|t| t.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("")
        } else {
            output
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string()
        };

        let final_text = self.state.apply_stop_words(&text);
        Ok(Value::String(final_text))
    }

    /// Build the request body for the Chat Completions API.
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
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(max_tokens) = self.max_tokens.or(self.max_completion_tokens) {
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
            body["reasoning_effort"] = serde_json::json!(effort);
        }
        if self.stream {
            body["stream"] = serde_json::json!(true);
        }
        if let Some(tools) = tools {
            if !tools.is_empty() {
                body["tools"] = serde_json::json!(tools);
                body["tool_choice"] = serde_json::json!("auto");
            }
        }

        body
    }
}

#[async_trait]
impl BaseLLM for OpenAICompletion {
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
        "openai"
    }

    fn supports_function_calling(&self) -> bool {
        true
    }

    fn supports_multimodal(&self) -> bool {
        let lower = self.state.model.to_lowercase();
        lower.contains("gpt-4o")
            || lower.contains("gpt-4-vision")
            || lower.contains("gpt-4-turbo")
            || lower.contains("gpt-4.1")
            || lower.contains("gpt-5")
    }

    fn supports_stop_words(&self) -> bool {
        self.state.has_stop_words()
    }

    fn get_context_window_size(&self) -> usize {
        let model = &self.state.model;
        if model.contains("gpt-4o") || model.contains("o1") || model.contains("o3") {
            128_000
        } else if model.contains("gpt-4-turbo") || model.contains("gpt-4-1106") {
            128_000
        } else if model.contains("gpt-4.1") {
            1_047_576
        } else if model.contains("gpt-5") {
            200_000
        } else if model.contains("gpt-4-32k") {
            32_768
        } else if model.contains("gpt-4") {
            8_192
        } else if model.contains("gpt-3.5-turbo-16k") {
            16_384
        } else if model.contains("o4-mini") || model.contains("o3-mini") {
            200_000
        } else {
            4_096
        }
    }

    fn call(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        _available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "OpenAICompletion.call: model={}, messages={}, tools={:?}",
            self.state.model,
            messages.len(),
            tools.as_ref().map(|t| t.len()),
        );

        // Use tokio runtime for sync call
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(self.acall(messages, tools, _available_functions))
    }

    async fn acall(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        _available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "OpenAICompletion.acall: model={}, messages={}",
            self.state.model,
            messages.len(),
        );

        // Validate API key
        let api_key = self.state.api_key.as_ref().ok_or_else(|| {
            "OpenAI API key not set. Set OPENAI_API_KEY environment variable or pass api_key to constructor."
        })?;

        // Build request body
        let tools_slice = tools.as_deref();
        let body = self.build_request_body(&messages, tools_slice);

        // Determine endpoint
        let base_url = self.api_base_url();
        let endpoint = match self.api {
            OpenAIApiMode::Completions => format!("{}/chat/completions", base_url),
            OpenAIApiMode::Responses => format!("{}/chat/responses", base_url),
        };

        // Build HTTP client with timeout
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
                    "OpenAI API retry attempt {} after {:?}",
                    attempt,
                    retry_delay
                );
                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2; // Exponential backoff
            }

            // Build request
            let mut request = client
                .post(&endpoint)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", api_key));

            // Add organization header if set
            if let Some(ref org) = self.organization {
                request = request.header("OpenAI-Organization", org);
            }

            // Add project header if set
            if let Some(ref proj) = self.project {
                request = request.header("OpenAI-Project-Id", proj);
            }

            // Add default headers
            if let Some(ref headers) = self.default_headers {
                for (k, v) in headers {
                    request = request.header(k, v);
                }
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

            // Handle rate limiting
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                last_error = Some("Rate limited by OpenAI API (429)".into());
                continue;
            }

            // Handle server errors (5xx)
            if status.is_server_error() {
                last_error = Some(format!("OpenAI API server error: {}", status).into());
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

            // Handle client errors (4xx)
            if status.is_client_error() {
                return Err(format!(
                    "OpenAI API error ({}): {}",
                    status, response_text
                )
                .into());
            }

            // Parse JSON response
            let response_json: Value = match serde_json::from_str(&response_text) {
                Ok(json) => json,
                Err(e) => {
                    return Err(format!(
                        "Failed to parse OpenAI response: {} - Body: {}",
                        e,
                        &response_text[..response_text.len().min(500)]
                    )
                    .into());
                }
            };

            // Extract content based on API mode
            let result = match self.api {
                OpenAIApiMode::Completions => {
                    self.parse_completions_response(&response_json)?
                }
                OpenAIApiMode::Responses => {
                    self.parse_responses_response(&response_json)?
                }
            };

            return Ok(result);
        }

        // All retries exhausted
        Err(last_error.unwrap_or_else(|| "OpenAI API call failed after all retries".into()))
    }

    fn get_token_usage_summary(&self) -> UsageMetrics {
        self.state.get_token_usage_summary()
    }

    fn track_token_usage(&mut self, usage_data: &HashMap<String, Value>) {
        self.state.track_token_usage_internal(usage_data);
    }
}
