//! Shared type definitions for CrewAI utilities.
//!
//! Corresponds to `crewai/utilities/types.py`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// An LLM message represented as a map with `role` and `content` fields.
///
/// This mirrors the Python `LLMMessage = TypedDict("LLMMessage", ...)` pattern.
pub type LLMMessage = HashMap<String, String>;

/// Construct an `LLMMessage` with the given role and content.
pub fn llm_message(role: &str, content: &str) -> LLMMessage {
    let mut msg = HashMap::new();
    msg.insert("role".to_string(), role.to_string());
    msg.insert("content".to_string(), content.to_string());
    msg
}

/// Structured LLM message for stronger typing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedLLMMessage {
    /// The role of the message sender (e.g., "system", "user", "assistant").
    pub role: String,
    /// The content of the message.
    pub content: String,
}

impl TypedLLMMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
        }
    }

    /// Convert to the HashMap-based `LLMMessage` type.
    pub fn to_map(&self) -> LLMMessage {
        llm_message(&self.role, &self.content)
    }
}
