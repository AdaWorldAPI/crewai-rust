//! Base event types for the CrewAI event system.
//!
//! Corresponds to `crewai/events/base_events.py`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Emission counter (per-thread, matching Python's contextvars counter)
// ---------------------------------------------------------------------------

thread_local! {
    static EMISSION_COUNTER: AtomicU64 = const { AtomicU64::new(1) };
}

/// Get the next emission sequence number for the current thread.
pub fn get_next_emission_sequence() -> u64 {
    EMISSION_COUNTER.with(|c| c.fetch_add(1, Ordering::Relaxed))
}

/// Reset the emission sequence counter to 1 for the current thread.
pub fn reset_emission_counter() {
    EMISSION_COUNTER.with(|c| c.store(1, Ordering::Relaxed));
}

// ---------------------------------------------------------------------------
// BaseEvent trait
// ---------------------------------------------------------------------------

/// Trait implemented by all events in the CrewAI event system.
///
/// Every event carries an auto-generated `event_id` (UUID v4), a UTC
/// `timestamp`, an event `type` string, and optional parent/chain fields
/// for hierarchical and linear event tracking.
pub trait BaseEvent: Send + Sync + std::fmt::Debug {
    /// Unique identifier for this event instance.
    fn event_id(&self) -> &str;

    /// UTC timestamp when the event was created.
    fn timestamp(&self) -> DateTime<Utc>;

    /// Event type discriminator string (e.g. `"crew_kickoff_started"`).
    fn event_type(&self) -> &str;

    /// UUID string of the source entity fingerprint, if available.
    fn source_fingerprint(&self) -> Option<&str>;

    /// Source entity kind (`"agent"`, `"task"`, `"crew"`, etc.).
    fn source_type(&self) -> Option<&str>;

    /// Arbitrary fingerprint metadata.
    fn fingerprint_metadata(&self) -> Option<&HashMap<String, serde_json::Value>>;

    /// Task ID associated with this event, if any.
    fn task_id(&self) -> Option<&str>;

    /// Task name associated with this event, if any.
    fn task_name(&self) -> Option<&str>;

    /// Agent ID associated with this event, if any.
    fn agent_id(&self) -> Option<&str>;

    /// Agent role associated with this event, if any.
    fn agent_role(&self) -> Option<&str>;

    /// Parent event ID for hierarchical scope tracking.
    fn parent_event_id(&self) -> Option<&str>;

    /// Set the parent event ID.
    fn set_parent_event_id(&mut self, id: Option<String>);

    /// Previous event ID for linear chain tracking.
    fn previous_event_id(&self) -> Option<&str>;

    /// Set the previous event ID.
    fn set_previous_event_id(&mut self, id: Option<String>);

    /// ID of the event that causally triggered this event.
    fn triggered_by_event_id(&self) -> Option<&str>;

    /// Set the triggered-by event ID.
    fn set_triggered_by_event_id(&mut self, id: Option<String>);

    /// Monotonically increasing emission sequence number.
    fn emission_sequence(&self) -> Option<u64>;

    /// Set the emission sequence number.
    fn set_emission_sequence(&mut self, seq: Option<u64>);
}

// ---------------------------------------------------------------------------
// BaseEventData – concrete, serialisable implementation of BaseEvent
// ---------------------------------------------------------------------------

/// Concrete event data structure that implements [`BaseEvent`].
///
/// Most domain-specific events embed this struct and delegate the trait
/// methods to it via the [`impl_base_event!`] macro.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEventData {
    /// Unique event identifier (UUID v4).
    pub event_id: String,

    /// UTC timestamp of event creation.
    pub timestamp: DateTime<Utc>,

    /// Event type discriminator.
    #[serde(rename = "type")]
    pub event_type: String,

    /// UUID string of the source entity fingerprint.
    pub source_fingerprint: Option<String>,

    /// Source entity kind.
    pub source_type: Option<String>,

    /// Arbitrary fingerprint metadata.
    pub fingerprint_metadata: Option<HashMap<String, serde_json::Value>>,

    /// Associated task ID.
    pub task_id: Option<String>,

    /// Associated task name.
    pub task_name: Option<String>,

    /// Associated agent ID.
    pub agent_id: Option<String>,

    /// Associated agent role.
    pub agent_role: Option<String>,

    /// Parent event ID (hierarchical scope).
    pub parent_event_id: Option<String>,

    /// Previous event ID (linear chain).
    pub previous_event_id: Option<String>,

    /// ID of the causally triggering event.
    pub triggered_by_event_id: Option<String>,

    /// Emission sequence number.
    pub emission_sequence: Option<u64>,
}

impl BaseEventData {
    /// Create a new `BaseEventData` with the given event type.
    ///
    /// Generates a fresh UUID v4 and captures the current UTC time.
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: event_type.into(),
            source_fingerprint: None,
            source_type: None,
            fingerprint_metadata: None,
            task_id: None,
            task_name: None,
            agent_id: None,
            agent_role: None,
            parent_event_id: None,
            previous_event_id: None,
            triggered_by_event_id: None,
            emission_sequence: None,
        }
    }
}

impl BaseEvent for BaseEventData {
    fn event_id(&self) -> &str {
        &self.event_id
    }
    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
    fn event_type(&self) -> &str {
        &self.event_type
    }
    fn source_fingerprint(&self) -> Option<&str> {
        self.source_fingerprint.as_deref()
    }
    fn source_type(&self) -> Option<&str> {
        self.source_type.as_deref()
    }
    fn fingerprint_metadata(&self) -> Option<&HashMap<String, serde_json::Value>> {
        self.fingerprint_metadata.as_ref()
    }
    fn task_id(&self) -> Option<&str> {
        self.task_id.as_deref()
    }
    fn task_name(&self) -> Option<&str> {
        self.task_name.as_deref()
    }
    fn agent_id(&self) -> Option<&str> {
        self.agent_id.as_deref()
    }
    fn agent_role(&self) -> Option<&str> {
        self.agent_role.as_deref()
    }
    fn parent_event_id(&self) -> Option<&str> {
        self.parent_event_id.as_deref()
    }
    fn set_parent_event_id(&mut self, id: Option<String>) {
        self.parent_event_id = id;
    }
    fn previous_event_id(&self) -> Option<&str> {
        self.previous_event_id.as_deref()
    }
    fn set_previous_event_id(&mut self, id: Option<String>) {
        self.previous_event_id = id;
    }
    fn triggered_by_event_id(&self) -> Option<&str> {
        self.triggered_by_event_id.as_deref()
    }
    fn set_triggered_by_event_id(&mut self, id: Option<String>) {
        self.triggered_by_event_id = id;
    }
    fn emission_sequence(&self) -> Option<u64> {
        self.emission_sequence
    }
    fn set_emission_sequence(&mut self, seq: Option<u64>) {
        self.emission_sequence = seq;
    }
}

// ---------------------------------------------------------------------------
// Helper macro – delegate BaseEvent trait methods to an embedded BaseEventData
// ---------------------------------------------------------------------------

/// Implement [`BaseEvent`] for a struct that contains a `base: BaseEventData` field.
///
/// Usage:
/// ```ignore
/// impl_base_event!(MyEvent);
/// ```
#[macro_export]
macro_rules! impl_base_event {
    ($ty:ty) => {
        impl $crate::events::base_event::BaseEvent for $ty {
            fn event_id(&self) -> &str {
                &self.base.event_id
            }
            fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
                self.base.timestamp
            }
            fn event_type(&self) -> &str {
                &self.base.event_type
            }
            fn source_fingerprint(&self) -> Option<&str> {
                self.base.source_fingerprint.as_deref()
            }
            fn source_type(&self) -> Option<&str> {
                self.base.source_type.as_deref()
            }
            fn fingerprint_metadata(
                &self,
            ) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
                self.base.fingerprint_metadata.as_ref()
            }
            fn task_id(&self) -> Option<&str> {
                self.base.task_id.as_deref()
            }
            fn task_name(&self) -> Option<&str> {
                self.base.task_name.as_deref()
            }
            fn agent_id(&self) -> Option<&str> {
                self.base.agent_id.as_deref()
            }
            fn agent_role(&self) -> Option<&str> {
                self.base.agent_role.as_deref()
            }
            fn parent_event_id(&self) -> Option<&str> {
                self.base.parent_event_id.as_deref()
            }
            fn set_parent_event_id(&mut self, id: Option<String>) {
                self.base.parent_event_id = id;
            }
            fn previous_event_id(&self) -> Option<&str> {
                self.base.previous_event_id.as_deref()
            }
            fn set_previous_event_id(&mut self, id: Option<String>) {
                self.base.previous_event_id = id;
            }
            fn triggered_by_event_id(&self) -> Option<&str> {
                self.base.triggered_by_event_id.as_deref()
            }
            fn set_triggered_by_event_id(&mut self, id: Option<String>) {
                self.base.triggered_by_event_id = id;
            }
            fn emission_sequence(&self) -> Option<u64> {
                self.base.emission_sequence
            }
            fn set_emission_sequence(&mut self, seq: Option<u64>) {
                self.base.emission_sequence = seq;
            }
        }
    };
}
