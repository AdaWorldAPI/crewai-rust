//! Translation strings for CrewAI prompts and messages.
//!
//! Corresponds to `crewai/translations/`.
//!
//! Loads and provides access to localized prompt templates, error messages,
//! and agent instruction strings. The default language is English, loaded
//! from the embedded `en.json` translation file.
//!
//! The translation data is organized into sections:
//! - `hierarchical_manager_agent`: Default manager agent configuration
//! - `slices`: Prompt template fragments (role_playing, tools, tasks, etc.)
//! - `errors`: Error message templates

use serde_json::Value;

/// Default English translations embedded at compile time.
///
/// This is loaded from `crewai/translations/en.json` in the Python source.
/// In Rust, we embed the JSON string and parse it at runtime.
/// Raw English translation JSON string, embedded at compile time.
///
/// Used by [`crate::utilities::i18n::I18N`] to load default prompts.
pub const EN_JSON: &str = include_str!("en.json");

/// Translation store providing access to localized strings.
///
/// Loads translations from JSON files and provides typed access to
/// prompt templates, error messages, and configuration strings.
#[derive(Debug, Clone)]
pub struct Translations {
    /// The parsed translation data.
    data: Value,
}

impl Translations {
    /// Load the default English translations.
    pub fn load_default() -> Self {
        let data = serde_json::from_str(EN_JSON)
            .expect("Failed to parse embedded en.json translations");
        Self { data }
    }

    /// Load translations from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, String> {
        let data = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse translations JSON: {}", e))?;
        Ok(Self { data })
    }

    /// Get a translation value by dotted path (e.g., "slices.observation").
    pub fn get(&self, path: &str) -> Option<&Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &self.data;
        for part in parts {
            current = current.get(part)?;
        }
        Some(current)
    }

    /// Get a translation string by dotted path.
    pub fn get_str(&self, path: &str) -> Option<&str> {
        self.get(path).and_then(|v| v.as_str())
    }

    /// Get a slice (prompt template fragment) by name.
    ///
    /// Looks up `slices.<name>` in the translation data.
    pub fn slice(&self, name: &str) -> Option<&str> {
        self.get_str(&format!("slices.{}", name))
    }

    /// Get an error message template by name.
    ///
    /// Looks up `errors.<name>` in the translation data.
    pub fn error(&self, name: &str) -> Option<&str> {
        self.get_str(&format!("errors.{}", name))
    }

    /// Get the hierarchical manager agent configuration.
    pub fn manager_agent(&self) -> Option<&Value> {
        self.get("hierarchical_manager_agent")
    }

    /// Get the full raw translation data.
    pub fn raw(&self) -> &Value {
        &self.data
    }
}

impl Default for Translations {
    fn default() -> Self {
        Self::load_default()
    }
}
