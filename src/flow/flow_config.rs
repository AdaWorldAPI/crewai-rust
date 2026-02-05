//! Global Flow configuration.
//!
//! Corresponds to `crewai/flow/flow_config.py`.

use std::sync::{Arc, Mutex};

/// Global configuration for Flow execution.
///
/// # Attributes
///
/// * `hitl_provider` - The human-in-the-loop feedback provider name.
///   Defaults to None (uses console input).
///   Can be overridden by deployments at startup.
#[derive(Debug, Clone)]
pub struct FlowConfig {
    /// The configured HITL provider name.
    hitl_provider: Option<String>,
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            hitl_provider: None,
        }
    }
}

impl FlowConfig {
    /// Create a new FlowConfig.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the configured HITL provider name.
    pub fn hitl_provider(&self) -> Option<&str> {
        self.hitl_provider.as_deref()
    }

    /// Set the HITL provider name.
    pub fn set_hitl_provider(&mut self, provider: Option<String>) {
        self.hitl_provider = provider;
    }
}

lazy_static::lazy_static! {
    /// Singleton FlowConfig instance (thread-safe).
    pub static ref FLOW_CONFIG: Arc<Mutex<FlowConfig>> = Arc::new(Mutex::new(FlowConfig::new()));
}

/// Get a reference to the global flow config.
pub fn flow_config() -> Arc<Mutex<FlowConfig>> {
    FLOW_CONFIG.clone()
}
