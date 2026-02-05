//! Human-in-the-loop (HITL) type definitions.
//!
//! Corresponds to `crewai/types/hitl.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HITL resume information passed from flow to crew.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HITLResumeInfo {
    /// Unique identifier for the task.
    #[serde(default)]
    pub task_id: Option<String>,
    /// Unique identifier for the crew execution.
    #[serde(default)]
    pub crew_execution_id: Option<String>,
    /// Key identifying the specific task.
    #[serde(default)]
    pub task_key: Option<String>,
    /// Output from the task before human intervention.
    #[serde(default)]
    pub task_output: Option<String>,
    /// Feedback provided by the human.
    #[serde(default)]
    pub human_feedback: Option<String>,
    /// History of messages in the conversation.
    #[serde(default)]
    pub previous_messages: Vec<HashMap<String, String>>,
}

/// Crew inputs that may contain HITL resume information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CrewInputsWithHITL {
    /// Optional HITL resume information for continuing execution.
    #[serde(rename = "_hitl_resume", default)]
    pub hitl_resume: Option<HITLResumeInfo>,
}
