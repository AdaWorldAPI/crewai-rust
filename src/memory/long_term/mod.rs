//! Long-term memory for managing cross-run data related to crew execution and performance.
//!
//! Port of crewai/memory/long_term/

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::memory::memory::Memory;
use crate::memory::storage::ltm_sqlite_storage::LTMSQLiteStorage;

/// An item stored in long-term memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermMemoryItem {
    /// The agent role that created this item.
    pub agent: String,
    /// The task description.
    pub task: String,
    /// The expected output description.
    pub expected_output: String,
    /// Timestamp of the memory.
    pub datetime: String,
    /// Optional quality score.
    pub quality: Option<f64>,
    /// Metadata associated with this memory item.
    pub metadata: HashMap<String, Value>,
}

impl LongTermMemoryItem {
    /// Create a new LongTermMemoryItem.
    pub fn new(
        agent: String,
        task: String,
        expected_output: String,
        datetime: String,
        quality: Option<f64>,
        metadata: Option<HashMap<String, Value>>,
    ) -> Self {
        Self {
            agent,
            task,
            expected_output,
            datetime,
            quality,
            metadata: metadata.unwrap_or_default(),
        }
    }
}

/// LongTermMemory manages cross-run data related to overall crew
/// execution and performance using SQLite storage.
pub struct LongTermMemory {
    /// The underlying LTM SQLite storage.
    pub storage: LTMSQLiteStorage,
}

impl LongTermMemory {
    /// Create a new LongTermMemory instance.
    ///
    /// # Arguments
    /// * `storage` - Optional pre-configured LTMSQLiteStorage.
    /// * `path` - Optional path to the database file.
    pub fn new(
        storage: Option<LTMSQLiteStorage>,
        path: Option<std::path::PathBuf>,
    ) -> Result<Self, anyhow::Error> {
        let storage = match storage {
            Some(s) => s,
            None => LTMSQLiteStorage::new(path, true)?,
        };
        Ok(Self { storage })
    }

    /// Save an item to long-term memory.
    ///
    /// # Arguments
    /// * `item` - The LongTermMemoryItem to save.
    pub fn save(&self, item: &LongTermMemoryItem) -> Result<(), anyhow::Error> {
        let mut metadata = item.metadata.clone();
        metadata.insert(
            "agent".to_string(),
            Value::String(item.agent.clone()),
        );
        metadata.insert(
            "expected_output".to_string(),
            Value::String(item.expected_output.clone()),
        );

        let score = metadata
            .get("quality")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        self.storage
            .save(&item.task, &metadata, &item.datetime, score)
    }

    /// Save an item to long-term memory asynchronously.
    pub async fn asave(&self, item: &LongTermMemoryItem) -> Result<(), anyhow::Error> {
        let mut metadata = item.metadata.clone();
        metadata.insert(
            "agent".to_string(),
            Value::String(item.agent.clone()),
        );
        metadata.insert(
            "expected_output".to_string(),
            Value::String(item.expected_output.clone()),
        );

        let score = metadata
            .get("quality")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        self.storage
            .asave(&item.task, &metadata, &item.datetime, score)
            .await
    }

    /// Search long-term memory for relevant entries.
    ///
    /// # Arguments
    /// * `task` - The task description to search for.
    /// * `latest_n` - Maximum number of results to return.
    pub fn search(
        &self,
        task: &str,
        latest_n: usize,
    ) -> Result<Vec<HashMap<String, Value>>, anyhow::Error> {
        match self.storage.load(task, latest_n)? {
            Some(results) => Ok(results),
            None => Ok(Vec::new()),
        }
    }

    /// Search long-term memory asynchronously.
    pub async fn asearch(
        &self,
        task: &str,
        latest_n: usize,
    ) -> Result<Vec<HashMap<String, Value>>, anyhow::Error> {
        match self.storage.aload(task, latest_n).await? {
            Some(results) => Ok(results),
            None => Ok(Vec::new()),
        }
    }

    /// Reset long-term memory.
    pub fn reset(&self) -> Result<(), anyhow::Error> {
        self.storage.reset()
    }
}
