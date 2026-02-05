//! Flow-related event types.
//!
//! Corresponds to `crewai/events/types/flow_events.py`.
//!
//! Contains events for the flow lifecycle (creation, start, finish, pause),
//! method execution events, and human feedback events.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// FlowStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a flow starts execution.
///
/// Corresponds to `FlowStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the flow.
    pub flow_name: String,
    /// Input parameters.
    pub inputs: Option<HashMap<String, Value>>,
}

impl FlowStartedEvent {
    pub fn new(flow_name: String, inputs: Option<HashMap<String, Value>>) -> Self {
        Self {
            base: BaseEventData::new("flow_started"),
            flow_name,
            inputs,
        }
    }
}

impl_base_event!(FlowStartedEvent);

// ---------------------------------------------------------------------------
// FlowCreatedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a flow is created.
///
/// Corresponds to `FlowCreatedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowCreatedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the flow.
    pub flow_name: String,
}

impl FlowCreatedEvent {
    pub fn new(flow_name: String) -> Self {
        Self {
            base: BaseEventData::new("flow_created"),
            flow_name,
        }
    }
}

impl_base_event!(FlowCreatedEvent);

// ---------------------------------------------------------------------------
// MethodExecutionStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a flow method starts execution.
///
/// Corresponds to `MethodExecutionStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodExecutionStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the flow.
    pub flow_name: String,
    /// Name of the method being executed.
    pub method_name: String,
    /// Current flow state (serialised).
    pub state: Value,
    /// Parameters passed to the method.
    pub params: Option<HashMap<String, Value>>,
}

impl MethodExecutionStartedEvent {
    pub fn new(
        flow_name: String,
        method_name: String,
        state: Value,
        params: Option<HashMap<String, Value>>,
    ) -> Self {
        Self {
            base: BaseEventData::new("method_execution_started"),
            flow_name,
            method_name,
            state,
            params,
        }
    }
}

impl_base_event!(MethodExecutionStartedEvent);

// ---------------------------------------------------------------------------
// MethodExecutionFinishedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a flow method completes execution.
///
/// Corresponds to `MethodExecutionFinishedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodExecutionFinishedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the flow.
    pub flow_name: String,
    /// Name of the method that completed.
    pub method_name: String,
    /// Result returned by the method.
    pub result: Option<Value>,
    /// Flow state after method execution.
    pub state: Value,
}

impl MethodExecutionFinishedEvent {
    pub fn new(
        flow_name: String,
        method_name: String,
        result: Option<Value>,
        state: Value,
    ) -> Self {
        Self {
            base: BaseEventData::new("method_execution_finished"),
            flow_name,
            method_name,
            result,
            state,
        }
    }
}

impl_base_event!(MethodExecutionFinishedEvent);

// ---------------------------------------------------------------------------
// MethodExecutionFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a flow method fails execution.
///
/// Corresponds to `MethodExecutionFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodExecutionFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the flow.
    pub flow_name: String,
    /// Name of the method that failed.
    pub method_name: String,
    /// Error message.
    pub error: String,
}

impl MethodExecutionFailedEvent {
    pub fn new(flow_name: String, method_name: String, error: String) -> Self {
        Self {
            base: BaseEventData::new("method_execution_failed"),
            flow_name,
            method_name,
            error,
        }
    }
}

impl_base_event!(MethodExecutionFailedEvent);

// ---------------------------------------------------------------------------
// MethodExecutionPausedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a flow method is paused waiting for human feedback.
///
/// This event is emitted when a `@human_feedback` decorated method with an
/// async provider raises `HumanFeedbackPending` to pause execution.
///
/// Corresponds to `MethodExecutionPausedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodExecutionPausedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the flow.
    pub flow_name: String,
    /// Name of the method waiting for feedback.
    pub method_name: String,
    /// Current flow state when paused (serialised).
    pub state: Value,
    /// Unique identifier for this flow execution.
    pub flow_id: String,
    /// The message shown when requesting feedback.
    pub message: String,
    /// Optional list of possible outcomes for routing.
    pub emit: Option<Vec<String>>,
}

impl MethodExecutionPausedEvent {
    pub fn new(
        flow_name: String,
        method_name: String,
        state: Value,
        flow_id: String,
        message: String,
        emit: Option<Vec<String>>,
    ) -> Self {
        Self {
            base: BaseEventData::new("method_execution_paused"),
            flow_name,
            method_name,
            state,
            flow_id,
            message,
            emit,
        }
    }
}

impl_base_event!(MethodExecutionPausedEvent);

// ---------------------------------------------------------------------------
// FlowFinishedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a flow completes execution.
///
/// Corresponds to `FlowFinishedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowFinishedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the flow.
    pub flow_name: String,
    /// Result of the flow execution.
    pub result: Option<Value>,
    /// Final flow state (serialised).
    pub state: Value,
}

impl FlowFinishedEvent {
    pub fn new(flow_name: String, result: Option<Value>, state: Value) -> Self {
        Self {
            base: BaseEventData::new("flow_finished"),
            flow_name,
            result,
            state,
        }
    }
}

impl_base_event!(FlowFinishedEvent);

// ---------------------------------------------------------------------------
// FlowPausedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a flow is paused waiting for human feedback.
///
/// Corresponds to `FlowPausedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowPausedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the flow.
    pub flow_name: String,
    /// Unique identifier for this flow execution.
    pub flow_id: String,
    /// Name of the method waiting for feedback.
    pub method_name: String,
    /// Current flow state when paused (serialised).
    pub state: Value,
    /// The message shown when requesting feedback.
    pub message: String,
    /// Optional list of possible outcomes for routing.
    pub emit: Option<Vec<String>>,
}

impl FlowPausedEvent {
    pub fn new(
        flow_name: String,
        flow_id: String,
        method_name: String,
        state: Value,
        message: String,
        emit: Option<Vec<String>>,
    ) -> Self {
        Self {
            base: BaseEventData::new("flow_paused"),
            flow_name,
            flow_id,
            method_name,
            state,
            message,
            emit,
        }
    }
}

impl_base_event!(FlowPausedEvent);

// ---------------------------------------------------------------------------
// FlowPlotEvent
// ---------------------------------------------------------------------------

/// Event emitted when a flow plot is created.
///
/// Corresponds to `FlowPlotEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowPlotEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the flow.
    pub flow_name: String,
}

impl FlowPlotEvent {
    pub fn new(flow_name: String) -> Self {
        Self {
            base: BaseEventData::new("flow_plot"),
            flow_name,
        }
    }
}

impl_base_event!(FlowPlotEvent);

// ---------------------------------------------------------------------------
// HumanFeedbackRequestedEvent
// ---------------------------------------------------------------------------

/// Event emitted when human feedback is requested.
///
/// This event is emitted when a `@human_feedback` decorated method requires
/// input from a human reviewer.
///
/// Corresponds to `HumanFeedbackRequestedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanFeedbackRequestedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the flow.
    pub flow_name: String,
    /// Name of the method decorated with `@human_feedback`.
    pub method_name: String,
    /// The method output shown to the human for review.
    pub output: Value,
    /// The message displayed when requesting feedback.
    pub message: String,
    /// Optional list of possible outcomes for routing.
    pub emit: Option<Vec<String>>,
}

impl HumanFeedbackRequestedEvent {
    pub fn new(
        flow_name: String,
        method_name: String,
        output: Value,
        message: String,
        emit: Option<Vec<String>>,
    ) -> Self {
        Self {
            base: BaseEventData::new("human_feedback_requested"),
            flow_name,
            method_name,
            output,
            message,
            emit,
        }
    }
}

impl_base_event!(HumanFeedbackRequestedEvent);

// ---------------------------------------------------------------------------
// HumanFeedbackReceivedEvent
// ---------------------------------------------------------------------------

/// Event emitted when human feedback is received.
///
/// This event is emitted after a human provides feedback in response to a
/// `@human_feedback` decorated method.
///
/// Corresponds to `HumanFeedbackReceivedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanFeedbackReceivedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the flow.
    pub flow_name: String,
    /// Name of the method that received feedback.
    pub method_name: String,
    /// The raw text feedback provided by the human.
    pub feedback: String,
    /// The collapsed outcome string (if emit was specified).
    pub outcome: Option<String>,
}

impl HumanFeedbackReceivedEvent {
    pub fn new(
        flow_name: String,
        method_name: String,
        feedback: String,
        outcome: Option<String>,
    ) -> Self {
        Self {
            base: BaseEventData::new("human_feedback_received"),
            flow_name,
            method_name,
            feedback,
            outcome,
        }
    }
}

impl_base_event!(HumanFeedbackReceivedEvent);
