//! Human input provider for HITL (Human-in-the-Loop) flows.
//!
//! Corresponds to `crewai/core/providers/human_input.py`.

use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Protocol for human input handling.
///
/// Implementations handle the full feedback flow:
/// - Sync: prompt user, loop until satisfied
/// - Async: raise exception for external handling
pub trait HumanInputProvider: Send + Sync {
    /// Set up messages for execution.
    ///
    /// Called before standard message setup. Allows providers to handle
    /// conversation resumption or other custom message initialization.
    ///
    /// Returns `true` if messages were set up (skip standard setup),
    /// `false` to use standard setup.
    fn setup_messages(&self) -> bool;

    /// Called after standard message setup.
    ///
    /// Allows providers to modify messages after standard setup completes.
    /// Only called when `setup_messages` returned `false`.
    fn post_setup_messages(&self);

    /// Handle the full human feedback flow.
    ///
    /// Returns the final answer string after feedback processing.
    fn handle_feedback(
        &self,
        formatted_answer: &str,
        is_training_mode: bool,
    ) -> String;
}

// ---------------------------------------------------------------------------
// Default sync implementation
// ---------------------------------------------------------------------------

/// Default synchronous human input via terminal.
pub struct SyncHumanInputProvider;

impl HumanInputProvider for SyncHumanInputProvider {
    fn setup_messages(&self) -> bool {
        // Use standard message setup.
        false
    }

    fn post_setup_messages(&self) {
        // No-op for sync provider.
    }

    fn handle_feedback(
        &self,
        formatted_answer: &str,
        is_training_mode: bool,
    ) -> String {
        let mut current_answer = formatted_answer.to_string();

        loop {
            let feedback = Self::prompt_input(is_training_mode);
            if feedback.trim().is_empty() {
                break;
            }
            // In a real implementation this would feed back to the agent loop.
            // For the port, we return the latest answer.
            current_answer = feedback;
            if is_training_mode {
                break;
            }
        }

        current_answer
    }
}

impl SyncHumanInputProvider {
    /// Show a prompt and read user input from stdin.
    fn prompt_input(is_training_mode: bool) -> String {
        if is_training_mode {
            println!(
                "\n--- Training Feedback Required ---\n\
                 Provide feedback to improve the agent's performance.\n\
                 This will be used to train better versions of the agent."
            );
        } else {
            println!(
                "\n--- Human Feedback Required ---\n\
                 Provide feedback on the result above.\n\
                 Press Enter without typing to accept the current result.\n\
                 Otherwise, provide specific improvement requests."
            );
        }

        print!("> ");
        io::stdout().flush().unwrap_or(());

        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line).unwrap_or(0);
        line.trim().to_string()
    }
}

// ---------------------------------------------------------------------------
// Context variable management
// ---------------------------------------------------------------------------

static PROVIDER: Lazy<Arc<Mutex<Option<Box<dyn HumanInputProvider>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Get the current human input provider.
///
/// Returns a reference to the provider. If none is set, sets and returns
/// the default `SyncHumanInputProvider`.
pub fn get_provider() -> Arc<Mutex<Option<Box<dyn HumanInputProvider>>>> {
    {
        let mut guard = PROVIDER.lock().unwrap();
        if guard.is_none() {
            *guard = Some(Box::new(SyncHumanInputProvider));
        }
    }
    Arc::clone(&PROVIDER)
}

/// Set the human input provider for the current context.
pub fn set_provider(provider: Box<dyn HumanInputProvider>) {
    let mut guard = PROVIDER.lock().unwrap();
    *guard = Some(provider);
}

/// Reset the provider to `None`.
pub fn reset_provider() {
    let mut guard = PROVIDER.lock().unwrap();
    *guard = None;
}
