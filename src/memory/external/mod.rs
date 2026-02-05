//! External memory for managing data from external memory providers.
//!
//! Port of crewai/memory/external/

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::memory::memory::Memory;
use crate::memory::storage::interface::Storage;

/// An item stored in external memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalMemoryItem {
    /// The value content of the memory item.
    pub value: String,
    /// Metadata associated with this memory item.
    pub metadata: Option<HashMap<String, Value>>,
    /// The agent role that created this item, if any.
    pub agent: Option<String>,
}

impl ExternalMemoryItem {
    /// Create a new ExternalMemoryItem.
    pub fn new(
        value: String,
        metadata: Option<HashMap<String, Value>>,
        agent: Option<String>,
    ) -> Self {
        Self {
            value,
            metadata,
            agent,
        }
    }
}

/// ExternalMemory manages data from external memory providers (e.g., Mem0).
pub struct ExternalMemory {
    /// The underlying memory instance.
    pub memory: Memory,
}

impl ExternalMemory {
    /// Create a new ExternalMemory with an optional pre-configured storage.
    pub fn new(storage: Option<Box<dyn Storage>>) -> Self {
        // If no storage is provided, create a no-op placeholder.
        // The actual storage will be configured via set_crew / create_storage.
        let storage = storage.unwrap_or_else(|| {
            Box::new(NoOpStorage)
        });
        let memory = Memory::new(storage);
        Self { memory }
    }

    /// Create a storage backend for external memory based on the embedder config.
    ///
    /// # Arguments
    /// * `embedder_config` - The embedder configuration with a "provider" key.
    ///
    /// # Returns
    /// A boxed Storage implementation, or an error if the provider is not supported.
    pub fn create_storage(
        embedder_config: &Value,
    ) -> Result<Box<dyn Storage>, anyhow::Error> {
        let provider = embedder_config
            .get("provider")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("embedder_config must include a 'provider' key"))?;

        match provider {
            "mem0" => {
                let config = embedder_config
                    .get("config")
                    .and_then(|c| {
                        serde_json::from_value::<HashMap<String, Value>>(c.clone()).ok()
                    });
                let storage = crate::memory::storage::mem0_storage::Mem0Storage::new(
                    "external",
                    None,
                    config,
                )?;
                Ok(Box::new(storage))
            }
            other => Err(anyhow::anyhow!("Provider {} not supported", other)),
        }
    }

    /// Get the list of supported external storage provider names.
    pub fn external_supported_storages() -> Vec<&'static str> {
        vec!["mem0"]
    }

    /// Save a value to external memory.
    pub fn save(
        &self,
        value: &str,
        metadata: Option<HashMap<String, Value>>,
        agent_role: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let item = ExternalMemoryItem::new(
            value.to_string(),
            metadata,
            agent_role.map(|s| s.to_string()),
        );
        self.memory
            .save(&item.value, item.metadata)
    }

    /// Save a value to external memory asynchronously.
    pub async fn asave(
        &self,
        value: &str,
        metadata: Option<HashMap<String, Value>>,
        agent_role: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let item = ExternalMemoryItem::new(
            value.to_string(),
            metadata,
            agent_role.map(|s| s.to_string()),
        );
        self.memory
            .asave(&item.value, item.metadata)
            .await
    }

    /// Search external memory for relevant entries.
    pub fn search(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        self.memory.search(query, limit, score_threshold)
    }

    /// Search external memory asynchronously.
    pub async fn asearch(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        self.memory
            .asearch(query, limit, score_threshold)
            .await
    }

    /// Reset external memory.
    pub fn reset(&self) -> Result<(), anyhow::Error> {
        self.memory.storage.reset()
    }
}

/// A no-op storage implementation used as a placeholder when no storage is configured.
struct NoOpStorage;

#[async_trait::async_trait]
impl Storage for NoOpStorage {
    fn save(
        &self,
        _value: &str,
        _metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn search(
        &self,
        _query: &str,
        _limit: usize,
        _score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        Ok(Vec::new())
    }

    fn reset(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
