//! Guardrail type aliases.
//!
//! Corresponds to `crewai/utilities/guardrail_types.py`.

use serde_json::Value;

/// Result type for a guardrail invocation.
///
/// The tuple contains `(pass: bool, message: Option<String>)`.
pub type GuardrailResult = (bool, Option<String>);

/// A guardrail callable: receives the task output value, returns pass/fail.
pub type GuardrailCallable = Box<dyn Fn(&Value) -> GuardrailResult + Send + Sync>;

/// A single guardrail: either a callable or a named guardrail reference.
pub enum GuardrailType {
    /// A runtime guardrail function.
    Callable(GuardrailCallable),
    /// A named/registered guardrail.
    Named(String),
}

/// A collection of guardrails to apply.
pub type GuardrailsType = Vec<GuardrailType>;
