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
        _available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "GeminiCompletion.call: model={}, vertexai={}, messages={}, tools={:?}",
            self.state.model,
            self.use_vertexai,
            messages.len(),
            tools.as_ref().map(|t| t.len()),
        );

        Err("GeminiCompletion.call is a stub - Google Gen AI SDK not yet implemented".into())
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
        let _ = tools;

        Err(
            "GeminiCompletion.acall is a stub - async Google Gen AI SDK not yet implemented"
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
