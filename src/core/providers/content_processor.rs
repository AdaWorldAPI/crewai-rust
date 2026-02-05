//! Content processor provider for extensible content processing.
//!
//! Corresponds to `crewai/core/providers/content_processor.py`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Protocol for content processing during task execution.
pub trait ContentProcessorProvider: Send + Sync {
    /// Process content before use.
    ///
    /// # Arguments
    /// * `content` - The content to process.
    /// * `context` - Optional context information.
    ///
    /// # Returns
    /// The processed content.
    fn process(
        &self,
        content: &str,
        context: Option<&HashMap<String, String>>,
    ) -> String;
}

// ---------------------------------------------------------------------------
// Default no-op implementation
// ---------------------------------------------------------------------------

/// Default processor that returns content unchanged.
pub struct NoOpContentProcessor;

impl ContentProcessorProvider for NoOpContentProcessor {
    fn process(
        &self,
        content: &str,
        _context: Option<&HashMap<String, String>>,
    ) -> String {
        content.to_string()
    }
}

// ---------------------------------------------------------------------------
// Context variable management
// ---------------------------------------------------------------------------

static PROCESSOR: Lazy<Arc<Mutex<Option<Box<dyn ContentProcessorProvider>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

static DEFAULT_PROCESSOR: Lazy<NoOpContentProcessor> =
    Lazy::new(|| NoOpContentProcessor);

/// Get the current content processor.
///
/// Returns the registered content processor or the default no-op processor.
pub fn get_processor() -> Arc<Mutex<Option<Box<dyn ContentProcessorProvider>>>> {
    Arc::clone(&PROCESSOR)
}

/// Set the content processor for the current context.
pub fn set_processor(processor: Box<dyn ContentProcessorProvider>) {
    let mut guard = PROCESSOR.lock().unwrap();
    *guard = Some(processor);
}

/// Process content using the registered processor (or default no-op).
pub fn process_content(
    content: &str,
    context: Option<&HashMap<String, String>>,
) -> String {
    let guard = PROCESSOR.lock().unwrap();
    match guard.as_ref() {
        Some(processor) => processor.process(content, context),
        None => DEFAULT_PROCESSOR.process(content, context),
    }
}
