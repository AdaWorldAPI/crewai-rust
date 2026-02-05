//! A2A agent wrapping logic for delegation.
//!
//! Corresponds to `crewai/a2a/wrapper.py`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::a2a::config::A2AClientConfig;

/// Context prepared for A2A delegation.
///
/// Groups all the values needed to execute a delegation to a remote A2A agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationContext {
    /// Configs for all available A2A agents.
    pub a2a_agents: Vec<A2AClientConfig>,
    /// The current request text being delegated.
    pub current_request: String,
    /// Identifier of the agent performing delegation.
    pub agent_id: String,
    /// The specific A2A agent config to delegate to.
    pub agent_config: A2AClientConfig,
    /// A2A context ID for conversation continuity.
    pub context_id: Option<String>,
    /// A2A task ID.
    pub task_id: Option<String>,
    /// Additional metadata for the delegation.
    pub metadata: Option<HashMap<String, Value>>,
    /// Extension-specific data.
    pub extensions: Option<HashMap<String, Value>>,
    /// IDs of referenced tasks.
    pub reference_task_ids: Vec<String>,
    /// Original task description (before delegation augmentation).
    pub original_task_description: String,
    /// Maximum conversation turns.
    pub max_turns: u32,
}

/// State accumulated during a delegation conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationState {
    /// Context ID for the conversation.
    pub context_id: Option<String>,
    /// Task ID for the current task.
    pub task_id: Option<String>,
    /// Number of turns completed.
    pub turns_completed: u32,
    /// Collected message history.
    pub messages: Vec<Value>,
    /// Whether the delegation is complete.
    pub is_complete: bool,
    /// Final result text (if complete).
    pub result: Option<String>,
}

impl Default for DelegationState {
    fn default() -> Self {
        Self {
            context_id: None,
            task_id: None,
            turns_completed: 0,
            messages: Vec::new(),
            is_complete: false,
            result: None,
        }
    }
}

impl DelegationState {
    /// Create a new empty `DelegationState`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new message in the conversation.
    pub fn add_message(&mut self, message: Value) {
        self.messages.push(message);
        self.turns_completed += 1;
    }

    /// Mark the delegation as complete with the given result.
    pub fn complete(&mut self, result: String) {
        self.is_complete = true;
        self.result = Some(result);
    }
}
