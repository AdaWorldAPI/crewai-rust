//! Agent logging event types.
//!
//! Corresponds to `crewai/events/types/logging_events.py`.
//!
//! These events do not reference BaseAgent directly to avoid circular
//! dependencies (mirroring the Python design).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// AgentLogsStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when agent logs should be shown at start.
///
/// Corresponds to `AgentLogsStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLogsStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Task description, if available.
    pub task_description: Option<String>,
    /// Whether verbose output is enabled.
    pub verbose: bool,
}

impl AgentLogsStartedEvent {
    pub fn new(agent_role: String, task_description: Option<String>, verbose: bool) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("agent_logs_started"),
            task_description,
            verbose,
        };
        evt.base.agent_role = Some(agent_role);
        evt
    }
}

impl_base_event!(AgentLogsStartedEvent);

// ---------------------------------------------------------------------------
// AgentLogsExecutionEvent
// ---------------------------------------------------------------------------

/// Event emitted when agent logs should be shown during execution.
///
/// Corresponds to `AgentLogsExecutionEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLogsExecutionEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Formatted answer content (serialised).
    pub formatted_answer: Value,
    /// Whether verbose output is enabled.
    pub verbose: bool,
}

impl AgentLogsExecutionEvent {
    pub fn new(agent_role: String, formatted_answer: Value, verbose: bool) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("agent_logs_execution"),
            formatted_answer,
            verbose,
        };
        evt.base.agent_role = Some(agent_role);
        evt
    }
}

impl_base_event!(AgentLogsExecutionEvent);
