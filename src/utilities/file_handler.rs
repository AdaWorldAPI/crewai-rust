//! File handler utility for reading/writing data files.
//!
//! Corresponds to `crewai/utilities/file_handler.py`.

use std::fs;
use std::path::Path;

use serde_json::Value;

/// Handles reading and writing JSON data files for training, storage, etc.
#[derive(Debug, Clone)]
pub struct FileHandler {
    /// Directory for file storage.
    pub directory: String,
}

impl FileHandler {
    /// Create a new `FileHandler` for the given directory.
    pub fn new(directory: impl Into<String>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    /// Load JSON data from a file in the handler's directory.
    ///
    /// Returns `None` if the file does not exist.
    pub fn load(&self, filename: &str) -> Option<Value> {
        let path = Path::new(&self.directory).join(filename);
        if path.exists() {
            let content = fs::read_to_string(&path).ok()?;
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }

    /// Save JSON data to a file in the handler's directory.
    ///
    /// Creates the directory if it does not exist.
    pub fn save(&self, filename: &str, data: &Value) -> std::io::Result<()> {
        let dir = Path::new(&self.directory);
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
        let path = dir.join(filename);
        let content = serde_json::to_string_pretty(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, content)
    }

    /// Check if a file exists in the handler's directory.
    pub fn exists(&self, filename: &str) -> bool {
        Path::new(&self.directory).join(filename).exists()
    }
}

impl Default for FileHandler {
    fn default() -> Self {
        Self::new(".")
    }
}
