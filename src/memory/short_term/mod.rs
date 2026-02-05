//! Short-term memory for managing transient data related to immediate tasks.
//!
//! Port of crewai/memory/short_term/

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::memory::memory::Memory;
use crate::memory::storage::interface::Storage;
use crate::memory::storage::rag_storage::RAGStorage;

/// An item stored in short-term memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemoryItem {
    /// The data content of the memory item.
    pub data: String,
    /// The agent role that created this item, if any.
    pub agent: Option<String>,
    /// Metadata associated with this memory item.
    pub metadata: HashMap<String, Value>,
}

impl ShortTermMemoryItem {
    /// Create a new ShortTermMemoryItem.
    ///
    /// # Arguments
    /// * `data` - The data content of the memory item.
    /// * `agent` - Optional agent role string.
    /// * `metadata` - Optional metadata dictionary.
    pub fn new(
        data: String,
        agent: Option<String>,
        metadata: Option<HashMap<String, Value>>,
    ) -> Self {
        Self {
            data,
            agent,
            metadata: metadata.unwrap_or_default(),
        }
    }
}

/// ShortTermMemory manages transient data related to immediate tasks
/// and interactions. Uses RAGStorage by default for vector-based retrieval.
pub struct ShortTermMemory {
    /// The underlying memory instance.
    pub memory: Memory,
    /// The memory provider name (e.g., "mem0" for Mem0 integration).
    memory_provider: Option<String>,
}

impl ShortTermMemory {
    /// Create a new ShortTermMemory instance.
    ///
    /// # Arguments
    /// * `embedder_config` - Optional embedder configuration.
    /// * `storage` - Optional pre-configured storage backend.
    /// * `crew_agent_roles` - Optional list of agent roles for collection naming.
    /// * `path` - Optional persist directory path.
    pub fn new(
        embedder_config: Option<Value>,
        storage: Option<Box<dyn Storage>>,
        crew_agent_roles: Option<Vec<String>>,
        path: Option<String>,
    ) -> Self {
        let memory_provider = embedder_config
            .as_ref()
            .and_then(|c| c.get("provider"))
            .and_then(|p| p.as_str())
            .map(|s| s.to_string());

        let storage: Box<dyn Storage> = if let Some(s) = storage {
            s
        } else if memory_provider.as_deref() == Some("mem0") {
            // Use Mem0Storage for mem0 provider
            let config = embedder_config
                .as_ref()
                .and_then(|c| c.get("config"))
                .and_then(|c| serde_json::from_value::<HashMap<String, Value>>(c.clone()).ok());
            Box::new(
                crate::memory::storage::mem0_storage::Mem0Storage::new(
                    "short_term",
                    None,
                    config,
                )
                .expect("Failed to create Mem0Storage"),
            )
        } else {
            Box::new(RAGStorage::new(
                "short_term",
                true,
                embedder_config.clone(),
                crew_agent_roles,
                path,
            ))
        };

        let memory = Memory::with_embedder(storage, embedder_config);

        Self {
            memory,
            memory_provider,
        }
    }

    /// Save a value to short-term memory.
    ///
    /// # Arguments
    /// * `value` - The value to save.
    /// * `metadata` - Optional metadata to associate with the value.
    /// * `agent_role` - Optional agent role string.
    pub fn save(
        &self,
        value: &str,
        metadata: Option<HashMap<String, Value>>,
        agent_role: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let mut item = ShortTermMemoryItem::new(
            value.to_string(),
            agent_role.map(|s| s.to_string()),
            metadata,
        );

        if self.memory_provider.as_deref() == Some("mem0") {
            item.data = format!(
                "Remember the following insights from Agent run: {}",
                item.data
            );
        }

        self.memory.save(&item.data, Some(item.metadata))
    }

    /// Save a value to short-term memory asynchronously.
    pub async fn asave(
        &self,
        value: &str,
        metadata: Option<HashMap<String, Value>>,
        agent_role: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let mut item = ShortTermMemoryItem::new(
            value.to_string(),
            agent_role.map(|s| s.to_string()),
            metadata,
        );

        if self.memory_provider.as_deref() == Some("mem0") {
            item.data = format!(
                "Remember the following insights from Agent run: {}",
                item.data
            );
        }

        self.memory.asave(&item.data, Some(item.metadata)).await
    }

    /// Search short-term memory for relevant entries.
    ///
    /// # Arguments
    /// * `query` - The search query.
    /// * `limit` - Maximum number of results to return.
    /// * `score_threshold` - Minimum similarity score for results.
    pub fn search(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        self.memory.search(query, limit, score_threshold)
    }

    /// Search short-term memory asynchronously.
    pub async fn asearch(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        self.memory.asearch(query, limit, score_threshold).await
    }

    /// Reset short-term memory.
    pub fn reset(&self) -> Result<(), anyhow::Error> {
        self.memory.storage.reset()
    }
}
