//! Streaming output types for crew and flow execution.
//!
//! Corresponds to `crewai/types/streaming.py`.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};

/// Type of streaming chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamChunkType {
    /// Text content chunk.
    Text,
    /// Tool call chunk.
    ToolCall,
}

impl Default for StreamChunkType {
    fn default() -> Self {
        StreamChunkType::Text
    }
}

/// Tool call information in a streaming chunk.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolCallChunk {
    /// Unique identifier for the tool call.
    pub tool_id: Option<String>,
    /// Name of the tool being called.
    pub tool_name: Option<String>,
    /// JSON string of tool arguments.
    pub arguments: String,
    /// Index of the tool call in the response.
    pub index: usize,
}

/// Base streaming chunk with full context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// The streaming content (text or partial content).
    pub content: String,
    /// Type of the chunk (text, tool_call, etc.).
    pub chunk_type: StreamChunkType,
    /// Index of the current task (0-based).
    pub task_index: usize,
    /// Name or description of the current task.
    pub task_name: String,
    /// Unique identifier of the task.
    pub task_id: String,
    /// Role of the agent executing the task.
    pub agent_role: String,
    /// Unique identifier of the agent.
    pub agent_id: String,
    /// Tool call information if chunk_type is TOOL_CALL.
    pub tool_call: Option<ToolCallChunk>,
}

impl Default for StreamChunk {
    fn default() -> Self {
        Self {
            content: String::new(),
            chunk_type: StreamChunkType::Text,
            task_index: 0,
            task_name: String::new(),
            task_id: String::new(),
            agent_role: String::new(),
            agent_id: String::new(),
            tool_call: None,
        }
    }
}

impl fmt::Display for StreamChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content)
    }
}

/// Base class for streaming output with result access.
///
/// Provides iteration over stream chunks and access to final result
/// via the `result()` method after streaming completes.
#[derive(Debug)]
pub struct StreamingOutputBase<T> {
    result: Arc<Mutex<Option<T>>>,
    completed: Arc<Mutex<bool>>,
    chunks: Arc<Mutex<Vec<StreamChunk>>>,
    error: Arc<Mutex<Option<String>>>,
}

impl<T: Clone> StreamingOutputBase<T> {
    /// Create a new StreamingOutputBase.
    pub fn new() -> Self {
        Self {
            result: Arc::new(Mutex::new(None)),
            completed: Arc::new(Mutex::new(false)),
            chunks: Arc::new(Mutex::new(Vec::new())),
            error: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the final result after streaming completes.
    pub fn result(&self) -> Result<T, String> {
        let completed = self.completed.lock().unwrap();
        if !*completed {
            return Err(
                "Streaming has not completed yet. Iterate over all chunks before accessing result."
                    .to_string(),
            );
        }
        if let Some(ref err) = *self.error.lock().unwrap() {
            return Err(err.clone());
        }
        let result = self.result.lock().unwrap();
        result
            .clone()
            .ok_or_else(|| "No result available".to_string())
    }

    /// Check if streaming has completed.
    pub fn is_completed(&self) -> bool {
        *self.completed.lock().unwrap()
    }

    /// Get all collected chunks so far.
    pub fn chunks(&self) -> Vec<StreamChunk> {
        self.chunks.lock().unwrap().clone()
    }

    /// Get all streamed text content concatenated.
    pub fn get_full_text(&self) -> String {
        self.chunks
            .lock()
            .unwrap()
            .iter()
            .filter(|chunk| chunk.chunk_type == StreamChunkType::Text)
            .map(|chunk| chunk.content.as_str())
            .collect()
    }

    /// Add a chunk to the internal buffer.
    pub fn add_chunk(&self, chunk: StreamChunk) {
        self.chunks.lock().unwrap().push(chunk);
    }

    /// Set the final result.
    pub fn set_result(&self, result: T) {
        *self.result.lock().unwrap() = Some(result);
        *self.completed.lock().unwrap() = true;
    }

    /// Set an error.
    pub fn set_error(&self, error: String) {
        *self.error.lock().unwrap() = Some(error);
        *self.completed.lock().unwrap() = true;
    }

    /// Mark as completed.
    pub fn set_completed(&self) {
        *self.completed.lock().unwrap() = true;
    }
}

impl<T: Clone> Default for StreamingOutputBase<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming output wrapper for crew execution.
pub type CrewStreamingOutput = StreamingOutputBase<crate::crews::crew_output::CrewOutput>;

/// Streaming output wrapper for flow execution.
pub type FlowStreamingOutput = StreamingOutputBase<serde_json::Value>;
