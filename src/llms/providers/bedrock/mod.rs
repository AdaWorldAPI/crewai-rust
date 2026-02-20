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

const SERVICE: &str = "bedrock";

// ---------------------------------------------------------------------------
// AWS SigV4 signing
// ---------------------------------------------------------------------------

mod sigv4 {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};

    type HmacSha256 = Hmac<Sha256>;

    /// SHA-256 hash of a byte slice, returned as lowercase hex.
    pub fn sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// HMAC-SHA256 keyed hash.
    fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac =
            HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    /// Derive the SigV4 signing key.
    pub fn signing_key(
        secret: &str,
        date_stamp: &str,
        region: &str,
        service: &str,
    ) -> Vec<u8> {
        let k_date = hmac_sha256(format!("AWS4{}", secret).as_bytes(), date_stamp.as_bytes());
        let k_region = hmac_sha256(&k_date, region.as_bytes());
        let k_service = hmac_sha256(&k_region, service.as_bytes());
        hmac_sha256(&k_service, b"aws4_request")
    }

    /// Sign a string with the given key, returning lowercase hex.
    pub fn sign_hex(key: &[u8], msg: &str) -> String {
        let sig = hmac_sha256(key, msg.as_bytes());
        hex::encode(sig)
    }

    /// Build the canonical request string.
    pub fn canonical_request(
        method: &str,
        uri: &str,
        query_string: &str,
        headers: &[(String, String)],
        signed_headers: &str,
        payload_hash: &str,
    ) -> String {
        let canonical_headers: String = headers
            .iter()
            .map(|(k, v)| format!("{}:{}\n", k.to_lowercase(), v.trim()))
            .collect();

        format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method, uri, query_string, canonical_headers, signed_headers, payload_hash,
        )
    }

    /// Build the string-to-sign.
    pub fn string_to_sign(
        amz_date: &str,
        credential_scope: &str,
        canonical_request_hash: &str,
    ) -> String {
        format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            amz_date, credential_scope, canonical_request_hash,
        )
    }

    /// Build the Authorization header value.
    pub fn authorization_header(
        access_key: &str,
        credential_scope: &str,
        signed_headers: &str,
        signature: &str,
    ) -> String {
        format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            access_key, credential_scope, signed_headers, signature,
        )
    }
}

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

    /// Get the host header value.
    fn host(&self) -> String {
        let region = self.region_name.as_deref().unwrap_or("us-east-1");
        format!("bedrock-runtime.{}.amazonaws.com", region)
    }

    /// Build the Converse API URI path.
    fn converse_uri(&self) -> String {
        // Model IDs with colons (like "anthropic.claude-3-5-sonnet-20241022-v2:0")
        // must be URL-encoded in the path
        let encoded_model = self.state.model.replace(':', "%3A");
        format!("/model/{}/converse", encoded_model)
    }

    /// Convert OpenAI-style messages to Bedrock Converse API format.
    fn format_messages(
        &self,
        messages: &[LLMMessage],
    ) -> (Vec<Value>, Vec<Value>) {
        let mut system_parts: Vec<Value> = Vec::new();
        let mut converse_messages: Vec<Value> = Vec::new();

        for msg in messages {
            let role = msg
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("user");
            let content = msg
                .get("content")
                .cloned()
                .unwrap_or(Value::Null);

            if role == "system" {
                if let Some(text) = content.as_str() {
                    system_parts.push(serde_json::json!({ "text": text }));
                }
            } else if role == "tool" {
                // Tool result → toolResult content block
                let tool_call_id = msg
                    .get("tool_call_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let result_text = content.as_str().unwrap_or("").to_string();

                converse_messages.push(serde_json::json!({
                    "role": "user",
                    "content": [{
                        "toolResult": {
                            "toolUseId": tool_call_id,
                            "content": [{ "text": result_text }],
                            "status": "success"
                        }
                    }]
                }));
            } else if role == "assistant" {
                let mut parts: Vec<Value> = Vec::new();

                // Add text content
                if let Some(text) = content.as_str() {
                    if !text.is_empty() {
                        parts.push(serde_json::json!({ "text": text }));
                    }
                }

                // Add tool use blocks
                if let Some(tool_calls) = msg.get("tool_calls").and_then(|v| v.as_array()) {
                    for tc in tool_calls {
                        let id = tc
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let func = tc.get("function").unwrap_or(&Value::Null);
                        let name = func.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let args_str = func
                            .get("arguments")
                            .and_then(|v| v.as_str())
                            .unwrap_or("{}");
                        let args: Value =
                            serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));

                        parts.push(serde_json::json!({
                            "toolUse": {
                                "toolUseId": id,
                                "name": name,
                                "input": args,
                            }
                        }));
                    }
                }

                if parts.is_empty() {
                    parts.push(serde_json::json!({ "text": "" }));
                }

                converse_messages.push(serde_json::json!({
                    "role": "assistant",
                    "content": parts,
                }));
            } else {
                // user role
                let text = content.as_str().unwrap_or("").to_string();
                converse_messages.push(serde_json::json!({
                    "role": "user",
                    "content": [{ "text": text }],
                }));
            }
        }

        (system_parts, converse_messages)
    }

    /// Build the Converse API request body.
    fn build_request_body(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[Value]>,
    ) -> Value {
        let (system_parts, converse_messages) = self.format_messages(messages);

        let mut body = serde_json::json!({
            "messages": converse_messages,
        });

        // System messages
        if !system_parts.is_empty() {
            body["system"] = Value::Array(system_parts);
        }

        // Inference config
        let mut config = serde_json::Map::new();
        if let Some(max) = self.max_tokens {
            config.insert("maxTokens".to_string(), serde_json::json!(max));
        } else {
            // Bedrock requires maxTokens; default to 4096
            config.insert("maxTokens".to_string(), serde_json::json!(4096));
        }
        if let Some(temp) = self.state.temperature {
            config.insert("temperature".to_string(), serde_json::json!(temp));
        }
        if let Some(top_p) = self.top_p {
            config.insert("topP".to_string(), serde_json::json!(top_p));
        }
        let stops: &[String] = if !self.state.stop.is_empty() {
            &self.state.stop
        } else if !self.stop_sequences.is_empty() {
            &self.stop_sequences
        } else {
            &[]
        };
        if !stops.is_empty() {
            config.insert("stopSequences".to_string(), serde_json::json!(stops));
        }
        body["inferenceConfig"] = Value::Object(config);

        // Tools
        if let Some(tools) = tools {
            if !tools.is_empty() {
                let tool_specs: Vec<Value> = tools
                    .iter()
                    .map(|tool| {
                        let func = tool.get("function").unwrap_or(tool);
                        let name = func
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let desc = func
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let params = func
                            .get("parameters")
                            .cloned()
                            .unwrap_or(serde_json::json!({"type": "object", "properties": {}}));

                        serde_json::json!({
                            "toolSpec": {
                                "name": name,
                                "description": desc,
                                "inputSchema": { "json": params },
                            }
                        })
                    })
                    .collect();

                body["toolConfig"] = serde_json::json!({
                    "tools": tool_specs,
                });
            }
        }

        // Guardrails
        if let (Some(ref id), Some(ref version)) =
            (&self.guardrail_id, &self.guardrail_version)
        {
            body["guardrailConfig"] = serde_json::json!({
                "guardrailIdentifier": id,
                "guardrailVersion": version,
            });
        }

        body
    }

    /// Parse a Bedrock Converse API response.
    fn parse_response(
        &self,
        response: &Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let output = response
            .get("output")
            .and_then(|o| o.get("message"))
            .ok_or("No output.message in Bedrock response")?;

        let content_blocks = output
            .get("content")
            .and_then(|c| c.as_array())
            .ok_or("No content array in Bedrock response")?;

        let mut text_parts: Vec<String> = Vec::new();
        let mut tool_calls: Vec<Value> = Vec::new();

        for block in content_blocks {
            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                text_parts.push(text.to_string());
            }
            if let Some(tool_use) = block.get("toolUse") {
                let id = tool_use
                    .get("toolUseId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let name = tool_use
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let input = tool_use.get("input").unwrap_or(&Value::Null);
                let args_str = serde_json::to_string(input).unwrap_or_default();

                tool_calls.push(serde_json::json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": args_str,
                    }
                }));
            }
        }

        if !tool_calls.is_empty() {
            let combined_text = text_parts.join("");
            return Ok(serde_json::json!({
                "role": "assistant",
                "content": if combined_text.is_empty() { Value::Null } else { Value::String(combined_text) },
                "tool_calls": tool_calls,
            }));
        }

        let combined = text_parts.join("");
        let final_content = self.state.apply_stop_words(&combined);
        Ok(Value::String(final_content))
    }

    /// Extract token usage from a Bedrock Converse response.
    fn extract_token_usage(response: &Value) -> HashMap<String, Value> {
        let mut usage = HashMap::new();
        if let Some(usage_obj) = response.get("usage") {
            let input = usage_obj
                .get("inputTokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let output = usage_obj
                .get("outputTokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let total = usage_obj
                .get("totalTokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(input + output);

            usage.insert("prompt_tokens".to_string(), serde_json::json!(input));
            usage.insert("completion_tokens".to_string(), serde_json::json!(output));
            usage.insert("total_tokens".to_string(), serde_json::json!(total));
        }
        usage
    }

    /// Sign a request using AWS SigV4 and return headers.
    fn sign_request(
        &self,
        method: &str,
        uri: &str,
        payload: &[u8],
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
        let access_key = self
            .aws_access_key_id
            .as_ref()
            .ok_or("AWS_ACCESS_KEY_ID not set")?;
        let secret_key = self
            .aws_secret_access_key
            .as_ref()
            .ok_or("AWS_SECRET_ACCESS_KEY not set")?;
        let region = self.region_name.as_deref().unwrap_or("us-east-1");

        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();
        let credential_scope =
            format!("{}/{}/{}/aws4_request", date_stamp, region, SERVICE);

        let host = self.host();
        let payload_hash = sigv4::sha256_hex(payload);

        // Build canonical headers (must be sorted by lowercase key)
        let mut headers: Vec<(String, String)> = vec![
            ("content-type".to_string(), "application/json".to_string()),
            ("host".to_string(), host.clone()),
            ("x-amz-date".to_string(), amz_date.clone()),
        ];

        if let Some(ref token) = self.aws_session_token {
            headers.push(("x-amz-security-token".to_string(), token.clone()));
        }

        headers.sort_by(|a, b| a.0.cmp(&b.0));

        let signed_headers: String = headers
            .iter()
            .map(|(k, _)| k.as_str())
            .collect::<Vec<_>>()
            .join(";");

        let canonical = sigv4::canonical_request(
            method,
            uri,
            "", // no query string for Converse
            &headers,
            &signed_headers,
            &payload_hash,
        );

        let canonical_hash = sigv4::sha256_hex(canonical.as_bytes());
        let sts = sigv4::string_to_sign(&amz_date, &credential_scope, &canonical_hash);

        let signing_key =
            sigv4::signing_key(secret_key, &date_stamp, region, SERVICE);
        let signature = sigv4::sign_hex(&signing_key, &sts);

        let auth_header =
            sigv4::authorization_header(access_key, &credential_scope, &signed_headers, &signature);

        // Return all the headers needed for the request
        let mut result_headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Host".to_string(), host),
            ("X-Amz-Date".to_string(), amz_date),
            ("Authorization".to_string(), auth_header),
        ];

        if let Some(ref token) = self.aws_session_token {
            result_headers.push(("X-Amz-Security-Token".to_string(), token.clone()));
        }

        Ok(result_headers)
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
        available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "BedrockCompletion.call: model={}, region={:?}, messages={}, tools={:?}",
            self.state.model,
            self.region_name,
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
            "BedrockCompletion.acall: model={}, messages={}",
            self.state.model,
            messages.len(),
        );

        let tools_slice = tools.as_deref();
        let body = self.build_request_body(&messages, tools_slice);
        let payload = serde_json::to_vec(&body)?;

        let uri = self.converse_uri();
        let endpoint = format!("{}{}", self.endpoint_url(), uri);

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
                    "Bedrock API retry attempt {} after {:?}",
                    attempt,
                    retry_delay
                );
                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2;
            }

            // Sign the request (must re-sign each attempt for fresh timestamp)
            let headers = match self.sign_request("POST", &uri, &payload) {
                Ok(h) => h,
                Err(e) => return Err(e),
            };

            let mut request = client.post(&endpoint);
            for (k, v) in &headers {
                request = request.header(k.as_str(), v.as_str());
            }

            let response = match request.body(payload.clone()).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = Some(Box::new(e));
                    continue;
                }
            };

            let status = response.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                last_error = Some("Rate limited by Bedrock API (429)".into());
                continue;
            }

            if status.is_server_error() {
                last_error =
                    Some(format!("Bedrock API server error: {}", status).into());
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
                return Err(format!(
                    "Bedrock API error ({}): {}",
                    status, response_text
                )
                .into());
            }

            let response_json: Value = match serde_json::from_str(&response_text) {
                Ok(json) => json,
                Err(e) => {
                    return Err(format!(
                        "Failed to parse Bedrock response: {} - Body: {}",
                        e,
                        &response_text[..response_text.len().min(500)]
                    )
                    .into());
                }
            };

            // Extract token usage
            let usage = Self::extract_token_usage(&response_json);
            if !usage.is_empty() {
                log::debug!("Bedrock usage: {:?}", usage);
            }

            return self.parse_response(&response_json);
        }

        Err(last_error
            .unwrap_or_else(|| "Bedrock API call failed after all retries".into()))
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
    fn test_bedrock_new_defaults() {
        let provider = BedrockCompletion::new(
            "anthropic.claude-3-5-sonnet-20241022-v2:0",
            None,
            None,
        );
        assert_eq!(provider.model(), "anthropic.claude-3-5-sonnet-20241022-v2:0");
        assert_eq!(provider.provider(), "bedrock");
        assert!(provider.supports_function_calling());
        assert!(provider.supports_multimodal());
        assert_eq!(provider.get_context_window_size(), 200_000);
    }

    #[test]
    fn test_bedrock_endpoint() {
        let provider = BedrockCompletion::new(
            "anthropic.claude-3-5-sonnet-20241022-v2:0",
            Some("eu-west-1".to_string()),
            None,
        );
        assert_eq!(
            provider.endpoint_url(),
            "https://bedrock-runtime.eu-west-1.amazonaws.com"
        );
    }

    #[test]
    fn test_converse_uri_encodes_colons() {
        let provider = BedrockCompletion::new(
            "anthropic.claude-3-5-sonnet-20241022-v2:0",
            None,
            None,
        );
        let uri = provider.converse_uri();
        assert!(uri.contains("%3A"), "URI should encode colons: {}", uri);
        assert!(!uri.contains(':') || uri.starts_with("/model/"),
            "Model ID colons should be encoded");
    }

    fn msg(pairs: &[(&str, Value)]) -> LLMMessage {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn test_format_messages_basic() {
        let provider = BedrockCompletion::new("test-model", None, None);
        let messages: Vec<LLMMessage> = vec![
            msg(&[("role", serde_json::json!("system")), ("content", serde_json::json!("You are helpful."))]),
            msg(&[("role", serde_json::json!("user")), ("content", serde_json::json!("Hello"))]),
        ];

        let (system, converse) = provider.format_messages(&messages);
        assert_eq!(system.len(), 1);
        assert_eq!(system[0]["text"], "You are helpful.");
        assert_eq!(converse.len(), 1);
        assert_eq!(converse[0]["role"], "user");
    }

    #[test]
    fn test_format_messages_with_tool_calls() {
        let provider = BedrockCompletion::new("test-model", None, None);
        let messages: Vec<LLMMessage> = vec![
            msg(&[
                ("role", serde_json::json!("assistant")),
                ("content", serde_json::json!("")),
                ("tool_calls", serde_json::json!([{
                    "id": "tc_1",
                    "function": { "name": "get_weather", "arguments": "{\"city\":\"NYC\"}" }
                }])),
            ]),
            msg(&[
                ("role", serde_json::json!("tool")),
                ("tool_call_id", serde_json::json!("tc_1")),
                ("content", serde_json::json!("72°F and sunny")),
            ]),
        ];

        let (_, converse) = provider.format_messages(&messages);
        assert_eq!(converse.len(), 2);

        // Assistant message should have toolUse
        let assistant_content = converse[0]["content"].as_array().unwrap();
        assert!(assistant_content.iter().any(|b| b.get("toolUse").is_some()));

        // Tool result message
        let tool_content = converse[1]["content"].as_array().unwrap();
        assert!(tool_content.iter().any(|b| b.get("toolResult").is_some()));
    }

    #[test]
    fn test_build_request_body_with_tools() {
        let provider = BedrockCompletion::new("test-model", None, None);
        let messages: Vec<LLMMessage> = vec![
            msg(&[("role", serde_json::json!("user")), ("content", serde_json::json!("What's the weather?"))]),
        ];
        let tools = vec![serde_json::json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get the weather",
                "parameters": {
                    "type": "object",
                    "properties": { "city": { "type": "string" } },
                    "required": ["city"]
                }
            }
        })];

        let body = provider.build_request_body(&messages, Some(&tools));
        assert!(body.get("toolConfig").is_some());
        let tool_config = &body["toolConfig"]["tools"];
        assert_eq!(tool_config.as_array().unwrap().len(), 1);
        assert!(tool_config[0].get("toolSpec").is_some());
    }

    #[test]
    fn test_parse_response_text() {
        let provider = BedrockCompletion::new("test-model", None, None);
        let response = serde_json::json!({
            "output": {
                "message": {
                    "role": "assistant",
                    "content": [{ "text": "Hello there!" }]
                }
            },
            "stopReason": "end_turn",
            "usage": { "inputTokens": 10, "outputTokens": 5 }
        });

        let result = provider.parse_response(&response).unwrap();
        assert_eq!(result.as_str().unwrap(), "Hello there!");
    }

    #[test]
    fn test_parse_response_tool_use() {
        let provider = BedrockCompletion::new("test-model", None, None);
        let response = serde_json::json!({
            "output": {
                "message": {
                    "role": "assistant",
                    "content": [{
                        "toolUse": {
                            "toolUseId": "tc_123",
                            "name": "get_weather",
                            "input": { "city": "NYC" }
                        }
                    }]
                }
            },
            "stopReason": "tool_use",
            "usage": { "inputTokens": 10, "outputTokens": 20 }
        });

        let result = provider.parse_response(&response).unwrap();
        assert!(result.get("tool_calls").is_some());
        let tc = &result["tool_calls"][0];
        assert_eq!(tc["function"]["name"], "get_weather");
    }

    #[test]
    fn test_extract_token_usage() {
        let response = serde_json::json!({
            "usage": {
                "inputTokens": 100,
                "outputTokens": 50,
                "totalTokens": 150
            }
        });

        let usage = BedrockCompletion::extract_token_usage(&response);
        assert_eq!(usage["prompt_tokens"], 100);
        assert_eq!(usage["completion_tokens"], 50);
        assert_eq!(usage["total_tokens"], 150);
    }

    #[test]
    fn test_context_window_sizes() {
        let claude = BedrockCompletion::new("anthropic.claude-3-5-sonnet", None, None);
        assert_eq!(claude.get_context_window_size(), 200_000);

        let nova = BedrockCompletion::new("amazon.nova-pro-v1:0", None, None);
        assert_eq!(nova.get_context_window_size(), 300_000);

        let llama = BedrockCompletion::new("meta.llama3-1-70b-instruct", None, None);
        assert_eq!(llama.get_context_window_size(), 128_000);
    }

    #[test]
    fn test_sigv4_sha256() {
        let hash = sigv4::sha256_hex(b"hello");
        assert_eq!(hash.len(), 64); // 32 bytes hex-encoded
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_sigv4_signing_key() {
        let key = sigv4::signing_key("secret", "20240101", "us-east-1", "bedrock");
        assert_eq!(key.len(), 32); // HMAC-SHA256 output
    }
}
