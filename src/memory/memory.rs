//! Base Memory struct with storage, embedder config, and crew references.
//!
//! Port of crewai/memory/memory.py

use std::any::Any;
use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::memory::storage::interface::Storage;

/// Base class for memory, supporting agent tags and generic metadata.
pub struct Memory {
    /// The storage backend for this memory instance.
    pub storage: Box<dyn Storage>,
    /// Optional embedder configuration (mirrors Python's EmbedderConfig | dict | None).
    pub embedder_config: Option<Value>,
    /// Optional reference to the crew that owns this memory.
    pub crew: Option<Box<dyn Any + Send + Sync>>,
    /// Optional reference to the current agent.
    agent: Option<Box<dyn Any + Send + Sync>>,
    /// Optional reference to the current task.
    task: Option<Box<dyn Any + Send + Sync>>,
}

impl Memory {
    /// Create a new Memory instance with the given storage backend.
    pub fn new(storage: Box<dyn Storage>) -> Self {
        Self {
            storage,
            embedder_config: None,
            crew: None,
            agent: None,
            task: None,
        }
    }

    /// Create a new Memory instance with storage and embedder config.
    pub fn with_embedder(storage: Box<dyn Storage>, embedder_config: Option<Value>) -> Self {
        Self {
            storage,
            embedder_config,
            crew: None,
            agent: None,
            task: None,
        }
    }

    /// Get the current task associated with this memory.
    pub fn task(&self) -> &Option<Box<dyn Any + Send + Sync>> {
        &self.task
    }

    /// Set the current task associated with this memory.
    pub fn set_task(&mut self, task: Option<Box<dyn Any + Send + Sync>>) {
        self.task = task;
    }

    /// Get the current agent associated with this memory.
    pub fn agent(&self) -> &Option<Box<dyn Any + Send + Sync>> {
        &self.agent
    }

    /// Set the current agent associated with this memory.
    pub fn set_agent(&mut self, agent: Option<Box<dyn Any + Send + Sync>>) {
        self.agent = agent;
    }

    /// Save a value to memory.
    ///
    /// # Arguments
    /// * `value` - The value to save.
    /// * `metadata` - Optional metadata to associate with the value.
    pub fn save(
        &self,
        value: &str,
        metadata: Option<HashMap<String, Value>>,
    ) -> Result<(), anyhow::Error> {
        let metadata = metadata.unwrap_or_default();
        self.storage.save(value, &metadata)
    }

    /// Save a value to memory asynchronously.
    ///
    /// # Arguments
    /// * `value` - The value to save.
    /// * `metadata` - Optional metadata to associate with the value.
    pub async fn asave(
        &self,
        value: &str,
        metadata: Option<HashMap<String, Value>>,
    ) -> Result<(), anyhow::Error> {
        let metadata = metadata.unwrap_or_default();
        self.storage.asave(value, &metadata).await
    }

    /// Search memory for relevant entries.
    ///
    /// # Arguments
    /// * `query` - The search query.
    /// * `limit` - Maximum number of results to return.
    /// * `score_threshold` - Minimum similarity score for results.
    ///
    /// # Returns
    /// List of matching memory entries as JSON values.
    pub fn search(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        self.storage.search(query, limit, score_threshold)
    }

    /// Search memory for relevant entries asynchronously.
    ///
    /// # Arguments
    /// * `query` - The search query.
    /// * `limit` - Maximum number of results to return.
    /// * `score_threshold` - Minimum similarity score for results.
    ///
    /// # Returns
    /// List of matching memory entries as JSON values.
    pub async fn asearch(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        self.storage.asearch(query, limit, score_threshold).await
    }

    /// Set the crew for this memory instance.
    pub fn set_crew(&mut self, crew: Box<dyn Any + Send + Sync>) {
        self.crew = Some(crew);
    }
}
