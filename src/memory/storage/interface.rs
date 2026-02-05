//! Abstract storage interface for the memory system.
//!
//! Port of crewai/memory/storage/interface.py

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

/// Abstract base trait defining the storage interface.
///
/// All memory storage backends must implement this trait.
#[async_trait]
pub trait Storage: Send + Sync {
    /// Save a value with associated metadata to storage.
    ///
    /// # Arguments
    /// * `value` - The value to save.
    /// * `metadata` - Metadata to associate with the value.
    fn save(
        &self,
        value: &str,
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error>;

    /// Save a value with associated metadata to storage asynchronously.
    ///
    /// # Arguments
    /// * `value` - The value to save.
    /// * `metadata` - Metadata to associate with the value.
    async fn asave(
        &self,
        value: &str,
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error> {
        self.save(value, metadata)
    }

    /// Search storage for entries matching the query.
    ///
    /// # Arguments
    /// * `query` - The search query string.
    /// * `limit` - Maximum number of results to return.
    /// * `score_threshold` - Minimum similarity score for results.
    ///
    /// # Returns
    /// A vector of matching entries as JSON values.
    fn search(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error>;

    /// Search storage for entries matching the query asynchronously.
    ///
    /// # Arguments
    /// * `query` - The search query string.
    /// * `limit` - Maximum number of results to return.
    /// * `score_threshold` - Minimum similarity score for results.
    ///
    /// # Returns
    /// A vector of matching entries as JSON values.
    async fn asearch(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        self.search(query, limit, score_threshold)
    }

    /// Reset the storage, removing all entries.
    fn reset(&self) -> Result<(), anyhow::Error>;
}
