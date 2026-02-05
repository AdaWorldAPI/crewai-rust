//! Base RAG storage abstraction.
//!
//! Port of crewai/rag/storage/base_rag_storage.py

use std::collections::HashMap;

use serde_json::Value;

use crate::rag::types::SearchResult;

/// Base trait for RAG-based storage implementations.
///
/// This trait is used by the memory system's RAGStorage to interface
/// with vector database backends.
pub trait BaseRAGStorage: Send + Sync {
    /// Get the storage type name.
    fn storage_type(&self) -> &str;

    /// Whether reset is allowed.
    fn allow_reset(&self) -> bool;

    /// Sanitize an agent role to ensure valid directory names.
    fn sanitize_role(&self, role: &str) -> String {
        role.replace('\n', "")
            .replace(' ', "_")
            .replace('/', "_")
    }

    /// Save a value with metadata to the storage.
    ///
    /// # Arguments
    /// * `value` - The value to save.
    /// * `metadata` - Metadata to associate with the value.
    fn save(
        &self,
        value: &str,
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error>;

    /// Search for entries in the storage.
    ///
    /// # Arguments
    /// * `query` - The search query.
    /// * `limit` - Maximum number of results.
    /// * `filter` - Optional metadata filter.
    /// * `score_threshold` - Minimum similarity score.
    ///
    /// # Returns
    /// Vector of search results.
    fn search(
        &self,
        query: &str,
        limit: usize,
        filter: Option<&HashMap<String, Value>>,
        score_threshold: f64,
    ) -> Result<Vec<SearchResult>, anyhow::Error>;

    /// Reset the storage by removing all data.
    fn reset(&self) -> Result<(), anyhow::Error>;
}
