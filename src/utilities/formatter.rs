//! Output formatting utilities.
//!
//! Corresponds to `crewai/utilities/formatter.py`.

/// Formats output text for display, with optional truncation and wrapping.
pub struct OutputFormatter {
    /// Maximum number of characters before truncation.
    pub max_length: Option<usize>,
}

impl Default for OutputFormatter {
    fn default() -> Self {
        Self { max_length: None }
    }
}

impl OutputFormatter {
    /// Create a new `OutputFormatter` with the given max length.
    pub fn new(max_length: Option<usize>) -> Self {
        Self { max_length }
    }

    /// Format the given text, truncating if necessary.
    pub fn format(&self, text: &str) -> String {
        match self.max_length {
            Some(max_len) if text.len() > max_len => {
                format!("{}...", &text[..max_len])
            }
            _ => text.to_string(),
        }
    }
}
