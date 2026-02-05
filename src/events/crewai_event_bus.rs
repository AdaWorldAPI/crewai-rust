//! Singleton event bus for managing and dispatching events in CrewAI.
//!
//! Corresponds to `crewai/events/crewai_event_bus.py` (named `event_bus.py` in Python).
//!
//! This module provides the global [`CrewAIEventsBus`] singleton, [`HandlerId`],
//! [`Depends`], handler types, and the [`ExecutionPlan`] alias. It is the
//! primary entry point for event registration and emission.

// Re-export everything from the canonical event_bus module so that
// `crate::events::crewai_event_bus::CrewAIEventsBus` etc. work seamlessly.
pub use crate::events::event_bus::{
    CrewAIEventsBus, Depends, ExecutionPlan, HandlerId, SyncHandler, CREWAI_EVENT_BUS,
};
