//! xAI gRPC client — typed Grok access via Protocol Buffers.
//!
//! This module provides a high-level gRPC client for xAI's API using the
//! tonic-generated stubs from `xai-proto`. It wraps the Chat, Embedder,
//! and Models services with bearer-token auth and TLS.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────┐     gRPC/TLS      ┌──────────────┐
//! │  XaiGrpcClient      │ ──────────────────→│ api.x.ai:443 │
//! │  ├ chat_client       │  GetCompletion     │  Chat service │
//! │  ├ embedder_client   │  Embed             │  Embed service│
//! │  └ models_client     │  ListModels        │  Models svc   │
//! └─────────────────────┘                    └──────────────┘
//!         │
//!         │ put_typed / get_typed
//!         ▼
//! ┌─────────────────────┐
//! │  Blackboard          │  ← XaiBlackboardAgent reads prompts,
//! │  ├ A2ARegistry       │     writes responses + embeddings
//! │  ├ typed_slots       │
//! │  └ slots             │
//! └─────────────────────┘
//! ```
//!
//! # Feature Gate
//!
//! This module requires `--features xai-grpc` and the `xai-proto` repo
//! cloned alongside crewai-rust.

// Generated protobuf types and client stubs
pub mod xai_api {
    tonic::include_proto!("xai_api");
}

use std::sync::Arc;
use tonic::metadata::MetadataValue;
use tonic::service::interceptor::InterceptedService;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};
use tonic::{Request, Status};

// Re-export the key generated types for ergonomic use
pub use xai_api::chat_client::ChatClient;
pub use xai_api::embedder_client::EmbedderClient;
pub use xai_api::models_client::ModelsClient;
pub use xai_api::{
    CompletionMessage, CompletionOutput, Content, EmbedEncodingFormat, EmbedInput, EmbedRequest,
    EmbedResponse, Embedding, FeatureVector, FinishReason, GetChatCompletionChunk,
    GetChatCompletionResponse, GetCompletionsRequest, Message, MessageRole,
};

/// Default xAI gRPC endpoint.
pub const XAI_GRPC_ENDPOINT: &str = "https://api.x.ai";

// ---------------------------------------------------------------------------
// Auth interceptor
// ---------------------------------------------------------------------------

/// Bearer token interceptor for gRPC calls.
#[derive(Clone)]
pub struct BearerAuth {
    token: Arc<String>,
}

impl BearerAuth {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            token: Arc::new(format!("Bearer {}", api_key.into())),
        }
    }
}

impl tonic::service::Interceptor for BearerAuth {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        let val: MetadataValue<_> = self
            .token
            .as_str()
            .parse()
            .map_err(|_| Status::internal("invalid auth token"))?;
        req.metadata_mut().insert("authorization", val);
        Ok(req)
    }
}

/// Type alias for intercepted channel (used by all clients).
pub type AuthChannel = InterceptedService<Channel, BearerAuth>;

// ---------------------------------------------------------------------------
// XaiGrpcClient — high-level wrapper
// ---------------------------------------------------------------------------

/// High-level xAI gRPC client with chat, embed, and models access.
///
/// All calls go through TLS to `api.x.ai:443` with bearer token auth.
///
/// # Example
///
/// ```ignore
/// let client = XaiGrpcClient::connect("your-api-key").await?;
/// let response = client.complete("grok-3-mini", "What is 2+2?").await?;
/// println!("{}", response);
/// ```
pub struct XaiGrpcClient {
    pub chat: ChatClient<AuthChannel>,
    pub embedder: EmbedderClient<AuthChannel>,
    pub models: ModelsClient<AuthChannel>,
}

impl XaiGrpcClient {
    /// Connect to xAI gRPC API with the given API key.
    ///
    /// Uses TLS with webpki roots to connect to `api.x.ai:443`.
    pub async fn connect(api_key: impl Into<String>) -> Result<Self, tonic::transport::Error> {
        Self::connect_with_endpoint(api_key, XAI_GRPC_ENDPOINT).await
    }

    /// Connect to a custom endpoint (for testing or proxies).
    pub async fn connect_with_endpoint(
        api_key: impl Into<String>,
        endpoint: &str,
    ) -> Result<Self, tonic::transport::Error> {
        let tls = ClientTlsConfig::new().with_webpki_roots();

        let channel = Endpoint::from_shared(endpoint.to_string())?
            .tls_config(tls)?
            .connect()
            .await?;

        let auth = BearerAuth::new(api_key);

        Ok(Self {
            chat: ChatClient::with_interceptor(channel.clone(), auth.clone()),
            embedder: EmbedderClient::with_interceptor(channel.clone(), auth.clone()),
            models: ModelsClient::with_interceptor(channel, auth),
        })
    }

    /// Simple chat completion: send a user message, get the response text.
    pub async fn complete(&mut self, model: &str, prompt: &str) -> Result<String, Status> {
        let request = GetCompletionsRequest {
            model: model.to_string(),
            messages: vec![Message {
                content: vec![Content {
                    content: Some(xai_api::content::Content::Text(prompt.to_string())),
                }],
                role: MessageRole::RoleUser.into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let response = self.chat.get_completion(request).await?;
        let inner = response.into_inner();

        // Extract text from first output
        let text = inner
            .outputs
            .first()
            .and_then(|o| o.message.as_ref())
            .map(|m| m.content.clone())
            .unwrap_or_default();

        Ok(text)
    }

    /// Chat completion with full message history.
    pub async fn complete_messages(
        &mut self,
        model: &str,
        messages: Vec<Message>,
        max_tokens: Option<i32>,
        temperature: Option<f32>,
    ) -> Result<GetChatCompletionResponse, Status> {
        let request = GetCompletionsRequest {
            model: model.to_string(),
            messages,
            max_tokens,
            temperature,
            ..Default::default()
        };

        let response = self.chat.get_completion(request).await?;
        Ok(response.into_inner())
    }

    /// Streaming chat completion — returns a stream of chunks.
    pub async fn complete_stream(
        &mut self,
        model: &str,
        prompt: &str,
    ) -> Result<tonic::Streaming<GetChatCompletionChunk>, Status> {
        let request = GetCompletionsRequest {
            model: model.to_string(),
            messages: vec![Message {
                content: vec![Content {
                    content: Some(xai_api::content::Content::Text(prompt.to_string())),
                }],
                role: MessageRole::RoleUser.into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let response = self.chat.get_completion_chunk(request).await?;
        Ok(response.into_inner())
    }

    /// Embed text strings — returns float vectors.
    pub async fn embed(&mut self, model: &str, texts: &[&str]) -> Result<Vec<Vec<f32>>, Status> {
        let request = EmbedRequest {
            model: model.to_string(),
            input: texts
                .iter()
                .map(|t| EmbedInput {
                    input: Some(xai_api::embed_input::Input::String(t.to_string())),
                })
                .collect(),
            encoding_format: EmbedEncodingFormat::FormatFloat.into(),
            ..Default::default()
        };

        let response = self.embedder.embed(request).await?;
        let inner = response.into_inner();

        let vectors: Vec<Vec<f32>> = inner
            .embeddings
            .iter()
            .flat_map(|emb| emb.embeddings.iter())
            .map(|fv| fv.float_array.clone())
            .collect();

        Ok(vectors)
    }

    /// List available language models.
    pub async fn list_models(&mut self) -> Result<Vec<String>, Status> {
        let response = self.models.list_language_models(()).await?;
        let names: Vec<String> = response
            .into_inner()
            .models
            .iter()
            .map(|m| m.name.clone())
            .collect();
        Ok(names)
    }
}

// ---------------------------------------------------------------------------
// Helper: build Message from role + text
// ---------------------------------------------------------------------------

/// Create a user message.
pub fn user_message(text: &str) -> Message {
    Message {
        content: vec![Content {
            content: Some(xai_api::content::Content::Text(text.to_string())),
        }],
        role: MessageRole::RoleUser.into(),
        ..Default::default()
    }
}

/// Create a system message.
pub fn system_message(text: &str) -> Message {
    Message {
        content: vec![Content {
            content: Some(xai_api::content::Content::Text(text.to_string())),
        }],
        role: MessageRole::RoleSystem.into(),
        ..Default::default()
    }
}

/// Create an assistant message.
pub fn assistant_message(text: &str) -> Message {
    Message {
        content: vec![Content {
            content: Some(xai_api::content::Content::Text(text.to_string())),
        }],
        role: MessageRole::RoleAssistant.into(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// XaiBlackboardAgent — xAI agent that operates via blackboard
// ---------------------------------------------------------------------------

use crate::blackboard::{AgentState, Blackboard};

/// Typed slot value for xAI responses stored on the blackboard.
#[derive(Debug, Clone)]
pub struct XaiResponse {
    /// The model used.
    pub model: String,
    /// Response text content.
    pub content: String,
    /// Reasoning trace (if model supports it).
    pub reasoning: Option<String>,
    /// Token usage.
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

/// Typed slot value for embedding vectors on the blackboard.
#[derive(Debug, Clone)]
pub struct XaiEmbedding {
    /// The model used.
    pub model: String,
    /// The input text that was embedded.
    pub text: String,
    /// The embedding vector (f32).
    pub vector: Vec<f32>,
    /// Vector dimensionality.
    pub dimensions: usize,
}

/// xAI agent that communicates via the Blackboard.
///
/// Registers itself in A2A, reads prompt slots, calls Grok via gRPC,
/// and writes response + embedding slots back.
///
/// # Blackboard Slot Convention
///
/// - Input:  `xai.prompt:{seq}` — JSON slot with `{"text": "...", "model": "..."}`
/// - Output: `xai.response:{seq}` — typed `XaiResponse` slot
/// - Embed:  `xai.embedding:{seq}` — typed `XaiEmbedding` slot
pub struct XaiBlackboardAgent {
    pub agent_id: String,
    pub client: XaiGrpcClient,
    pub default_model: String,
    pub embed_model: String,
}

impl XaiBlackboardAgent {
    /// Create a new xAI blackboard agent and register it in A2A.
    pub async fn new(
        api_key: impl Into<String>,
        bb: &mut Blackboard,
    ) -> Result<Self, tonic::transport::Error> {
        Self::with_models(api_key, "grok-3-mini", "v3-embedding", bb).await
    }

    /// Create with specific model names.
    pub async fn with_models(
        api_key: impl Into<String>,
        chat_model: &str,
        embed_model: &str,
        bb: &mut Blackboard,
    ) -> Result<Self, tonic::transport::Error> {
        let client = XaiGrpcClient::connect(api_key).await?;

        let agent_id = "xai-grpc-agent".to_string();

        bb.a2a.register(
            &agent_id,
            "Grok (gRPC)",
            "xAI language model via gRPC",
            vec![
                "chat".into(),
                "complete".into(),
                "embed".into(),
                "reason".into(),
                "search".into(),
            ],
        );

        Ok(Self {
            agent_id,
            client,
            default_model: chat_model.to_string(),
            embed_model: embed_model.to_string(),
        })
    }

    /// Process a prompt from the blackboard and write the response back.
    ///
    /// Reads `input_key` (JSON with `text` and optional `model` fields),
    /// calls xAI gRPC, writes typed `XaiResponse` to `output_key`.
    pub async fn process_slot(
        &mut self,
        bb: &mut Blackboard,
        input_key: &str,
        output_key: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Mark active
        bb.a2a.set_state(&self.agent_id, AgentState::Active);
        bb.a2a
            .set_goal(&self.agent_id, format!("Processing {}", input_key));

        // Read prompt from blackboard
        let prompt_value = bb
            .get_value(input_key)
            .ok_or("Input slot not found")?
            .clone();

        let text = prompt_value
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("Input slot missing 'text' field")?
            .to_string();

        let model = prompt_value
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.default_model)
            .to_string();

        // Call xAI gRPC
        let response = self
            .client
            .complete_messages(&model, vec![user_message(&text)], None, None)
            .await?;

        // Extract content
        let content = response
            .outputs
            .first()
            .and_then(|o| o.message.as_ref())
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let reasoning = response
            .outputs
            .first()
            .and_then(|o| o.message.as_ref())
            .map(|m| m.reasoning_content.clone())
            .filter(|r| !r.is_empty());

        // Extract usage
        let (prompt_tokens, completion_tokens, total_tokens) = response
            .usage
            .as_ref()
            .map(|u| (u.prompt_tokens, u.completion_tokens, u.total_tokens))
            .unwrap_or((0, 0, 0));

        // Write typed response to blackboard
        let xai_response = XaiResponse {
            model: response.model.clone(),
            content,
            reasoning,
            prompt_tokens,
            completion_tokens,
            total_tokens,
        };

        bb.put_typed(
            output_key,
            xai_response,
            &self.agent_id,
            "xai.grpc.complete",
        );

        // Mark completed
        bb.a2a.set_state(&self.agent_id, AgentState::Completed);

        Ok(())
    }

    /// Embed text and write the vector to a typed blackboard slot.
    pub async fn embed_to_slot(
        &mut self,
        bb: &mut Blackboard,
        text: &str,
        output_key: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        bb.a2a.set_state(&self.agent_id, AgentState::Active);

        let vectors = self.client.embed(&self.embed_model, &[text]).await?;

        let vector = vectors.into_iter().next().ok_or("No embedding returned")?;

        let dimensions = vector.len();

        let embedding = XaiEmbedding {
            model: self.embed_model.clone(),
            text: text.to_string(),
            vector,
            dimensions,
        };

        bb.put_typed(output_key, embedding, &self.agent_id, "xai.grpc.embed");

        bb.a2a.set_state(&self.agent_id, AgentState::Completed);

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::Blackboard;

    #[test]
    fn test_bearer_auth() {
        let auth = BearerAuth::new("test-key-123");
        assert!(auth.token.starts_with("Bearer "));
    }

    #[test]
    fn test_user_message() {
        let msg = user_message("Hello, Grok!");
        assert_eq!(msg.role, i32::from(MessageRole::RoleUser));
        assert_eq!(msg.content.len(), 1);
        match &msg.content[0].content {
            Some(xai_api::content::Content::Text(t)) => assert_eq!(t, "Hello, Grok!"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_system_message() {
        let msg = system_message("You are helpful.");
        assert_eq!(msg.role, i32::from(MessageRole::RoleSystem));
    }

    #[test]
    fn test_assistant_message() {
        let msg = assistant_message("Sure, let me help.");
        assert_eq!(msg.role, i32::from(MessageRole::RoleAssistant));
    }

    #[test]
    fn test_xai_response_typed_slot() {
        let mut bb = Blackboard::new();

        // Register agents
        bb.a2a
            .register("xai-agent", "Grok", "xAI model", vec!["chat".into()]);
        bb.a2a
            .register("claude-agent", "Claude", "reasoning", vec!["reason".into()]);

        assert_eq!(bb.a2a.len(), 2);

        // Claude writes a prompt
        bb.put(
            "prompt:0",
            serde_json::json!({"text": "What is 2+2?", "model": "grok-3-mini"}),
            "claude-agent",
            "claude.reason",
        );

        // Simulate xAI response (normally done by process_slot via gRPC)
        let response = XaiResponse {
            model: "grok-3-mini".to_string(),
            content: "2+2 equals 4.".to_string(),
            reasoning: Some("Simple arithmetic.".to_string()),
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        };

        bb.put_typed("response:0", response, "xai-agent", "xai.grpc.complete");
        bb.a2a.set_state("xai-agent", AgentState::Completed);

        // Claude reads the response
        let resp = bb.get_typed::<XaiResponse>("response:0").unwrap();
        assert_eq!(resp.content, "2+2 equals 4.");
        assert_eq!(resp.model, "grok-3-mini");
        assert!(resp.reasoning.as_deref() == Some("Simple arithmetic."));
        assert_eq!(resp.total_tokens, 15);

        // Verify A2A state
        let xai = bb.a2a.get("xai-agent").unwrap();
        assert_eq!(xai.state, AgentState::Completed);
        assert!(xai.capabilities.contains(&"chat".to_string()));
    }

    #[test]
    fn test_xai_embedding_typed_slot() {
        let mut bb = Blackboard::new();

        let embedding = XaiEmbedding {
            model: "v3-embedding".to_string(),
            text: "Hello world".to_string(),
            vector: vec![0.1, 0.2, 0.3, 0.4],
            dimensions: 4,
        };

        bb.put_typed("embed:0", embedding, "xai-agent", "xai.grpc.embed");

        let emb = bb.get_typed::<XaiEmbedding>("embed:0").unwrap();
        assert_eq!(emb.dimensions, 4);
        assert_eq!(emb.vector.len(), 4);
        assert!((emb.vector[0] - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_multi_agent_blackboard_roundtrip() {
        let mut bb = Blackboard::new();

        // Register 3 agents
        bb.a2a
            .register("claude", "Claude", "reasoning", vec!["reason".into()]);
        bb.a2a.register(
            "grok",
            "Grok",
            "completion",
            vec!["chat".into(), "embed".into()],
        );
        bb.a2a.register(
            "orchestrator",
            "Orchestrator",
            "routing",
            vec!["route".into()],
        );

        // Orchestrator posts a task
        bb.put(
            "task:0",
            serde_json::json!({
                "goal": "Analyze xAI mission",
                "assigned_to": "grok",
                "fallback": "claude"
            }),
            "orchestrator",
            "meta.orchestrate",
        );

        // Grok picks up the task and responds
        bb.a2a.set_state("grok", AgentState::Active);
        let response = XaiResponse {
            model: "grok-3-mini".to_string(),
            content: "xAI aims to understand the true nature of the universe.".to_string(),
            reasoning: None,
            prompt_tokens: 20,
            completion_tokens: 12,
            total_tokens: 32,
        };
        bb.put_typed("response:0", response, "grok", "xai.grpc.complete");
        bb.a2a.set_state("grok", AgentState::Completed);

        // Claude reads Grok's response and adds analysis
        bb.a2a.set_state("claude", AgentState::Active);
        let grok_resp = bb.get_typed::<XaiResponse>("response:0").unwrap();
        let analysis = format!(
            "Grok says: '{}'. Analysis: mission-aligned.",
            grok_resp.content
        );
        bb.put(
            "analysis:0",
            serde_json::json!({"text": analysis, "source": "claude"}),
            "claude",
            "claude.analyze",
        );
        bb.a2a.set_state("claude", AgentState::Completed);

        // Orchestrator reads the analysis
        let analysis_val = bb.get_value("analysis:0").unwrap();
        assert!(analysis_val["text"].as_str().unwrap().contains("Grok says"));
        assert!(analysis_val["text"]
            .as_str()
            .unwrap()
            .contains("mission-aligned"));

        // Verify all agents completed
        assert_eq!(bb.a2a.get("grok").unwrap().state, AgentState::Completed);
        assert_eq!(bb.a2a.get("claude").unwrap().state, AgentState::Completed);

        // Verify trace shows the execution order
        let trace = bb.trace();
        assert!(trace.contains(&"task:0".to_string()));
        assert!(trace.contains(&"analysis:0".to_string()));
    }

    #[test]
    fn test_a2a_capability_discovery() {
        let mut bb = Blackboard::new();

        bb.a2a.register(
            "grok",
            "Grok",
            "xAI model",
            vec!["chat".into(), "embed".into(), "search".into()],
        );
        bb.a2a.register(
            "claude",
            "Claude",
            "Anthropic model",
            vec!["reason".into(), "code".into()],
        );
        bb.a2a.register(
            "searcher",
            "WebSearch",
            "search tool",
            vec!["search".into()],
        );

        // Find all agents that can search
        let searchers = bb.a2a.by_capability("search");
        assert_eq!(searchers.len(), 2); // grok + searcher

        // Find all agents that can embed
        let embedders = bb.a2a.by_capability("embed");
        assert_eq!(embedders.len(), 1);
        assert_eq!(embedders[0].name, "Grok");
    }

    /// Integration test — requires XAI_API_KEY env var.
    ///
    /// Tests real gRPC connection to api.x.ai:443.
    /// Run with: `cargo test --features xai-grpc test_xai_grpc_real -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn test_xai_grpc_real_completion() {
        let api_key =
            std::env::var("XAI_API_KEY").expect("XAI_API_KEY required for integration test");

        let mut client = XaiGrpcClient::connect(&api_key)
            .await
            .expect("Failed to connect to xAI gRPC");

        let response = client
            .complete("grok-3-mini", "Say hello in exactly 3 words.")
            .await
            .expect("Completion failed");

        assert!(!response.is_empty(), "Empty response from Grok");
        println!("Grok says: {}", response);
    }

    /// Integration test — gRPC embedding.
    #[tokio::test]
    #[ignore]
    async fn test_xai_grpc_real_embedding() {
        let api_key =
            std::env::var("XAI_API_KEY").expect("XAI_API_KEY required for integration test");

        let mut client = XaiGrpcClient::connect(&api_key)
            .await
            .expect("Failed to connect to xAI gRPC");

        let vectors = client
            .embed("v3-embedding", &["Hello world", "Goodbye world"])
            .await
            .expect("Embedding failed");

        assert_eq!(vectors.len(), 2, "Expected 2 embeddings");
        assert!(!vectors[0].is_empty(), "Empty embedding vector");
        println!("Embedding dimensions: {}", vectors[0].len());
    }

    /// Integration test — full blackboard roundtrip with real gRPC.
    #[tokio::test]
    #[ignore]
    async fn test_xai_grpc_blackboard_real() {
        let api_key =
            std::env::var("XAI_API_KEY").expect("XAI_API_KEY required for integration test");

        let mut bb = Blackboard::new();

        // Register Claude agent
        bb.a2a.register(
            "claude",
            "Claude",
            "reasoning",
            vec!["reason".into(), "code".into()],
        );

        // Create xAI agent (registers itself in A2A)
        let mut xai_agent =
            XaiBlackboardAgent::with_models(&api_key, "grok-3-mini", "v3-embedding", &mut bb)
                .await
                .expect("Failed to connect xAI agent");

        assert_eq!(bb.a2a.len(), 2);

        // Claude writes a prompt
        bb.put(
            "prompt:0",
            serde_json::json!({"text": "What is the capital of France? Answer in one word."}),
            "claude",
            "claude.ask",
        );

        // xAI processes it via gRPC
        xai_agent
            .process_slot(&mut bb, "prompt:0", "response:0")
            .await
            .expect("process_slot failed");

        // Read response
        let resp = bb
            .get_typed::<XaiResponse>("response:0")
            .expect("No response slot");
        println!("Grok response: {}", resp.content);
        assert!(
            resp.content.to_lowercase().contains("paris"),
            "Expected 'paris' in response, got: {}",
            resp.content
        );

        // Verify A2A state
        assert_eq!(
            bb.a2a.get("xai-grpc-agent").unwrap().state,
            AgentState::Completed
        );
    }

    /// Integration test — streaming completion.
    #[tokio::test]
    #[ignore]
    async fn test_xai_grpc_real_streaming() {
        use futures::StreamExt;

        let api_key =
            std::env::var("XAI_API_KEY").expect("XAI_API_KEY required for integration test");

        let mut client = XaiGrpcClient::connect(&api_key)
            .await
            .expect("Failed to connect to xAI gRPC");

        let mut stream = client
            .complete_stream("grok-3-mini", "Count from 1 to 5.")
            .await
            .expect("Stream failed");

        let mut chunks = 0;
        let mut full_text = String::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.expect("Stream error");
            for output in &chunk.outputs {
                if let Some(ref delta) = output.delta {
                    full_text.push_str(&delta.content);
                }
            }
            chunks += 1;
        }

        println!("Received {} chunks, full text: {}", chunks, full_text);
        assert!(chunks > 0, "No chunks received");
        assert!(!full_text.is_empty(), "Empty streamed response");
    }
}
