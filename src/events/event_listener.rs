//! Base event types and event listener trait for the CrewAI event system.
//!
//! Corresponds to `crewai/events/base_events.py` and `crewai/events/base_event_listener.py`.
//!
//! This module provides the core event infrastructure:
//! - [`BaseEvent`] trait: The interface all events implement.
//! - [`BaseEventData`] struct: Concrete, serialisable base event data.
//! - [`impl_base_event!`] macro: Delegates `BaseEvent` trait methods to an embedded `BaseEventData`.
//! - [`BaseEventListener`] trait: Abstract base for event listeners that register handlers.
//! - [`Listener`] struct: A concrete no-op listener for use as a default.
//! - [`CrewAIBaseEvent`] type alias: Convenience alias for boxed events.

// Re-export base event types and helpers.
pub use crate::events::base_event::{
    get_next_emission_sequence, reset_emission_counter, BaseEvent, BaseEventData,
};

// Re-export the BaseEventListener trait.
pub use crate::events::base_event_listener::BaseEventListener;

// Re-export the impl_base_event macro (it is exported at crate root).
pub use crate::impl_base_event;

use crate::events::crewai_event_bus::CrewAIEventsBus;

// ---------------------------------------------------------------------------
// CrewAIBaseEvent -- boxed type-erased event alias
// ---------------------------------------------------------------------------

/// Type alias for a boxed, type-erased event.
///
/// Useful when storing heterogeneous events in collections.
pub type CrewAIBaseEvent = Box<dyn BaseEvent>;

// ---------------------------------------------------------------------------
// Listener -- a concrete no-op listener for use as a default
// ---------------------------------------------------------------------------

/// A concrete no-op event listener.
///
/// Useful as a placeholder or default when no custom listener is needed.
/// Call [`init`](BaseEventListener::init) to register (no-op) handlers.
///
/// Corresponds to `crewai/events/event_listener.py::EventListener` (minimal skeleton).
pub struct Listener {
    /// Whether verbose output is enabled.
    pub verbose: bool,
}

impl Listener {
    /// Create a new no-op listener.
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl Default for Listener {
    fn default() -> Self {
        Self::new(false)
    }
}

impl BaseEventListener for Listener {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn setup_listeners(&self, _bus: &CrewAIEventsBus) {
        // No-op: concrete listener implementations should override this
        // to register their event handlers on the bus.
    }
}
