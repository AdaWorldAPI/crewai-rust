//! Agent metaclass/metadata for extension support.
//!
//! Corresponds to `crewai/agent/internal/meta.py`.
//!
//! In the Python implementation, `AgentMeta` is a metaclass that extends
//! Pydantic's `ModelMetaclass` to detect extension fields (like `a2a`) in
//! class annotations and apply appropriate wrapper logic during
//! `post_init_setup`. In Rust, we model this as a metadata struct that
//! tracks extension state and provides hooks for post-initialization
//! extension processing.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Agent metadata for extension support.
///
/// Detects extension fields (like `a2a`) and applies the appropriate
/// wrapper logic to enable extension functionality. This is the Rust
/// equivalent of the Python `AgentMeta` metaclass.
///
/// In the Rust port, rather than a metaclass, this struct holds metadata
/// about which extensions are active and provides methods to apply them
/// during agent initialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMeta {
    /// Whether the agent type has been validated.
    pub validated: bool,
    /// Whether A2A extensions are active on this agent.
    pub has_a2a_extension: bool,
    /// Extension registry configuration (serialized).
    pub extension_config: Option<HashMap<String, Value>>,
}

impl AgentMeta {
    /// Create a new `AgentMeta`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark the agent type as validated.
    pub fn validate(&mut self) {
        self.validated = true;
    }

    /// Apply post-initialization extensions to an agent.
    ///
    /// This mirrors the Python `post_init_setup_with_extensions` wrapper
    /// that checks for `a2a` configuration and applies the A2A extension
    /// registry and wrapper.
    ///
    /// # Arguments
    ///
    /// * `a2a_config` - Optional A2A configuration value from the agent.
    ///
    /// # Returns
    ///
    /// Whether any extensions were applied.
    pub fn apply_extensions(&mut self, a2a_config: Option<&Value>) -> bool {
        if let Some(_config) = a2a_config {
            self.has_a2a_extension = true;
            // TODO: Create extension registry from config and wrap agent
            // with A2A instance, mirroring:
            //   extension_registry = create_extension_registry_from_config(a2a_value)
            //   wrap_agent_with_a2a_instance(self, extension_registry)
            true
        } else {
            false
        }
    }
}
