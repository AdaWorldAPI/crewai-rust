//! Crew chat input models.
//!
//! Corresponds to `crewai/types/crew_chat.py`.

use serde::{Deserialize, Serialize};

/// Represents a single required input for the crew.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatInputField {
    /// The name of the input field.
    pub name: String,
    /// A short description of the input field.
    pub description: String,
}

/// Holds crew metadata and input field definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatInputs {
    /// The name of the crew.
    pub crew_name: String,
    /// A description of the crew's purpose.
    pub crew_description: String,
    /// A list of input fields for the crew.
    pub inputs: Vec<ChatInputField>,
}

impl ChatInputs {
    /// Create a new ChatInputs with an empty inputs list.
    pub fn new(crew_name: String, crew_description: String) -> Self {
        Self {
            crew_name,
            crew_description,
            inputs: Vec::new(),
        }
    }
}
