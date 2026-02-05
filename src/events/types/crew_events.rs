//! Crew-related event types.
//!
//! Corresponds to `crewai/events/types/crew_events.py`.
//!
//! Contains events for the full crew lifecycle: kickoff, train, test,
//! and their corresponding completion / failure events.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// CrewKickoffStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a crew starts execution.
///
/// Corresponds to `CrewKickoffStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewKickoffStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the crew.
    pub crew_name: Option<String>,
    /// Input parameters.
    pub inputs: Option<HashMap<String, Value>>,
}

impl CrewKickoffStartedEvent {
    pub fn new(crew_name: Option<String>, inputs: Option<HashMap<String, Value>>) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("crew_kickoff_started"),
            crew_name,
            inputs,
        };
        evt.base.source_type = Some("crew".to_string());
        evt
    }
}

impl_base_event!(CrewKickoffStartedEvent);

// ---------------------------------------------------------------------------
// CrewKickoffCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a crew completes execution.
///
/// Corresponds to `CrewKickoffCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewKickoffCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the crew.
    pub crew_name: Option<String>,
    /// Crew output (serialised).
    pub output: Value,
    /// Total tokens consumed across all LLM calls.
    pub total_tokens: i64,
}

impl CrewKickoffCompletedEvent {
    pub fn new(crew_name: Option<String>, output: Value, total_tokens: i64) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("crew_kickoff_completed"),
            crew_name,
            output,
            total_tokens,
        };
        evt.base.source_type = Some("crew".to_string());
        evt
    }
}

impl_base_event!(CrewKickoffCompletedEvent);

// ---------------------------------------------------------------------------
// CrewKickoffFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a crew fails to complete execution.
///
/// Corresponds to `CrewKickoffFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewKickoffFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the crew.
    pub crew_name: Option<String>,
    /// Error message.
    pub error: String,
}

impl CrewKickoffFailedEvent {
    pub fn new(crew_name: Option<String>, error: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("crew_kickoff_failed"),
            crew_name,
            error,
        };
        evt.base.source_type = Some("crew".to_string());
        evt
    }
}

impl_base_event!(CrewKickoffFailedEvent);

// ---------------------------------------------------------------------------
// CrewTrainStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a crew starts training.
///
/// Corresponds to `CrewTrainStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewTrainStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the crew.
    pub crew_name: Option<String>,
    /// Number of training iterations.
    pub n_iterations: i64,
    /// Filename for training output.
    pub filename: String,
    /// Input parameters.
    pub inputs: Option<HashMap<String, Value>>,
}

impl CrewTrainStartedEvent {
    pub fn new(
        crew_name: Option<String>,
        n_iterations: i64,
        filename: String,
        inputs: Option<HashMap<String, Value>>,
    ) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("crew_train_started"),
            crew_name,
            n_iterations,
            filename,
            inputs,
        };
        evt.base.source_type = Some("crew".to_string());
        evt
    }
}

impl_base_event!(CrewTrainStartedEvent);

// ---------------------------------------------------------------------------
// CrewTrainCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a crew completes training.
///
/// Corresponds to `CrewTrainCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewTrainCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the crew.
    pub crew_name: Option<String>,
    /// Number of training iterations completed.
    pub n_iterations: i64,
    /// Filename for training output.
    pub filename: String,
}

impl CrewTrainCompletedEvent {
    pub fn new(crew_name: Option<String>, n_iterations: i64, filename: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("crew_train_completed"),
            crew_name,
            n_iterations,
            filename,
        };
        evt.base.source_type = Some("crew".to_string());
        evt
    }
}

impl_base_event!(CrewTrainCompletedEvent);

// ---------------------------------------------------------------------------
// CrewTrainFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a crew fails to complete training.
///
/// Corresponds to `CrewTrainFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewTrainFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the crew.
    pub crew_name: Option<String>,
    /// Error message.
    pub error: String,
}

impl CrewTrainFailedEvent {
    pub fn new(crew_name: Option<String>, error: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("crew_train_failed"),
            crew_name,
            error,
        };
        evt.base.source_type = Some("crew".to_string());
        evt
    }
}

impl_base_event!(CrewTrainFailedEvent);

// ---------------------------------------------------------------------------
// CrewTestStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a crew starts testing.
///
/// Corresponds to `CrewTestStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewTestStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the crew.
    pub crew_name: Option<String>,
    /// Number of test iterations.
    pub n_iterations: i64,
    /// Evaluation LLM model string (or null).
    pub eval_llm: Option<String>,
    /// Input parameters.
    pub inputs: Option<HashMap<String, Value>>,
}

impl CrewTestStartedEvent {
    pub fn new(
        crew_name: Option<String>,
        n_iterations: i64,
        eval_llm: Option<String>,
        inputs: Option<HashMap<String, Value>>,
    ) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("crew_test_started"),
            crew_name,
            n_iterations,
            eval_llm,
            inputs,
        };
        evt.base.source_type = Some("crew".to_string());
        evt
    }
}

impl_base_event!(CrewTestStartedEvent);

// ---------------------------------------------------------------------------
// CrewTestCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a crew completes testing.
///
/// Corresponds to `CrewTestCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewTestCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the crew.
    pub crew_name: Option<String>,
}

impl CrewTestCompletedEvent {
    pub fn new(crew_name: Option<String>) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("crew_test_completed"),
            crew_name,
        };
        evt.base.source_type = Some("crew".to_string());
        evt
    }
}

impl_base_event!(CrewTestCompletedEvent);

// ---------------------------------------------------------------------------
// CrewTestFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a crew fails to complete testing.
///
/// Corresponds to `CrewTestFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewTestFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the crew.
    pub crew_name: Option<String>,
    /// Error message.
    pub error: String,
}

impl CrewTestFailedEvent {
    pub fn new(crew_name: Option<String>, error: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("crew_test_failed"),
            crew_name,
            error,
        };
        evt.base.source_type = Some("crew".to_string());
        evt
    }
}

impl_base_event!(CrewTestFailedEvent);

// ---------------------------------------------------------------------------
// CrewTestResultEvent
// ---------------------------------------------------------------------------

/// Event emitted when a crew test result is available.
///
/// Corresponds to `CrewTestResultEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewTestResultEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Name of the crew.
    pub crew_name: Option<String>,
    /// Quality score.
    pub quality: f64,
    /// Execution duration in seconds.
    pub execution_duration: f64,
    /// Model used for evaluation.
    pub model: String,
}

impl CrewTestResultEvent {
    pub fn new(
        crew_name: Option<String>,
        quality: f64,
        execution_duration: f64,
        model: String,
    ) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("crew_test_result"),
            crew_name,
            quality,
            execution_duration,
            model,
        };
        evt.base.source_type = Some("crew".to_string());
        evt
    }
}

impl_base_event!(CrewTestResultEvent);
