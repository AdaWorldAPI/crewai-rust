//! RAG-based storage extending the base storage with embedding support.
//!
//! Port of crewai/memory/storage/rag_storage.py

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::memory::storage::interface::Storage;

/// Maximum file name length for storage paths.
const MAX_FILE_NAME_LENGTH: usize = 255;

/// RAGStorage extends Storage to handle embeddings for memory entries,
/// improving search efficiency through vector-based retrieval.
pub struct RAGStorage {
    /// The type of memory (e.g., "short_term", "entities").
    pub storage_type: String,
    /// Whether reset is allowed.
    pub allow_reset: bool,
    /// Embedder configuration.
    pub embedder_config: Option<Value>,
    /// Concatenated sanitized agent roles.
    pub agents: String,
    /// The constructed storage file name.
    pub storage_file_name: String,
    /// Optional persist path.
    pub path: Option<String>,
    /// Optional reference to a RAG client (type-erased).
    client: Option<Box<dyn std::any::Any + Send + Sync>>,
}

impl RAGStorage {
    /// Create a new RAGStorage instance.
    ///
    /// # Arguments
    /// * `storage_type` - The type of memory storage.
    /// * `allow_reset` - Whether reset is allowed.
    /// * `embedder_config` - Optional embedder configuration.
    /// * `crew_agent_roles` - Optional list of agent role strings.
    /// * `path` - Optional persist directory path.
    pub fn new(
        storage_type: &str,
        allow_reset: bool,
        embedder_config: Option<Value>,
        crew_agent_roles: Option<Vec<String>>,
        path: Option<String>,
    ) -> Self {
        let sanitized_roles: Vec<String> = crew_agent_roles
            .unwrap_or_default()
            .iter()
            .map(|role| Self::sanitize_role(role))
            .collect();
        let agents_str = sanitized_roles.join("_");
        let storage_file_name =
            Self::build_storage_file_name(storage_type, &agents_str);

        Self {
            storage_type: storage_type.to_string(),
            allow_reset,
            embedder_config,
            agents: agents_str,
            storage_file_name,
            path,
            client: None,
        }
    }

    /// Sanitize an agent role to ensure valid directory names.
    fn sanitize_role(role: &str) -> String {
        role.replace('\n', "")
            .replace(' ', "_")
            .replace('/', "_")
    }

    /// Build the storage file name, ensuring it does not exceed max allowed length.
    fn build_storage_file_name(storage_type: &str, file_name: &str) -> String {
        let base_path = format!(
            "{}/{}",
            crate::utilities::paths::db_storage_path(),
            storage_type
        );
        let trimmed = if file_name.len() > MAX_FILE_NAME_LENGTH {
            log::warn!(
                "Trimming file name from {} to {} characters.",
                file_name.len(),
                MAX_FILE_NAME_LENGTH
            );
            &file_name[..MAX_FILE_NAME_LENGTH]
        } else {
            file_name
        };
        format!("{}/{}", base_path, trimmed)
    }

    /// Get the collection name for this storage instance.
    fn collection_name(&self) -> String {
        if self.agents.is_empty() {
            format!("memory_{}", self.storage_type)
        } else {
            format!("memory_{}_{}", self.storage_type, self.agents)
        }
    }
}

#[async_trait]
impl Storage for RAGStorage {
    fn save(
        &self,
        value: &str,
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error> {
        let collection_name = self.collection_name();
        log::debug!(
            "RAGStorage save to collection '{}': value='{}'",
            collection_name,
            &value[..std::cmp::min(value.len(), 100)]
        );
        // TODO: Integrate with actual RAG client (ChromaDB/Qdrant)
        // client.get_or_create_collection(collection_name)
        // client.add_documents(collection_name, documents)
        Ok(())
    }

    async fn asave(
        &self,
        value: &str,
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error> {
        let collection_name = self.collection_name();
        log::debug!(
            "RAGStorage async save to collection '{}': value='{}'",
            collection_name,
            &value[..std::cmp::min(value.len(), 100)]
        );
        // TODO: Integrate with actual RAG client (ChromaDB/Qdrant) async API
        Ok(())
    }

    fn search(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        let collection_name = self.collection_name();
        log::debug!(
            "RAGStorage search in collection '{}': query='{}'",
            collection_name,
            query
        );
        // TODO: Integrate with actual RAG client search
        // client.search(collection_name, query, limit, score_threshold)
        Ok(Vec::new())
    }

    async fn asearch(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        let collection_name = self.collection_name();
        log::debug!(
            "RAGStorage async search in collection '{}': query='{}'",
            collection_name,
            query
        );
        // TODO: Integrate with actual RAG client async search
        Ok(Vec::new())
    }

    fn reset(&self) -> Result<(), anyhow::Error> {
        let collection_name = self.collection_name();
        log::debug!(
            "RAGStorage reset collection '{}'",
            collection_name
        );
        // TODO: Integrate with actual RAG client reset
        // client.delete_collection(collection_name)
        Ok(())
    }
}
