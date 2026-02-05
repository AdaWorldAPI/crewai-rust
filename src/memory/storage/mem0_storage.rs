//! Mem0 storage provider for external memory integration.
//!
//! Port of crewai/memory/storage/mem0_storage.py
//!
//! This is a placeholder implementation for the optional Mem0 provider.
//! The actual Mem0 integration requires the mem0 service.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::memory::storage::interface::Storage;

/// Maximum agent ID length for Mem0.
const MAX_AGENT_ID_LENGTH_MEM0: usize = 255;

/// Supported memory types for Mem0.
const SUPPORTED_TYPES: &[&str] = &["short_term", "long_term", "entities", "external"];

/// Mem0Storage extends Storage to handle embedding and searching
/// across entities using the Mem0 service.
pub struct Mem0Storage {
    /// The type of memory (e.g., "short_term", "long_term", "entities", "external").
    pub memory_type: String,
    /// Optional reference to the crew.
    pub crew: Option<Box<dyn std::any::Any + Send + Sync>>,
    /// Configuration for the Mem0 provider.
    pub config: HashMap<String, Value>,
    /// Optional run ID for short-term memory scoping.
    pub mem0_run_id: Option<String>,
    /// Optional includes filter.
    pub includes: Option<Value>,
    /// Optional excludes filter.
    pub excludes: Option<Value>,
    /// Optional custom categories.
    pub custom_categories: Option<Value>,
    /// Whether to use inference.
    pub infer: bool,
}

impl Mem0Storage {
    /// Create a new Mem0Storage instance.
    ///
    /// # Arguments
    /// * `memory_type` - The type of memory. Must be one of: "short_term", "long_term", "entities", "external".
    /// * `crew` - Optional reference to the crew.
    /// * `config` - Configuration for the Mem0 provider.
    ///
    /// # Errors
    /// Returns an error if the memory_type is not supported.
    pub fn new(
        memory_type: &str,
        crew: Option<Box<dyn std::any::Any + Send + Sync>>,
        config: Option<HashMap<String, Value>>,
    ) -> Result<Self, anyhow::Error> {
        Self::validate_type(memory_type)?;

        let config = config.unwrap_or_default();
        let mem0_run_id = config
            .get("run_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let includes = config.get("includes").cloned();
        let excludes = config.get("excludes").cloned();
        let custom_categories = config.get("custom_categories").cloned();
        let infer = config
            .get("infer")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        Ok(Self {
            memory_type: memory_type.to_string(),
            crew,
            config,
            mem0_run_id,
            includes,
            excludes,
            custom_categories,
            infer,
        })
    }

    /// Validate the memory type.
    fn validate_type(memory_type: &str) -> Result<(), anyhow::Error> {
        if !SUPPORTED_TYPES.contains(&memory_type) {
            return Err(anyhow::anyhow!(
                "Invalid type '{}' for Mem0Storage. Must be one of: {}",
                memory_type,
                SUPPORTED_TYPES.join(", ")
            ));
        }
        Ok(())
    }

    /// Sanitize an agent role to ensure valid directory names.
    fn sanitize_role(role: &str) -> String {
        role.replace('\n', "")
            .replace(' ', "_")
            .replace('/', "_")
    }

    /// Extract the assistant message from text.
    fn get_assistant_message(text: &str) -> &str {
        let marker = "Final Answer:";
        if let Some(pos) = text.find(marker) {
            text[pos + marker.len()..].trim()
        } else {
            text
        }
    }

    /// Extract the user message from text.
    fn get_user_message(text: &str) -> &str {
        let prefix = "User message:";
        if let Some(pos) = text.find(prefix) {
            text[pos + prefix.len()..].trim()
        } else {
            text
        }
    }
}

#[async_trait]
impl Storage for Mem0Storage {
    fn save(
        &self,
        value: &str,
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error> {
        // Placeholder: Mem0 integration requires the mem0 service
        log::warn!(
            "Mem0Storage save called but Mem0 integration is not yet implemented in Rust. \
             Memory type: {}, value: '{}'",
            self.memory_type,
            &value[..std::cmp::min(value.len(), 100)]
        );
        Ok(())
    }

    fn search(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        // Placeholder: Mem0 integration requires the mem0 service
        log::warn!(
            "Mem0Storage search called but Mem0 integration is not yet implemented in Rust. \
             Memory type: {}, query: '{}'",
            self.memory_type,
            query
        );
        Ok(Vec::new())
    }

    fn reset(&self) -> Result<(), anyhow::Error> {
        // Placeholder: Mem0 integration requires the mem0 service
        log::warn!("Mem0Storage reset called but Mem0 integration is not yet implemented in Rust.");
        Ok(())
    }
}
