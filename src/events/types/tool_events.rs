//! Tool usage event types.
//!
//! Corresponds to `crewai/events/types/tool_usage_events.py`.
//!
//! Contains events for tool usage lifecycle: started, finished, and
//! various error conditions (validation, selection, execution).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// ToolUsageEvent (base)
// ---------------------------------------------------------------------------

/// Base event for tool usage tracking.
///
/// Corresponds to `ToolUsageEvent` in Python. All tool lifecycle events
/// embed or extend this structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Key identifying the agent performing the tool call.
    pub agent_key: Option<String>,
    /// Name of the tool being used.
    pub tool_name: String,
    /// Arguments passed to the tool (dict or raw string).
    pub tool_args: Value,
    /// Fully-qualified class/struct name of the tool.
    pub tool_class: Option<String>,
    /// Number of retry attempts so far.
    pub run_attempts: i64,
    /// Number of delegation hops, if applicable.
    pub delegations: Option<i64>,
}

impl ToolUsageEvent {
    pub fn new(tool_name: String, tool_args: Value) -> Self {
        Self {
            base: BaseEventData::new("tool_usage"),
            agent_key: None,
            tool_name,
            tool_args,
            tool_class: None,
            run_attempts: 0,
            delegations: None,
        }
    }
}

impl_base_event!(ToolUsageEvent);

// ---------------------------------------------------------------------------
// ToolUsageStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a tool execution is started.
///
/// Corresponds to `ToolUsageStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Key identifying the agent performing the tool call.
    pub agent_key: Option<String>,
    /// Name of the tool being used.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: Value,
    /// Fully-qualified class/struct name of the tool.
    pub tool_class: Option<String>,
    /// Number of retry attempts so far.
    pub run_attempts: i64,
    /// Number of delegation hops.
    pub delegations: Option<i64>,
}

impl ToolUsageStartedEvent {
    pub fn new(tool_name: String, tool_args: Value, run_attempts: i64) -> Self {
        Self {
            base: BaseEventData::new("tool_usage_started"),
            agent_key: None,
            tool_name,
            tool_args,
            tool_class: None,
            run_attempts,
            delegations: None,
        }
    }
}

impl_base_event!(ToolUsageStartedEvent);

// ---------------------------------------------------------------------------
// ToolUsageFinishedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a tool execution is completed.
///
/// Corresponds to `ToolUsageFinishedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageFinishedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Key identifying the agent performing the tool call.
    pub agent_key: Option<String>,
    /// Name of the tool being used.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: Value,
    /// Fully-qualified class/struct name of the tool.
    pub tool_class: Option<String>,
    /// Number of retry attempts so far.
    pub run_attempts: i64,
    /// Number of delegation hops.
    pub delegations: Option<i64>,
    /// Timestamp when the tool execution started.
    pub started_at: DateTime<Utc>,
    /// Timestamp when the tool execution finished.
    pub finished_at: DateTime<Utc>,
    /// Whether the result was served from cache.
    pub from_cache: bool,
    /// Tool output (arbitrary JSON).
    pub output: Value,
}

impl ToolUsageFinishedEvent {
    pub fn new(
        tool_name: String,
        tool_args: Value,
        run_attempts: i64,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
        from_cache: bool,
        output: Value,
    ) -> Self {
        Self {
            base: BaseEventData::new("tool_usage_finished"),
            agent_key: None,
            tool_name,
            tool_args,
            tool_class: None,
            run_attempts,
            delegations: None,
            started_at,
            finished_at,
            from_cache,
            output,
        }
    }
}

impl_base_event!(ToolUsageFinishedEvent);

// ---------------------------------------------------------------------------
// ToolUsageErrorEvent
// ---------------------------------------------------------------------------

/// Event emitted when a tool execution encounters an error.
///
/// Corresponds to `ToolUsageErrorEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageErrorEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Key identifying the agent performing the tool call.
    pub agent_key: Option<String>,
    /// Name of the tool being used.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: Value,
    /// Fully-qualified class/struct name of the tool.
    pub tool_class: Option<String>,
    /// Number of retry attempts so far.
    pub run_attempts: i64,
    /// Number of delegation hops.
    pub delegations: Option<i64>,
    /// Error message or serialised error value.
    pub error: Value,
}

impl ToolUsageErrorEvent {
    pub fn new(tool_name: String, tool_args: Value, run_attempts: i64, error: Value) -> Self {
        Self {
            base: BaseEventData::new("tool_usage_error"),
            agent_key: None,
            tool_name,
            tool_args,
            tool_class: None,
            run_attempts,
            delegations: None,
            error,
        }
    }
}

impl_base_event!(ToolUsageErrorEvent);

// ---------------------------------------------------------------------------
// ToolValidateInputErrorEvent
// ---------------------------------------------------------------------------

/// Event emitted when a tool input validation encounters an error.
///
/// Corresponds to `ToolValidateInputErrorEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolValidateInputErrorEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the tool.
    pub tool_name: String,
    /// Arguments that failed validation.
    pub tool_args: Value,
    /// Number of retry attempts so far.
    pub run_attempts: i64,
    /// Validation error.
    pub error: Value,
}

impl ToolValidateInputErrorEvent {
    pub fn new(tool_name: String, tool_args: Value, run_attempts: i64, error: Value) -> Self {
        Self {
            base: BaseEventData::new("tool_validate_input_error"),
            tool_name,
            tool_args,
            run_attempts,
            error,
        }
    }
}

impl_base_event!(ToolValidateInputErrorEvent);

// ---------------------------------------------------------------------------
// ToolSelectionErrorEvent
// ---------------------------------------------------------------------------

/// Event emitted when a tool selection encounters an error.
///
/// Corresponds to `ToolSelectionErrorEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelectionErrorEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the tool that failed selection.
    pub tool_name: String,
    /// Arguments attempted.
    pub tool_args: Value,
    /// Number of retry attempts so far.
    pub run_attempts: i64,
    /// Selection error.
    pub error: Value,
}

impl ToolSelectionErrorEvent {
    pub fn new(tool_name: String, tool_args: Value, run_attempts: i64, error: Value) -> Self {
        Self {
            base: BaseEventData::new("tool_selection_error"),
            tool_name,
            tool_args,
            run_attempts,
            error,
        }
    }
}

impl_base_event!(ToolSelectionErrorEvent);

// ---------------------------------------------------------------------------
// ToolExecutionErrorEvent
// ---------------------------------------------------------------------------

/// Event emitted when a tool execution encounters an error.
///
/// Unlike `ToolUsageErrorEvent`, this does not carry full tool-usage metadata
/// (agent_key, delegations, etc.) -- it is a simpler error event.
///
/// Corresponds to `ToolExecutionErrorEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionErrorEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Error message or serialised error value.
    pub error: Value,
    /// Name of the tool.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: HashMap<String, Value>,
    /// Fully-qualified class/struct name of the tool.
    pub tool_class: String,
}

impl ToolExecutionErrorEvent {
    pub fn new(
        error: Value,
        tool_name: String,
        tool_args: HashMap<String, Value>,
        tool_class: String,
    ) -> Self {
        Self {
            base: BaseEventData::new("tool_execution_error"),
            error,
            tool_name,
            tool_args,
            tool_class,
        }
    }
}

impl_base_event!(ToolExecutionErrorEvent);
