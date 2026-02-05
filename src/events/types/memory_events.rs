//! Memory-related event types.
//!
//! Corresponds to `crewai/events/types/memory_events.py`.
//!
//! Contains events for memory query, save, and retrieval operations.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// MemoryQueryStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a memory query is started.
///
/// Corresponds to `MemoryQueryStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The query string.
    pub query: String,
    /// Maximum number of results to return.
    pub limit: i64,
    /// Minimum similarity score threshold.
    pub score_threshold: Option<f64>,
}

impl MemoryQueryStartedEvent {
    pub fn new(query: String, limit: i64, score_threshold: Option<f64>) -> Self {
        Self {
            base: BaseEventData::new("memory_query_started"),
            query,
            limit,
            score_threshold,
        }
    }
}

impl_base_event!(MemoryQueryStartedEvent);

// ---------------------------------------------------------------------------
// MemoryQueryCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a memory query is completed successfully.
///
/// Corresponds to `MemoryQueryCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The query string.
    pub query: String,
    /// Query results (arbitrary JSON).
    pub results: Value,
    /// Maximum number of results requested.
    pub limit: i64,
    /// Minimum similarity score threshold.
    pub score_threshold: Option<f64>,
    /// Query execution time in milliseconds.
    pub query_time_ms: f64,
}

impl MemoryQueryCompletedEvent {
    pub fn new(
        query: String,
        results: Value,
        limit: i64,
        score_threshold: Option<f64>,
        query_time_ms: f64,
    ) -> Self {
        Self {
            base: BaseEventData::new("memory_query_completed"),
            query,
            results,
            limit,
            score_threshold,
            query_time_ms,
        }
    }
}

impl_base_event!(MemoryQueryCompletedEvent);

// ---------------------------------------------------------------------------
// MemoryQueryFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a memory query fails.
///
/// Corresponds to `MemoryQueryFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The query string.
    pub query: String,
    /// Maximum number of results requested.
    pub limit: i64,
    /// Minimum similarity score threshold.
    pub score_threshold: Option<f64>,
    /// Error message.
    pub error: String,
}

impl MemoryQueryFailedEvent {
    pub fn new(
        query: String,
        limit: i64,
        score_threshold: Option<f64>,
        error: String,
    ) -> Self {
        Self {
            base: BaseEventData::new("memory_query_failed"),
            query,
            limit,
            score_threshold,
            error,
        }
    }
}

impl_base_event!(MemoryQueryFailedEvent);

// ---------------------------------------------------------------------------
// MemorySaveStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a memory save operation is started.
///
/// Corresponds to `MemorySaveStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySaveStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The value being saved.
    pub value: Option<String>,
    /// Metadata associated with the value.
    pub metadata: Option<HashMap<String, Value>>,
}

impl MemorySaveStartedEvent {
    pub fn new(value: Option<String>, metadata: Option<HashMap<String, Value>>) -> Self {
        Self {
            base: BaseEventData::new("memory_save_started"),
            value,
            metadata,
        }
    }
}

impl_base_event!(MemorySaveStartedEvent);

// ---------------------------------------------------------------------------
// MemorySaveCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a memory save operation is completed successfully.
///
/// Corresponds to `MemorySaveCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySaveCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The value that was saved.
    pub value: String,
    /// Metadata associated with the value.
    pub metadata: Option<HashMap<String, Value>>,
    /// Save operation time in milliseconds.
    pub save_time_ms: f64,
}

impl MemorySaveCompletedEvent {
    pub fn new(
        value: String,
        metadata: Option<HashMap<String, Value>>,
        save_time_ms: f64,
    ) -> Self {
        Self {
            base: BaseEventData::new("memory_save_completed"),
            value,
            metadata,
            save_time_ms,
        }
    }
}

impl_base_event!(MemorySaveCompletedEvent);

// ---------------------------------------------------------------------------
// MemorySaveFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when a memory save operation fails.
///
/// Corresponds to `MemorySaveFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySaveFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The value that was being saved.
    pub value: Option<String>,
    /// Metadata associated with the value.
    pub metadata: Option<HashMap<String, Value>>,
    /// Error message.
    pub error: String,
}

impl MemorySaveFailedEvent {
    pub fn new(
        value: Option<String>,
        metadata: Option<HashMap<String, Value>>,
        error: String,
    ) -> Self {
        Self {
            base: BaseEventData::new("memory_save_failed"),
            value,
            metadata,
            error,
        }
    }
}

impl_base_event!(MemorySaveFailedEvent);

// ---------------------------------------------------------------------------
// MemoryRetrievalStartedEvent
// ---------------------------------------------------------------------------

/// Event emitted when memory retrieval for a task prompt starts.
///
/// Corresponds to `MemoryRetrievalStartedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRetrievalStartedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
}

impl MemoryRetrievalStartedEvent {
    pub fn new(task_id: Option<String>) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("memory_retrieval_started"),
        };
        evt.base.task_id = task_id;
        evt
    }
}

impl_base_event!(MemoryRetrievalStartedEvent);

// ---------------------------------------------------------------------------
// MemoryRetrievalCompletedEvent
// ---------------------------------------------------------------------------

/// Event emitted when memory retrieval for a task prompt completes successfully.
///
/// Corresponds to `MemoryRetrievalCompletedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRetrievalCompletedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The memory content that was retrieved.
    pub memory_content: String,
    /// Retrieval time in milliseconds.
    pub retrieval_time_ms: f64,
}

impl MemoryRetrievalCompletedEvent {
    pub fn new(task_id: Option<String>, memory_content: String, retrieval_time_ms: f64) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("memory_retrieval_completed"),
            memory_content,
            retrieval_time_ms,
        };
        evt.base.task_id = task_id;
        evt
    }
}

impl_base_event!(MemoryRetrievalCompletedEvent);

// ---------------------------------------------------------------------------
// MemoryRetrievalFailedEvent
// ---------------------------------------------------------------------------

/// Event emitted when memory retrieval for a task prompt fails.
///
/// Corresponds to `MemoryRetrievalFailedEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRetrievalFailedEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// Error message.
    pub error: String,
}

impl MemoryRetrievalFailedEvent {
    pub fn new(task_id: Option<String>, error: String) -> Self {
        let mut evt = Self {
            base: BaseEventData::new("memory_retrieval_failed"),
            error,
        };
        evt.base.task_id = task_id;
        evt
    }
}

impl_base_event!(MemoryRetrievalFailedEvent);
