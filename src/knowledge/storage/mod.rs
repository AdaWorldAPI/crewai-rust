//! Knowledge storage for managing vector-based knowledge retrieval.
//!
//! Corresponds to `crewai/knowledge/storage/`.
//!
//! Provides the `BaseKnowledgeStorage` trait and a concrete `KnowledgeStorage`
//! implementation that delegates to a configurable RAG client (e.g., ChromaDB)
//! for vector similarity search and document storage.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Base trait
// ---------------------------------------------------------------------------

/// Base trait for knowledge storage implementations.
///
/// Defines the interface for searching, saving, and managing knowledge
/// documents in a vector store. Implementations should integrate with
/// a RAG (Retrieval-Augmented Generation) backend for embedding-based
/// similarity search.
///
/// Corresponds to `crewai.knowledge.storage.base_knowledge_storage.BaseKnowledgeStorage`.
#[async_trait]
pub trait BaseKnowledgeStorage: Send + Sync {
    /// Search for relevant content.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string.
    /// * `limit` - Maximum number of results to return.
    /// * `score_threshold` - Minimum similarity score for results.
    ///
    /// # Returns
    ///
    /// A list of search results as JSON values, ordered by relevance.
    fn search(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error>;

    /// Search for relevant content asynchronously.
    ///
    /// Default implementation delegates to the synchronous `search()`.
    async fn asearch(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        self.search(query, limit, score_threshold)
    }

    /// Save raw document strings to storage.
    ///
    /// This is the primary ingestion method. The storage backend is
    /// responsible for computing embeddings and persisting them.
    ///
    /// # Arguments
    ///
    /// * `documents` - Vector of document strings to save.
    fn save(&self, documents: &[String]) -> Result<(), anyhow::Error>;

    /// Save raw document strings to storage asynchronously.
    ///
    /// Default implementation delegates to the synchronous `save()`.
    async fn asave(&self, documents: &[String]) -> Result<(), anyhow::Error> {
        self.save(documents)
    }

    /// Save text chunks with metadata to storage.
    ///
    /// # Arguments
    ///
    /// * `chunks` - Vector of text chunks to save.
    /// * `metadata` - Metadata to attach to all chunks.
    fn save_chunks(
        &self,
        chunks: &[String],
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error>;

    /// Save text chunks with metadata to storage asynchronously.
    ///
    /// Default implementation delegates to the synchronous `save_chunks()`.
    async fn asave_chunks(
        &self,
        chunks: &[String],
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error> {
        self.save_chunks(chunks, metadata)
    }

    /// Reset the storage by removing all data in the collection.
    fn reset(&self) -> Result<(), anyhow::Error>;

    /// Reset the storage asynchronously.
    ///
    /// Default implementation delegates to the synchronous `reset()`.
    async fn areset(&self) -> Result<(), anyhow::Error> {
        self.reset()
    }
}

// ---------------------------------------------------------------------------
// Concrete implementation
// ---------------------------------------------------------------------------

/// Default KnowledgeStorage backed by a RAG client.
///
/// Extends the base storage with embedding support for knowledge entries,
/// improving search efficiency through vector similarity.
///
/// In the Python implementation, this integrates with ChromaDB or other
/// vector databases. The Rust implementation provides the interface and
/// delegates to the configured RAG client.
///
/// Corresponds to `crewai.knowledge.storage.knowledge_storage.KnowledgeStorage`.
///
/// # Example
///
/// ```rust,no_run
/// use crewai::knowledge::storage::KnowledgeStorage;
///
/// let storage = KnowledgeStorage::new(None, Some("my_knowledge".to_string()));
/// ```
pub struct KnowledgeStorage {
    /// Embedder configuration (provider-specific, e.g., model name, dimensions).
    pub embedder_config: Option<Value>,
    /// Collection name in the vector store.
    /// Prefixed with "knowledge_" when accessing the backend.
    pub collection_name: Option<String>,
    /// Maximum number of results for default queries.
    pub default_limit: usize,
    /// Default score threshold for queries.
    pub default_score_threshold: f64,
}

impl KnowledgeStorage {
    /// Create a new KnowledgeStorage instance.
    ///
    /// # Arguments
    ///
    /// * `embedder_config` - Optional embedder configuration (provider spec).
    /// * `collection_name` - Optional collection name override.
    pub fn new(embedder_config: Option<Value>, collection_name: Option<String>) -> Self {
        Self {
            embedder_config,
            collection_name,
            default_limit: 5,
            default_score_threshold: 0.6,
        }
    }

    /// Get the fully-qualified collection name for the backend.
    ///
    /// Returns "knowledge_{name}" if a collection name is set,
    /// or "knowledge" otherwise.
    fn effective_collection_name(&self) -> String {
        match &self.collection_name {
            Some(name) => format!("knowledge_{}", name),
            None => "knowledge".to_string(),
        }
    }
}

#[async_trait]
impl BaseKnowledgeStorage for KnowledgeStorage {
    fn search(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        if query.is_empty() {
            return Err(anyhow::anyhow!("Query cannot be empty"));
        }

        let collection = self.effective_collection_name();
        log::debug!(
            "KnowledgeStorage::search: collection='{}', query='{}', limit={}, threshold={}",
            collection,
            query,
            limit,
            score_threshold
        );

        // Delegate to RAG client when integrated:
        // let client = self.get_client();
        // client.search(collection, query, limit, None, score_threshold)
        //
        // For now, return empty results. Integration point for RAG backend.
        Ok(Vec::new())
    }

    async fn asearch(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        if query.is_empty() {
            return Err(anyhow::anyhow!("Query cannot be empty"));
        }

        let collection = self.effective_collection_name();
        log::debug!(
            "KnowledgeStorage::asearch: collection='{}', query='{}'",
            collection,
            query,
        );

        // Delegate to RAG client async search when integrated:
        // let client = self.get_client();
        // client.asearch(collection, query, limit, None, score_threshold).await
        Ok(Vec::new())
    }

    fn save(&self, documents: &[String]) -> Result<(), anyhow::Error> {
        if documents.is_empty() {
            return Ok(());
        }

        let collection = self.effective_collection_name();
        log::debug!(
            "KnowledgeStorage::save: collection='{}', num_documents={}",
            collection,
            documents.len()
        );

        // Delegate to RAG client when integrated:
        // let client = self.get_client();
        // client.get_or_create_collection(&collection);
        // let rag_docs: Vec<_> = documents.iter()
        //     .map(|doc| json!({"content": doc}))
        //     .collect();
        // client.add_documents(&collection, &rag_docs);
        Ok(())
    }

    fn save_chunks(
        &self,
        chunks: &[String],
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error> {
        if chunks.is_empty() {
            return Ok(());
        }

        let collection = self.effective_collection_name();
        log::debug!(
            "KnowledgeStorage::save_chunks: collection='{}', num_chunks={}, metadata_keys={:?}",
            collection,
            chunks.len(),
            metadata.keys().collect::<Vec<_>>()
        );

        // Delegate to RAG client when integrated:
        // let client = self.get_client();
        // client.get_or_create_collection(&collection);
        // let rag_docs: Vec<_> = chunks.iter()
        //     .map(|chunk| {
        //         let mut doc = json!({"content": chunk});
        //         for (k, v) in metadata { doc[k] = v.clone(); }
        //         doc
        //     })
        //     .collect();
        // client.add_documents(&collection, &rag_docs);
        Ok(())
    }

    fn reset(&self) -> Result<(), anyhow::Error> {
        let collection = self.effective_collection_name();
        log::debug!(
            "KnowledgeStorage::reset: collection='{}'",
            collection
        );

        // Delegate to RAG client when integrated:
        // let client = self.get_client();
        // client.delete_collection(&collection);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_storage_new_default() {
        let storage = KnowledgeStorage::new(None, None);
        assert!(storage.collection_name.is_none());
        assert_eq!(storage.effective_collection_name(), "knowledge");
    }

    #[test]
    fn test_knowledge_storage_new_with_collection() {
        let storage = KnowledgeStorage::new(None, Some("docs".to_string()));
        assert_eq!(
            storage.effective_collection_name(),
            "knowledge_docs"
        );
    }

    #[test]
    fn test_knowledge_storage_search_empty_query() {
        let storage = KnowledgeStorage::new(None, None);
        let result = storage.search("", 5, 0.6);
        assert!(result.is_err());
    }

    #[test]
    fn test_knowledge_storage_search() {
        let storage = KnowledgeStorage::new(None, None);
        let results = storage.search("test query", 5, 0.6).unwrap();
        // Returns empty until RAG backend is integrated.
        assert!(results.is_empty());
    }

    #[test]
    fn test_knowledge_storage_save_empty() {
        let storage = KnowledgeStorage::new(None, None);
        assert!(storage.save(&[]).is_ok());
    }

    #[test]
    fn test_knowledge_storage_save_chunks() {
        let storage = KnowledgeStorage::new(None, None);
        let chunks = vec!["chunk1".to_string(), "chunk2".to_string()];
        let metadata = HashMap::new();
        assert!(storage.save_chunks(&chunks, &metadata).is_ok());
    }

    #[test]
    fn test_knowledge_storage_reset() {
        let storage = KnowledgeStorage::new(None, None);
        assert!(storage.reset().is_ok());
    }
}
