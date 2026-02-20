//! Main LLM struct for CrewAI.
//!
//! Corresponds to `crewai/llm.py`.
//!
//! This module contains the top-level [`LLM`] struct that wraps a language model
//! with configuration for API calls. It includes model routing logic, context
//! window size lookups, and provider inference. Provider-level abstractions
//! (BaseLLM trait, provider SDK wrappers) live in the [`crate::llms`] module.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::llms::base_llm::BaseLLM;
use crate::llms::providers::openai::OpenAICompletion;
use crate::llms::providers::xai::XAICompletion;

/// Minimum context window size.
pub const MIN_CONTEXT: i64 = 1024;

/// Maximum context window size (current max from gemini-1.5-pro).
pub const MAX_CONTEXT: i64 = 2_097_152;

/// Default context window size.
pub const DEFAULT_CONTEXT_WINDOW_SIZE: i64 = 8192;

/// Context window usage ratio (use 85% of the context window).
pub const CONTEXT_WINDOW_USAGE_RATIO: f64 = 0.85;

/// Anthropic model name prefixes.
///
/// Corresponds to `ANTHROPIC_PREFIXES` in Python.
pub const ANTHROPIC_PREFIXES: &[&str] = &["anthropic/", "claude-", "claude/"];

/// Known context window sizes for various models.
///
/// Corresponds to `LLM_CONTEXT_WINDOW_SIZES` in the Python implementation.
pub fn llm_context_window_sizes() -> HashMap<&'static str, i64> {
    let mut m = HashMap::new();
    // OpenAI
    m.insert("gpt-4", 8192);
    m.insert("gpt-4o", 128000);
    m.insert("gpt-4o-mini", 200000);
    m.insert("gpt-4-turbo", 128000);
    m.insert("gpt-4.1", 1047576);
    m.insert("gpt-4.1-mini-2025-04-14", 1047576);
    m.insert("gpt-4.1-nano-2025-04-14", 1047576);
    m.insert("o1-preview", 128000);
    m.insert("o1-mini", 128000);
    m.insert("o3-mini", 200000);
    m.insert("o4-mini", 200000);
    // Gemini
    m.insert("gemini-3-pro-preview", 1048576);
    m.insert("gemini-2.0-flash", 1048576);
    m.insert("gemini-2.0-flash-thinking-exp-01-21", 32768);
    m.insert("gemini-2.0-flash-lite-001", 1048576);
    m.insert("gemini-2.0-flash-001", 1048576);
    m.insert("gemini-2.5-flash-preview-04-17", 1048576);
    m.insert("gemini-2.5-pro-exp-03-25", 1048576);
    m.insert("gemini-1.5-pro", 2097152);
    m.insert("gemini-1.5-flash", 1048576);
    m.insert("gemini-1.5-flash-8b", 1048576);
    m.insert("gemini/gemma-3-1b-it", 32000);
    m.insert("gemini/gemma-3-4b-it", 128000);
    m.insert("gemini/gemma-3-12b-it", 128000);
    m.insert("gemini/gemma-3-27b-it", 128000);
    // DeepSeek
    m.insert("deepseek-chat", 128000);
    // Groq
    m.insert("gemma2-9b-it", 8192);
    m.insert("gemma-7b-it", 8192);
    m.insert("llama3-groq-70b-8192-tool-use-preview", 8192);
    m.insert("llama3-groq-8b-8192-tool-use-preview", 8192);
    m.insert("llama-3.1-70b-versatile", 131072);
    m.insert("llama-3.1-8b-instant", 131072);
    m.insert("llama-3.2-1b-preview", 8192);
    m.insert("llama-3.2-3b-preview", 8192);
    m.insert("llama-3.2-11b-text-preview", 8192);
    m.insert("llama-3.2-90b-text-preview", 8192);
    m.insert("llama3-70b-8192", 8192);
    m.insert("llama3-8b-8192", 8192);
    m.insert("mixtral-8x7b-32768", 32768);
    m.insert("llama-3.3-70b-versatile", 128000);
    m.insert("llama-3.3-70b-instruct", 128000);
    // SambaNova
    m.insert("Meta-Llama-3.3-70B-Instruct", 131072);
    m.insert("QwQ-32B-Preview", 8192);
    m.insert("Qwen2.5-72B-Instruct", 8192);
    m.insert("Qwen2.5-Coder-32B-Instruct", 8192);
    m.insert("Meta-Llama-3.1-405B-Instruct", 8192);
    m.insert("Meta-Llama-3.1-70B-Instruct", 131072);
    m.insert("Meta-Llama-3.1-8B-Instruct", 131072);
    m.insert("Llama-3.2-90B-Vision-Instruct", 16384);
    m.insert("Llama-3.2-11B-Vision-Instruct", 16384);
    m.insert("Meta-Llama-3.2-3B-Instruct", 4096);
    m.insert("Meta-Llama-3.2-1B-Instruct", 16384);
    // Bedrock
    m.insert("us.amazon.nova-pro-v1:0", 300000);
    m.insert("us.amazon.nova-micro-v1:0", 128000);
    m.insert("us.amazon.nova-lite-v1:0", 300000);
    m.insert("us.anthropic.claude-opus-4-5-20251101-v1:0", 200000);
    m.insert("us.meta.llama3-2-11b-instruct-v1:0", 128000);
    m.insert("us.meta.llama3-2-3b-instruct-v1:0", 131000);
    m.insert("us.meta.llama3-2-90b-instruct-v1:0", 128000);
    m.insert("us.meta.llama3-2-1b-instruct-v1:0", 131000);
    m.insert("us.meta.llama3-1-8b-instruct-v1:0", 128000);
    m.insert("us.meta.llama3-1-70b-instruct-v1:0", 128000);
    m.insert("us.meta.llama3-3-70b-instruct-v1:0", 128000);
    m.insert("us.meta.llama3-1-405b-instruct-v1:0", 128000);
    m.insert("eu.anthropic.claude-opus-4-5-20251101-v1:0", 200000);
    m.insert("eu.meta.llama3-2-3b-instruct-v1:0", 131000);
    m.insert("eu.meta.llama3-2-1b-instruct-v1:0", 131000);
    m.insert("apac.anthropic.claude-opus-4-5-20251101-v1:0", 200000);
    m.insert("amazon.nova-pro-v1:0", 300000);
    m.insert("amazon.nova-micro-v1:0", 128000);
    m.insert("amazon.nova-lite-v1:0", 300000);
    m.insert("anthropic.claude-opus-4-5-20251101-v1:0", 200000);
    m.insert("anthropic.claude-opus-4-5-20251101", 200000);
    m.insert("meta.llama3-1-405b-instruct-v1:0", 128000);
    m.insert("meta.llama3-1-70b-instruct-v1:0", 128000);
    m.insert("meta.llama3-1-8b-instruct-v1:0", 128000);
    m.insert("meta.llama3-70b-instruct-v1:0", 8000);
    m.insert("meta.llama3-8b-instruct-v1:0", 8000);
    m.insert("amazon.titan-text-lite-v1", 4000);
    m.insert("amazon.titan-text-express-v1", 8000);
    m.insert("cohere.command-text-v14", 4000);
    m.insert("ai21.j2-mid-v1", 8191);
    m.insert("ai21.j2-ultra-v1", 8191);
    m.insert("ai21.jamba-instruct-v1:0", 256000);
    m.insert("mistral.mistral-7b-instruct-v0:2", 32000);
    m.insert("mistral.mixtral-8x7b-instruct-v0:1", 32000);
    // Mistral
    m.insert("mistral-tiny", 32768);
    m.insert("mistral-small-latest", 32768);
    m.insert("mistral-medium-latest", 32768);
    m.insert("mistral-large-latest", 32768);
    m.insert("mistral-large-2407", 32768);
    m.insert("mistral-large-2402", 32768);
    m.insert("mistral/mistral-tiny", 32768);
    m.insert("mistral/mistral-small-latest", 32768);
    m.insert("mistral/mistral-medium-latest", 32768);
    m.insert("mistral/mistral-large-latest", 32768);
    m.insert("mistral/mistral-large-2407", 32768);
    m.insert("mistral/mistral-large-2402", 32768);
    m
}

/// Supported native providers.
pub const SUPPORTED_NATIVE_PROVIDERS: &[&str] = &[
    "openai",
    "anthropic",
    "claude",
    "azure",
    "azure_openai",
    "google",
    "gemini",
    "bedrock",
    "aws",
];

/// Reasoning effort levels for models that support it.
///
/// Corresponds to `Literal["none", "low", "medium", "high"]` in Python.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    /// No reasoning effort.
    None,
    /// Low reasoning effort.
    Low,
    /// Medium reasoning effort.
    Medium,
    /// High reasoning effort.
    High,
}

impl std::fmt::Display for ReasoningEffort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReasoningEffort::None => write!(f, "none"),
            ReasoningEffort::Low => write!(f, "low"),
            ReasoningEffort::Medium => write!(f, "medium"),
            ReasoningEffort::High => write!(f, "high"),
        }
    }
}

/// Response format specification for structured output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    /// Type of response format (e.g., "json_object", "json_schema").
    #[serde(rename = "type")]
    pub format_type: String,
    /// Optional JSON schema for the response.
    pub json_schema: Option<Value>,
}

/// Main LLM struct.
///
/// Wraps a language model with configuration for API calls.
/// In the Python implementation, the `LLM` class extends `BaseLLM` and
/// can route to native providers or fall back to LiteLLM.
///
/// Corresponds to `class LLM(BaseLLM)` in `crewai/llm.py`.
///
/// # Fields
///
/// See field documentation below. All fields from the Python `LLM.__init__`
/// are included. Methods are simplified stubs.
#[derive(Debug, Serialize, Deserialize)]
pub struct LLM {
    /// Model identifier (e.g., "gpt-4", "claude-opus-4-5-20251101").
    pub model: String,
    /// Timeout for API calls in seconds.
    pub timeout: Option<f64>,
    /// Temperature parameter for generation.
    pub temperature: Option<f64>,
    /// Top-p (nucleus) sampling parameter.
    pub top_p: Option<f64>,
    /// Number of completions to generate.
    pub n: Option<i32>,
    /// Stop sequences for the model.
    pub stop: Vec<String>,
    /// Maximum completion tokens (newer OpenAI parameter name).
    pub max_completion_tokens: Option<i64>,
    /// Maximum number of tokens to generate.
    pub max_tokens: Option<i64>,
    /// Presence penalty for the model (-2 to 2).
    pub presence_penalty: Option<f64>,
    /// Frequency penalty for the model (-2 to 2).
    pub frequency_penalty: Option<f64>,
    /// Logit bias for token generation.
    pub logit_bias: Option<HashMap<i64, f64>>,
    /// Response format specification for structured output.
    pub response_format: Option<Value>,
    /// Random seed for reproducibility.
    pub seed: Option<i64>,
    /// Whether to return log probabilities.
    pub logprobs: Option<i32>,
    /// Number of top log probabilities to return.
    pub top_logprobs: Option<i32>,
    /// Base URL for the API endpoint.
    pub base_url: Option<String>,
    /// Alias for base_url (OpenAI compatibility).
    pub api_base: Option<String>,
    /// API version (for Azure and other versioned APIs).
    pub api_version: Option<String>,
    /// API key for authentication.
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
    /// Callbacks to be executed during LLM calls.
    #[serde(skip)]
    pub callbacks: Vec<Box<dyn std::any::Any + Send + Sync>>,
    /// Reasoning effort level for models that support it.
    pub reasoning_effort: Option<ReasoningEffort>,
    /// Whether to enable streaming responses.
    pub stream: bool,
    /// Whether to prefer file upload over inline base64.
    pub prefer_upload: bool,
    /// Override the context window size for the model.
    pub context_window_size: i64,
    /// Additional provider-specific parameters.
    pub additional_params: HashMap<String, Value>,
    /// Whether this model is an Anthropic model.
    pub is_anthropic: bool,
    /// Whether this LLM uses LiteLLM as a backend.
    pub is_litellm: bool,
    /// Explicit provider override (e.g., "openai", "anthropic").
    pub provider: Option<String>,
    /// Completion cost from the last call.
    pub completion_cost: Option<f64>,
}

impl Clone for LLM {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            timeout: self.timeout,
            temperature: self.temperature,
            top_p: self.top_p,
            n: self.n,
            stop: self.stop.clone(),
            max_completion_tokens: self.max_completion_tokens,
            max_tokens: self.max_tokens,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            logit_bias: self.logit_bias.clone(),
            response_format: self.response_format.clone(),
            seed: self.seed,
            logprobs: self.logprobs,
            top_logprobs: self.top_logprobs,
            base_url: self.base_url.clone(),
            api_base: self.api_base.clone(),
            api_version: self.api_version.clone(),
            api_key: self.api_key.clone(),
            callbacks: Vec::new(), // Callbacks cannot be cloned
            reasoning_effort: self.reasoning_effort.clone(),
            stream: self.stream,
            prefer_upload: self.prefer_upload,
            context_window_size: self.context_window_size,
            additional_params: self.additional_params.clone(),
            is_anthropic: self.is_anthropic,
            is_litellm: self.is_litellm,
            provider: self.provider.clone(),
            completion_cost: self.completion_cost,
        }
    }
}

impl Default for LLM {
    fn default() -> Self {
        Self {
            model: String::new(),
            timeout: None,
            temperature: None,
            top_p: None,
            n: None,
            stop: Vec::new(),
            max_completion_tokens: None,
            max_tokens: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            response_format: None,
            seed: None,
            logprobs: None,
            top_logprobs: None,
            base_url: None,
            api_base: None,
            api_version: None,
            api_key: None,
            callbacks: Vec::new(),
            reasoning_effort: None,
            stream: false,
            prefer_upload: false,
            context_window_size: 0,
            additional_params: HashMap::new(),
            is_anthropic: false,
            is_litellm: false,
            provider: None,
            completion_cost: None,
        }
    }
}

impl LLM {
    /// Create a new LLM with a model identifier.
    ///
    /// Corresponds to `LLM.__init__` in Python. In the Python implementation,
    /// `__new__` acts as a factory that routes to native provider classes.
    /// In Rust, the LLM struct stores configuration and delegates to providers.
    ///
    /// # Arguments
    ///
    /// * `model` - Model identifier (e.g., "gpt-4o", "claude-opus-4-5-20251101").
    pub fn new(model: impl Into<String>) -> Self {
        let model = model.into();
        let is_anthropic = Self::_is_anthropic_model(&model);
        Self {
            model,
            is_anthropic,
            ..Default::default()
        }
    }

    /// Create a new LLM with model and explicit provider.
    pub fn with_provider(model: impl Into<String>, provider: impl Into<String>) -> Self {
        let model = model.into();
        let is_anthropic = Self::_is_anthropic_model(&model);
        Self {
            model,
            is_anthropic,
            provider: Some(provider.into()),
            ..Default::default()
        }
    }

    // --- Builder-style setters ---

    /// Set the temperature.
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the API key.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the base URL.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Set the timeout in seconds.
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set max tokens.
    pub fn max_tokens(mut self, max_tokens: i64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Enable streaming.
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Set stop sequences.
    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.stop = stop;
        self
    }

    /// Set reasoning effort.
    pub fn reasoning_effort(mut self, effort: ReasoningEffort) -> Self {
        self.reasoning_effort = Some(effort);
        self
    }

    // --- Anthropic detection ---

    /// Check if a model name is an Anthropic model.
    ///
    /// Corresponds to `LLM._is_anthropic_model` in Python.
    fn _is_anthropic_model(model: &str) -> bool {
        let lower = model.to_lowercase();
        ANTHROPIC_PREFIXES
            .iter()
            .any(|prefix| lower.starts_with(prefix))
            || lower.contains("claude")
    }

    // --- Provider inference ---

    /// Check if a model matches a provider's naming pattern.
    ///
    /// Corresponds to `LLM._matches_provider_pattern` in Python.
    pub fn matches_provider_pattern(model: &str, provider: &str) -> bool {
        let model_lower = model.to_lowercase();

        match provider {
            "openai" => ["gpt-", "o1", "o3", "o4", "whisper-"]
                .iter()
                .any(|p| model_lower.starts_with(p)),
            "anthropic" | "claude" => ["claude-", "anthropic."]
                .iter()
                .any(|p| model_lower.starts_with(p)),
            "gemini" | "google" => ["gemini-", "gemma-", "learnlm-"]
                .iter()
                .any(|p| model_lower.starts_with(p)),
            "bedrock" => model_lower.contains('.'),
            "azure" => ["gpt-", "gpt-35-", "o1", "o3", "o4", "azure-"]
                .iter()
                .any(|p| model_lower.starts_with(p)),
            _ => false,
        }
    }

    /// Infer the provider from the model name.
    ///
    /// This method first checks the explicit provider, then model string prefix,
    /// then falls back to model name pattern matching.
    ///
    /// Corresponds to `LLM._infer_provider_from_model` in Python.
    pub fn infer_provider(&self) -> String {
        // Check explicit provider
        if let Some(ref provider) = self.provider {
            return provider.clone();
        }

        let model_lower = self.model.to_lowercase();

        // Check prefix (e.g., "openai/gpt-4")
        if let Some((prefix, _)) = model_lower.split_once('/') {
            match prefix {
                "openai" => return "openai".to_string(),
                "anthropic" | "claude" => return "anthropic".to_string(),
                "azure" | "azure_openai" => return "azure".to_string(),
                "google" | "gemini" => return "gemini".to_string(),
                "bedrock" | "aws" => return "bedrock".to_string(),
                "xai" | "grok" => return "xai".to_string(),
                _ => {}
            }
        }

        // Check model name patterns
        if model_lower.starts_with("gpt-")
            || model_lower.starts_with("o1")
            || model_lower.starts_with("o3")
            || model_lower.starts_with("o4")
        {
            return "openai".to_string();
        }
        if model_lower.starts_with("claude-") {
            return "anthropic".to_string();
        }
        if model_lower.starts_with("gemini-") || model_lower.starts_with("gemma-") {
            return "gemini".to_string();
        }
        if model_lower.starts_with("mistral") {
            return "mistral".to_string();
        }
        if model_lower.starts_with("grok-") {
            return "xai".to_string();
        }

        // Default to openai
        "openai".to_string()
    }

    // --- Core call methods ---

    /// Call the LLM with a list of messages (synchronous).
    ///
    /// In the full implementation this routes to the appropriate provider
    /// SDK (OpenAI, Anthropic, etc.) or falls back to LiteLLM.
    ///
    /// Corresponds to `LLM.call` in Python.
    ///
    /// # Arguments
    ///
    /// * `messages` - Chat messages as a list of role/content maps.
    /// * `tools` - Optional list of tool definitions for function calling.
    ///
    /// # Returns
    ///
    /// The LLM response string.
    pub fn call(
        &self,
        messages: &[HashMap<String, String>],
        tools: Option<&[Value]>,
    ) -> Result<String, String> {
        let provider = self.infer_provider();
        log::debug!(
            "LLM.call: model={}, provider={}, {} messages, {} tools",
            self.model,
            provider,
            messages.len(),
            tools.map_or(0, |t| t.len())
        );

        // Convert HashMap<String, String> → Vec<LLMMessage> (HashMap<String, Value>)
        let llm_messages: Vec<HashMap<String, Value>> = messages
            .iter()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                    .collect()
            })
            .collect();

        let tools_vec = tools.map(|t| t.to_vec());

        let result = match provider.as_str() {
            "openai" => {
                let completion = OpenAICompletion::new(
                    &self.model,
                    self.api_key.clone(),
                    self.api_base.clone(),
                );
                completion
                    .call(llm_messages, tools_vec, None)
                    .map_err(|e| e.to_string())
            }
            "xai" => {
                let completion = XAICompletion::new(
                    &self.model,
                    self.api_key.clone(),
                    self.api_base.clone(),
                );
                completion
                    .call(llm_messages, tools_vec, None)
                    .map_err(|e| e.to_string())
            }
            other => {
                return Err(format!(
                    "Provider '{}' not yet wired. Supported: openai, xai",
                    other
                ));
            }
        }?;

        // Extract text content from provider response Value
        Self::extract_text_from_response(&result)
    }

    /// Async version of call.
    ///
    /// Corresponds to `LLM.acall` in Python (which wraps sync `call` by default).
    pub async fn acall(
        &self,
        messages: &[HashMap<String, String>],
        tools: Option<&[Value]>,
    ) -> Result<String, String> {
        let provider = self.infer_provider();
        log::debug!(
            "LLM.acall: model={}, provider={}, {} messages, {} tools",
            self.model,
            provider,
            messages.len(),
            tools.map_or(0, |t| t.len())
        );

        // Convert HashMap<String, String> → Vec<LLMMessage> (HashMap<String, Value>)
        let llm_messages: Vec<HashMap<String, Value>> = messages
            .iter()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                    .collect()
            })
            .collect();

        let tools_vec = tools.map(|t| t.to_vec());

        let result = match provider.as_str() {
            "openai" => {
                let completion = OpenAICompletion::new(
                    &self.model,
                    self.api_key.clone(),
                    self.api_base.clone(),
                );
                completion
                    .acall(llm_messages, tools_vec, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            "xai" => {
                let completion = XAICompletion::new(
                    &self.model,
                    self.api_key.clone(),
                    self.api_base.clone(),
                );
                completion
                    .acall(llm_messages, tools_vec, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            other => {
                return Err(format!(
                    "Provider '{}' not yet wired. Supported: openai, xai",
                    other
                ));
            }
        }?;

        Self::extract_text_from_response(&result)
    }

    /// Extract the text content from a provider response Value.
    ///
    /// Providers return a serde_json::Value that may be a plain string,
    /// an object with choices[0].message.content (OpenAI format), or
    /// an object with tool_calls.
    fn extract_text_from_response(response: &Value) -> Result<String, String> {
        // If it's already a string, return it directly
        if let Some(s) = response.as_str() {
            return Ok(s.to_string());
        }
        // Try OpenAI chat completions format: choices[0].message.content
        if let Some(content) = response
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
        {
            return Ok(content.to_string());
        }
        // Try direct content field
        if let Some(content) = response.get("content").and_then(|c| c.as_str()) {
            return Ok(content.to_string());
        }
        // Try tool_calls — return them as JSON for the agent to process
        if let Some(tool_calls) = response.get("tool_calls") {
            return Ok(tool_calls.to_string());
        }
        // Fallback: serialize the whole response
        Ok(response.to_string())
    }

    // --- Capability queries ---

    /// Check if the model supports function calling (tool use).
    ///
    /// This checks against known model families that support function calling.
    pub fn supports_function_calling(&self) -> bool {
        let model_lower = self.model.to_lowercase();

        let function_calling_prefixes = [
            "gpt-4",
            "gpt-3.5-turbo",
            "claude-3",
            "claude-opus-4-5-20251101",
            "gemini",
            "mistral",
            "llama-3",
            "command",
            "o1",
            "o3",
            "o4",
        ];

        function_calling_prefixes
            .iter()
            .any(|prefix| model_lower.starts_with(prefix) || model_lower.contains(prefix))
    }

    // --- Context window ---

    /// Get the context window size for the model.
    ///
    /// Returns the context window size from:
    /// 1. Explicit override (self.context_window_size > 0)
    /// 2. Known model sizes (LLM_CONTEXT_WINDOW_SIZES)
    /// 3. Default (DEFAULT_CONTEXT_WINDOW_SIZE)
    ///
    /// Corresponds to `LLM.get_context_window_size` in Python.
    pub fn get_context_window_size(&self) -> i64 {
        if self.context_window_size > 0 {
            return self.context_window_size.max(MIN_CONTEXT).min(MAX_CONTEXT);
        }

        let sizes = llm_context_window_sizes();

        // Try exact match
        if let Some(&size) = sizes.get(self.model.as_str()) {
            return size;
        }

        // Try without provider prefix (e.g., "openai/gpt-4" -> "gpt-4")
        if let Some((_prefix, model_part)) = self.model.split_once('/') {
            if let Some(&size) = sizes.get(model_part) {
                return size;
            }
        }

        DEFAULT_CONTEXT_WINDOW_SIZE
    }

    /// Get the usable context window size (accounting for the usage ratio).
    pub fn get_usable_context_window_size(&self) -> i64 {
        (self.get_context_window_size() as f64 * CONTEXT_WINDOW_USAGE_RATIO) as i64
    }

    // --- Completion parameters ---

    /// Prepare the completion parameters dict for the LLM call.
    ///
    /// Gathers all configured fields into a single map for passing to
    /// the provider SDK. Corresponds to the parameter preparation logic
    /// in `LLM.call` / `LLM._prepare_completion_params` in Python.
    pub fn prepare_completion_params(&self) -> HashMap<String, Value> {
        let mut params = HashMap::new();

        params.insert("model".to_string(), serde_json::json!(self.model));

        if let Some(temp) = self.temperature {
            params.insert("temperature".to_string(), serde_json::json!(temp));
        }
        if let Some(top_p) = self.top_p {
            params.insert("top_p".to_string(), serde_json::json!(top_p));
        }
        if let Some(n) = self.n {
            params.insert("n".to_string(), serde_json::json!(n));
        }
        if !self.stop.is_empty() {
            params.insert("stop".to_string(), serde_json::json!(self.stop));
        }
        if let Some(max_tokens) = self.max_tokens {
            params.insert("max_tokens".to_string(), serde_json::json!(max_tokens));
        }
        if let Some(max_completion_tokens) = self.max_completion_tokens {
            params.insert(
                "max_completion_tokens".to_string(),
                serde_json::json!(max_completion_tokens),
            );
        }
        if let Some(presence_penalty) = self.presence_penalty {
            params.insert(
                "presence_penalty".to_string(),
                serde_json::json!(presence_penalty),
            );
        }
        if let Some(frequency_penalty) = self.frequency_penalty {
            params.insert(
                "frequency_penalty".to_string(),
                serde_json::json!(frequency_penalty),
            );
        }
        if let Some(ref logit_bias) = self.logit_bias {
            params.insert("logit_bias".to_string(), serde_json::json!(logit_bias));
        }
        if let Some(ref response_format) = self.response_format {
            params.insert("response_format".to_string(), response_format.clone());
        }
        if let Some(seed) = self.seed {
            params.insert("seed".to_string(), serde_json::json!(seed));
        }
        if let Some(logprobs) = self.logprobs {
            params.insert("logprobs".to_string(), serde_json::json!(logprobs));
        }
        if let Some(top_logprobs) = self.top_logprobs {
            params.insert("top_logprobs".to_string(), serde_json::json!(top_logprobs));
        }
        if let Some(ref effort) = self.reasoning_effort {
            params.insert(
                "reasoning_effort".to_string(),
                serde_json::json!(effort.to_string()),
            );
        }
        if self.stream {
            params.insert("stream".to_string(), serde_json::json!(true));
        }
        if let Some(timeout) = self.timeout {
            params.insert("timeout".to_string(), serde_json::json!(timeout));
        }
        if let Some(ref api_key) = self.api_key {
            params.insert("api_key".to_string(), serde_json::json!(api_key));
        }
        if let Some(ref base_url) = self.base_url.as_ref().or(self.api_base.as_ref()) {
            params.insert("api_base".to_string(), serde_json::json!(base_url));
        }
        if let Some(ref api_version) = self.api_version {
            params.insert("api_version".to_string(), serde_json::json!(api_version));
        }

        // Include additional params
        for (key, value) in &self.additional_params {
            params.insert(key.clone(), value.clone());
        }

        params
    }
}

/// BaseLLM trait providing the interface for all LLM implementations.
///
/// This is a simplified trait used when the LLM struct itself is used as a
/// trait object. For the full provider-level trait, see
/// [`crate::llms::base_llm::BaseLLM`].
///
/// Corresponds to `crewai/llms/base_llm.py::BaseLLM`.
#[async_trait]
pub trait BaseLLMTrait: Send + Sync {
    /// Call the LLM with messages.
    fn call(
        &self,
        messages: &[HashMap<String, String>],
        tools: Option<&[Value]>,
    ) -> Result<String, String>;

    /// Async call the LLM with messages.
    async fn acall(
        &self,
        messages: &[HashMap<String, String>],
        tools: Option<&[Value]>,
    ) -> Result<String, String>;

    /// Check if the model supports function calling.
    fn supports_function_calling(&self) -> bool;

    /// Get the model name.
    fn model_name(&self) -> &str;

    /// Get the context window size.
    fn get_context_window_size(&self) -> i64;
}

#[async_trait]
impl BaseLLMTrait for LLM {
    fn call(
        &self,
        messages: &[HashMap<String, String>],
        tools: Option<&[Value]>,
    ) -> Result<String, String> {
        LLM::call(self, messages, tools)
    }

    async fn acall(
        &self,
        messages: &[HashMap<String, String>],
        tools: Option<&[Value]>,
    ) -> Result<String, String> {
        LLM::acall(self, messages, tools).await
    }

    fn supports_function_calling(&self) -> bool {
        LLM::supports_function_calling(self)
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn get_context_window_size(&self) -> i64 {
        LLM::get_context_window_size(self)
    }
}

impl std::fmt::Display for LLM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LLM(model={})", self.model)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_new() {
        let llm = LLM::new("gpt-4o");
        assert_eq!(llm.model, "gpt-4o");
        assert!(!llm.is_anthropic);
        assert!(!llm.is_litellm);
        assert_eq!(llm.context_window_size, 0);
    }

    #[test]
    fn test_llm_new_anthropic() {
        let llm = LLM::new("claude-opus-4-5-20251101");
        assert!(llm.is_anthropic);
    }

    #[test]
    fn test_llm_with_provider() {
        let llm = LLM::with_provider("my-model", "anthropic");
        assert_eq!(llm.provider, Some("anthropic".to_string()));
    }

    #[test]
    fn test_builder_pattern() {
        let llm = LLM::new("gpt-4o")
            .temperature(0.7)
            .max_tokens(1000)
            .stream(true)
            .api_key("test-key");
        assert_eq!(llm.temperature, Some(0.7));
        assert_eq!(llm.max_tokens, Some(1000));
        assert!(llm.stream);
        assert_eq!(llm.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_is_anthropic_model() {
        assert!(LLM::_is_anthropic_model("claude-opus-4-5-20251101"));
        assert!(LLM::_is_anthropic_model("anthropic/claude-opus-4-5-20251101"));
        assert!(LLM::_is_anthropic_model("claude/claude-3"));
        assert!(!LLM::_is_anthropic_model("gpt-4o"));
        assert!(!LLM::_is_anthropic_model("gemini-2.0-flash"));
    }

    #[test]
    fn test_infer_provider_explicit() {
        let llm = LLM::with_provider("my-model", "bedrock");
        assert_eq!(llm.infer_provider(), "bedrock");
    }

    #[test]
    fn test_infer_provider_from_prefix() {
        let llm = LLM::new("openai/gpt-4o");
        assert_eq!(llm.infer_provider(), "openai");

        let llm = LLM::new("anthropic/claude-3");
        assert_eq!(llm.infer_provider(), "anthropic");

        let llm = LLM::new("azure/gpt-4");
        assert_eq!(llm.infer_provider(), "azure");

        let llm = LLM::new("google/gemini-2.0-flash");
        assert_eq!(llm.infer_provider(), "gemini");

        let llm = LLM::new("bedrock/anthropic.claude-3");
        assert_eq!(llm.infer_provider(), "bedrock");
    }

    #[test]
    fn test_infer_provider_from_model_name() {
        let llm = LLM::new("gpt-4o");
        assert_eq!(llm.infer_provider(), "openai");

        let llm = LLM::new("claude-opus-4-5-20251101");
        assert_eq!(llm.infer_provider(), "anthropic");

        let llm = LLM::new("gemini-2.0-flash");
        assert_eq!(llm.infer_provider(), "gemini");

        let llm = LLM::new("o3-mini");
        assert_eq!(llm.infer_provider(), "openai");

        let llm = LLM::new("mistral-large-latest");
        assert_eq!(llm.infer_provider(), "mistral");
    }

    #[test]
    fn test_matches_provider_pattern() {
        assert!(LLM::matches_provider_pattern("gpt-4o", "openai"));
        assert!(LLM::matches_provider_pattern("o3-mini", "openai"));
        assert!(LLM::matches_provider_pattern("claude-opus-4-5-20251101", "anthropic"));
        assert!(LLM::matches_provider_pattern("gemini-2.0-flash", "gemini"));
        assert!(LLM::matches_provider_pattern("anthropic.claude-3", "bedrock"));
        assert!(!LLM::matches_provider_pattern("gpt-4o", "anthropic"));
    }

    #[test]
    fn test_context_window_size_exact() {
        let llm = LLM::new("gpt-4o");
        assert_eq!(llm.get_context_window_size(), 128000);

        let llm = LLM::new("gemini-1.5-pro");
        assert_eq!(llm.get_context_window_size(), 2097152);
    }

    #[test]
    fn test_context_window_size_prefix() {
        let llm = LLM::new("openai/gpt-4o");
        assert_eq!(llm.get_context_window_size(), 128000);
    }

    #[test]
    fn test_context_window_size_override() {
        let mut llm = LLM::new("gpt-4");
        llm.context_window_size = 16384;
        assert_eq!(llm.get_context_window_size(), 16384);
    }

    #[test]
    fn test_context_window_size_override_clamped() {
        let mut llm = LLM::new("gpt-4");
        llm.context_window_size = 100; // below MIN_CONTEXT
        assert_eq!(llm.get_context_window_size(), MIN_CONTEXT);

        llm.context_window_size = 999_999_999; // above MAX_CONTEXT
        assert_eq!(llm.get_context_window_size(), MAX_CONTEXT);
    }

    #[test]
    fn test_context_window_size_default() {
        let llm = LLM::new("unknown-model-xyz");
        assert_eq!(llm.get_context_window_size(), DEFAULT_CONTEXT_WINDOW_SIZE);
    }

    #[test]
    fn test_usable_context_window_size() {
        let llm = LLM::new("gpt-4o");
        let usable = llm.get_usable_context_window_size();
        assert_eq!(usable, (128000_f64 * 0.85) as i64);
    }

    #[test]
    fn test_supports_function_calling() {
        assert!(LLM::new("gpt-4o").supports_function_calling());
        assert!(LLM::new("claude-opus-4-5-20251101").supports_function_calling());
        assert!(LLM::new("gemini-2.0-flash").supports_function_calling());
        assert!(LLM::new("o3-mini").supports_function_calling());
    }

    #[test]
    fn test_prepare_completion_params() {
        let llm = LLM::new("gpt-4o")
            .temperature(0.5)
            .max_tokens(500)
            .stream(true);

        let params = llm.prepare_completion_params();
        assert_eq!(params["model"], serde_json::json!("gpt-4o"));
        assert_eq!(params["temperature"], serde_json::json!(0.5));
        assert_eq!(params["max_tokens"], serde_json::json!(500));
        assert_eq!(params["stream"], serde_json::json!(true));
    }

    #[test]
    fn test_display() {
        let llm = LLM::new("gpt-4o");
        assert_eq!(format!("{}", llm), "LLM(model=gpt-4o)");
    }

    #[test]
    fn test_reasoning_effort_display() {
        assert_eq!(ReasoningEffort::None.to_string(), "none");
        assert_eq!(ReasoningEffort::Low.to_string(), "low");
        assert_eq!(ReasoningEffort::Medium.to_string(), "medium");
        assert_eq!(ReasoningEffort::High.to_string(), "high");
    }

    #[test]
    fn test_supported_native_providers() {
        assert!(SUPPORTED_NATIVE_PROVIDERS.contains(&"openai"));
        assert!(SUPPORTED_NATIVE_PROVIDERS.contains(&"anthropic"));
        assert!(SUPPORTED_NATIVE_PROVIDERS.contains(&"azure"));
        assert!(SUPPORTED_NATIVE_PROVIDERS.contains(&"gemini"));
        assert!(SUPPORTED_NATIVE_PROVIDERS.contains(&"bedrock"));
    }
}
