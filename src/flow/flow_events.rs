//! Flow-specific event types.
//!
//! Corresponds to the flow event types from `crewai/events/types/flow_events.py`.
//! Defines events emitted during flow lifecycle: creation, start, pause, finish,
//! and per-method execution events.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Event emitted when a flow is created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowCreatedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub flow_name: String,
}

/// Event emitted when a flow starts executing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStartedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub flow_name: String,
    #[serde(default)]
    pub inputs: Option<Value>,
}

/// Event emitted when a flow is paused (e.g., waiting for human feedback).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowPausedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub flow_name: String,
    pub flow_id: String,
    pub method_name: String,
    #[serde(default)]
    pub state: Option<Value>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub emit: Option<Vec<String>>,
}

/// Event emitted when a flow finishes executing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowFinishedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub flow_name: String,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub state: Option<Value>,
}

/// Event emitted when a flow is plotted/visualized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowPlotEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub flow_name: String,
    #[serde(default)]
    pub filename: Option<String>,
}

/// Event emitted when a method starts executing within a flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodExecutionStartedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub flow_name: String,
    pub method_name: String,
    #[serde(default)]
    pub state: Option<Value>,
}

/// Event emitted when a method finishes executing within a flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodExecutionFinishedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub flow_name: String,
    pub method_name: String,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub state: Option<Value>,
}

/// Event emitted when a method execution fails within a flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodExecutionFailedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub flow_name: String,
    pub method_name: String,
    pub error: String,
    #[serde(default)]
    pub state: Option<Value>,
}

/// Event emitted when a method execution is paused (e.g., human feedback).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodExecutionPausedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub flow_name: String,
    pub method_name: String,
    #[serde(default)]
    pub state: Option<Value>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub emit: Option<Vec<String>>,
}

/// Event emitted when human feedback is requested.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanFeedbackRequestedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub flow_name: String,
    pub method_name: String,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub emit: Option<Vec<String>>,
}

/// Event emitted when human feedback is received.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanFeedbackReceivedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub flow_name: String,
    pub method_name: String,
    #[serde(default)]
    pub feedback: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
}

/// Enum covering all possible flow events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FlowEvent {
    #[serde(rename = "flow_created")]
    FlowCreated(FlowCreatedEvent),
    #[serde(rename = "flow_started")]
    FlowStarted(FlowStartedEvent),
    #[serde(rename = "flow_paused")]
    FlowPaused(FlowPausedEvent),
    #[serde(rename = "flow_finished")]
    FlowFinished(FlowFinishedEvent),
    #[serde(rename = "flow_plot")]
    FlowPlot(FlowPlotEvent),
    #[serde(rename = "method_execution_started")]
    MethodExecutionStarted(MethodExecutionStartedEvent),
    #[serde(rename = "method_execution_finished")]
    MethodExecutionFinished(MethodExecutionFinishedEvent),
    #[serde(rename = "method_execution_failed")]
    MethodExecutionFailed(MethodExecutionFailedEvent),
    #[serde(rename = "method_execution_paused")]
    MethodExecutionPaused(MethodExecutionPausedEvent),
    #[serde(rename = "human_feedback_requested")]
    HumanFeedbackRequested(HumanFeedbackRequestedEvent),
    #[serde(rename = "human_feedback_received")]
    HumanFeedbackReceived(HumanFeedbackReceivedEvent),
}
