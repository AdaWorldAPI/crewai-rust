//! Streaming LLM response support.
//!
//! Provides the [`StreamingLLM`] trait for LLM providers that support
//! streaming responses. This is critical for openclaw-rs block-streaming
//! and live-edit channel support (e.g., Discord message editing, Slack
//! streaming responses).
//!
//! # Design
//!
//! - Additive: `StreamingLLM` is a separate trait from [`BaseLLM`].
//!   Providers that support streaming implement both traits.
//! - Chunk-based: the stream yields `StreamChunk` values, each containing
//!   a text delta, optional tool call delta, or a final message.
//! - Hook integration: chunks are forwarded to `AgentHook::on_stream_chunk`
//!   via the hook registry.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::base_llm::LLMMessage;

// ---------------------------------------------------------------------------
// StreamChunk
// ---------------------------------------------------------------------------

/// A single chunk from a streaming LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamChunk {
    /// A text delta (partial text content).
    TextDelta {
        /// The text fragment.
        text: String,
    },

    /// A tool call delta (partial tool call being built up).
    ToolCallDelta {
        /// Tool call index (for multiple parallel tool calls).
        index: usize,
        /// Tool call ID (set in the first delta for this tool call).
        id: Option<String>,
        /// Function name (set in the first delta).
        name: Option<String>,
        /// Arguments fragment (JSON string fragment, accumulated).
        arguments: Option<String>,
    },

    /// Thinking/reasoning delta (for models that support extended thinking).
    ThinkingDelta {
        /// The thinking text fragment.
        text: String,
    },

    /// The stream is done. Contains the final assembled message.
    Done {
        /// The complete text content.
        content: String,
        /// Complete tool calls (if any).
        tool_calls: Option<Vec<Value>>,
        /// Usage information.
        usage: Option<StreamUsage>,
    },

    /// An error occurred during streaming.
    Error {
        /// Error message.
        message: String,
    },
}

/// Token usage from a streaming response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamUsage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}

// ---------------------------------------------------------------------------
// StreamingLLM trait
// ---------------------------------------------------------------------------

/// Trait for LLM providers that support streaming responses.
///
/// Implement alongside [`BaseLLM`] for providers that can return
/// incremental response chunks.
///
/// # Example
///
/// ```ignore
/// use crewai::llms::streaming::{StreamingLLM, StreamChunk};
///
/// struct MyProvider { /* ... */ }
///
/// #[async_trait]
/// impl StreamingLLM for MyProvider {
///     async fn stream(
///         &self,
///         messages: Vec<LLMMessage>,
///         tools: Option<Vec<Value>>,
///     ) -> Result<Box<dyn StreamReceiver>, Box<dyn std::error::Error + Send + Sync>> {
///         // ... connect to provider's streaming API ...
///     }
/// }
/// ```
#[async_trait]
pub trait StreamingLLM: Send + Sync {
    /// Start a streaming LLM call.
    ///
    /// Returns a `StreamReceiver` that yields `StreamChunk` values.
    async fn stream(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
    ) -> Result<Box<dyn StreamReceiver>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Receiver for streaming chunks.
///
/// Abstracts over the underlying transport (SSE, WebSocket, etc.).
#[async_trait]
pub trait StreamReceiver: Send + Sync {
    /// Get the next chunk from the stream.
    ///
    /// Returns `None` when the stream is complete (after yielding `StreamChunk::Done`).
    async fn next(&mut self) -> Option<StreamChunk>;
}

// ---------------------------------------------------------------------------
// ChannelStreamReceiver — wraps a tokio channel
// ---------------------------------------------------------------------------

/// A `StreamReceiver` backed by a tokio mpsc channel.
///
/// This is the default implementation used by providers that push chunks
/// via a background task.
pub struct ChannelStreamReceiver {
    rx: tokio::sync::mpsc::Receiver<StreamChunk>,
}

impl ChannelStreamReceiver {
    /// Create a new channel-backed stream receiver.
    pub fn new(rx: tokio::sync::mpsc::Receiver<StreamChunk>) -> Self {
        Self { rx }
    }

    /// Create a matched pair of sender + receiver.
    pub fn pair(buffer: usize) -> (tokio::sync::mpsc::Sender<StreamChunk>, Self) {
        let (tx, rx) = tokio::sync::mpsc::channel(buffer);
        (tx, Self { rx })
    }
}

#[async_trait]
impl StreamReceiver for ChannelStreamReceiver {
    async fn next(&mut self) -> Option<StreamChunk> {
        self.rx.recv().await
    }
}

// ---------------------------------------------------------------------------
// StreamAccumulator — assemble a full response from chunks
// ---------------------------------------------------------------------------

/// Accumulates streaming chunks into a complete response.
///
/// Useful for callers that want the full response but also need to
/// forward chunks to hooks (e.g., `AgentHook::on_stream_chunk`).
pub struct StreamAccumulator {
    text: String,
    tool_calls: Vec<Value>,
    usage: Option<StreamUsage>,
}

impl StreamAccumulator {
    /// Create a new empty accumulator.
    pub fn new() -> Self {
        Self {
            text: String::new(),
            tool_calls: Vec::new(),
            usage: None,
        }
    }

    /// Process a chunk, returning `true` if the stream is done.
    pub fn push(&mut self, chunk: &StreamChunk) -> bool {
        match chunk {
            StreamChunk::TextDelta { text } => {
                self.text.push_str(text);
                false
            }
            StreamChunk::ThinkingDelta { .. } => false,
            StreamChunk::ToolCallDelta { .. } => false,
            StreamChunk::Done { content, tool_calls, usage } => {
                // The Done chunk carries the final assembled content
                self.text = content.clone();
                if let Some(tc) = tool_calls {
                    self.tool_calls = tc.clone();
                }
                self.usage = usage.clone();
                true
            }
            StreamChunk::Error { .. } => true,
        }
    }

    /// Get the accumulated text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the accumulated tool calls.
    pub fn tool_calls(&self) -> &[Value] {
        &self.tool_calls
    }

    /// Get the usage info.
    pub fn usage(&self) -> Option<&StreamUsage> {
        self.usage.as_ref()
    }
}

impl Default for StreamAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_chunk_serde() {
        let delta = StreamChunk::TextDelta { text: "hello ".into() };
        let json = serde_json::to_string(&delta).unwrap();
        let back: StreamChunk = serde_json::from_str(&json).unwrap();
        match back {
            StreamChunk::TextDelta { text } => assert_eq!(text, "hello "),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_stream_chunk_done() {
        let done = StreamChunk::Done {
            content: "full response".into(),
            tool_calls: None,
            usage: Some(StreamUsage { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 }),
        };
        let json = serde_json::to_string(&done).unwrap();
        assert!(json.contains("full response"));
        assert!(json.contains("prompt_tokens"));
    }

    #[test]
    fn test_accumulator() {
        let mut acc = StreamAccumulator::new();

        assert!(!acc.push(&StreamChunk::TextDelta { text: "Hello ".into() }));
        assert!(!acc.push(&StreamChunk::TextDelta { text: "world!".into() }));
        assert_eq!(acc.text(), "Hello world!");

        let done = acc.push(&StreamChunk::Done {
            content: "Hello world!".into(),
            tool_calls: None,
            usage: Some(StreamUsage { prompt_tokens: 5, completion_tokens: 3, total_tokens: 8 }),
        });
        assert!(done);
        assert_eq!(acc.text(), "Hello world!");
        assert_eq!(acc.usage().unwrap().total_tokens, 8);
    }

    #[test]
    fn test_accumulator_error() {
        let mut acc = StreamAccumulator::new();
        let done = acc.push(&StreamChunk::Error { message: "timeout".into() });
        assert!(done);
    }

    #[tokio::test]
    async fn test_channel_stream_receiver() {
        let (tx, mut rx) = ChannelStreamReceiver::pair(16);

        tx.send(StreamChunk::TextDelta { text: "hi".into() }).await.unwrap();
        tx.send(StreamChunk::Done {
            content: "hi".into(),
            tool_calls: None,
            usage: None,
        }).await.unwrap();
        drop(tx);

        let c1 = rx.next().await.unwrap();
        assert!(matches!(c1, StreamChunk::TextDelta { .. }));

        let c2 = rx.next().await.unwrap();
        assert!(matches!(c2, StreamChunk::Done { .. }));

        let c3 = rx.next().await;
        assert!(c3.is_none());
    }
}
