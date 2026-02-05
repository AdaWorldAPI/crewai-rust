//! Read file tool.
//!
//! Corresponds to `crewai/tools/agent_tools/read_file_tool.py`.
//!
//! Provides agents with the ability to read input files provided to the crew.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Schema for read file tool arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileToolSchema {
    /// The name of the input file to read.
    pub file_name: String,
}

/// Tool for reading input files provided to the crew kickoff.
///
/// Provides agents access to files passed via the `files` key in inputs.
/// Returns file content as text for text files, or base64 for binary files.
#[derive(Debug, Clone)]
pub struct ReadFileTool {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// Available input files, keyed by filename.
    /// Value is the file content as bytes.
    files: Option<HashMap<String, FileInput>>,
}

/// Representation of a file input.
#[derive(Debug, Clone)]
pub struct FileInput {
    /// The raw file content.
    pub content: Vec<u8>,
    /// MIME content type (e.g., "text/plain", "image/png").
    pub content_type: String,
    /// Original filename.
    pub filename: Option<String>,
}

impl Default for ReadFileTool {
    fn default() -> Self {
        Self {
            name: "read_file".to_string(),
            description: "Read content from an input file by name. \
                          Returns file content as text for text files, or base64 for binary files."
                .to_string(),
            files: None,
        }
    }
}

impl ReadFileTool {
    /// Create a new `ReadFileTool`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set available input files.
    pub fn set_files(&mut self, files: Option<HashMap<String, FileInput>>) {
        self.files = files;
    }

    /// Get the JSON schema for the tool's arguments.
    pub fn args_schema() -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_name": {
                    "type": "string",
                    "description": "The name of the input file to read"
                }
            },
            "required": ["file_name"]
        })
    }

    /// Read an input file by name.
    pub fn run(&self, file_name: &str) -> String {
        let files = match &self.files {
            Some(f) => f,
            None => return "No input files available.".to_string(),
        };

        let file_input = match files.get(file_name) {
            Some(f) => f,
            None => {
                let available = files.keys().cloned().collect::<Vec<_>>().join(", ");
                return format!("File '{}' not found. Available files: {}", file_name, available);
            }
        };

        let filename = file_input
            .filename
            .as_deref()
            .unwrap_or(file_name);

        let text_types = [
            "text/",
            "application/json",
            "application/xml",
            "application/x-yaml",
        ];

        if text_types
            .iter()
            .any(|t| file_input.content_type.starts_with(t))
        {
            match String::from_utf8(file_input.content.clone()) {
                Ok(text) => text,
                Err(_) => format!("[Binary file: {} ({})]", filename, file_input.content_type),
            }
        } else {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&file_input.content);
            format!(
                "[Binary file: {} ({})]\nBase64: {}",
                filename, file_input.content_type, encoded
            )
        }
    }
}
