//! Internationalization support for CrewAI prompts and messages.
//!
//! Corresponds to `crewai/utilities/i18n.py`.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Embedded English translations JSON (used when no custom file is provided).
const EMBEDDED_EN_JSON: &str = include_str!("../translations/en.json");

/// Handles loading and retrieving internationalized prompts.
///
/// Prompts are stored in a nested map: `kind -> key -> value`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct I18N {
    /// The loaded prompts, keyed by `kind` then by `key`.
    #[serde(skip)]
    prompts: HashMap<String, HashMap<String, String>>,
    /// Optional path to a custom JSON file containing prompts.
    pub prompt_file: Option<String>,
}

impl Default for I18N {
    fn default() -> Self {
        Self::new(None)
    }
}

impl I18N {
    /// Create a new `I18N` instance, loading prompts from the given file
    /// or the embedded default `en.json`.
    pub fn new(prompt_file: Option<String>) -> Self {
        let prompts = match &prompt_file {
            Some(path) => {
                let content = std::fs::read_to_string(path)
                    .unwrap_or_else(|_| panic!("Prompt file '{}' not found.", path));
                serde_json::from_str(&content)
                    .unwrap_or_else(|_| panic!("Error decoding JSON from prompts file '{}'.", path))
            }
            None => {
                serde_json::from_str(EMBEDDED_EN_JSON)
                    .expect("Error decoding embedded en.json translations.")
            }
        };

        Self {
            prompts,
            prompt_file,
        }
    }

    /// Retrieve a prompt slice by key.
    pub fn slice(&self, slice: &str) -> String {
        self.retrieve("slices", slice)
    }

    /// Retrieve an error message by key.
    pub fn errors(&self, error: &str) -> String {
        self.retrieve("errors", error)
    }

    /// Retrieve a tool prompt by key.
    pub fn tools(&self, tool: &str) -> String {
        self.retrieve("tools", tool)
    }

    /// Retrieve a prompt by `kind` and `key`.
    ///
    /// # Panics
    /// Panics if the prompt for the given kind and key is not found.
    pub fn retrieve(&self, kind: &str, key: &str) -> String {
        self.prompts
            .get(kind)
            .and_then(|section| section.get(key))
            .cloned()
            .unwrap_or_else(|| panic!("Prompt for '{}':'{}' not found.", kind, key))
    }
}

/// Global cached `I18N` instance (default prompts).
static DEFAULT_I18N: OnceLock<I18N> = OnceLock::new();

/// Get the global cached `I18N` instance using the default embedded prompts.
pub fn get_i18n() -> &'static I18N {
    DEFAULT_I18N.get_or_init(|| I18N::new(None))
}
