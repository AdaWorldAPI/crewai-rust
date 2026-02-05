//! A2A client for remote agent delegation.
//!
//! Corresponds to `crewai/a2a/wrapper.py` (the client/wrapper portion).
//!
//! Provides an async client for communicating with remote A2A agents,
//! including sending messages, retrieving agent cards, and handling
//! various update mechanisms.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::auth::ClientAuthScheme;
use super::types::{PartsDict, ProtocolVersion, TransportType};
use super::updates::UpdateConfig;

// ---------------------------------------------------------------------------
// Agent card types
// ---------------------------------------------------------------------------

/// Describes a skill/capability that an A2A agent offers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkill {
    /// Unique identifier for the skill.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what the skill does.
    pub description: Option<String>,
    /// Input modes supported (e.g., "text", "image").
    #[serde(default)]
    pub input_modes: Vec<String>,
    /// Output modes supported.
    #[serde(default)]
    pub output_modes: Vec<String>,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Capabilities advertised by an A2A agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Whether the agent supports streaming.
    #[serde(default)]
    pub streaming: bool,
    /// Whether the agent supports push notifications.
    #[serde(default)]
    pub push_notifications: bool,
    /// Whether the agent supports multi-turn conversations.
    #[serde(default)]
    pub multi_turn: bool,
}

/// Provider information for an A2A agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProvider {
    /// Name of the provider organization.
    pub organization: String,
    /// Contact URL.
    pub url: Option<String>,
}

/// Agent card describing a remote A2A agent's capabilities and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    /// Agent name.
    pub name: String,
    /// Agent description.
    pub description: Option<String>,
    /// Agent URL endpoint.
    pub url: String,
    /// Protocol version.
    pub version: Option<String>,
    /// Agent capabilities.
    #[serde(default)]
    pub capabilities: AgentCapabilities,
    /// Agent skills.
    #[serde(default)]
    pub skills: Vec<AgentSkill>,
    /// Agent provider.
    pub provider: Option<AgentProvider>,
    /// Default input modes.
    #[serde(default)]
    pub default_input_modes: Vec<String>,
    /// Default output modes.
    #[serde(default)]
    pub default_output_modes: Vec<String>,
    /// Security schemes.
    #[serde(default)]
    pub security_schemes: Vec<Value>,
    /// Supported extensions.
    #[serde(default)]
    pub extensions: Vec<Value>,
}

// ---------------------------------------------------------------------------
// A2A Message types
// ---------------------------------------------------------------------------

/// A message in the A2A protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    /// Role of the message sender (e.g., "user", "agent").
    pub role: String,
    /// Message parts.
    pub parts: Vec<PartsDict>,
    /// Optional message metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}

/// Task state as returned by the A2A protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum A2ATaskState {
    Submitted,
    Working,
    InputRequired,
    Completed,
    Failed,
    Canceled,
}

/// An A2A task as returned by the protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATask {
    /// Task ID.
    pub id: String,
    /// Context ID for multi-turn conversations.
    pub context_id: Option<String>,
    /// Current task state.
    pub status: A2ATaskStatus,
    /// Messages in the task conversation.
    #[serde(default)]
    pub history: Vec<A2AMessage>,
    /// Task artifacts/outputs.
    #[serde(default)]
    pub artifacts: Vec<Value>,
    /// Task metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}

/// Status of an A2A task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATaskStatus {
    /// Current state.
    pub state: A2ATaskState,
    /// Status message.
    pub message: Option<A2AMessage>,
    /// Timestamp.
    pub timestamp: Option<String>,
}

// ---------------------------------------------------------------------------
// A2A Client
// ---------------------------------------------------------------------------

/// Result of a task state query.
#[derive(Debug, Clone)]
pub struct TaskStateResult {
    /// Whether the task completed successfully.
    pub success: bool,
    /// The result text (if successful).
    pub result: Option<String>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Message history.
    pub history: Vec<A2AMessage>,
}

/// Client for communicating with remote A2A agents.
///
/// Handles agent card retrieval, message sending, and update mechanism
/// negotiation for the A2A protocol.
pub struct A2AClient {
    /// Base URL of the remote A2A agent.
    pub endpoint: String,
    /// Authentication scheme (type-erased).
    pub auth: Option<Arc<dyn ClientAuthScheme>>,
    /// Update configuration.
    pub update_config: Option<UpdateConfig>,
    /// Cached agent card.
    pub agent_card: Option<AgentCard>,
    /// HTTP client timeout in seconds.
    pub timeout: u64,
}

impl std::fmt::Debug for A2AClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("A2AClient")
            .field("endpoint", &self.endpoint)
            .field("auth", &self.auth.as_ref().map(|_| "<auth>"))
            .field("update_config", &self.update_config)
            .field("agent_card", &self.agent_card)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl A2AClient {
    /// Create a new `A2AClient`.
    pub fn new(
        endpoint: impl Into<String>,
        auth: Option<Arc<dyn ClientAuthScheme>>,
        update_config: Option<UpdateConfig>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            auth,
            update_config,
            agent_card: None,
            timeout: 30,
        }
    }

    /// Retrieve the agent card from the remote agent.
    ///
    /// Fetches `/.well-known/agent.json` from the agent endpoint.
    pub async fn get_agent_card(&mut self) -> Result<AgentCard, anyhow::Error> {
        // TODO: Implement actual HTTP request to fetch agent card.
        // For now, return an error indicating this is not yet implemented.
        anyhow::bail!(
            "A2A client get_agent_card not yet implemented for endpoint: {}",
            self.endpoint
        )
    }

    /// Send a message to the remote agent and get a response.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to send.
    /// * `context_id` - Optional context ID for multi-turn conversations.
    /// * `task_id` - Optional existing task ID to continue.
    pub async fn send_message(
        &self,
        message: A2AMessage,
        context_id: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<TaskStateResult, anyhow::Error> {
        // TODO: Implement actual A2A message sending via the configured transport.
        anyhow::bail!(
            "A2A client send_message not yet implemented for endpoint: {}",
            self.endpoint
        )
    }

    /// Send a message and wait for the task to complete using the configured
    /// update mechanism (polling, streaming, or push notifications).
    pub async fn send_and_wait(
        &self,
        message: A2AMessage,
        context_id: Option<&str>,
    ) -> Result<TaskStateResult, anyhow::Error> {
        // TODO: Implement send + wait using the update config.
        anyhow::bail!(
            "A2A client send_and_wait not yet implemented for endpoint: {}",
            self.endpoint
        )
    }

    /// Cancel a running task.
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), anyhow::Error> {
        // TODO: Implement task cancellation.
        anyhow::bail!(
            "A2A client cancel_task not yet implemented for task: {}",
            task_id
        )
    }
}
