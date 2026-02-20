//! Blackboard slot — a single entry in the shared blackboard.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Metadata attached to a blackboard slot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SlotMeta {
    /// Source system that wrote this slot (e.g., "oc.channel", "crew.agent", "lb").
    pub source: String,
    /// Step type that produced this data.
    pub step_type: String,
    /// Monotonic epoch for ordering.
    pub epoch: i64,
    /// Agent confidence (0.0-1.0).
    pub confidence: f64,
}

/// A single slot in the blackboard.
///
/// Holds the payload as `bytes::Bytes` for zero-copy sharing, plus an
/// optional `Value` for structured access without re-parsing.
#[derive(Debug, Clone)]
pub struct BlackboardSlot {
    /// Raw payload bytes (reference-counted, zero-copy sliceable).
    pub payload: bytes::Bytes,
    /// Structured view of the payload (lazy-parsed on demand).
    pub structured: Option<Value>,
    /// Slot metadata.
    pub meta: SlotMeta,
    /// Optional 8192-bit fingerprint (when ladybug feature is enabled).
    #[cfg(feature = "ladybug")]
    pub fingerprint: Option<ladybug_contract::Container>,
}

impl BlackboardSlot {
    /// Create a new slot from raw bytes.
    pub fn from_bytes(payload: bytes::Bytes, source: impl Into<String>, step_type: impl Into<String>) -> Self {
        Self {
            payload,
            structured: None,
            meta: SlotMeta {
                source: source.into(),
                step_type: step_type.into(),
                epoch: chrono::Utc::now().timestamp_millis(),
                confidence: 1.0,
            },
            #[cfg(feature = "ladybug")]
            fingerprint: None,
        }
    }

    /// Create a new slot from a JSON value.
    ///
    /// Serializes the value to bytes for zero-copy sharing, and keeps
    /// the structured view.
    pub fn from_value(value: Value, source: impl Into<String>, step_type: impl Into<String>) -> Self {
        let payload = bytes::Bytes::from(serde_json::to_vec(&value).unwrap_or_default());
        Self {
            payload,
            structured: Some(value),
            meta: SlotMeta {
                source: source.into(),
                step_type: step_type.into(),
                epoch: chrono::Utc::now().timestamp_millis(),
                confidence: 1.0,
            },
            #[cfg(feature = "ladybug")]
            fingerprint: None,
        }
    }

    /// Get the structured view, parsing from bytes if needed.
    pub fn as_value(&mut self) -> Option<&Value> {
        if self.structured.is_none() && !self.payload.is_empty() {
            self.structured = serde_json::from_slice(&self.payload).ok();
        }
        self.structured.as_ref()
    }

    /// Get the payload as a string slice (if valid UTF-8).
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.payload).ok()
    }

    /// Set confidence on this slot.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.meta.confidence = confidence;
        self
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_from_bytes() {
        let data = bytes::Bytes::from_static(b"hello world");
        let slot = BlackboardSlot::from_bytes(data.clone(), "test", "oc.channel.receive");
        assert_eq!(slot.payload, data);
        assert_eq!(slot.meta.source, "test");
        assert_eq!(slot.meta.step_type, "oc.channel.receive");
        assert_eq!(slot.as_str(), Some("hello world"));
    }

    #[test]
    fn test_slot_from_value() {
        let value = serde_json::json!({"key": "value"});
        let slot = BlackboardSlot::from_value(value.clone(), "test", "crew.agent");
        assert!(slot.structured.is_some());
        assert_eq!(slot.structured.unwrap(), value);
    }

    #[test]
    fn test_slot_lazy_parse() {
        let data = bytes::Bytes::from(r#"{"key": "value"}"#);
        let mut slot = BlackboardSlot::from_bytes(data, "test", "n8n.set");
        assert!(slot.structured.is_none());
        let value = slot.as_value();
        assert!(value.is_some());
        assert_eq!(value.unwrap()["key"], "value");
    }

    #[test]
    fn test_slot_confidence() {
        let data = bytes::Bytes::from_static(b"data");
        let slot = BlackboardSlot::from_bytes(data, "test", "oc.agent.think")
            .with_confidence(0.85);
        assert_eq!(slot.meta.confidence, 0.85);
    }
}
