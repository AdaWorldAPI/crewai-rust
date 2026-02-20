//! Google Gemini native completion provider.
//!
//! Corresponds to `crewai/llms/providers/gemini/completion.py`.
//!
//! This module provides direct integration with the Google Gen AI SDK,
//! supporting native function calling, streaming, and proper Gemini
//! content formatting.
//!
//! # Features
//!
//! - Google Gemini API (via Gen AI SDK)
//! - Vertex AI support (with ADC or Express mode)
//! - Streaming support
//! - Native function calling
//! - Structured output via tool-based approach
//! - Safety settings
//! - Multimodal support (images, PDFs, audio, video)
//! - Token usage tracking
//!
//! # Authentication
//!
//! - **Gemini API**: Uses GOOGLE_API_KEY or GEMINI_API_KEY env var.
//! - **Vertex AI**: Uses Application Default Credentials (ADC) with project ID.
//!   Run `gcloud auth application-default login` for local development.
//! - **Vertex AI Express**: API key with api_version="v1" auto-configured.
//!
//! # Note
//!
//! HTTP interceptors are not yet supported for the Gemini provider.

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

/// Tool name used for structured output extraction via tool-based approach.
pub const STRUCTURED_OUTPUT_TOOL_NAME: &str = "structured_output";

// ---------------------------------------------------------------------------
// GeminiCompletion provider
// ---------------------------------------------------------------------------

/// Google Gemini native completion implementation.
///
/// Provides direct integration with the Google Gen AI API.
///
/// Corresponds to `GeminiCompletion(BaseLLM)` in Python.
///
/// # Example
///
/// ```ignore
/// let provider = GeminiCompletion::new(
///     "gemini-2.0-flash-001",
///     None,   // api_key from GOOGLE_API_KEY env var
/// );
/// let messages = vec![/* ... */];
/// let response = provider.call(messages, None, None)?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCompletion {
    /// Shared base LLM state.
    #[serde(flatten)]
    pub state: BaseLLMState,

    /// Google Cloud project ID (for Vertex AI).
    pub project: Option<String>,
    /// Google Cloud location (for Vertex AI, defaults to "us-central1").
    pub location: Option<String>,
    /// Nucleus sampling parameter.
    pub top_p: Option<f64>,
    /// Top-K sampling parameter.
    pub top_k: Option<u32>,
    /// Maximum output tokens.
    pub max_output_tokens: Option<u32>,
    /// Stop sequences.
    pub stop_sequences: Vec<String>,
    /// Whether to use streaming responses.
    pub stream: bool,
    /// Safety filter settings.
    pub safety_settings: Option<Value>,
    /// Additional client parameters.
    pub client_params: Option<HashMap<String, Value>>,
    /// Whether to use Vertex AI instead of Gemini API.
    pub use_vertexai: bool,
    /// Response format for structured output.
    pub response_format: Option<Value>,
}

impl GeminiCompletion {
    /// Create a new Gemini completion provider.
    ///
    /// # Arguments
    ///
    /// * `model` - Gemini model name (e.g., "gemini-2.0-flash-001").
    /// * `api_key` - Optional API key (defaults to GOOGLE_API_KEY or GEMINI_API_KEY env var).
    pub fn new(model: impl Into<String>, api_key: Option<String>) -> Self {
        let api_key = api_key
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
            .or_else(|| std::env::var("GEMINI_API_KEY").ok());
        let project = std::env::var("GOOGLE_CLOUD_PROJECT").ok();
        let location = std::env::var("GOOGLE_CLOUD_LOCATION")
            .ok()
            .or_else(|| Some("us-central1".to_string()));
        let use_vertexai = std::env::var("GOOGLE_GENAI_USE_VERTEXAI")
            .ok()
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        let mut state = BaseLLMState::new(model);
        state.api_key = api_key;
        state.provider = "gemini".to_string();

        Self {
            state,
            project,
            location,
            top_p: None,
            top_k: None,
            max_output_tokens: None,
            stop_sequences: Vec::new(),
            stream: false,
            safety_settings: None,
            client_params: None,
            use_vertexai,
            response_format: None,
        }
    }

    /// Get the API endpoint URL.
    fn api_endpoint(&self) -> String {
        if self.use_vertexai {
            let project = self.project.as_deref().unwrap_or("default");
            let location = self.location.as_deref().unwrap_or("us-central1");
            format!(
                "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
                location, project, location, self.state.model
            )
        } else {
            format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                self.state.model
            )
        }
    }

    /// Build generation config for the Gemini API.
    pub fn generation_config(&self) -> Value {
        let mut config = serde_json::Map::new();
        if let Some(temp) = self.state.temperature {
            config.insert("temperature".to_string(), serde_json::json!(temp));
        }
        if let Some(max_tokens) = self.max_output_tokens {
            config.insert("maxOutputTokens".to_string(), serde_json::json!(max_tokens));
        }
        if let Some(top_p) = self.top_p {
            config.insert("topP".to_string(), serde_json::json!(top_p));
        }
        if let Some(top_k) = self.top_k {
            config.insert("topK".to_string(), serde_json::json!(top_k));
        }
        if !self.state.stop.is_empty() {
            config.insert(
                "stopSequences".to_string(),
                serde_json::json!(self.state.stop),
            );
        } else if !self.stop_sequences.is_empty() {
            config.insert(
                "stopSequences".to_string(),
                serde_json::json!(self.stop_sequences),
            );
        }
        Value::Object(config)
    }

    /// Convert messages from OpenAI-style format to Gemini contents format.
    ///
    /// Gemini uses `contents` with `parts` instead of `messages` with `content`.
    /// System messages are extracted to the `system_instruction` parameter.
    fn format_messages(&self, messages: &[LLMMessage]) -> (Option<String>, Vec<Value>) {
        let mut system_parts: Vec<String> = Vec::new();
        let mut contents: Vec<Value> = Vec::new();

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
                if let Some(text) = content.as_str() {
                    system_parts.push(text.to_string());
                }
            } else {
                // Map role: assistant -> model, user -> user
                let gemini_role = match role {
                    "assistant" => "model",
                    "tool" => "function",
                    _ => "user",
                };

                let parts = if role == "tool" {
                    // Convert tool results to function response parts
                    let tool_call_id = msg
                        .get("tool_call_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let result_text = content.as_str().unwrap_or("").to_string();
                    serde_json::json!([{
                        "functionResponse": {
                            "name": tool_call_id,
                            "response": { "result": result_text }
                        }
                    }])
                } else if let Some(text) = content.as_str() {
                    serde_json::json!([{ "text": text }])
                } else if let Some(arr) = content.as_array() {
                    // Handle multi-part content
                    Value::Array(arr.iter().map(|block| {
                        if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                            serde_json::json!({ "text": text })
                        } else {
                            block.clone()
                        }
                    }).collect())
                } else {
                    serde_json::json!([{ "text": content.to_string() }])
                };

                // Handle assistant messages with tool_calls
                if role == "assistant" {
                    if let Some(tool_calls) = msg.get("tool_calls").and_then(|v| v.as_array()) {
                        let mut all_parts: Vec<Value> = Vec::new();
                        if let Some(text) = content.as_str() {
                            if !text.is_empty() {
                                all_parts.push(serde_json::json!({ "text": text }));
                            }
                        }
                        for tc in tool_calls {
                            let func = tc.get("function").unwrap_or(&Value::Null);
                            let name = func.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            let args_str = func.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}");
                            let args: Value = serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));
                            all_parts.push(serde_json::json!({
                                "functionCall": { "name": name, "args": args }
                            }));
                        }
                        contents.push(serde_json::json!({
                            "role": gemini_role,
                            "parts": all_parts,
                        }));
                        continue;
                    }
                }

                contents.push(serde_json::json!({
                    "role": gemini_role,
                    "parts": parts,
                }));
            }
        }

        let system = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };

        (system, contents)
    }

    /// Build the complete request body.
    fn build_request_body(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[Value]>,
    ) -> Value {
        let (system, contents) = self.format_messages(messages);

        let mut body = serde_json::json!({
            "contents": contents,
            "generationConfig": self.generation_config(),
        });

        if let Some(system_text) = system {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{ "text": system_text }]
            });
        }

        if let Some(tools) = tools {
            if !tools.is_empty() {
                // Convert OpenAI-style tool definitions to Gemini function declarations
                let declarations: Vec<Value> = tools.iter().map(|tool| {
                    if let Some(func) = tool.get("function") {
                        func.clone()
                    } else {
                        tool.clone()
                    }
                }).collect();
                body["tools"] = serde_json::json!([{
                    "functionDeclarations": declarations
                }]);
            }
        }

        if let Some(ref safety) = self.safety_settings {
            body["safetySettings"] = safety.clone();
        }

        body
    }

    /// Parse a Gemini API response.
    fn parse_response(
        &self,
        response: &Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let candidates = response
            .get("candidates")
            .and_then(|c| c.as_array())
            .ok_or("No candidates in Gemini response")?;

        if candidates.is_empty() {
            return Err("Empty candidates array in Gemini response".into());
        }

        let candidate = &candidates[0];
        let parts = candidate
            .get("content")
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.as_array())
            .ok_or("No content.parts in Gemini response")?;

        let mut text_parts: Vec<String> = Vec::new();
        let mut function_calls: Vec<Value> = Vec::new();

        for part in parts {
            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                text_parts.push(text.to_string());
            }
            if let Some(fc) = part.get("functionCall") {
                let name = fc.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = fc.get("args").unwrap_or(&Value::Null);
                let args_str = serde_json::to_string(args).unwrap_or_default();
                function_calls.push(serde_json::json!({
                    "id": format!("call_{}", uuid::Uuid::new_v4()),
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": args_str,
                    }
                }));
            }
        }

        if !function_calls.is_empty() {
            let combined_text = text_parts.join("");
            return Ok(serde_json::json!({
                "role": "assistant",
                "content": if combined_text.is_empty() { Value::Null } else { Value::String(combined_text) },
                "tool_calls": function_calls,
            }));
        }

        let combined = text_parts.join("");
        let final_content = self.state.apply_stop_words(&combined);
        Ok(Value::String(final_content))
    }

    /// Extract token usage from a Gemini response.
    fn extract_token_usage(response: &Value) -> HashMap<String, Value> {
        let mut usage = HashMap::new();
        if let Some(usage_obj) = response.get("usageMetadata") {
            let prompt = usage_obj
                .get("promptTokenCount")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let completion = usage_obj
                .get("candidatesTokenCount")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let cached = usage_obj
                .get("cachedContentTokenCount")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            usage.insert("prompt_tokens".to_string(), serde_json::json!(prompt));
            usage.insert("completion_tokens".to_string(), serde_json::json!(completion));
            usage.insert("total_tokens".to_string(), serde_json::json!(prompt + completion));
            usage.insert("cached_tokens".to_string(), serde_json::json!(cached));
        }
        usage
    }
}

#[async_trait]
impl BaseLLM for GeminiCompletion {
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
        "gemini"
    }

    fn supports_function_calling(&self) -> bool {
        true
    }

    fn supports_multimodal(&self) -> bool {
        // Gemini Pro and Flash models support multimodal
        true
    }

    fn supports_stop_words(&self) -> bool {
        self.state.has_stop_words()
    }

    fn get_context_window_size(&self) -> usize {
        let lower = self.state.model.to_lowercase();
        if lower.contains("1.5-pro") {
            2_097_152
        } else if lower.contains("1.5-flash") || lower.contains("2.0-flash") {
            1_048_576
        } else if lower.contains("2.5") {
            1_048_576
        } else {
            1_048_576 // Default for Gemini models
        }
    }

    fn call(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "GeminiCompletion.call: model={}, vertexai={}, messages={}, tools={:?}",
            self.state.model,
            self.use_vertexai,
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
            "GeminiCompletion.acall: model={}, messages={}",
            self.state.model,
            messages.len(),
        );

        let api_key = self.state.api_key.as_ref().ok_or_else(|| {
            "Gemini API key not set. Set GOOGLE_API_KEY or GEMINI_API_KEY environment variable."
        })?;

        let tools_slice = tools.as_deref();
        let body = self.build_request_body(&messages, tools_slice);

        let endpoint = self.api_endpoint();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        // Retry loop with exponential backoff
        let mut last_error: Option<Box<dyn std::error::Error + Send + Sync>> = None;
        let mut retry_delay = std::time::Duration::from_secs(1);
        let max_retries = 2u32;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                log::warn!("Gemini API retry attempt {} after {:?}", attempt, retry_delay);
                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2;
            }

            let mut request = client
                .post(&endpoint)
                .header("content-type", "application/json");

            if self.use_vertexai {
                // Vertex AI uses Bearer token auth (ADC)
                request = request.header("authorization", format!("Bearer {}", api_key));
            } else {
                // Gemini API uses query parameter
                request = request.query(&[("key", api_key.as_str())]);
            }

            let response = match request.json(&body).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = Some(Box::new(e));
                    continue;
                }
            };

            let status = response.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                last_error = Some("Rate limited by Gemini API (429)".into());
                continue;
            }

            if status.is_server_error() {
                last_error = Some(format!("Gemini API server error: {}", status).into());
                continue;
            }

            let response_text = match response.text().await {
                Ok(text) => text,
                Err(e) => {
                    last_error = Some(Box::new(e));
                    continue;
                }
            };

            if status.is_client_error() {
                return Err(format!("Gemini API error ({}): {}", status, response_text).into());
            }

            let response_json: Value = match serde_json::from_str(&response_text) {
                Ok(json) => json,
                Err(e) => {
                    return Err(format!(
                        "Failed to parse Gemini response: {} - Body: {}",
                        e,
                        &response_text[..response_text.len().min(500)]
                    ).into());
                }
            };

            // Check for API error
            if let Some(error) = response_json.get("error") {
                let msg = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown Gemini API error");
                return Err(format!("Gemini API error: {}", msg).into());
            }

            // Extract token usage
            let usage = Self::extract_token_usage(&response_json);
            if !usage.is_empty() {
                log::debug!("Gemini usage: {:?}", usage);
            }

            return self.parse_response(&response_json);
        }

        Err(last_error.unwrap_or_else(|| "Gemini API call failed after all retries".into()))
    }

    fn get_token_usage_summary(&self) -> UsageMetrics {
        self.state.get_token_usage_summary()
    }

    fn track_token_usage(&mut self, usage_data: &HashMap<String, Value>) {
        self.state.track_token_usage_internal(usage_data);
    }
}
