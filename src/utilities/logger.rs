//! Logger utility for CrewAI.
//!
//! Corresponds to `crewai/utilities/logger.py`.

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::utilities::printer::{Printer, PrinterColor};

/// Logger with optional verbose output and timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logger {
    /// Enables verbose logging with timestamps.
    pub verbose: bool,
    /// Default color for log messages.
    #[serde(default = "default_color")]
    pub default_color: PrinterColor,
    /// Internal printer (not serialized).
    #[serde(skip)]
    printer: Printer,
}

fn default_color() -> PrinterColor {
    PrinterColor::BoldYellow
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            verbose: false,
            default_color: PrinterColor::BoldYellow,
            printer: Printer::default(),
        }
    }
}

impl Logger {
    /// Create a new `Logger`.
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose,
            default_color: PrinterColor::BoldYellow,
            printer: Printer::default(),
        }
    }

    /// Log a message with timestamp if verbose mode is enabled.
    ///
    /// # Arguments
    /// * `level` - The log level (e.g., "info", "warning", "error").
    /// * `message` - The message to log.
    /// * `color` - Optional color override for the message.
    pub fn log(&self, level: &str, message: &str, color: Option<PrinterColor>) {
        if self.verbose {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let color = color.unwrap_or(self.default_color);
            let formatted = format!(
                "\n[{}][{}]: {}",
                timestamp,
                level.to_uppercase(),
                message
            );
            self.printer.print(&formatted, color);
        }
    }
}
