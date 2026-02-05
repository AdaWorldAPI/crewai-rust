//! Reasoning event types.
//!
//! Corresponds to `crewai/events/types/reasoning_events.py`.

use serde::{Deserialize, Serialize};

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// AgentReasoningStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when an agent starts reasoning about a task.
///
/// Corresponds to `AgentReasoningStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReasoningStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Reasoning attempt number.
    pub attempt: i64,
}

impl AgentReasoningStartedEvent {
    pub fn new(agent_role: String, task_id: String, attempt: i64) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("agent_reasoning_started"),
            attempt,
        };
        evt.base.agent_role = Some(agent_role);
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(AgentReasoningStartedEvent);

// ---------------------------------------------------------------------------
// AgentReasoningCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when an agent finishes its reasoning process.
///
/// Corresponds to `AgentReasoningCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReasoningCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Reasoning attempt number.
    pub attempt: i64,
    /// The plan produced by reasoning.
    pub plan: String,
    /// Whether the agent is ready to proceed.
    pub ready: bool,
}

impl AgentReasoningCompletedEvent {
    pub fn new(agent_role: String, task_id: String, attempt: i64, plan: String, ready: bool) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("agent_reasoning_completed"),
            attempt,
            plan,
            ready,
        };
        evt.base.agent_role = Some(agent_role);
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(AgentReasoningCompletedEvent);

// ---------------------------------------------------------------------------
// AgentReasoningFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when the reasoning process fails.
///
/// Corresponds to `AgentReasoningFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReasoningFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Reasoning attempt number.
    pub attempt: i64,
    /// Error message.
    pub error: String,
}

impl AgentReasoningFailedEvent {
    pub fn new(agent_role: String, task_id: String, attempt: i64, error: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("agent_reasoning_failed"),
            attempt,
            error,
        };
        evt.base.agent_role = Some(agent_role);
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(AgentReasoningFailedEvent);
