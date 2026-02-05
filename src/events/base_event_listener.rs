//! Abstract base event listener for the CrewAI event system.
//!
//! Corresponds to `crewai/events/base_event_listener.py`.

use super::event_bus::CrewAIEventsBus;

/// Abstract base trait for event listeners.
///
/// Implementations must provide [`setup_listeners`](BaseEventListener::setup_listeners)
/// to register their handlers on the global event bus.
///
/// Corresponds to `crewai/events/base_event_listener.py::BaseEventListener`.
pub trait BaseEventListener: Send + Sync {
    /// Whether this listener produces verbose output.
    fn verbose(&self) -> bool {
        false
    }

    /// Register event handlers on the provided event bus.
    ///
    /// Called once during listener initialisation.
    fn setup_listeners(&self, bus: &CrewAIEventsBus);

    /// Initialise the listener: register handlers and validate dependencies.
    ///
    /// The default implementation calls [`setup_listeners`](Self::setup_listeners)
    /// with the global event bus and then validates all handler dependencies.
    fn init(&self) {
        let bus = CrewAIEventsBus::global();
        self.setup_listeners(bus);
        let _ = bus.validate_dependencies();
    }
}
