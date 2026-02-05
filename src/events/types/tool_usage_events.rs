//! Tool usage event types.
//!
//! Corresponds to `crewai/events/types/tool_usage_events.py`.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

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
    /// Key identifying the agent.
    pub agent_key: Option<String>,
    /// Name of the tool.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: Value,
    /// Tool class name.
    pub tool_class: Option<String>,
    /// Number of run attempts.
    pub run_attempts: i64,
    /// Number of delegations.
    pub delegations: Option<i64>,
}

impl ToolUsageStartedEvent {
    pub fn new(
        tool_name: String,
        tool_args: Value,
        tool_class: Option<String>,
        run_attempts: i64,
        delegations: Option<i64>,
    ) -> Self {
        Self {
            base: BaseEventData::new("tool_usage_started"),
            agent_key: None,
            tool_name,
            tool_args,
            tool_class,
            run_attempts,
            delegations,
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
    /// Key identifying the agent.
    pub agent_key: Option<String>,
    /// Name of the tool.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: Value,
    /// Tool class name.
    pub tool_class: Option<String>,
    /// Number of run attempts.
    pub run_attempts: i64,
    /// Number of delegations.
    pub delegations: Option<i64>,
    /// When the tool execution started.
    pub started_at: DateTime<Utc>,
    /// When the tool execution finished.
    pub finished_at: DateTime<Utc>,
    /// Whether the result was served from cache.
    pub from_cache: bool,
    /// Tool output (serialised).
    pub output: Value,
}

impl ToolUsageFinishedEvent {
    pub fn new(
        tool_name: String,
        tool_args: Value,
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
            run_attempts: 0,
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
    /// Key identifying the agent.
    pub agent_key: Option<String>,
    /// Name of the tool.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: Value,
    /// Tool class name.
    pub tool_class: Option<String>,
    /// Number of run attempts.
    pub run_attempts: i64,
    /// Number of delegations.
    pub delegations: Option<i64>,
    /// Error description.
    pub error: Value,
}

impl ToolUsageErrorEvent {
    pub fn new(tool_name: String, tool_args: Value, error: Value) -> Self {
        Self {
            base: BaseEventData::new("tool_usage_error"),
            agent_key: None,
            tool_name,
            tool_args,
            tool_class: None,
            run_attempts: 0,
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
    /// Key identifying the agent.
    pub agent_key: Option<String>,
    /// Name of the tool.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: Value,
    /// Tool class name.
    pub tool_class: Option<String>,
    /// Number of run attempts.
    pub run_attempts: i64,
    /// Number of delegations.
    pub delegations: Option<i64>,
    /// Validation error description.
    pub error: Value,
}

impl ToolValidateInputErrorEvent {
    pub fn new(tool_name: String, tool_args: Value, error: Value) -> Self {
        Self {
            base: BaseEventData::new("tool_validate_input_error"),
            agent_key: None,
            tool_name,
            tool_args,
            tool_class: None,
            run_attempts: 0,
            delegations: None,
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
    /// Key identifying the agent.
    pub agent_key: Option<String>,
    /// Name of the tool.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: Value,
    /// Tool class name.
    pub tool_class: Option<String>,
    /// Number of run attempts.
    pub run_attempts: i64,
    /// Number of delegations.
    pub delegations: Option<i64>,
    /// Selection error description.
    pub error: Value,
}

impl ToolSelectionErrorEvent {
    pub fn new(tool_name: String, tool_args: Value, error: Value) -> Self {
        Self {
            base: BaseEventData::new("tool_selection_error"),
            agent_key: None,
            tool_name,
            tool_args,
            tool_class: None,
            run_attempts: 0,
            delegations: None,
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
/// Corresponds to `ToolExecutionErrorEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionErrorEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Error description.
    pub error: Value,
    /// Name of the tool.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub tool_args: HashMap<String, Value>,
    /// Tool class name.
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
