//! Azure AI Inference native completion provider.
//!
//! Corresponds to `crewai/llms/providers/azure/completion.py`.
//!
//! This module provides direct integration with the Azure AI Inference SDK,
//! offering native function calling, streaming support, and proper Azure
//! authentication via API key or Azure credentials.
//!
//! # Features
//!
//! - Azure AI Inference Chat Completions
//! - Streaming support
//! - Function/tool calling
//! - Structured output (JSON schema)
//! - Azure Key Credential authentication
//! - Token usage tracking
//!
//! # Note
//!
//! HTTP interceptors are not yet supported for the Azure provider.

use std::any::Any;
use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::llms::base_llm::{BaseLLM, BaseLLMState, LLMMessage};
use crate::types::usage_metrics::UsageMetrics;

// ---------------------------------------------------------------------------
// AzureCompletion provider
// ---------------------------------------------------------------------------

/// Azure AI Inference native completion implementation.
///
/// Provides direct integration with the Azure AI Inference API.
/// Azure uses the same chat completions format as OpenAI, making
/// this provider relatively straightforward.
///
/// Corresponds to `AzureCompletion(BaseLLM)` in Python.
///
/// # Example
///
/// ```ignore
/// let provider = AzureCompletion::new(
///     "gpt-4o",
///     None,      // api_key from AZURE_API_KEY env var
///     None,      // endpoint from AZURE_ENDPOINT env var
/// );
/// let messages = vec![/* ... */];
/// let response = provider.call(messages, None, None)?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureCompletion {
    /// Shared base LLM state.
    #[serde(flatten)]
    pub state: BaseLLMState,

    /// Azure endpoint URL.
    pub endpoint: Option<String>,
    /// Azure API version.
    pub api_version: Option<String>,
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
    /// Whether to use streaming responses.
    pub stream: bool,
    /// Response format for structured output.
    pub response_format: Option<Value>,
}

impl AzureCompletion {
    /// Create a new Azure completion provider.
    ///
    /// # Arguments
    ///
    /// * `model` - Azure deployment name or model name.
    /// * `api_key` - Optional API key (defaults to AZURE_API_KEY env var).
    /// * `endpoint` - Optional endpoint URL (defaults to AZURE_ENDPOINT env var).
    pub fn new(
        model: impl Into<String>,
        api_key: Option<String>,
        endpoint: Option<String>,
    ) -> Self {
        let api_key = api_key.or_else(|| std::env::var("AZURE_API_KEY").ok());
        let endpoint = endpoint.or_else(|| std::env::var("AZURE_ENDPOINT").ok());
        let api_version = std::env::var("AZURE_API_VERSION").ok();

        let mut state = BaseLLMState::new(model);
        state.api_key = api_key;
        state.base_url = endpoint.clone();
        state.provider = "azure".to_string();

        Self {
            state,
            endpoint,
            api_version,
            timeout: None,
            max_retries: 2,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            max_tokens: None,
            stream: false,
            response_format: None,
        }
    }

    /// Get the full API URL for chat completions.
    pub fn api_url(&self) -> String {
        let ep = self
            .endpoint
            .as_deref()
            .or(self.state.base_url.as_deref())
            .unwrap_or("https://YOUR_RESOURCE.openai.azure.com");
        let version = self.api_version.as_deref().unwrap_or("2024-02-01");

        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            ep.trim_end_matches('/'),
            self.state.model,
            version
        )
    }

    /// Build the OpenAI-compatible request body for Azure.
    fn build_request_body(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[Value]>,
    ) -> Value {
        let mut body = serde_json::json!({
            "messages": messages,
        });

        if let Some(temp) = self.state.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(max) = self.max_tokens {
            body["max_tokens"] = serde_json::json!(max);
        }
        if let Some(top_p) = self.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }
        if let Some(fp) = self.frequency_penalty {
            body["frequency_penalty"] = serde_json::json!(fp);
        }
        if let Some(pp) = self.presence_penalty {
            body["presence_penalty"] = serde_json::json!(pp);
        }
        if !self.state.stop.is_empty() {
            body["stop"] = serde_json::json!(self.state.stop);
        }

        if let Some(tools) = tools {
            if !tools.is_empty() {
                body["tools"] = Value::Array(tools.to_vec());
            }
        }

        if let Some(ref fmt) = self.response_format {
            body["response_format"] = fmt.clone();
        }

        body
    }

    /// Parse an Azure/OpenAI-style chat completion response.
    fn parse_response(
        &self,
        response: &Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let choices = response
            .get("choices")
            .and_then(|c| c.as_array())
            .ok_or("No choices in Azure response")?;

        if choices.is_empty() {
            return Err("Empty choices array in Azure response".into());
        }

        let message = choices[0]
            .get("message")
            .ok_or("No message in Azure response choice")?;

        // Check for tool calls
        if let Some(tool_calls) = message.get("tool_calls") {
            if tool_calls.as_array().map_or(false, |a| !a.is_empty()) {
                return Ok(serde_json::json!({
                    "role": "assistant",
                    "content": message.get("content").cloned().unwrap_or(Value::Null),
                    "tool_calls": tool_calls,
                }));
            }
        }

        // Plain text response
        let content = message
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let final_content = self.state.apply_stop_words(content);
        Ok(Value::String(final_content))
    }

    /// Extract token usage from an Azure/OpenAI-style response.
    fn extract_token_usage(response: &Value) -> HashMap<String, Value> {
        let mut usage = HashMap::new();
        if let Some(usage_obj) = response.get("usage") {
            let prompt = usage_obj
                .get("prompt_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let completion = usage_obj
                .get("completion_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let total = usage_obj
                .get("total_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(prompt + completion);

            usage.insert("prompt_tokens".to_string(), serde_json::json!(prompt));
            usage.insert("completion_tokens".to_string(), serde_json::json!(completion));
            usage.insert("total_tokens".to_string(), serde_json::json!(total));
        }
        usage
    }
}

#[async_trait]
impl BaseLLM for AzureCompletion {
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
        "azure"
    }

    fn supports_function_calling(&self) -> bool {
        true
    }

    fn supports_multimodal(&self) -> bool {
        let lower = self.state.model.to_lowercase();
        lower.contains("gpt-4o") || lower.contains("gpt-4-vision") || lower.contains("gpt-5")
    }

    fn supports_stop_words(&self) -> bool {
        self.state.has_stop_words()
    }

    fn call(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "AzureCompletion.call: model={}, endpoint={:?}, messages={}, tools={:?}",
            self.state.model,
            self.endpoint,
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
            "AzureCompletion.acall: model={}, messages={}",
            self.state.model,
            messages.len(),
        );

        let api_key = self.state.api_key.as_ref().ok_or_else(|| {
            "Azure API key not set. Set AZURE_API_KEY environment variable."
        })?;

        let tools_slice = tools.as_deref();
        let body = self.build_request_body(&messages, tools_slice);

        let url = self.api_url();

        let timeout_secs = self.timeout.unwrap_or(120.0) as u64;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()?;

        // Retry loop with exponential backoff
        let mut last_error: Option<Box<dyn std::error::Error + Send + Sync>> = None;
        let mut retry_delay = std::time::Duration::from_secs(1);

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                log::warn!(
                    "Azure API retry attempt {} after {:?}",
                    attempt,
                    retry_delay
                );
                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2;
            }

            let response = match client
                .post(&url)
                .header("api-key", api_key.as_str())
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = Some(Box::new(e));
                    continue;
                }
            };

            let status = response.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                last_error = Some("Rate limited by Azure API (429)".into());
                continue;
            }

            if status.is_server_error() {
                last_error =
                    Some(format!("Azure API server error: {}", status).into());
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
                return Err(
                    format!("Azure API error ({}): {}", status, response_text).into()
                );
            }

            let response_json: Value = match serde_json::from_str(&response_text) {
                Ok(json) => json,
                Err(e) => {
                    return Err(format!(
                        "Failed to parse Azure response: {} - Body: {}",
                        e,
                        &response_text[..response_text.len().min(500)]
                    )
                    .into());
                }
            };

            // Check for API error
            if let Some(error) = response_json.get("error") {
                let msg = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown Azure API error");
                return Err(format!("Azure API error: {}", msg).into());
            }

            // Extract token usage
            let usage = Self::extract_token_usage(&response_json);
            if !usage.is_empty() {
                log::debug!("Azure usage: {:?}", usage);
            }

            return self.parse_response(&response_json);
        }

        Err(last_error
            .unwrap_or_else(|| "Azure API call failed after all retries".into()))
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
    fn test_azure_new_defaults() {
        let provider = AzureCompletion::new("gpt-4o", None, None);
        assert_eq!(provider.model(), "gpt-4o");
        assert_eq!(provider.provider(), "azure");
        assert!(provider.supports_function_calling());
        assert!(provider.supports_multimodal());
    }

    #[test]
    fn test_azure_api_url() {
        let provider = AzureCompletion::new(
            "gpt-4o",
            None,
            Some("https://myresource.openai.azure.com".to_string()),
        );
        let url = provider.api_url();
        assert!(url.contains("myresource.openai.azure.com"));
        assert!(url.contains("gpt-4o"));
        assert!(url.contains("api-version="));
    }

    fn msg(pairs: &[(&str, Value)]) -> LLMMessage {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn test_build_request_body() {
        let mut provider = AzureCompletion::new("gpt-4o", None, None);
        provider.max_tokens = Some(1024);
        provider.top_p = Some(0.9);

        let messages: Vec<LLMMessage> = vec![
            msg(&[("role", serde_json::json!("user")), ("content", serde_json::json!("Hello"))]),
        ];

        let body = provider.build_request_body(&messages, None);
        assert!(body.get("messages").is_some());
        assert_eq!(body["max_tokens"], 1024);
        assert_eq!(body["top_p"], 0.9);
    }

    #[test]
    fn test_parse_response_text() {
        let provider = AzureCompletion::new("gpt-4o", None, None);
        let response = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello there!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        });

        let result = provider.parse_response(&response).unwrap();
        assert_eq!(result.as_str().unwrap(), "Hello there!");
    }

    #[test]
    fn test_parse_response_tool_calls() {
        let provider = AzureCompletion::new("gpt-4o", None, None);
        let response = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"city\":\"NYC\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30 }
        });

        let result = provider.parse_response(&response).unwrap();
        assert!(result.get("tool_calls").is_some());
        assert_eq!(result["tool_calls"][0]["function"]["name"], "get_weather");
    }

    #[test]
    fn test_extract_token_usage() {
        let response = serde_json::json!({
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150
            }
        });

        let usage = AzureCompletion::extract_token_usage(&response);
        assert_eq!(usage["prompt_tokens"], 100);
        assert_eq!(usage["completion_tokens"], 50);
        assert_eq!(usage["total_tokens"], 150);
    }

    #[test]
    fn test_multimodal_support() {
        let gpt4o = AzureCompletion::new("gpt-4o", None, None);
        assert!(gpt4o.supports_multimodal());

        let gpt35 = AzureCompletion::new("gpt-35-turbo", None, None);
        assert!(!gpt35.supports_multimodal());
    }
}
