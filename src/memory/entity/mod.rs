//! Entity memory for managing structured information about entities and their relationships.
//!
//! Port of crewai/memory/entity/

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::memory::memory::Memory;
use crate::memory::storage::interface::Storage;
use crate::memory::storage::rag_storage::RAGStorage;

/// An item stored in entity memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMemoryItem {
    /// The name of the entity.
    pub name: String,
    /// The type of the entity.
    pub entity_type: String,
    /// A description of the entity.
    pub description: String,
    /// Metadata including relationships information.
    pub metadata: HashMap<String, Value>,
}

impl EntityMemoryItem {
    /// Create a new EntityMemoryItem.
    ///
    /// # Arguments
    /// * `name` - The name of the entity.
    /// * `entity_type` - The type of the entity.
    /// * `description` - A description of the entity.
    /// * `relationships` - A string describing the entity's relationships.
    pub fn new(
        name: String,
        entity_type: String,
        description: String,
        relationships: String,
    ) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert(
            "relationships".to_string(),
            Value::String(relationships),
        );
        Self {
            name,
            entity_type,
            description,
            metadata,
        }
    }
}

/// EntityMemory manages structured information about entities
/// and their relationships using RAG-based storage.
pub struct EntityMemory {
    /// The underlying memory instance.
    pub memory: Memory,
    /// The memory provider name (e.g., "mem0" for Mem0 integration).
    memory_provider: Option<String>,
}

impl EntityMemory {
    /// Create a new EntityMemory instance.
    ///
    /// # Arguments
    /// * `embedder_config` - Optional embedder configuration.
    /// * `storage` - Optional pre-configured storage backend.
    /// * `crew_agent_roles` - Optional list of agent roles.
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
            let config = embedder_config
                .as_ref()
                .and_then(|c| c.get("config"))
                .and_then(|c| serde_json::from_value::<HashMap<String, Value>>(c.clone()).ok());
            Box::new(
                crate::memory::storage::mem0_storage::Mem0Storage::new(
                    "entities",
                    None,
                    config,
                )
                .expect("Failed to create Mem0Storage"),
            )
        } else {
            Box::new(RAGStorage::new(
                "entities",
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

    /// Save one or more entity items to memory.
    ///
    /// # Arguments
    /// * `items` - A vector of EntityMemoryItem instances to save.
    pub fn save(&self, items: Vec<EntityMemoryItem>) -> Result<(), anyhow::Error> {
        if items.is_empty() {
            return Ok(());
        }

        let mut saved_count = 0;
        let mut errors: Vec<String> = Vec::new();

        for item in &items {
            let data = if self.memory_provider.as_deref() == Some("mem0") {
                format!(
                    "Remember details about the following entity:\n\
                     Name: {}\nType: {}\nEntity Description: {}",
                    item.name, item.entity_type, item.description
                )
            } else {
                format!(
                    "{}({}): {}",
                    item.name, item.entity_type, item.description
                )
            };

            match self.memory.save(&data, Some(item.metadata.clone())) {
                Ok(()) => saved_count += 1,
                Err(e) => errors.push(format!("{}: {}", item.name, e)),
            }
        }

        if !errors.is_empty() {
            return Err(anyhow::anyhow!(
                "Partial save: {} failed out of {}",
                errors.len(),
                items.len()
            ));
        }

        Ok(())
    }

    /// Save entity items asynchronously.
    pub async fn asave(&self, items: Vec<EntityMemoryItem>) -> Result<(), anyhow::Error> {
        if items.is_empty() {
            return Ok(());
        }

        let mut errors: Vec<String> = Vec::new();

        for item in &items {
            let data = if self.memory_provider.as_deref() == Some("mem0") {
                format!(
                    "Remember details about the following entity:\n\
                     Name: {}\nType: {}\nEntity Description: {}",
                    item.name, item.entity_type, item.description
                )
            } else {
                format!(
                    "{}({}): {}",
                    item.name, item.entity_type, item.description
                )
            };

            if let Err(e) = self.memory.asave(&data, Some(item.metadata.clone())).await {
                errors.push(format!("{}: {}", item.name, e));
            }
        }

        if !errors.is_empty() {
            return Err(anyhow::anyhow!(
                "Partial save: {} failed out of {}",
                errors.len(),
                items.len()
            ));
        }

        Ok(())
    }

    /// Search entity memory for relevant entries.
    pub fn search(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        self.memory.search(query, limit, score_threshold)
    }

    /// Search entity memory asynchronously.
    pub async fn asearch(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        self.memory.asearch(query, limit, score_threshold).await
    }

    /// Reset entity memory.
    pub fn reset(&self) -> Result<(), anyhow::Error> {
        self.memory.storage.reset()
    }
}
