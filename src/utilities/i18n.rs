//! Internationalization support for CrewAI prompts and messages.
//!
//! Corresponds to `crewai/utilities/i18n.py`.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Embedded English translations JSON (used when no custom file is provided).
const EMBEDDED_EN_JSON: &str = include_str!("../translations/en.json");

/// Handles loading and retrieving internationalized prompts.
///
/// Prompts are stored in a nested map: `kind -> key -> value`.
/// Values can be strings or nested objects (which are serialized to JSON
/// strings when retrieved).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct I18N {
    /// The loaded prompts, keyed by `kind` then by `key`.
    /// Leaf values are `serde_json::Value` to support both string and
    /// nested-object entries (e.g. `tools.add_image`).
    #[serde(skip)]
    prompts: HashMap<String, HashMap<String, Value>>,
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
        let raw: HashMap<String, Value> = match &prompt_file {
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

        // Convert the raw two-level JSON into our internal representation.
        // Top-level keys map to sections; each section can be either a flat
        // object whose values are strings/objects, or a single-level object
        // that we store as-is.
        let mut prompts: HashMap<String, HashMap<String, Value>> = HashMap::new();
        for (section_key, section_val) in raw {
            match section_val {
                Value::Object(map) => {
                    let inner: HashMap<String, Value> = map.into_iter().collect();
                    prompts.insert(section_key, inner);
                }
                // If a top-level key is a plain string, wrap it in a
                // single-entry map so retrieve("key", "value") still works.
                other => {
                    let mut inner = HashMap::new();
                    inner.insert(String::new(), other);
                    prompts.insert(section_key, inner);
                }
            }
        }

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
    /// For string values, returns the string directly.
    /// For non-string values (e.g. nested objects), returns the JSON
    /// serialization.
    ///
    /// # Panics
    /// Panics if the prompt for the given kind and key is not found.
    pub fn retrieve(&self, kind: &str, key: &str) -> String {
        let value = self
            .prompts
            .get(kind)
            .and_then(|section| section.get(key))
            .unwrap_or_else(|| panic!("Prompt for '{}':'{}' not found.", kind, key));

        match value {
            Value::String(s) => s.clone(),
            other => serde_json::to_string(other).unwrap_or_default(),
        }
    }

    /// Retrieve a prompt value as a raw `serde_json::Value`.
    ///
    /// Useful for entries like `tools.add_image` that contain nested
    /// structured data rather than a single string.
    pub fn retrieve_value(&self, kind: &str, key: &str) -> Option<&Value> {
        self.prompts
            .get(kind)
            .and_then(|section| section.get(key))
    }
}

/// Global cached `I18N` instance (default prompts).
static DEFAULT_I18N: OnceLock<I18N> = OnceLock::new();

/// Get the global cached `I18N` instance using the default embedded prompts.
pub fn get_i18n() -> &'static I18N {
    DEFAULT_I18N.get_or_init(|| I18N::new(None))
}
