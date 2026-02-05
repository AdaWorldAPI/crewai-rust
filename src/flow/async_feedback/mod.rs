//! Core types for async human feedback in Flows.
//!
//! Corresponds to `crewai/flow/async_feedback/`.
//!
//! Provides types and traits for requesting, collecting, and routing
//! human feedback during flow execution. Supports both synchronous
//! (console-based) and asynchronous (webhook/API-based) feedback patterns.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context capturing everything needed to resume a paused flow.
///
/// When a flow is paused waiting for async human feedback, this struct
/// stores all the information needed to:
/// 1. Identify which flow execution is waiting
/// 2. What method triggered the feedback request
/// 3. What was shown to the human
/// 4. How to route the response when it arrives
///
/// Corresponds to `crewai.flow.async_feedback.types.PendingFeedbackContext`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingFeedbackContext {
    /// Unique identifier for the flow instance (from state.id).
    pub flow_id: String,
    /// Fully qualified class name (e.g., "myapp.flows.ReviewFlow").
    pub flow_class: String,
    /// Name of the method that triggered feedback request.
    pub method_name: String,
    /// The output that was shown to the human for review.
    pub method_output: serde_json::Value,
    /// The message displayed when requesting feedback.
    pub message: String,
    /// Optional list of outcome strings for routing.
    pub emit: Option<Vec<String>>,
    /// Outcome to use when no feedback is provided.
    pub default_outcome: Option<String>,
    /// Optional metadata for external system integration.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// LLM model string for outcome collapsing.
    pub llm: Option<String>,
    /// When the feedback was requested.
    pub requested_at: DateTime<Utc>,
}

impl PendingFeedbackContext {
    /// Create a new PendingFeedbackContext.
    pub fn new(
        flow_id: String,
        flow_class: String,
        method_name: String,
        method_output: serde_json::Value,
        message: String,
    ) -> Self {
        Self {
            flow_id,
            flow_class,
            method_name,
            method_output,
            message,
            emit: None,
            default_outcome: None,
            metadata: HashMap::new(),
            llm: None,
            requested_at: Utc::now(),
        }
    }

    /// Builder: set emit options for routing.
    pub fn with_emit(mut self, emit: Vec<String>) -> Self {
        self.emit = Some(emit);
        self
    }

    /// Builder: set default outcome.
    pub fn with_default_outcome(mut self, outcome: String) -> Self {
        self.default_outcome = Some(outcome);
        self
    }

    /// Builder: set LLM model for outcome collapsing.
    pub fn with_llm(mut self, llm: String) -> Self {
        self.llm = Some(llm);
        self
    }

    /// Builder: set metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Serialize context to a dictionary for persistence.
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert(
            "flow_id".to_string(),
            serde_json::Value::String(self.flow_id.clone()),
        );
        map.insert(
            "flow_class".to_string(),
            serde_json::Value::String(self.flow_class.clone()),
        );
        map.insert(
            "method_name".to_string(),
            serde_json::Value::String(self.method_name.clone()),
        );
        map.insert("method_output".to_string(), self.method_output.clone());
        map.insert(
            "message".to_string(),
            serde_json::Value::String(self.message.clone()),
        );
        if let Some(ref emit) = self.emit {
            map.insert(
                "emit".to_string(),
                serde_json::to_value(emit).unwrap_or_default(),
            );
        }
        if let Some(ref default_outcome) = self.default_outcome {
            map.insert(
                "default_outcome".to_string(),
                serde_json::Value::String(default_outcome.clone()),
            );
        }
        map.insert(
            "metadata".to_string(),
            serde_json::to_value(&self.metadata).unwrap_or_default(),
        );
        if let Some(ref llm) = self.llm {
            map.insert(
                "llm".to_string(),
                serde_json::Value::String(llm.clone()),
            );
        }
        map.insert(
            "requested_at".to_string(),
            serde_json::Value::String(self.requested_at.to_rfc3339()),
        );
        map
    }

    /// Deserialize context from a dictionary.
    pub fn from_dict(data: &HashMap<String, serde_json::Value>) -> Result<Self, String> {
        let flow_id = data
            .get("flow_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing flow_id")?
            .to_string();
        let flow_class = data
            .get("flow_class")
            .and_then(|v| v.as_str())
            .ok_or("Missing flow_class")?
            .to_string();
        let method_name = data
            .get("method_name")
            .and_then(|v| v.as_str())
            .ok_or("Missing method_name")?
            .to_string();
        let method_output = data
            .get("method_output")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let message = data
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let emit: Option<Vec<String>> = data
            .get("emit")
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        let default_outcome = data
            .get("default_outcome")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let metadata: HashMap<String, serde_json::Value> = data
            .get("metadata")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let llm = data
            .get("llm")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let requested_at = data
            .get("requested_at")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        Ok(Self {
            flow_id,
            flow_class,
            method_name,
            method_output,
            message,
            emit,
            default_outcome,
            metadata,
            llm,
            requested_at,
        })
    }
}

/// Signal that flow execution should pause for async human feedback.
///
/// When returned by a provider, the flow framework will:
/// 1. Stop execution at the current method
/// 2. Automatically persist state and context (if persistence is configured)
/// 3. Return this object to the caller
///
/// Corresponds to `crewai.flow.async_feedback.types.HumanFeedbackPending`.
#[derive(Debug, Clone)]
pub struct HumanFeedbackPending {
    /// The PendingFeedbackContext with all details needed to resume.
    pub context: PendingFeedbackContext,
    /// Optional dict with information for external systems
    /// (e.g., webhook URL, ticket ID, Slack thread ID).
    pub callback_info: HashMap<String, serde_json::Value>,
    /// Human-readable message about the pending feedback.
    pub message: String,
}

impl HumanFeedbackPending {
    /// Create a new HumanFeedbackPending.
    pub fn new(
        context: PendingFeedbackContext,
        callback_info: Option<HashMap<String, serde_json::Value>>,
        message: Option<String>,
    ) -> Self {
        let msg = message.unwrap_or_else(|| {
            format!(
                "Human feedback pending for flow '{}' at method '{}'",
                context.flow_id, context.method_name
            )
        });
        Self {
            context,
            callback_info: callback_info.unwrap_or_default(),
            message: msg,
        }
    }
}

impl std::fmt::Display for HumanFeedbackPending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for HumanFeedbackPending {}

/// Protocol for human feedback collection strategies.
///
/// Implement this trait to create custom feedback providers that integrate
/// with external systems like Slack, Teams, email, or custom APIs.
///
/// For synchronous providers, `request_feedback` blocks and returns the
/// feedback string directly. For asynchronous providers, it returns
/// `Err(HumanFeedbackPending)` to signal the flow should pause.
///
/// Corresponds to `crewai.flow.async_feedback.types.HumanFeedbackProvider`.
pub trait HumanFeedbackProvider: Send + Sync {
    /// Request feedback from a human.
    ///
    /// For synchronous providers, block and return the feedback string.
    /// For async providers, return Err with HumanFeedbackPending info.
    ///
    /// # Arguments
    ///
    /// * `context` - The pending feedback context.
    /// * `flow_id` - The flow identifier.
    ///
    /// # Returns
    ///
    /// The human's feedback as a string, or an error for async pending.
    fn request_feedback(
        &self,
        context: &PendingFeedbackContext,
        flow_id: &str,
    ) -> Result<String, HumanFeedbackPending>;
}

/// Console-based human feedback provider for development/testing.
///
/// Prompts the user on stdout/stdin for synchronous feedback.
///
/// Corresponds to `crewai.flow.async_feedback.providers.ConsoleProvider`.
pub struct ConsoleProvider;

impl HumanFeedbackProvider for ConsoleProvider {
    fn request_feedback(
        &self,
        context: &PendingFeedbackContext,
        _flow_id: &str,
    ) -> Result<String, HumanFeedbackPending> {
        // Print the output and prompt.
        println!("\n--- Human Feedback Requested ---");
        println!("Method: {}", context.method_name);
        println!("Output: {}", context.method_output);
        println!("Message: {}", context.message);

        if let Some(ref emit) = context.emit {
            println!("Available outcomes: {:?}", emit);
        }

        print!("Your feedback: ");
        use std::io::Write;
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(_) => Ok(input.trim().to_string()),
            Err(e) => {
                log::error!("Failed to read feedback input: {}", e);
                // Fall back to default outcome.
                if let Some(ref default) = context.default_outcome {
                    Ok(default.clone())
                } else {
                    Ok(String::new())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_feedback_context_new() {
        let ctx = PendingFeedbackContext::new(
            "flow-1".to_string(),
            "TestFlow".to_string(),
            "review".to_string(),
            serde_json::json!({"text": "Hello"}),
            "Please review".to_string(),
        );
        assert_eq!(ctx.flow_id, "flow-1");
        assert_eq!(ctx.method_name, "review");
        assert!(ctx.emit.is_none());
    }

    #[test]
    fn test_pending_feedback_context_builders() {
        let ctx = PendingFeedbackContext::new(
            "flow-2".to_string(),
            "TestFlow".to_string(),
            "decide".to_string(),
            serde_json::Value::Null,
            "Choose".to_string(),
        )
        .with_emit(vec!["approve".to_string(), "reject".to_string()])
        .with_default_outcome("approve".to_string())
        .with_llm("gpt-4o-mini".to_string());

        assert_eq!(ctx.emit.as_ref().unwrap().len(), 2);
        assert_eq!(ctx.default_outcome.as_ref().unwrap(), "approve");
        assert_eq!(ctx.llm.as_ref().unwrap(), "gpt-4o-mini");
    }

    #[test]
    fn test_pending_feedback_context_roundtrip() {
        let ctx = PendingFeedbackContext::new(
            "flow-3".to_string(),
            "TestFlow".to_string(),
            "check".to_string(),
            serde_json::json!(42),
            "Is this ok?".to_string(),
        )
        .with_emit(vec!["yes".to_string(), "no".to_string()]);

        let dict = ctx.to_dict();
        let restored = PendingFeedbackContext::from_dict(&dict).unwrap();

        assert_eq!(restored.flow_id, "flow-3");
        assert_eq!(restored.method_name, "check");
        assert_eq!(restored.message, "Is this ok?");
        assert_eq!(restored.emit.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_human_feedback_pending() {
        let ctx = PendingFeedbackContext::new(
            "flow-4".to_string(),
            "TestFlow".to_string(),
            "review".to_string(),
            serde_json::Value::Null,
            "Review needed".to_string(),
        );
        let pending = HumanFeedbackPending::new(ctx, None, None);
        assert!(pending.message.contains("flow-4"));
        assert!(pending.message.contains("review"));
        assert!(pending.callback_info.is_empty());
    }

    #[test]
    fn test_human_feedback_pending_display() {
        let ctx = PendingFeedbackContext::new(
            "flow-5".to_string(),
            "TestFlow".to_string(),
            "step".to_string(),
            serde_json::Value::Null,
            "msg".to_string(),
        );
        let pending =
            HumanFeedbackPending::new(ctx, None, Some("Custom message".to_string()));
        assert_eq!(format!("{}", pending), "Custom message");
    }
}
