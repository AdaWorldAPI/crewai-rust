//! Console printer utility with color support.
//!
//! Corresponds to `crewai/utilities/printer.py`.

use serde::{Deserialize, Serialize};

/// Available colors for printed output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrinterColor {
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BoldRed,
    BoldGreen,
    BoldYellow,
    BoldBlue,
    BoldMagenta,
    BoldCyan,
    BoldWhite,
    Purple,
    BoldPurple,
}

impl PrinterColor {
    /// ANSI escape code for this color.
    fn ansi_code(&self) -> &'static str {
        match self {
            Self::Red => "\x1b[31m",
            Self::Green => "\x1b[32m",
            Self::Yellow => "\x1b[33m",
            Self::Blue => "\x1b[34m",
            Self::Magenta => "\x1b[35m",
            Self::Cyan => "\x1b[36m",
            Self::White => "\x1b[37m",
            Self::BoldRed => "\x1b[1;31m",
            Self::BoldGreen => "\x1b[1;32m",
            Self::BoldYellow => "\x1b[1;33m",
            Self::BoldBlue => "\x1b[1;34m",
            Self::BoldMagenta => "\x1b[1;35m",
            Self::BoldCyan => "\x1b[1;36m",
            Self::BoldWhite => "\x1b[1;37m",
            Self::Purple => "\x1b[35m",
            Self::BoldPurple => "\x1b[1;35m",
        }
    }
}

/// ANSI reset code.
const RESET: &str = "\x1b[0m";

/// A piece of colored text.
pub struct ColoredText {
    pub text: String,
    pub color: PrinterColor,
}

impl ColoredText {
    pub fn new(text: impl Into<String>, color: PrinterColor) -> Self {
        Self {
            text: text.into(),
            color,
        }
    }
}

/// Printer for console output with color support.
#[derive(Debug, Clone, Default)]
pub struct Printer;

impl Printer {
    /// Create a new `Printer`.
    pub fn new() -> Self {
        Self
    }

    /// Print a message with the specified color.
    pub fn print(&self, content: &str, color: PrinterColor) {
        println!("{}{}{}", color.ansi_code(), content, RESET);
    }

    /// Print multiple colored text segments on a single line.
    pub fn print_colored(&self, segments: &[ColoredText]) {
        let mut line = String::new();
        for segment in segments {
            line.push_str(segment.color.ansi_code());
            line.push_str(&segment.text);
            line.push_str(RESET);
        }
        println!("{}", line);
    }
}
