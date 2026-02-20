//! AWS Bedrock native completion provider.
//!
//! Corresponds to `crewai/llms/providers/bedrock/completion.py`.
//!
//! This module provides direct integration with the AWS Bedrock Converse API,
//! supporting native tool use, streaming, structured output, and multi-model
//! access through a unified AWS interface.
//!
//! # Features
//!
//! - AWS Bedrock Converse API
//! - Streaming support via ConverseStream
//! - Native tool use (function calling)
//! - Structured output via tool-based approach
//! - AWS credential chain authentication (env vars, profiles, IAM roles)
//! - Guardrail support
//! - Cross-region inference
//! - Token usage tracking
//!
//! # Supported Models
//!
//! - Anthropic Claude (3, 3.5, 4.x via Bedrock)
//! - Amazon Nova (Pro, Lite, Micro, Premier)
//! - Meta Llama (3.x, 4.x)
//! - Mistral models
//! - Cohere Command
//! - AI21 Jamba
//!
//! # Note
//!
//! HTTP interceptors are not supported for the Bedrock provider as it uses
//! the AWS SDK (boto3 equivalent) rather than direct HTTP calls.

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
// BedrockCompletion provider
// ---------------------------------------------------------------------------

/// AWS Bedrock native completion implementation.
///
/// Provides direct integration with the AWS Bedrock Converse API.
///
/// Corresponds to `BedrockCompletion(BaseLLM)` in Python.
///
/// # Example
///
/// ```ignore
/// let provider = BedrockCompletion::new(
///     "anthropic.claude-opus-4-5-20251101-v1:0",
///     None,   // region defaults to AWS_DEFAULT_REGION or us-east-1
///     None,   // profile from AWS_PROFILE env var
/// );
/// let messages = vec![/* ... */];
/// let response = provider.call(messages, None, None)?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockCompletion {
    /// Shared base LLM state.
    #[serde(flatten)]
    pub state: BaseLLMState,

    /// AWS region name.
    pub region_name: Option<String>,
    /// AWS profile name.
    pub profile_name: Option<String>,
    /// AWS access key ID.
    #[serde(skip_serializing)]
    pub aws_access_key_id: Option<String>,
    /// AWS secret access key.
    #[serde(skip_serializing)]
    pub aws_secret_access_key: Option<String>,
    /// AWS session token (for temporary credentials).
    #[serde(skip_serializing)]
    pub aws_session_token: Option<String>,

    /// Request timeout in seconds.
    pub timeout: Option<f64>,
    /// Maximum number of retries.
    pub max_retries: u32,
    /// Maximum tokens in response.
    pub max_tokens: Option<u32>,
    /// Nucleus sampling parameter.
    pub top_p: Option<f64>,
    /// Top-K sampling parameter.
    pub top_k: Option<u32>,
    /// Stop sequences.
    pub stop_sequences: Vec<String>,
    /// Whether to use streaming responses.
    pub stream: bool,
    /// Whether to use the Converse API (vs. InvokeModel).
    pub use_converse_api: bool,
    /// Response format for structured output.
    pub response_format: Option<Value>,

    /// Guardrail identifier.
    pub guardrail_id: Option<String>,
    /// Guardrail version.
    pub guardrail_version: Option<String>,
}

impl BedrockCompletion {
    /// Create a new Bedrock completion provider.
    ///
    /// # Arguments
    ///
    /// * `model` - Bedrock model ID (e.g., "anthropic.claude-opus-4-5-20251101-v1:0").
    /// * `region_name` - Optional AWS region (defaults to AWS_DEFAULT_REGION or us-east-1).
    /// * `profile_name` - Optional AWS profile (defaults to AWS_PROFILE env var).
    pub fn new(
        model: impl Into<String>,
        region_name: Option<String>,
        profile_name: Option<String>,
    ) -> Self {
        let region_name = region_name
            .or_else(|| std::env::var("AWS_DEFAULT_REGION").ok())
            .or_else(|| std::env::var("AWS_REGION").ok())
            .or_else(|| Some("us-east-1".to_string()));
        let profile_name = profile_name.or_else(|| std::env::var("AWS_PROFILE").ok());

        let mut state = BaseLLMState::new(model);
        state.provider = "bedrock".to_string();

        Self {
            state,
            region_name,
            profile_name,
            aws_access_key_id: std::env::var("AWS_ACCESS_KEY_ID").ok(),
            aws_secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
            aws_session_token: std::env::var("AWS_SESSION_TOKEN").ok(),
            timeout: None,
            max_retries: 2,
            max_tokens: None,
            top_p: None,
            top_k: None,
            stop_sequences: Vec::new(),
            stream: false,
            use_converse_api: true,
            response_format: None,
            guardrail_id: None,
            guardrail_version: None,
        }
    }

    /// Get the Bedrock endpoint URL.
    pub fn endpoint_url(&self) -> String {
        let region = self.region_name.as_deref().unwrap_or("us-east-1");
        format!("https://bedrock-runtime.{}.amazonaws.com", region)
    }
}

#[async_trait]
impl BaseLLM for BedrockCompletion {
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
        "bedrock"
    }

    fn supports_function_calling(&self) -> bool {
        // Most Bedrock models support tool use via Converse API
        true
    }

    fn supports_multimodal(&self) -> bool {
        let lower = self.state.model.to_lowercase();
        lower.contains("claude")
            || lower.contains("nova-pro")
            || lower.contains("nova-lite")
            || lower.contains("llama3-2-11b")
            || lower.contains("llama3-2-90b")
    }

    fn supports_stop_words(&self) -> bool {
        self.state.has_stop_words()
    }

    fn get_context_window_size(&self) -> usize {
        let lower = self.state.model.to_lowercase();
        if lower.contains("claude") {
            200_000
        } else if lower.contains("nova-pro") || lower.contains("nova-lite") {
            300_000
        } else if lower.contains("nova-micro") {
            128_000
        } else if lower.contains("nova-premier") {
            1_000_000
        } else if lower.contains("llama3-1") || lower.contains("llama3-3") {
            128_000
        } else if lower.contains("llama4") {
            256_000
        } else {
            32_000
        }
    }

    fn call(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        _available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "BedrockCompletion.call: model={}, region={:?}, messages={}, tools={:?}",
            self.state.model,
            self.region_name,
            messages.len(),
            tools.as_ref().map(|t| t.len()),
        );

        Err("BedrockCompletion.call is a stub - AWS SDK not yet implemented".into())
    }

    async fn acall(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        _available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "BedrockCompletion.acall: model={}, messages={}",
            self.state.model,
            messages.len(),
        );
        let _ = tools;

        Err("BedrockCompletion.acall is a stub - async AWS SDK not yet implemented".into())
    }

    fn get_token_usage_summary(&self) -> UsageMetrics {
        self.state.get_token_usage_summary()
    }

    fn track_token_usage(&mut self, usage_data: &HashMap<String, Value>) {
        self.state.track_token_usage_internal(usage_data);
    }
}
