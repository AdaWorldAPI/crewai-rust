//! FlowTrackable mixin for tracking flow context.
//!
//! Corresponds to `crewai/flow/flow_trackable.py`.

use std::sync::Arc;
use tokio::sync::Mutex;

/// Tracks flow execution context for objects created within flows.
///
/// When a Crew or Agent is instantiated inside a flow execution, this struct
/// captures the flow ID and request ID from context variables, enabling
/// proper tracking and association with the parent flow execution.
///
/// In the Python implementation, this is a Pydantic mixin that uses
/// `model_validator(mode="after")`. In Rust, we use explicit initialization.
///
/// Corresponds to `crewai.flow.flow_trackable.FlowTrackable`.
#[derive(Debug, Clone)]
pub struct FlowTrackable {
    /// The request ID from the current flow context.
    pub request_id: Option<String>,
    /// The flow ID from the current flow context.
    pub flow_id: Option<String>,
}

impl Default for FlowTrackable {
    fn default() -> Self {
        Self {
            request_id: None,
            flow_id: None,
        }
    }
}

impl FlowTrackable {
    /// Create a new FlowTrackable.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new FlowTrackable with flow context.
    pub fn with_context(flow_id: String, request_id: String) -> Self {
        Self {
            request_id: Some(request_id),
            flow_id: Some(flow_id),
        }
    }

    /// Set the flow context from context variables.
    ///
    /// This mimics the Python `model_validator(mode="after")` behavior.
    pub fn set_flow_context(&mut self, flow_id: Option<String>, request_id: Option<String>) {
        if let Some(rid) = request_id {
            self.request_id = Some(rid);
            self.flow_id = flow_id;
        }
    }

    /// Check if this object is being tracked within a flow.
    pub fn is_in_flow(&self) -> bool {
        self.flow_id.is_some()
    }
}

/// Thread-local flow context (simulates Python's contextvars).
///
/// In the Python implementation, `current_flow_id` and `current_flow_request_id`
/// are ContextVar instances. In Rust, we use tokio task-local or thread-local
/// alternatives.
thread_local! {
    static CURRENT_FLOW_ID: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    static CURRENT_FLOW_REQUEST_ID: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

/// Get the current flow ID from thread-local context.
pub fn current_flow_id() -> Option<String> {
    CURRENT_FLOW_ID.with(|id| id.borrow().clone())
}

/// Set the current flow ID in thread-local context.
pub fn set_current_flow_id(id: Option<String>) {
    CURRENT_FLOW_ID.with(|current| {
        *current.borrow_mut() = id;
    });
}

/// Get the current flow request ID from thread-local context.
pub fn current_flow_request_id() -> Option<String> {
    CURRENT_FLOW_REQUEST_ID.with(|id| id.borrow().clone())
}

/// Set the current flow request ID in thread-local context.
pub fn set_current_flow_request_id(id: Option<String>) {
    CURRENT_FLOW_REQUEST_ID.with(|current| {
        *current.borrow_mut() = id;
    });
}
