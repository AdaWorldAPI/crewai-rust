//! Third-party LLM implementations for CrewAI.
//!
//! Corresponds to `crewai/llms/third_party/` Python package.
//!
//! This module provides integration with third-party LLM providers that
//! are not part of the core native SDK provider set. The primary integration
//! is with LiteLLM, which acts as a universal bridge to 100+ LLM providers.
//!
//! # LiteLLM Bridge
//!
//! In the Python implementation, LiteLLM is the default fallback when a
//! model string doesn't match any native SDK provider. It supports:
//!
//! - Groq, Together AI, Fireworks, Perplexity, Replicate
//! - Local models via Ollama, vLLM, LM Studio
//! - Hugging Face Inference Endpoints
//! - Custom OpenAI-compatible endpoints
//!
//! In the Rust port, the LiteLLM bridge is represented as a struct that
//! can be configured to call LiteLLM via its OpenAI-compatible proxy API,
//! or directly via the individual provider APIs.

use std::any::Any;
use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::llms::base_llm::{BaseLLM, BaseLLMState, LLMMessage};
use crate::types::usage_metrics::UsageMetrics;

// ---------------------------------------------------------------------------
// LiteLLMBridge
// ---------------------------------------------------------------------------

/// LiteLLM bridge for third-party LLM providers.
///
/// Acts as a universal adapter for LLM providers not natively supported.
/// In the full implementation, this would call LiteLLM's proxy API or
/// directly invoke the provider's API via an OpenAI-compatible endpoint.
///
/// # Example
///
/// ```ignore
/// let bridge = LiteLLMBridge::new("groq/llama-3.1-70b-versatile", None, None);
/// let messages = vec![/* ... */];
/// let response = bridge.call(messages, None, None)?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteLLMBridge {
    /// Shared base LLM state.
    #[serde(flatten)]
    pub state: BaseLLMState,

    /// The original model string (may include provider prefix like "groq/").
    pub original_model: String,
    /// LiteLLM proxy base URL (if using proxy mode).
    pub proxy_base_url: Option<String>,
    /// Request timeout in seconds.
    pub timeout: Option<f64>,
    /// Whether to use streaming responses.
    pub stream: bool,
    /// Maximum tokens in response.
    pub max_tokens: Option<u32>,
}

impl LiteLLMBridge {
    /// Create a new LiteLLM bridge.
    ///
    /// # Arguments
    ///
    /// * `model` - Full model string (e.g., "groq/llama-3.1-70b", "ollama/llama3").
    /// * `api_key` - Optional API key for the target provider.
    /// * `base_url` - Optional base URL for LiteLLM proxy or provider endpoint.
    pub fn new(
        model: impl Into<String>,
        api_key: Option<String>,
        base_url: Option<String>,
    ) -> Self {
        let model = model.into();
        let proxy_base_url = base_url.clone().or_else(|| {
            std::env::var("LITELLM_PROXY_URL").ok()
        });

        let mut state = BaseLLMState::new(&model);
        state.api_key = api_key;
        state.base_url = base_url;
        state.provider = "litellm".to_string();

        Self {
            original_model: model,
            state,
            proxy_base_url,
            timeout: None,
            stream: false,
            max_tokens: None,
        }
    }
}

#[async_trait]
impl BaseLLM for LiteLLMBridge {
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
        "litellm"
    }

    fn is_litellm(&self) -> bool {
        true
    }

    fn supports_function_calling(&self) -> bool {
        // Most LiteLLM-supported models support function calling
        true
    }

    fn call(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        _available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "LiteLLMBridge.call: model={}, proxy={:?}, messages={}, tools={:?}",
            self.original_model,
            self.proxy_base_url,
            messages.len(),
            tools.as_ref().map(|t| t.len()),
        );

        Err(
            "LiteLLMBridge.call is a stub - LiteLLM proxy integration not yet implemented"
                .into(),
        )
    }

    async fn acall(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        _available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "LiteLLMBridge.acall: model={}, messages={}",
            self.original_model,
            messages.len(),
        );
        let _ = tools;

        Err(
            "LiteLLMBridge.acall is a stub - async LiteLLM proxy integration not yet implemented"
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
