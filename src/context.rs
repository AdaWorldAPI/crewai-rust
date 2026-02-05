//! Platform integration context management.
//!
//! Corresponds to `crewai/context.py`.

use std::sync::Mutex;
use std::env;

/// Thread-local storage for platform integration token.
static PLATFORM_INTEGRATION_TOKEN: Mutex<Option<String>> = Mutex::new(None);

/// Set the platform integration token in the current context.
pub fn set_platform_integration_token(integration_token: &str) {
    let mut token = PLATFORM_INTEGRATION_TOKEN.lock().unwrap();
    *token = Some(integration_token.to_string());
}

/// Get the platform integration token from the current context or environment.
///
/// Returns the integration token if set, otherwise checks the
/// `CREWAI_PLATFORM_INTEGRATION_TOKEN` environment variable.
pub fn get_platform_integration_token() -> Option<String> {
    let token = PLATFORM_INTEGRATION_TOKEN.lock().unwrap();
    if let Some(ref t) = *token {
        return Some(t.clone());
    }
    env::var("CREWAI_PLATFORM_INTEGRATION_TOKEN").ok()
}

/// RAII guard for temporarily setting the platform integration token.
///
/// When dropped, restores the previous token value.
pub struct PlatformContext {
    previous: Option<String>,
}

impl PlatformContext {
    /// Create a new platform context with the given integration token.
    pub fn new(integration_token: &str) -> Self {
        let mut token = PLATFORM_INTEGRATION_TOKEN.lock().unwrap();
        let previous = token.clone();
        *token = Some(integration_token.to_string());
        PlatformContext { previous }
    }
}

impl Drop for PlatformContext {
    fn drop(&mut self) {
        let mut token = PLATFORM_INTEGRATION_TOKEN.lock().unwrap();
        *token = self.previous.take();
    }
}
