//! LLM-related event types.
//!
//! Corresponds to `crewai/events/types/llm_events.py`.
//!
//! Contains events for LLM call lifecycle (started, completed, failed)
//! and streaming chunk events. Also defines supporting types such as
//! [`LLMCallType`], [`FunctionCall`], and [`ToolCall`].

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// LLMCallType
// ---------------------------------------------------------------------------

/// Type of LLM call being made.
///
/// Corresponds to `LLMCallType` enum in Python.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LLMCallType {
    /// A tool-calling invocation.
    ToolCall,
    /// A regular LLM generation call.
    LlmCall,
}

impl std::fmt::Display for LLMCallType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMCallType::ToolCall => write!(f, "tool_call"),
            LLMCallType::LlmCall => write!(f, "llm_call"),
        }
    }
}

// ---------------------------------------------------------------------------
// FunctionCall / ToolCall (supporting models)
// ---------------------------------------------------------------------------

/// A single function call description within a tool call.
///
/// Corresponds to `FunctionCall` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Serialised JSON string of the function arguments.
    pub arguments: String,
    /// Function name.
    pub name: Option<String>,
}

/// A tool call within an LLM streaming response.
///
/// Corresponds to `ToolCall` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool call identifier.
    pub id: Option<String>,
    /// The function being called.
    pub function: FunctionCall,
    /// Tool call type discriminator.
    #[serde(rename = "type")]
    pub call_type: Option<String>,
    /// Index within the tool-calls array.
    pub index: i64,
}

// ---------------------------------------------------------------------------
// LLMCallStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a LLM call starts.
///
/// Corresponds to `LLMCallStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMCallStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// LLM model name.
    pub model: Option<String>,
    /// Unique call identifier.
    pub call_id: String,
    /// Messages sent to the LLM (string or structured list).
    pub messages: Option<Value>,
    /// Tool definitions provided to the LLM.
    pub tools: Option<Vec<HashMap<String, Value>>>,
    /// Callbacks attached to this LLM call.
    pub callbacks: Option<Vec<Value>>,
    /// Available function mappings for tool calls.
    pub available_functions: Option<HashMap<String, Value>>,
}

impl LLMCallStartedEvent {
    pub fn new(call_id: String, model: Option<String>) -> Self {
        Self {
            base: BaseEventData::new("llm_call_started"),
            model,
            call_id,
            messages: None,
            tools: None,
            callbacks: None,
            available_functions: None,
        }
    }
}

impl_base_event!(LLMCallStartedEvent);

// ---------------------------------------------------------------------------
// LLMCallCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a LLM call completes.
///
/// Corresponds to `LLMCallCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMCallCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// LLM model name.
    pub model: Option<String>,
    /// Unique call identifier.
    pub call_id: String,
    /// Messages that were sent to the LLM.
    pub messages: Option<Value>,
    /// The LLM response (arbitrary JSON).
    pub response: Value,
    /// Type of LLM call that completed.
    pub call_type: LLMCallType,
}

impl LLMCallCompletedEvent {
    pub fn new(
        call_id: String,
        model: Option<String>,
        response: Value,
        call_type: LLMCallType,
    ) -> Self {
        Self {
            base: BaseEventData::new("llm_call_completed"),
            model,
            call_id,
            messages: None,
            response,
            call_type,
        }
    }
}

impl_base_event!(LLMCallCompletedEvent);

// ---------------------------------------------------------------------------
// LLMCallFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a LLM call fails.
///
/// Corresponds to `LLMCallFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMCallFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// LLM model name.
    pub model: Option<String>,
    /// Unique call identifier.
    pub call_id: String,
    /// Error message.
    pub error: String,
}

impl LLMCallFailedEvent {
    pub fn new(call_id: String, model: Option<String>, error: String) -> Self {
        Self {
            base: BaseEventData::new("llm_call_failed"),
            model,
            call_id,
            error,
        }
    }
}

impl_base_event!(LLMCallFailedEvent);

// ---------------------------------------------------------------------------
// LLMStreamChunkEvent
// ---------------------------------------------------------------------------

/// Event emitted when a streaming chunk is received from an LLM.
///
/// Corresponds to `LLMStreamChunkEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMStreamChunkEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// LLM model name.
    pub model: Option<String>,
    /// Unique call identifier.
    pub call_id: String,
    /// The text chunk received.
    pub chunk: String,
    /// Tool call data, if this chunk is part of a tool call stream.
    pub tool_call: Option<ToolCall>,
    /// Type of call this chunk belongs to.
    pub call_type: Option<LLMCallType>,
    /// Response ID from the provider.
    pub response_id: Option<String>,
}

impl LLMStreamChunkEvent {
    pub fn new(call_id: String, model: Option<String>, chunk: String) -> Self {
        Self {
            base: BaseEventData::new("llm_stream_chunk"),
            model,
            call_id,
            chunk,
            tool_call: None,
            call_type: None,
            response_id: None,
        }
    }
}

impl_base_event!(LLMStreamChunkEvent);
