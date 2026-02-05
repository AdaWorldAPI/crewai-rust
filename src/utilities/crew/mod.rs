//! Crew context utilities.
//!
//! Corresponds to `crewai/utilities/crew/`.

use std::sync::Arc;
use parking_lot::RwLock;

/// Thread-safe container for crew execution context.
///
/// Stores the current crew's identifier and shared state that
/// utility functions may need during execution.
#[derive(Debug, Clone, Default)]
pub struct CrewContext {
    /// Current crew identifier (if any).
    pub crew_id: Option<String>,
    /// Whether the crew is in training mode.
    pub is_training: bool,
    /// Shared crew-level metadata.
    pub metadata: std::collections::HashMap<String, String>,
}

/// Global crew context, accessible from utility code.
static CREW_CONTEXT: once_cell::sync::Lazy<Arc<RwLock<Option<CrewContext>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(None)));

/// Set the global crew context.
pub fn set_crew_context(ctx: CrewContext) {
    *CREW_CONTEXT.write() = Some(ctx);
}

/// Get a clone of the current crew context (if set).
pub fn get_crew_context() -> Option<CrewContext> {
    CREW_CONTEXT.read().clone()
}

/// Clear the global crew context.
pub fn clear_crew_context() {
    *CREW_CONTEXT.write() = None;
}
