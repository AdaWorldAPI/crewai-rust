//! Task output storage handler.
//!
//! Corresponds to `crewai/utilities/task_output_storage_handler.py`.

use serde_json::Value;

use crate::utilities::file_handler::FileHandler;

/// Handles loading and saving task output data.
#[derive(Debug, Clone)]
pub struct TaskOutputStorageHandler {
    file_handler: FileHandler,
}

impl TaskOutputStorageHandler {
    /// Create a new `TaskOutputStorageHandler` with the given data directory.
    pub fn new(directory: impl Into<String>) -> Self {
        Self {
            file_handler: FileHandler::new(directory),
        }
    }

    /// Load all stored task outputs.
    pub fn load(&self) -> Option<Value> {
        self.file_handler.load("task_outputs.json")
    }

    /// Save task outputs to disk.
    pub fn save(&self, data: &Value) -> std::io::Result<()> {
        self.file_handler.save("task_outputs.json", data)
    }

    /// Add a task output to storage.
    pub fn add(&self, task_id: &str, output: Value) -> std::io::Result<()> {
        let mut data = self.load().unwrap_or_else(|| Value::Object(Default::default()));

        if let Value::Object(ref mut map) = data {
            map.insert(task_id.to_string(), output);
        }

        self.save(&data)
    }

    /// Retrieve a specific task output by ID.
    pub fn get(&self, task_id: &str) -> Option<Value> {
        let data = self.load()?;
        data.get(task_id).cloned()
    }
}

impl Default for TaskOutputStorageHandler {
    fn default() -> Self {
        Self::new("task_outputs")
    }
}
