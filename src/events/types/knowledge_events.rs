//! Knowledge-related event types.
//!
//! Corresponds to `crewai/events/types/knowledge_events.py`.
//!
//! Contains events for knowledge retrieval and query operations.

use serde::{Deserialize, Serialize};

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// KnowledgeRetrievalStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a knowledge retrieval is started.
///
/// Corresponds to `KnowledgeRetrievalStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRetrievalStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
}

impl KnowledgeRetrievalStartedEvent {
    pub fn new() -> Self {
        Self {
            base: BaseEventData::new("knowledge_search_query_started"),
        }
    }
}

impl Default for KnowledgeRetrievalStartedEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl_base_event!(KnowledgeRetrievalStartedEvent);

// ---------------------------------------------------------------------------
// KnowledgeRetrievalCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a knowledge retrieval is completed.
///
/// Corresponds to `KnowledgeRetrievalCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRetrievalCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The query that was executed.
    pub query: String,
    /// The knowledge content that was retrieved.
    pub retrieved_knowledge: String,
}

impl KnowledgeRetrievalCompletedEvent {
    pub fn new(query: String, retrieved_knowledge: String) -> Self {
        Self {
            base: BaseEventData::new("knowledge_search_query_completed"),
            query,
            retrieved_knowledge,
        }
    }
}

impl_base_event!(KnowledgeRetrievalCompletedEvent);

// ---------------------------------------------------------------------------
// KnowledgeQueryStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a knowledge query is started.
///
/// Corresponds to `KnowledgeQueryStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeQueryStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The task prompt associated with the query.
    pub task_prompt: String,
}

impl KnowledgeQueryStartedEvent {
    pub fn new(task_prompt: String) -> Self {
        Self {
            base: BaseEventData::new("knowledge_query_started"),
            task_prompt,
        }
    }
}

impl_base_event!(KnowledgeQueryStartedEvent);

// ---------------------------------------------------------------------------
// KnowledgeQueryFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a knowledge query fails.
///
/// Corresponds to `KnowledgeQueryFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeQueryFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Error message.
    pub error: String,
}

impl KnowledgeQueryFailedEvent {
    pub fn new(error: String) -> Self {
        Self {
            base: BaseEventData::new("knowledge_query_failed"),
            error,
        }
    }
}

impl_base_event!(KnowledgeQueryFailedEvent);

// ---------------------------------------------------------------------------
// KnowledgeQueryCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a knowledge query is completed.
///
/// Corresponds to `KnowledgeQueryCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeQueryCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The query that was executed.
    pub query: String,
}

impl KnowledgeQueryCompletedEvent {
    pub fn new(query: String) -> Self {
        Self {
            base: BaseEventData::new("knowledge_query_completed"),
            query,
        }
    }
}

impl_base_event!(KnowledgeQueryCompletedEvent);

// ---------------------------------------------------------------------------
// KnowledgeSearchQueryFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a knowledge search query fails.
///
/// Corresponds to `KnowledgeSearchQueryFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSearchQueryFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The query that failed.
    pub query: String,
    /// Error message.
    pub error: String,
}

impl KnowledgeSearchQueryFailedEvent {
    pub fn new(query: String, error: String) -> Self {
        Self {
            base: BaseEventData::new("knowledge_search_query_failed"),
            query,
            error,
        }
    }
}

impl_base_event!(KnowledgeSearchQueryFailedEvent);
