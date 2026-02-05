//! Main Knowledge struct for managing knowledge sources and queries.
//!
//! Corresponds to `crewai/knowledge/knowledge.py`.
//!
//! The `Knowledge` struct manages a collection of knowledge sources and provides
//! query and ingestion capabilities backed by a configurable storage layer
//! (with optional RAG/vector search integration).

use std::sync::Arc;

use serde_json::Value;

use super::source::BaseKnowledgeSource;
use super::storage::{BaseKnowledgeStorage, KnowledgeStorage};

/// Knowledge manages a collection of knowledge sources and provides
/// query and ingestion capabilities.
///
/// Corresponds to `crewai.knowledge.knowledge.Knowledge`.
///
/// # Example
///
/// ```rust,no_run
/// use crewai::knowledge::{Knowledge, StringKnowledgeSource, KnowledgeStorage};
///
/// let source = StringKnowledgeSource::new("Some important text".to_string());
/// let knowledge = Knowledge::new(
///     vec![Box::new(source)],
///     None,
///     Some("my_collection".to_string()),
///     None,
/// );
/// // knowledge.add_sources().unwrap();
/// // let results = knowledge.query("important", None, None).unwrap();
/// ```
pub struct Knowledge {
    /// The collection of knowledge sources.
    pub sources: Vec<Box<dyn BaseKnowledgeSource>>,
    /// The knowledge storage backend.
    pub storage: Arc<KnowledgeStorage>,
    /// Optional embedder configuration (provider-specific).
    pub embedder_config: Option<Value>,
    /// Optional collection name override.
    pub collection_name: Option<String>,
}

impl Knowledge {
    /// Create a new Knowledge instance.
    ///
    /// # Arguments
    ///
    /// * `sources` - Vector of knowledge sources to manage.
    /// * `embedder_config` - Optional embedder configuration
    ///   (e.g., provider name, model, dimensions).
    /// * `collection_name` - Optional collection name override
    ///   (defaults to "knowledge").
    /// * `storage` - Optional pre-configured storage backend.
    ///   If None, a new `KnowledgeStorage` is created using the
    ///   provided embedder_config and collection_name.
    pub fn new(
        sources: Vec<Box<dyn BaseKnowledgeSource>>,
        embedder_config: Option<Value>,
        collection_name: Option<String>,
        storage: Option<KnowledgeStorage>,
    ) -> Self {
        let storage = storage.unwrap_or_else(|| {
            KnowledgeStorage::new(embedder_config.clone(), collection_name.clone())
        });

        Self {
            sources,
            storage: Arc::new(storage),
            embedder_config,
            collection_name,
        }
    }

    /// Query the knowledge base.
    ///
    /// Searches all ingested knowledge for content matching the query.
    /// Uses the underlying storage's search implementation (typically
    /// vector similarity search).
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string.
    /// * `limit` - Maximum number of results to return (default: 3).
    /// * `score_threshold` - Minimum similarity score for results (default: 0.35).
    ///
    /// # Returns
    ///
    /// List of matching results as JSON values, or an error.
    pub fn query(
        &self,
        query: &str,
        limit: Option<usize>,
        score_threshold: Option<f64>,
    ) -> Result<Vec<Value>, anyhow::Error> {
        let limit = limit.unwrap_or(3);
        let score_threshold = score_threshold.unwrap_or(0.35);
        self.storage.search(query, limit, score_threshold)
    }

    /// Query the knowledge base asynchronously.
    ///
    /// Async version of `query()` for use in async contexts.
    pub async fn aquery(
        &self,
        query: &str,
        limit: Option<usize>,
        score_threshold: Option<f64>,
    ) -> Result<Vec<Value>, anyhow::Error> {
        let limit = limit.unwrap_or(3);
        let score_threshold = score_threshold.unwrap_or(0.35);
        self.storage.asearch(query, limit, score_threshold).await
    }

    /// Add and ingest all configured knowledge sources into storage.
    ///
    /// Iterates over all registered knowledge sources, calling their `add()`
    /// method to process content, chunk it, compute embeddings, and save
    /// them to the storage backend.
    pub fn add_sources(&self) -> Result<(), anyhow::Error> {
        for source in &self.sources {
            source.add(&self.storage)?;
        }
        Ok(())
    }

    /// Add and ingest all configured knowledge sources asynchronously.
    ///
    /// Async version of `add_sources()` for use in async contexts.
    pub async fn aadd_sources(&self) -> Result<(), anyhow::Error> {
        for source in &self.sources {
            source.aadd(&self.storage).await?;
        }
        Ok(())
    }

    /// Reset all knowledge by clearing the storage.
    ///
    /// This removes all stored documents, embeddings, and collections
    /// from the storage backend.
    pub fn reset(&self) -> Result<(), anyhow::Error> {
        self.storage.reset()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::source::StringKnowledgeSource;

    #[test]
    fn test_knowledge_new_default() {
        let knowledge = Knowledge::new(Vec::new(), None, None, None);
        assert!(knowledge.sources.is_empty());
        assert!(knowledge.embedder_config.is_none());
        assert!(knowledge.collection_name.is_none());
    }

    #[test]
    fn test_knowledge_new_with_collection() {
        let source = StringKnowledgeSource::new("Hello world".to_string());
        let knowledge = Knowledge::new(
            vec![Box::new(source)],
            None,
            Some("test_collection".to_string()),
            None,
        );
        assert_eq!(knowledge.sources.len(), 1);
        assert_eq!(
            knowledge.collection_name.as_deref(),
            Some("test_collection")
        );
    }
}
