//! Training data handler.
//!
//! Corresponds to `crewai/utilities/training_handler.py`.

use serde_json::Value;

use crate::utilities::file_handler::FileHandler;

/// Handles loading and saving crew training data.
#[derive(Debug, Clone)]
pub struct CrewTrainingHandler {
    file_handler: FileHandler,
}

impl CrewTrainingHandler {
    /// Create a new `CrewTrainingHandler` with the given data directory.
    pub fn new(directory: impl Into<String>) -> Self {
        Self {
            file_handler: FileHandler::new(directory),
        }
    }

    /// Load training data from disk.
    pub fn load(&self) -> Option<Value> {
        self.file_handler.load("training_data.json")
    }

    /// Save training data to disk.
    pub fn save(&self, data: &Value) -> std::io::Result<()> {
        self.file_handler.save("training_data.json", data)
    }

    /// Append a new training entry.
    pub fn append(&self, agent_id: &str, data: Value) -> std::io::Result<()> {
        let mut training_data = self.load().unwrap_or_else(|| Value::Object(Default::default()));

        if let Value::Object(ref mut map) = training_data {
            let entries = map
                .entry(agent_id.to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
            if let Value::Array(ref mut arr) = entries {
                arr.push(data);
            }
        }

        self.save(&training_data)
    }
}

impl Default for CrewTrainingHandler {
    fn default() -> Self {
        Self::new("training_data")
    }
}
