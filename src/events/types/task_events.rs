//! Task-related event types.
//!
//! Corresponds to `crewai/events/types/task_events.py`.
//!
//! Contains events for the task lifecycle: started, completed, failed,
//! and evaluation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// TaskStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a task starts.
///
/// Corresponds to `TaskStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Context provided for the task.
    pub context: Option<String>,
}

impl TaskStartedEvent {
    pub fn new(task_id: Option<String>, task_name: Option<String>, context: Option<String>) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("task_started"),
            context,
        };
        evt.base.task_id = task_id;
        evt.base.task_name = task_name;
        evt.base.source_type = Some("task".to_string());
        evt
    }
}

impl_base_event!(TaskStartedEvent);

// ---------------------------------------------------------------------------
// TaskCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a task completes.
///
/// Corresponds to `TaskCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Task output (serialised as JSON value).
    pub output: Value,
}

impl TaskCompletedEvent {
    pub fn new(task_id: Option<String>, task_name: Option<String>, output: Value) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("task_completed"),
            output,
        };
        evt.base.task_id = task_id;
        evt.base.task_name = task_name;
        evt.base.source_type = Some("task".to_string());
        evt
    }
}

impl_base_event!(TaskCompletedEvent);

// ---------------------------------------------------------------------------
// TaskFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a task fails.
///
/// Corresponds to `TaskFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Error message.
    pub error: String,
}

impl TaskFailedEvent {
    pub fn new(task_id: Option<String>, task_name: Option<String>, error: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("task_failed"),
            error,
        };
        evt.base.task_id = task_id;
        evt.base.task_name = task_name;
        evt.base.source_type = Some("task".to_string());
        evt
    }
}

impl_base_event!(TaskFailedEvent);

// ---------------------------------------------------------------------------
// TaskEvaluationEvent
// ---------------------------------------------------------------------------

/// Event emitted when a task evaluation is completed.
///
/// Corresponds to `TaskEvaluationEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvaluationEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Type of evaluation performed.
    pub evaluation_type: String,
}

impl TaskEvaluationEvent {
    pub fn new(
        task_id: Option<String>,
        task_name: Option<String>,
        evaluation_type: String,
    ) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("task_evaluation"),
            evaluation_type,
        };
        evt.base.task_id = task_id;
        evt.base.task_name = task_name;
        evt.base.source_type = Some("task".to_string());
        evt
    }
}

impl_base_event!(TaskEvaluationEvent);
