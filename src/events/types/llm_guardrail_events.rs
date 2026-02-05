//! LLM guardrail event types.
//!
//! Corresponds to `crewai/events/types/llm_guardrail_events.py`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// LLMGuardrailStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a guardrail task starts.
///
/// Corresponds to `LLMGuardrailStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMGuardrailStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// String representation of the guardrail (source or description).
    pub guardrail: String,
    /// Number of retries so far.
    pub retry_count: i64,
}

impl LLMGuardrailStartedEvent {
    pub fn new(guardrail: String, retry_count: i64) -> Self {
        Self {
            base: BaseEventData::new("llm_guardrail_started"),
            guardrail,
            retry_count,
        }
    }
}

impl_base_event!(LLMGuardrailStartedEvent);

// ---------------------------------------------------------------------------
// LLMGuardrailCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a guardrail task completes.
///
/// Corresponds to `LLMGuardrailCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMGuardrailCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Whether the guardrail validation passed.
    pub success: bool,
    /// The validation result (serialised).
    pub result: Value,
    /// Error message if validation failed.
    pub error: Option<String>,
    /// Number of retries so far.
    pub retry_count: i64,
}

impl LLMGuardrailCompletedEvent {
    pub fn new(success: bool, result: Value, error: Option<String>, retry_count: i64) -> Self {
        Self {
            base: BaseEventData::new("llm_guardrail_completed"),
            success,
            result,
            error,
            retry_count,
        }
    }
}

impl_base_event!(LLMGuardrailCompletedEvent);

// ---------------------------------------------------------------------------
// LLMGuardrailFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a guardrail task fails.
///
/// Corresponds to `LLMGuardrailFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMGuardrailFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Error message.
    pub error: String,
    /// Number of retries so far.
    pub retry_count: i64,
}

impl LLMGuardrailFailedEvent {
    pub fn new(error: String, retry_count: i64) -> Self {
        Self {
            base: BaseEventData::new("llm_guardrail_failed"),
            error,
            retry_count,
        }
    }
}

impl_base_event!(LLMGuardrailFailedEvent);
