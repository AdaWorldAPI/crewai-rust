//! Add image tool.
//!
//! Corresponds to `crewai/tools/agent_tools/add_image_tool.py`.
//!
//! Allows agents to add images to the conversation content.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Schema for add image tool arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddImageToolSchema {
    /// The URL or path of the image to add.
    pub image_url: String,
    /// Optional context or question about the image.
    pub action: Option<String>,
}

/// Tool for adding images to the conversation content.
///
/// Creates a multimodal content block with the image URL and an optional
/// text action/question about the image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddImageTool {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
}

impl Default for AddImageTool {
    fn default() -> Self {
        Self {
            name: "Add Image".to_string(),
            description: "Add an image to the conversation for analysis or reference.".to_string(),
        }
    }
}

impl AddImageTool {
    /// Create a new `AddImageTool`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom name and description (e.g., from i18n).
    pub fn with_i18n(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }

    /// Get the JSON schema for the tool's arguments.
    pub fn args_schema() -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "image_url": {
                    "type": "string",
                    "description": "The URL or path of the image to add"
                },
                "action": {
                    "type": "string",
                    "description": "Optional context or question about the image"
                }
            },
            "required": ["image_url"]
        })
    }

    /// Execute the tool: create a multimodal content block.
    ///
    /// Returns a JSON Value representing the message with image content.
    pub fn run(&self, image_url: &str, action: Option<&str>) -> Value {
        let action_text = action.unwrap_or("Analyze this image");

        serde_json::json!({
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": action_text
                },
                {
                    "type": "image_url",
                    "image_url": {
                        "url": image_url
                    }
                }
            ]
        })
    }
}
