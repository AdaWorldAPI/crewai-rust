//! A2A (Agent-to-Agent) delegation event types.
//!
//! Corresponds to `crewai/events/types/a2a_events.py`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// A2ADelegationStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when A2A delegation starts.
///
/// Corresponds to `A2ADelegationStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ADelegationStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A agent endpoint URL (AgentCard URL).
    pub endpoint: String,
    /// Task being delegated to the A2A agent.
    pub task_description: String,
    /// A2A context ID grouping related tasks.
    pub context_id: Option<String>,
    /// Whether this is part of a multiturn conversation.
    pub is_multiturn: bool,
    /// Current turn number (1-indexed, 1 for single-turn).
    pub turn_number: i64,
    /// Name of the A2A agent from agent card.
    pub a2a_agent_name: Option<String>,
    /// Full A2A agent card metadata.
    pub agent_card: Option<HashMap<String, Value>>,
    /// A2A protocol version being used.
    pub protocol_version: Option<String>,
    /// Agent provider/organization info from agent card.
    pub provider: Option<HashMap<String, Value>>,
    /// ID of the specific skill being invoked.
    pub skill_id: Option<String>,
    /// Custom A2A metadata key-value pairs.
    pub metadata: Option<HashMap<String, Value>>,
    /// List of A2A extension URIs in use.
    pub extensions: Option<Vec<String>>,
}

impl A2ADelegationStartedEvent {
    pub fn new(
        endpoint: String,
        task_description: String,
        agent_id: String,
    ) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("a2a_delegation_started"),
            endpoint,
            task_description,
            context_id: None,
            is_multiturn: false,
            turn_number: 1,
            a2a_agent_name: None,
            agent_card: None,
            protocol_version: None,
            provider: None,
            skill_id: None,
            metadata: None,
            extensions: None,
        };
        evt.base.agent_id = Some(agent_id);
        evt
    }
}

impl_base_event!(A2ADelegationStartedEvent);

// ---------------------------------------------------------------------------
// A2ADelegationCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when A2A delegation completes.
///
/// Corresponds to `A2ADelegationCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ADelegationCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Completion status.
    pub status: String,
    /// Result message if completed.
    pub result: Option<String>,
    /// Error/response message.
    pub error: Option<String>,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// Whether this is part of a multiturn conversation.
    pub is_multiturn: bool,
    /// A2A agent endpoint URL.
    pub endpoint: Option<String>,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Full A2A agent card metadata.
    pub agent_card: Option<HashMap<String, Value>>,
    /// Agent provider/organization info.
    pub provider: Option<HashMap<String, Value>>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
    /// A2A extension URIs in use.
    pub extensions: Option<Vec<String>>,
}

impl A2ADelegationCompletedEvent {
    pub fn new(status: String) -> Self {
        Self {
            base: BaseEventData::new("a2a_delegation_completed"),
            status,
            result: None,
            error: None,
            context_id: None,
            is_multiturn: false,
            endpoint: None,
            a2a_agent_name: None,
            agent_card: None,
            provider: None,
            metadata: None,
            extensions: None,
        }
    }
}

impl_base_event!(A2ADelegationCompletedEvent);

// ---------------------------------------------------------------------------
// A2AConversationStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a multiturn A2A conversation starts.
///
/// Corresponds to `A2AConversationStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AConversationStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A agent endpoint URL.
    pub endpoint: String,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Full A2A agent card metadata.
    pub agent_card: Option<HashMap<String, Value>>,
    /// A2A protocol version.
    pub protocol_version: Option<String>,
    /// Agent provider/organization info.
    pub provider: Option<HashMap<String, Value>>,
    /// ID of the specific skill being invoked.
    pub skill_id: Option<String>,
    /// Related task IDs for context.
    pub reference_task_ids: Option<Vec<String>>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
    /// A2A extension URIs in use.
    pub extensions: Option<Vec<String>>,
}

impl A2AConversationStartedEvent {
    pub fn new(agent_id: String, endpoint: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("a2a_conversation_started"),
            endpoint,
            context_id: None,
            a2a_agent_name: None,
            agent_card: None,
            protocol_version: None,
            provider: None,
            skill_id: None,
            reference_task_ids: None,
            metadata: None,
            extensions: None,
        };
        evt.base.agent_id = Some(agent_id);
        evt
    }
}

impl_base_event!(A2AConversationStartedEvent);

// ---------------------------------------------------------------------------
// A2AMessageSentEvent
// ---------------------------------------------------------------------------

/// Event emitted when a message is sent to the A2A agent.
///
/// Corresponds to `A2AMessageSentEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessageSentEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Message content sent.
    pub message: String,
    /// Current turn number (1-indexed).
    pub turn_number: i64,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// Unique A2A message identifier.
    pub message_id: Option<String>,
    /// Whether this is part of a multiturn conversation.
    pub is_multiturn: bool,
    /// A2A agent endpoint URL.
    pub endpoint: Option<String>,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// ID of the specific skill being invoked.
    pub skill_id: Option<String>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
    /// A2A extension URIs in use.
    pub extensions: Option<Vec<String>>,
}

impl A2AMessageSentEvent {
    pub fn new(message: String, turn_number: i64) -> Self {
        Self {
            base: BaseEventData::new("a2a_message_sent"),
            message,
            turn_number,
            context_id: None,
            message_id: None,
            is_multiturn: false,
            endpoint: None,
            a2a_agent_name: None,
            skill_id: None,
            metadata: None,
            extensions: None,
        }
    }
}

impl_base_event!(A2AMessageSentEvent);

// ---------------------------------------------------------------------------
// A2AResponseReceivedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a response is received from the A2A agent.
///
/// Corresponds to `A2AResponseReceivedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AResponseReceivedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Response content.
    pub response: String,
    /// Current turn number (1-indexed).
    pub turn_number: i64,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// Unique A2A message identifier.
    pub message_id: Option<String>,
    /// Whether this is part of a multiturn conversation.
    pub is_multiturn: bool,
    /// Response status.
    pub status: String,
    /// Whether this is the final response.
    pub final_response: bool,
    /// A2A agent endpoint URL.
    pub endpoint: Option<String>,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
    /// A2A extension URIs in use.
    pub extensions: Option<Vec<String>>,
}

impl A2AResponseReceivedEvent {
    pub fn new(response: String, turn_number: i64, status: String) -> Self {
        Self {
            base: BaseEventData::new("a2a_response_received"),
            response,
            turn_number,
            context_id: None,
            message_id: None,
            is_multiturn: false,
            status,
            final_response: false,
            endpoint: None,
            a2a_agent_name: None,
            metadata: None,
            extensions: None,
        }
    }
}

impl_base_event!(A2AResponseReceivedEvent);

// ---------------------------------------------------------------------------
// A2AConversationCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a multiturn A2A conversation completes.
///
/// Corresponds to `A2AConversationCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AConversationCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Final status: "completed" or "failed".
    pub status: String,
    /// Final result if completed successfully.
    pub final_result: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// Total number of turns in the conversation.
    pub total_turns: i64,
    /// A2A agent endpoint URL.
    pub endpoint: Option<String>,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Full A2A agent card metadata.
    pub agent_card: Option<HashMap<String, Value>>,
    /// Related task IDs for context.
    pub reference_task_ids: Option<Vec<String>>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
    /// A2A extension URIs in use.
    pub extensions: Option<Vec<String>>,
}

impl A2AConversationCompletedEvent {
    pub fn new(status: String, total_turns: i64) -> Self {
        Self {
            base: BaseEventData::new("a2a_conversation_completed"),
            status,
            final_result: None,
            error: None,
            context_id: None,
            total_turns,
            endpoint: None,
            a2a_agent_name: None,
            agent_card: None,
            reference_task_ids: None,
            metadata: None,
            extensions: None,
        }
    }
}

impl_base_event!(A2AConversationCompletedEvent);

// ---------------------------------------------------------------------------
// A2APollingStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when polling mode begins for A2A delegation.
///
/// Corresponds to `A2APollingStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2APollingStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// Seconds between poll attempts.
    pub polling_interval: f64,
    /// A2A agent endpoint URL.
    pub endpoint: String,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2APollingStartedEvent {
    pub fn new(task_id: String, polling_interval: f64, endpoint: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("a2a_polling_started"),
            context_id: None,
            polling_interval,
            endpoint,
            a2a_agent_name: None,
            metadata: None,
        };
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(A2APollingStartedEvent);

// ---------------------------------------------------------------------------
// A2APollingStatusEvent
// ---------------------------------------------------------------------------

/// Event emitted on each polling iteration.
///
/// Corresponds to `A2APollingStatusEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2APollingStatusEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// Current task state from remote agent.
    pub state: String,
    /// Time since polling started.
    pub elapsed_seconds: f64,
    /// Number of polls completed.
    pub poll_count: i64,
    /// A2A agent endpoint URL.
    pub endpoint: Option<String>,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2APollingStatusEvent {
    pub fn new(task_id: String, state: String, elapsed_seconds: f64, poll_count: i64) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("a2a_polling_status"),
            context_id: None,
            state,
            elapsed_seconds,
            poll_count,
            endpoint: None,
            a2a_agent_name: None,
            metadata: None,
        };
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(A2APollingStatusEvent);

// ---------------------------------------------------------------------------
// A2APushNotificationRegisteredEvent
// ---------------------------------------------------------------------------

/// Event emitted when push notification callback is registered.
///
/// Corresponds to `A2APushNotificationRegisteredEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2APushNotificationRegisteredEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// URL where agent will send push notifications.
    pub callback_url: String,
    /// A2A agent endpoint URL.
    pub endpoint: Option<String>,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2APushNotificationRegisteredEvent {
    pub fn new(task_id: String, callback_url: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("a2a_push_notification_registered"),
            context_id: None,
            callback_url,
            endpoint: None,
            a2a_agent_name: None,
            metadata: None,
        };
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(A2APushNotificationRegisteredEvent);

// ---------------------------------------------------------------------------
// A2APushNotificationReceivedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a push notification is received.
///
/// Corresponds to `A2APushNotificationReceivedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2APushNotificationReceivedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// Current task state from the notification.
    pub state: String,
    /// A2A agent endpoint URL.
    pub endpoint: Option<String>,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2APushNotificationReceivedEvent {
    pub fn new(task_id: String, state: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("a2a_push_notification_received"),
            context_id: None,
            state,
            endpoint: None,
            a2a_agent_name: None,
            metadata: None,
        };
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(A2APushNotificationReceivedEvent);

// ---------------------------------------------------------------------------
// A2APushNotificationSentEvent
// ---------------------------------------------------------------------------

/// Event emitted when a push notification is sent to a callback URL.
///
/// Corresponds to `A2APushNotificationSentEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2APushNotificationSentEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// URL the notification was sent to.
    pub callback_url: String,
    /// Task state being reported.
    pub state: String,
    /// Whether the notification was successfully delivered.
    pub success: bool,
    /// Error message if delivery failed.
    pub error: Option<String>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2APushNotificationSentEvent {
    pub fn new(task_id: String, callback_url: String, state: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("a2a_push_notification_sent"),
            context_id: None,
            callback_url,
            state,
            success: true,
            error: None,
            metadata: None,
        };
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(A2APushNotificationSentEvent);

// ---------------------------------------------------------------------------
// A2APushNotificationTimeoutEvent
// ---------------------------------------------------------------------------

/// Event emitted when push notification wait times out.
///
/// Corresponds to `A2APushNotificationTimeoutEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2APushNotificationTimeoutEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// Timeout duration in seconds.
    pub timeout_seconds: f64,
    /// A2A agent endpoint URL.
    pub endpoint: Option<String>,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2APushNotificationTimeoutEvent {
    pub fn new(task_id: String, timeout_seconds: f64) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("a2a_push_notification_timeout"),
            context_id: None,
            timeout_seconds,
            endpoint: None,
            a2a_agent_name: None,
            metadata: None,
        };
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(A2APushNotificationTimeoutEvent);

// ---------------------------------------------------------------------------
// A2AStreamingStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when streaming mode begins for A2A delegation.
///
/// Corresponds to `A2AStreamingStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AStreamingStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// A2A agent endpoint URL.
    pub endpoint: String,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Current turn number (1-indexed).
    pub turn_number: i64,
    /// Whether this is part of a multiturn conversation.
    pub is_multiturn: bool,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
    /// A2A extension URIs in use.
    pub extensions: Option<Vec<String>>,
}

impl A2AStreamingStartedEvent {
    pub fn new(endpoint: String) -> Self {
        Self {
            base: BaseEventData::new("a2a_streaming_started"),
            context_id: None,
            endpoint,
            a2a_agent_name: None,
            turn_number: 1,
            is_multiturn: false,
            metadata: None,
            extensions: None,
        }
    }
}

impl_base_event!(A2AStreamingStartedEvent);

// ---------------------------------------------------------------------------
// A2AStreamingChunkEvent
// ---------------------------------------------------------------------------

/// Event emitted when a streaming chunk is received.
///
/// Corresponds to `A2AStreamingChunkEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AStreamingChunkEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// The text content of the chunk.
    pub chunk: String,
    /// Index of this chunk in the stream (0-indexed).
    pub chunk_index: i64,
    /// Whether this is the final chunk.
    pub final_chunk: bool,
    /// A2A agent endpoint URL.
    pub endpoint: Option<String>,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Current turn number (1-indexed).
    pub turn_number: i64,
    /// Whether this is part of a multiturn conversation.
    pub is_multiturn: bool,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
    /// A2A extension URIs in use.
    pub extensions: Option<Vec<String>>,
}

impl A2AStreamingChunkEvent {
    pub fn new(chunk: String, chunk_index: i64) -> Self {
        Self {
            base: BaseEventData::new("a2a_streaming_chunk"),
            context_id: None,
            chunk,
            chunk_index,
            final_chunk: false,
            endpoint: None,
            a2a_agent_name: None,
            turn_number: 1,
            is_multiturn: false,
            metadata: None,
            extensions: None,
        }
    }
}

impl_base_event!(A2AStreamingChunkEvent);

// ---------------------------------------------------------------------------
// A2AAgentCardFetchedEvent
// ---------------------------------------------------------------------------

/// Event emitted when an agent card is successfully fetched.
///
/// Corresponds to `A2AAgentCardFetchedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AAgentCardFetchedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A agent endpoint URL.
    pub endpoint: String,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Full A2A agent card metadata.
    pub agent_card: Option<HashMap<String, Value>>,
    /// A2A protocol version.
    pub protocol_version: Option<String>,
    /// Agent provider/organization info.
    pub provider: Option<HashMap<String, Value>>,
    /// Whether the card was from cache.
    pub cached: bool,
    /// Time taken to fetch in milliseconds.
    pub fetch_time_ms: Option<f64>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2AAgentCardFetchedEvent {
    pub fn new(endpoint: String) -> Self {
        Self {
            base: BaseEventData::new("a2a_agent_card_fetched"),
            endpoint,
            a2a_agent_name: None,
            agent_card: None,
            protocol_version: None,
            provider: None,
            cached: false,
            fetch_time_ms: None,
            metadata: None,
        }
    }
}

impl_base_event!(A2AAgentCardFetchedEvent);

// ---------------------------------------------------------------------------
// A2AAuthenticationFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when authentication to an A2A agent fails.
///
/// Corresponds to `A2AAuthenticationFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AAuthenticationFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A agent endpoint URL.
    pub endpoint: String,
    /// Type of authentication attempted.
    pub auth_type: Option<String>,
    /// Error message.
    pub error: String,
    /// HTTP status code if applicable.
    pub status_code: Option<i64>,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// A2A protocol version.
    pub protocol_version: Option<String>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2AAuthenticationFailedEvent {
    pub fn new(endpoint: String, error: String) -> Self {
        Self {
            base: BaseEventData::new("a2a_authentication_failed"),
            endpoint,
            auth_type: None,
            error,
            status_code: None,
            a2a_agent_name: None,
            protocol_version: None,
            metadata: None,
        }
    }
}

impl_base_event!(A2AAuthenticationFailedEvent);

// ---------------------------------------------------------------------------
// A2AArtifactReceivedEvent
// ---------------------------------------------------------------------------

/// Event emitted when an artifact is received from a remote A2A agent.
///
/// Corresponds to `A2AArtifactReceivedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AArtifactReceivedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Unique identifier for the artifact.
    pub artifact_id: String,
    /// Name of the artifact.
    pub artifact_name: Option<String>,
    /// Purpose description.
    pub artifact_description: Option<String>,
    /// MIME type of the artifact content.
    pub mime_type: Option<String>,
    /// Size of the artifact in bytes.
    pub size_bytes: Option<i64>,
    /// Whether content should be appended to existing artifact.
    pub append: bool,
    /// Whether this is the final chunk.
    pub last_chunk: bool,
    /// A2A agent endpoint URL.
    pub endpoint: Option<String>,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Context ID for correlation.
    pub context_id: Option<String>,
    /// Current turn number (1-indexed).
    pub turn_number: i64,
    /// Whether this is part of a multiturn conversation.
    pub is_multiturn: bool,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
    /// A2A extension URIs in use.
    pub extensions: Option<Vec<String>>,
}

impl A2AArtifactReceivedEvent {
    pub fn new(task_id: String, artifact_id: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("a2a_artifact_received"),
            artifact_id,
            artifact_name: None,
            artifact_description: None,
            mime_type: None,
            size_bytes: None,
            append: false,
            last_chunk: false,
            endpoint: None,
            a2a_agent_name: None,
            context_id: None,
            turn_number: 1,
            is_multiturn: false,
            metadata: None,
            extensions: None,
        };
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(A2AArtifactReceivedEvent);

// ---------------------------------------------------------------------------
// A2AConnectionErrorEvent
// ---------------------------------------------------------------------------

/// Event emitted when a connection error occurs during A2A communication.
///
/// Corresponds to `A2AConnectionErrorEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AConnectionErrorEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A agent endpoint URL.
    pub endpoint: String,
    /// Error message.
    pub error: String,
    /// Error type: "timeout", "connection_refused", "dns_error", etc.
    pub error_type: Option<String>,
    /// HTTP status code if applicable.
    pub status_code: Option<i64>,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// The operation being attempted.
    pub operation: Option<String>,
    /// A2A context ID.
    pub context_id: Option<String>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2AConnectionErrorEvent {
    pub fn new(endpoint: String, error: String) -> Self {
        Self {
            base: BaseEventData::new("a2a_connection_error"),
            endpoint,
            error,
            error_type: None,
            status_code: None,
            a2a_agent_name: None,
            operation: None,
            context_id: None,
            metadata: None,
        }
    }
}

impl_base_event!(A2AConnectionErrorEvent);

// ---------------------------------------------------------------------------
// A2AServerTaskStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when an A2A server task execution starts.
///
/// Corresponds to `A2AServerTaskStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AServerTaskStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A context ID.
    pub context_id: String,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2AServerTaskStartedEvent {
    pub fn new(task_id: String, context_id: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("a2a_server_task_started"),
            context_id,
            metadata: None,
        };
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(A2AServerTaskStartedEvent);

// ---------------------------------------------------------------------------
// A2AServerTaskCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when an A2A server task execution completes.
///
/// Corresponds to `A2AServerTaskCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AServerTaskCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A context ID.
    pub context_id: String,
    /// Task result.
    pub result: String,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2AServerTaskCompletedEvent {
    pub fn new(task_id: String, context_id: String, result: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("a2a_server_task_completed"),
            context_id,
            result,
            metadata: None,
        };
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(A2AServerTaskCompletedEvent);

// ---------------------------------------------------------------------------
// A2AServerTaskCanceledEvent
// ---------------------------------------------------------------------------

/// Event emitted when an A2A server task execution is canceled.
///
/// Corresponds to `A2AServerTaskCanceledEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AServerTaskCanceledEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A context ID.
    pub context_id: String,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2AServerTaskCanceledEvent {
    pub fn new(task_id: String, context_id: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("a2a_server_task_canceled"),
            context_id,
            metadata: None,
        };
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(A2AServerTaskCanceledEvent);

// ---------------------------------------------------------------------------
// A2AServerTaskFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when an A2A server task execution fails.
///
/// Corresponds to `A2AServerTaskFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AServerTaskFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A context ID.
    pub context_id: String,
    /// Error message.
    pub error: String,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2AServerTaskFailedEvent {
    pub fn new(task_id: String, context_id: String, error: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("a2a_server_task_failed"),
            context_id,
            error,
            metadata: None,
        };
        evt.base.task_id = Some(task_id);
        evt
    }
}

impl_base_event!(A2AServerTaskFailedEvent);

// ---------------------------------------------------------------------------
// A2AParallelDelegationStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when parallel delegation to multiple A2A agents begins.
///
/// Corresponds to `A2AParallelDelegationStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AParallelDelegationStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// List of A2A agent endpoints being delegated to.
    pub endpoints: Vec<String>,
    /// Description of the task being delegated.
    pub task_description: String,
}

impl A2AParallelDelegationStartedEvent {
    pub fn new(endpoints: Vec<String>, task_description: String) -> Self {
        Self {
            base: BaseEventData::new("a2a_parallel_delegation_started"),
            endpoints,
            task_description,
        }
    }
}

impl_base_event!(A2AParallelDelegationStartedEvent);

// ---------------------------------------------------------------------------
// A2AParallelDelegationCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when parallel delegation to multiple A2A agents completes.
///
/// Corresponds to `A2AParallelDelegationCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AParallelDelegationCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// List of A2A agent endpoints.
    pub endpoints: Vec<String>,
    /// Number of successful delegations.
    pub success_count: i64,
    /// Number of failed delegations.
    pub failure_count: i64,
    /// Summary of results from each agent.
    pub results: Option<HashMap<String, String>>,
}

impl A2AParallelDelegationCompletedEvent {
    pub fn new(endpoints: Vec<String>, success_count: i64, failure_count: i64) -> Self {
        Self {
            base: BaseEventData::new("a2a_parallel_delegation_completed"),
            endpoints,
            success_count,
            failure_count,
            results: None,
        }
    }
}

impl_base_event!(A2AParallelDelegationCompletedEvent);

// ---------------------------------------------------------------------------
// A2ATransportNegotiatedEvent
// ---------------------------------------------------------------------------

/// Event emitted when transport protocol is negotiated with an A2A agent.
///
/// Corresponds to `A2ATransportNegotiatedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATransportNegotiatedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Original A2A agent endpoint URL.
    pub endpoint: String,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// The transport protocol selected.
    pub negotiated_transport: String,
    /// The URL to use for the selected transport.
    pub negotiated_url: String,
    /// How the transport was selected.
    pub source: String,
    /// Transports the client can use.
    pub client_supported_transports: Vec<String>,
    /// Transports the server supports.
    pub server_supported_transports: Vec<String>,
    /// Server's preferred transport.
    pub server_preferred_transport: String,
    /// Client's preferred transport if set.
    pub client_preferred_transport: Option<String>,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2ATransportNegotiatedEvent {
    pub fn new(
        endpoint: String,
        negotiated_transport: String,
        negotiated_url: String,
        source: String,
        client_supported_transports: Vec<String>,
        server_supported_transports: Vec<String>,
        server_preferred_transport: String,
    ) -> Self {
        Self {
            base: BaseEventData::new("a2a_transport_negotiated"),
            endpoint,
            a2a_agent_name: None,
            negotiated_transport,
            negotiated_url,
            source,
            client_supported_transports,
            server_supported_transports,
            server_preferred_transport,
            client_preferred_transport: None,
            metadata: None,
        }
    }
}

impl_base_event!(A2ATransportNegotiatedEvent);

// ---------------------------------------------------------------------------
// A2AContentTypeNegotiatedEvent
// ---------------------------------------------------------------------------

/// Event emitted when content types are negotiated with an A2A agent.
///
/// Corresponds to `A2AContentTypeNegotiatedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AContentTypeNegotiatedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// A2A agent endpoint URL.
    pub endpoint: String,
    /// Name of the A2A agent.
    pub a2a_agent_name: Option<String>,
    /// Skill name if negotiation was skill-specific.
    pub skill_name: Option<String>,
    /// MIME types the client can send.
    pub client_input_modes: Vec<String>,
    /// MIME types the client can accept.
    pub client_output_modes: Vec<String>,
    /// MIME types the server accepts.
    pub server_input_modes: Vec<String>,
    /// MIME types the server produces.
    pub server_output_modes: Vec<String>,
    /// Compatible input MIME types selected.
    pub negotiated_input_modes: Vec<String>,
    /// Compatible output MIME types selected.
    pub negotiated_output_modes: Vec<String>,
    /// Whether compatible types were found.
    pub negotiation_success: bool,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2AContentTypeNegotiatedEvent {
    pub fn new(
        endpoint: String,
        client_input_modes: Vec<String>,
        client_output_modes: Vec<String>,
        server_input_modes: Vec<String>,
        server_output_modes: Vec<String>,
        negotiated_input_modes: Vec<String>,
        negotiated_output_modes: Vec<String>,
    ) -> Self {
        Self {
            base: BaseEventData::new("a2a_content_type_negotiated"),
            endpoint,
            a2a_agent_name: None,
            skill_name: None,
            client_input_modes,
            client_output_modes,
            server_input_modes,
            server_output_modes,
            negotiated_input_modes,
            negotiated_output_modes,
            negotiation_success: true,
            metadata: None,
        }
    }
}

impl_base_event!(A2AContentTypeNegotiatedEvent);

// ---------------------------------------------------------------------------
// Context Lifecycle Events
// ---------------------------------------------------------------------------

/// Event emitted when an A2A context is created.
///
/// Corresponds to `A2AContextCreatedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AContextCreatedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Unique identifier for the context.
    pub context_id: String,
    /// Unix timestamp when context was created.
    pub created_at: f64,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2AContextCreatedEvent {
    pub fn new(context_id: String, created_at: f64) -> Self {
        Self {
            base: BaseEventData::new("a2a_context_created"),
            context_id,
            created_at,
            metadata: None,
        }
    }
}

impl_base_event!(A2AContextCreatedEvent);

/// Event emitted when an A2A context expires due to TTL.
///
/// Corresponds to `A2AContextExpiredEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AContextExpiredEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The expired context identifier.
    pub context_id: String,
    /// Unix timestamp when context was created.
    pub created_at: f64,
    /// How long the context existed before expiring.
    pub age_seconds: f64,
    /// Number of tasks in the context when expired.
    pub task_count: i64,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2AContextExpiredEvent {
    pub fn new(context_id: String, created_at: f64, age_seconds: f64, task_count: i64) -> Self {
        Self {
            base: BaseEventData::new("a2a_context_expired"),
            context_id,
            created_at,
            age_seconds,
            task_count,
            metadata: None,
        }
    }
}

impl_base_event!(A2AContextExpiredEvent);

/// Event emitted when an A2A context becomes idle.
///
/// Corresponds to `A2AContextIdleEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AContextIdleEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The idle context identifier.
    pub context_id: String,
    /// Seconds since last activity.
    pub idle_seconds: f64,
    /// Number of tasks in the context.
    pub task_count: i64,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2AContextIdleEvent {
    pub fn new(context_id: String, idle_seconds: f64, task_count: i64) -> Self {
        Self {
            base: BaseEventData::new("a2a_context_idle"),
            context_id,
            idle_seconds,
            task_count,
            metadata: None,
        }
    }
}

impl_base_event!(A2AContextIdleEvent);

/// Event emitted when all tasks in an A2A context complete.
///
/// Corresponds to `A2AContextCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AContextCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The completed context identifier.
    pub context_id: String,
    /// Total number of tasks.
    pub total_tasks: i64,
    /// Total context lifetime in seconds.
    pub duration_seconds: f64,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2AContextCompletedEvent {
    pub fn new(context_id: String, total_tasks: i64, duration_seconds: f64) -> Self {
        Self {
            base: BaseEventData::new("a2a_context_completed"),
            context_id,
            total_tasks,
            duration_seconds,
            metadata: None,
        }
    }
}

impl_base_event!(A2AContextCompletedEvent);

/// Event emitted when an A2A context is pruned (deleted).
///
/// Corresponds to `A2AContextPrunedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AContextPrunedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The pruned context identifier.
    pub context_id: String,
    /// Number of tasks that were in the context.
    pub task_count: i64,
    /// How long the context existed before pruning.
    pub age_seconds: f64,
    /// Custom A2A metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl A2AContextPrunedEvent {
    pub fn new(context_id: String, task_count: i64, age_seconds: f64) -> Self {
        Self {
            base: BaseEventData::new("a2a_context_pruned"),
            context_id,
            task_count,
            age_seconds,
            metadata: None,
        }
    }
}

impl_base_event!(A2AContextPrunedEvent);
