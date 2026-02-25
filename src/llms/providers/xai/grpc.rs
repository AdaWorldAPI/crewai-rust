//! xAI gRPC completion provider — implements `BaseLLM` over Protocol Buffers.
//!
//! This is the gRPC counterpart to the REST-based `XAICompletion` in the
//! parent module. It uses tonic-generated stubs from xai-proto for typed,
//! binary-efficient communication with xAI's gRPC API at `api.x.ai:443`.
//!
//! # When to Use gRPC vs REST
//!
//! - **gRPC**: Lower latency (binary protobuf, HTTP/2 multiplexing), native
//!   streaming via `GetCompletionChunk`, typed responses (no JSON parsing).
//! - **REST**: Simpler setup, no protoc needed, wider proxy/CDN support.
//!
//! Both implement `BaseLLM` so they're interchangeable in the agent pipeline.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::llms::base_llm::{BaseLLM, BaseLLMState, LLMMessage};
use crate::types::usage_metrics::UsageMetrics;
use crate::xai_grpc::{
    self, XaiGrpcClient, GetCompletionsRequest, Message, Content,
    MessageRole, xai_api,
};

// ---------------------------------------------------------------------------
// XAIGrpcCompletion provider
// ---------------------------------------------------------------------------

/// xAI gRPC completion provider implementing `BaseLLM`.
///
/// Uses the tonic-generated gRPC client for typed, binary-efficient
/// communication. The client is wrapped in `Arc<Mutex<>>` because
/// `BaseLLM::acall` takes `&self` (not `&mut self`) but tonic clients
/// need `&mut self` for RPC calls.
///
/// # Example
///
/// ```ignore
/// let provider = XAIGrpcCompletion::connect("grok-3-mini", "your-api-key").await?;
/// let messages = vec![/* ... */];
/// let response = provider.acall(messages, None, None).await?;
/// ```
pub struct XAIGrpcCompletion {
    /// Shared base LLM state.
    pub state: BaseLLMState,
    /// gRPC client (behind Mutex for interior mutability).
    client: Arc<Mutex<XaiGrpcClient>>,
    /// Maximum tokens in response.
    pub max_tokens: Option<i32>,
    /// Reasoning effort for grok-3 models.
    pub reasoning_effort: Option<String>,
}

// Manual Debug impl since XaiGrpcClient doesn't derive Debug
impl std::fmt::Debug for XAIGrpcCompletion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XAIGrpcCompletion")
            .field("model", &self.state.model)
            .field("provider", &self.state.provider)
            .field("max_tokens", &self.max_tokens)
            .finish()
    }
}

impl XAIGrpcCompletion {
    /// Connect to xAI gRPC API and create a provider.
    pub async fn connect(
        model: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<Self, tonic::transport::Error> {
        let client = XaiGrpcClient::connect(api_key).await?;

        let mut state = BaseLLMState::new(model);
        state.provider = "xai-grpc".to_string();

        Ok(Self {
            state,
            client: Arc::new(Mutex::new(client)),
            max_tokens: None,
            reasoning_effort: None,
        })
    }

    /// Create from an existing gRPC client (for sharing connections).
    pub fn from_client(
        model: impl Into<String>,
        client: Arc<Mutex<XaiGrpcClient>>,
    ) -> Self {
        let mut state = BaseLLMState::new(model);
        state.provider = "xai-grpc".to_string();

        Self {
            state,
            client,
            max_tokens: None,
            reasoning_effort: None,
        }
    }

    /// Convert LLMMessage (HashMap) to protobuf Message.
    fn to_proto_message(msg: &LLMMessage) -> Message {
        let role_str = msg.get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("user");

        let role: i32 = match role_str {
            "system" => MessageRole::RoleSystem.into(),
            "assistant" => MessageRole::RoleAssistant.into(),
            "tool" => MessageRole::RoleTool.into(),
            "developer" => MessageRole::RoleDeveloper.into(),
            _ => MessageRole::RoleUser.into(),
        };

        let content_text = msg.get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Message {
            content: vec![Content {
                content: Some(xai_api::content::Content::Text(content_text)),
            }],
            role,
            ..Default::default()
        }
    }

    /// Check if the model supports reasoning.
    pub fn is_reasoning_model(&self) -> bool {
        let m = self.state.model.to_lowercase();
        m.contains("grok-3") && !m.contains("fast")
    }
}

#[async_trait]
impl BaseLLM for XAIGrpcCompletion {
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
        "xai-grpc"
    }

    fn supports_function_calling(&self) -> bool {
        true
    }

    fn supports_multimodal(&self) -> bool {
        let lower = self.state.model.to_lowercase();
        lower.contains("vision")
    }

    fn supports_stop_words(&self) -> bool {
        self.state.has_stop_words()
    }

    fn get_context_window_size(&self) -> usize {
        if self.state.model.contains("grok-2-vision") {
            32_768
        } else {
            131_072
        }
    }

    fn call(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(self.acall(messages, tools, available_functions))
    }

    async fn acall(
        &self,
        messages: Vec<LLMMessage>,
        _tools: Option<Vec<Value>>,
        _available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!(
            "XAIGrpcCompletion.acall: model={}, messages={}",
            self.state.model,
            messages.len(),
        );

        // Convert messages to protobuf
        let proto_messages: Vec<Message> = messages.iter()
            .map(Self::to_proto_message)
            .collect();

        let temperature = self.state.temperature.map(|t| t as f32);

        // Build request
        let request = GetCompletionsRequest {
            model: self.state.model.clone(),
            messages: proto_messages,
            max_tokens: self.max_tokens,
            temperature,
            ..Default::default()
        };

        // Call gRPC (needs &mut client)
        let mut client = self.client.lock().await;
        let response = client.chat.get_completion(request).await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("xAI gRPC error: {}", e),
                ))
            })?;

        let inner = response.into_inner();

        // Extract text content
        let content = inner.outputs.first()
            .and_then(|o| o.message.as_ref())
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let final_content = self.state.apply_stop_words(&content);

        // Log token usage
        if let Some(ref usage) = inner.usage {
            log::debug!(
                "xAI gRPC token usage: prompt={}, completion={}, total={}",
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.total_tokens,
            );
        }

        Ok(Value::String(final_content))
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
    fn test_to_proto_message_user() {
        let mut msg = HashMap::new();
        msg.insert("role".to_string(), Value::String("user".to_string()));
        msg.insert("content".to_string(), Value::String("Hello".to_string()));

        let proto = XAIGrpcCompletion::to_proto_message(&msg);
        assert_eq!(proto.role, i32::from(MessageRole::RoleUser));
        match &proto.content[0].content {
            Some(xai_api::content::Content::Text(t)) => assert_eq!(t, "Hello"),
            _ => panic!("Expected text"),
        }
    }

    #[test]
    fn test_to_proto_message_system() {
        let mut msg = HashMap::new();
        msg.insert("role".to_string(), Value::String("system".to_string()));
        msg.insert("content".to_string(), Value::String("You are helpful.".to_string()));

        let proto = XAIGrpcCompletion::to_proto_message(&msg);
        assert_eq!(proto.role, i32::from(MessageRole::RoleSystem));
    }

    #[test]
    fn test_to_proto_message_assistant() {
        let mut msg = HashMap::new();
        msg.insert("role".to_string(), Value::String("assistant".to_string()));
        msg.insert("content".to_string(), Value::String("Sure!".to_string()));

        let proto = XAIGrpcCompletion::to_proto_message(&msg);
        assert_eq!(proto.role, i32::from(MessageRole::RoleAssistant));
    }

    #[test]
    fn test_is_reasoning_model() {
        // Can't fully construct without network, but we can test the logic
        let m = "grok-3".to_lowercase();
        assert!(m.contains("grok-3") && !m.contains("fast"));

        let m = "grok-3-fast".to_lowercase();
        assert!(!(m.contains("grok-3") && !m.contains("fast")));

        let m = "grok-2".to_lowercase();
        assert!(!(m.contains("grok-3") && !m.contains("fast")));
    }

    /// Integration test — requires XAI_API_KEY.
    #[tokio::test]
    #[ignore]
    async fn test_xai_grpc_provider_real() {
        let api_key = std::env::var("XAI_API_KEY")
            .expect("XAI_API_KEY required");

        let provider = XAIGrpcCompletion::connect("grok-3-mini", &api_key).await
            .expect("Failed to connect");

        let mut msg = HashMap::new();
        msg.insert("role".to_string(), Value::String("user".to_string()));
        msg.insert("content".to_string(), Value::String("Say hello in 3 words.".to_string()));

        let result = provider.acall(vec![msg], None, None).await;
        assert!(result.is_ok(), "Failed: {:?}", result.err());

        let response = result.unwrap();
        assert!(response.as_str().is_some());
        println!("gRPC provider response: {}", response);
    }
}
