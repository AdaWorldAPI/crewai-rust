//! crewAI-specific envelope conversions.
//!
//! Converts between crew execution artifacts (task outputs, memory, agent
//! results) and the unified `DataEnvelope` wire format.

use chrono::Utc;
use serde_json::Value;

use super::types::{DataEnvelope, EnvelopeMetadata};

/// Create a `DataEnvelope` from a crew task output.
///
/// Wraps the raw task output string as a JSON object with source metadata.
pub fn from_task_output(
    task_id: &str,
    output: &str,
    agent_id: Option<&str>,
    confidence: f64,
) -> DataEnvelope {
    let data = serde_json::json!({
        "task_id": task_id,
        "output": output,
        "agent_id": agent_id,
    });

    DataEnvelope {
        data,
        metadata: EnvelopeMetadata {
            source_step: task_id.to_string(),
            confidence,
            epoch: Utc::now().timestamp_millis(),
            version: Some("1.0.0".to_string()),
        },
    }
}

/// Create a `DataEnvelope` from a memory query result.
pub fn from_memory(memory_type: &str, content: &str, metadata: Value) -> DataEnvelope {
    let data = serde_json::json!({
        "memory_type": memory_type,
        "content": content,
        "metadata": metadata,
    });

    DataEnvelope {
        data,
        metadata: EnvelopeMetadata {
            source_step: format!("memory.{}", memory_type),
            confidence: 1.0,
            epoch: Utc::now().timestamp_millis(),
            version: None,
        },
    }
}

/// Create a `DataEnvelope` from a crew callback response.
///
/// Used when crewai-rust returns results to n8n-rs via the contract.
pub fn from_crew_callback(output: Value, source_step: &str, confidence: f64) -> DataEnvelope {
    DataEnvelope {
        data: output,
        metadata: EnvelopeMetadata {
            source_step: source_step.to_string(),
            confidence,
            epoch: Utc::now().timestamp_millis(),
            version: None,
        },
    }
}

/// Extract task input from a `DataEnvelope`.
///
/// If the envelope data has a `query` or `input` field, returns that.
/// Otherwise returns the entire data payload serialized as a string.
pub fn to_task_input(envelope: &DataEnvelope) -> String {
    if let Some(query) = envelope.data.get("query").and_then(|v| v.as_str()) {
        return query.to_string();
    }
    if let Some(input) = envelope.data.get("input").and_then(|v| v.as_str()) {
        return input.to_string();
    }
    if let Some(items) = envelope.data.get("items") {
        return serde_json::to_string_pretty(items).unwrap_or_default();
    }
    match &envelope.data {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_task_output() {
        let env = from_task_output("task-1", "Analysis complete", Some("agent-1"), 0.95);
        assert_eq!(env.metadata.source_step, "task-1");
        assert_eq!(env.metadata.confidence, 0.95);
        assert_eq!(env.data["task_id"], "task-1");
        assert_eq!(env.data["output"], "Analysis complete");
        assert_eq!(env.data["agent_id"], "agent-1");
    }

    #[test]
    fn test_from_task_output_no_agent() {
        let env = from_task_output("task-2", "Done", None, 1.0);
        assert!(env.data["agent_id"].is_null());
    }

    #[test]
    fn test_from_memory() {
        let meta = serde_json::json!({"source": "rag"});
        let env = from_memory("long_term", "Important fact", meta);
        assert_eq!(env.metadata.source_step, "memory.long_term");
        assert_eq!(env.data["memory_type"], "long_term");
        assert_eq!(env.data["content"], "Important fact");
    }

    #[test]
    fn test_from_crew_callback() {
        let output = serde_json::json!({"analysis": "Market is bullish"});
        let env = from_crew_callback(output, "researcher", 0.85);
        assert_eq!(env.metadata.confidence, 0.85);
        assert_eq!(env.metadata.source_step, "researcher");
    }

    #[test]
    fn test_to_task_input_query() {
        let env = DataEnvelope::new(serde_json::json!({"query": "What is Rust?"}), "trigger");
        assert_eq!(to_task_input(&env), "What is Rust?");
    }

    #[test]
    fn test_to_task_input_input_field() {
        let env = DataEnvelope::new(serde_json::json!({"input": "Process this data"}), "trigger");
        assert_eq!(to_task_input(&env), "Process this data");
    }

    #[test]
    fn test_to_task_input_items() {
        let env = DataEnvelope::new(serde_json::json!({"items": [1, 2, 3]}), "trigger");
        let result = to_task_input(&env);
        assert!(result.contains("1"));
        assert!(result.contains("3"));
    }

    #[test]
    fn test_to_task_input_string() {
        let env = DataEnvelope::new(Value::String("plain text".into()), "trigger");
        assert_eq!(to_task_input(&env), "plain text");
    }

    #[test]
    fn test_to_task_input_null() {
        let env = DataEnvelope::new(Value::Null, "trigger");
        assert_eq!(to_task_input(&env), "");
    }

    #[test]
    fn test_to_task_input_object_fallback() {
        let env = DataEnvelope::new(serde_json::json!({"complex": {"nested": true}}), "trigger");
        let result = to_task_input(&env);
        assert!(result.contains("complex"));
        assert!(result.contains("nested"));
    }

    #[test]
    fn test_envelope_roundtrip() {
        let env = from_task_output("t-1", "Result text", Some("a-1"), 0.9);
        let json = serde_json::to_string(&env).unwrap();
        let back: DataEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back.metadata.source_step, "t-1");
        let input = to_task_input(&back);
        // The output is nested in the data object, so it won't directly be "Result text"
        assert!(!input.is_empty());
    }
}
