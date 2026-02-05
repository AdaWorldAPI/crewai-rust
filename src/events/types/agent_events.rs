//! Agent-related event types.
//!
//! Corresponds to `crewai/events/types/agent_events.py`.
//!
//! Contains events for agent execution lifecycle, lite-agent execution,
//! and agent evaluation. The Python version references `BaseAgent` directly;
//! here we use serialisable primitives to avoid circular dependencies.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// AgentExecutionStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when an agent starts executing a task.
///
/// Corresponds to `AgentExecutionStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The task prompt being executed.
    pub task_prompt: String,
    /// Tool names available to the agent.
    pub tools: Option<Vec<String>>,
}

impl AgentExecutionStartedEvent {
    pub fn new(
        agent_role: String,
        agent_id: String,
        task_prompt: String,
        tools: Option<Vec<String>>,
    ) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("agent_execution_started"),
            task_prompt,
            tools,
        };
        evt.base.agent_role = Some(agent_role);
        evt.base.agent_id = Some(agent_id);
        evt.base.source_type = Some("agent".to_string());
        evt
    }
}

impl_base_event!(AgentExecutionStartedEvent);

// ---------------------------------------------------------------------------
// AgentExecutionCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when an agent completes executing a task.
///
/// Corresponds to `AgentExecutionCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The output produced by the agent.
    pub output: String,
}

impl AgentExecutionCompletedEvent {
    pub fn new(agent_role: String, agent_id: String, output: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("agent_execution_completed"),
            output,
        };
        evt.base.agent_role = Some(agent_role);
        evt.base.agent_id = Some(agent_id);
        evt.base.source_type = Some("agent".to_string());
        evt
    }
}

impl_base_event!(AgentExecutionCompletedEvent);

// ---------------------------------------------------------------------------
// AgentExecutionErrorEvent
// ---------------------------------------------------------------------------

/// Event emitted when an agent encounters an error during execution.
///
/// Corresponds to `AgentExecutionErrorEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionErrorEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Error message.
    pub error: String,
}

impl AgentExecutionErrorEvent {
    pub fn new(agent_role: String, agent_id: String, error: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("agent_execution_error"),
            error,
        };
        evt.base.agent_role = Some(agent_role);
        evt.base.agent_id = Some(agent_id);
        evt.base.source_type = Some("agent".to_string());
        evt
    }
}

impl_base_event!(AgentExecutionErrorEvent);

// ---------------------------------------------------------------------------
// LiteAgentExecutionStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a LiteAgent starts executing.
///
/// Corresponds to `LiteAgentExecutionStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteAgentExecutionStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Agent information dictionary (role, goal, backstory, etc.).
    pub agent_info: HashMap<String, Value>,
    /// Tool names available to the agent.
    pub tools: Option<Vec<String>>,
    /// Messages sent to the agent (string or structured list).
    pub messages: Value,
}

impl LiteAgentExecutionStartedEvent {
    pub fn new(
        agent_info: HashMap<String, Value>,
        tools: Option<Vec<String>>,
        messages: Value,
    ) -> Self {
        Self {
            base: BaseEventData::new("lite_agent_execution_started"),
            agent_info,
            tools,
            messages,
        }
    }
}

impl_base_event!(LiteAgentExecutionStartedEvent);

// ---------------------------------------------------------------------------
// LiteAgentExecutionCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a LiteAgent completes execution.
///
/// Corresponds to `LiteAgentExecutionCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteAgentExecutionCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Agent information dictionary.
    pub agent_info: HashMap<String, Value>,
    /// The output produced by the lite agent.
    pub output: String,
}

impl LiteAgentExecutionCompletedEvent {
    pub fn new(agent_info: HashMap<String, Value>, output: String) -> Self {
        Self {
            base: BaseEventData::new("lite_agent_execution_completed"),
            agent_info,
            output,
        }
    }
}

impl_base_event!(LiteAgentExecutionCompletedEvent);

// ---------------------------------------------------------------------------
// LiteAgentExecutionErrorEvent
// ---------------------------------------------------------------------------

/// Event emitted when a LiteAgent encounters an error during execution.
///
/// Corresponds to `LiteAgentExecutionErrorEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteAgentExecutionErrorEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Agent information dictionary.
    pub agent_info: HashMap<String, Value>,
    /// Error message.
    pub error: String,
}

impl LiteAgentExecutionErrorEvent {
    pub fn new(agent_info: HashMap<String, Value>, error: String) -> Self {
        Self {
            base: BaseEventData::new("lite_agent_execution_error"),
            agent_info,
            error,
        }
    }
}

impl_base_event!(LiteAgentExecutionErrorEvent);

// ---------------------------------------------------------------------------
// AgentEvaluationStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when an agent evaluation starts.
///
/// Corresponds to `AgentEvaluationStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvaluationStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Evaluation iteration number.
    pub iteration: i64,
}

impl AgentEvaluationStartedEvent {
    pub fn new(
        agent_id: String,
        agent_role: String,
        task_id: Option<String>,
        iteration: i64,
    ) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("agent_evaluation_started"),
            iteration,
        };
        evt.base.agent_id = Some(agent_id);
        evt.base.agent_role = Some(agent_role);
        evt.base.task_id = task_id;
        evt
    }
}

impl_base_event!(AgentEvaluationStartedEvent);

// ---------------------------------------------------------------------------
// AgentEvaluationCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when an agent evaluation completes.
///
/// Corresponds to `AgentEvaluationCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvaluationCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Evaluation iteration number.
    pub iteration: i64,
    /// Metric category of the evaluation.
    pub metric_category: Value,
    /// Evaluation score.
    pub score: Value,
}

impl AgentEvaluationCompletedEvent {
    pub fn new(
        agent_id: String,
        agent_role: String,
        task_id: Option<String>,
        iteration: i64,
        metric_category: Value,
        score: Value,
    ) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("agent_evaluation_completed"),
            iteration,
            metric_category,
            score,
        };
        evt.base.agent_id = Some(agent_id);
        evt.base.agent_role = Some(agent_role);
        evt.base.task_id = task_id;
        evt
    }
}

impl_base_event!(AgentEvaluationCompletedEvent);

// ---------------------------------------------------------------------------
// AgentEvaluationFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when an agent evaluation fails.
///
/// Corresponds to `AgentEvaluationFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvaluationFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Evaluation iteration number.
    pub iteration: i64,
    /// Error message.
    pub error: String,
}

impl AgentEvaluationFailedEvent {
    pub fn new(
        agent_id: String,
        agent_role: String,
        task_id: Option<String>,
        iteration: i64,
        error: String,
    ) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("agent_evaluation_failed"),
            iteration,
            error,
        };
        evt.base.agent_id = Some(agent_id);
        evt.base.agent_role = Some(agent_role);
        evt.base.task_id = task_id;
        evt
    }
}

impl_base_event!(AgentEvaluationFailedEvent);
