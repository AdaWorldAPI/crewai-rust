//! Blackboard — the central shared-state structure.
//!
//! The blackboard holds all data flowing through a unified execution.
//! Subsystems access it via phase methods that borrow `&mut self`,
//! ensuring only one system writes at a time.

use std::any::Any;
use std::collections::HashMap;

use serde_json::Value;

use super::a2a::A2ARegistry;
use super::slot::{BlackboardSlot, SlotMeta};
use super::typed_slot::TypedSlot;
use crate::contract::types::{DataEnvelope, EnvelopeMetadata};

/// The central blackboard for multi-system execution.
///
/// Each slot is keyed by a string identifier (typically `{step_type}:{sequence}`).
/// Subsystems borrow the blackboard mutably during their processing phase.
///
/// # Example
///
/// ```
/// use crewai::blackboard::Blackboard;
///
/// let mut bb = Blackboard::new();
///
/// // Channel phase: write inbound message
/// bb.put("oc.channel.receive:0", serde_json::json!({"text": "hello"}), "channel", "oc.channel.receive");
///
/// // Agent phase: read message, write response
/// let msg = bb.get_value("oc.channel.receive:0");
/// bb.put("oc.agent.think:1", serde_json::json!({"response": "hi!"}), "agent", "oc.agent.think");
/// ```
#[derive(Debug)]
pub struct Blackboard {
    /// Named slots holding execution data (bytes/JSON payloads).
    slots: HashMap<String, BlackboardSlot>,
    /// Typed slots holding native Rust values (zero-serde).
    typed_slots: HashMap<String, TypedSlot>,
    /// A2A awareness registry — agent discovery and coordination.
    pub a2a: A2ARegistry,
    /// Execution trace (step keys in order of insertion).
    trace: Vec<String>,
}

impl Default for Blackboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Blackboard {
    /// Create an empty blackboard.
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
            typed_slots: HashMap::new(),
            a2a: A2ARegistry::new(),
            trace: Vec::new(),
        }
    }

    /// Create a blackboard with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: HashMap::with_capacity(capacity),
            typed_slots: HashMap::with_capacity(capacity),
            a2a: A2ARegistry::new(),
            trace: Vec::with_capacity(capacity),
        }
    }

    // --- Write operations ---

    /// Put a JSON value into a named slot.
    pub fn put(
        &mut self,
        key: impl Into<String>,
        value: Value,
        source: impl Into<String>,
        step_type: impl Into<String>,
    ) {
        let key = key.into();
        let slot = BlackboardSlot::from_value(value, source, step_type);
        self.trace.push(key.clone());
        self.slots.insert(key, slot);
    }

    /// Put raw bytes into a named slot.
    pub fn put_bytes(
        &mut self,
        key: impl Into<String>,
        payload: bytes::Bytes,
        source: impl Into<String>,
        step_type: impl Into<String>,
    ) {
        let key = key.into();
        let slot = BlackboardSlot::from_bytes(payload, source, step_type);
        self.trace.push(key.clone());
        self.slots.insert(key, slot);
    }

    /// Put a pre-built slot into the blackboard.
    pub fn put_slot(&mut self, key: impl Into<String>, slot: BlackboardSlot) {
        let key = key.into();
        self.trace.push(key.clone());
        self.slots.insert(key, slot);
    }

    // --- Read operations ---

    /// Get a slot by key.
    pub fn get(&self, key: &str) -> Option<&BlackboardSlot> {
        self.slots.get(key)
    }

    /// Get a mutable slot by key.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut BlackboardSlot> {
        self.slots.get_mut(key)
    }

    /// Get the structured value from a slot (parsing from bytes if needed).
    pub fn get_value(&mut self, key: &str) -> Option<&Value> {
        self.slots.get_mut(key).and_then(|slot| slot.as_value())
    }

    /// Check if a key exists.
    pub fn contains(&self, key: &str) -> bool {
        self.slots.contains_key(key)
    }

    /// Get the number of slots.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Check if the blackboard is empty.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    // --- Trace operations ---

    /// Get the execution trace (ordered list of slot keys).
    pub fn trace(&self) -> &[String] {
        &self.trace
    }

    /// Get the last N entries from the trace.
    pub fn recent_trace(&self, n: usize) -> &[String] {
        let start = self.trace.len().saturating_sub(n);
        &self.trace[start..]
    }

    // --- Query operations ---

    /// Find all slots matching a step_type prefix.
    pub fn slots_by_prefix(&self, prefix: &str) -> Vec<(&str, &BlackboardSlot)> {
        self.slots
            .iter()
            .filter(|(_, slot)| slot.meta.step_type.starts_with(prefix))
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Get the most recent slot matching a step_type prefix (by epoch).
    pub fn latest_by_prefix(&self, prefix: &str) -> Option<(&str, &BlackboardSlot)> {
        self.slots
            .iter()
            .filter(|(_, slot)| slot.meta.step_type.starts_with(prefix))
            .max_by_key(|(_, slot)| slot.meta.epoch)
            .map(|(k, v)| (k.as_str(), v))
    }

    // --- Conversion ---

    /// Convert a blackboard slot to a DataEnvelope (for cross-system routing).
    pub fn to_envelope(&mut self, key: &str) -> Option<DataEnvelope> {
        let slot = self.slots.get_mut(key)?;
        let data = slot.as_value()?.clone();
        Some(DataEnvelope {
            data,
            metadata: EnvelopeMetadata {
                source_step: slot.meta.step_type.clone(),
                confidence: slot.meta.confidence,
                epoch: slot.meta.epoch,
                version: None,
                dominant_layer: None,
                layer_activations: None,
                nars_frequency: None,
                calibration_error: None,
            },
        })
    }

    /// Import a DataEnvelope into the blackboard.
    pub fn from_envelope(
        &mut self,
        key: impl Into<String>,
        envelope: &DataEnvelope,
    ) {
        let slot = BlackboardSlot::from_value(
            envelope.data.clone(),
            &envelope.metadata.source_step,
            &envelope.metadata.source_step,
        )
        .with_confidence(envelope.metadata.confidence);
        self.put_slot(key, slot);
    }

    // --- Typed slots (zero-serde, in-process only) ---

    /// Put a typed value into the blackboard (no serialization).
    ///
    /// This is the preferred way to share data between subsystems
    /// compiled into the same binary. No serde overhead.
    pub fn put_typed<T: Any + Send + Sync>(
        &mut self,
        key: impl Into<String>,
        value: T,
        source: impl Into<String>,
        step_type: impl Into<String>,
    ) {
        let key = key.into();
        let slot = TypedSlot::new(value, source, step_type);
        self.trace.push(key.clone());
        self.typed_slots.insert(key, slot);
    }

    /// Put a pre-built typed slot.
    pub fn put_typed_slot(&mut self, key: impl Into<String>, slot: TypedSlot) {
        let key = key.into();
        self.trace.push(key.clone());
        self.typed_slots.insert(key, slot);
    }

    /// Get a typed value by key.
    pub fn get_typed<T: Any>(&self, key: &str) -> Option<&T> {
        self.typed_slots.get(key).and_then(|s| s.downcast_ref::<T>())
    }

    /// Get a mutable typed value by key.
    pub fn get_typed_mut<T: Any>(&mut self, key: &str) -> Option<&mut T> {
        self.typed_slots
            .get_mut(key)
            .and_then(|s| s.downcast_mut::<T>())
    }

    /// Get a typed slot by key (for metadata access).
    pub fn get_typed_slot(&self, key: &str) -> Option<&TypedSlot> {
        self.typed_slots.get(key)
    }

    /// Remove a typed slot and try to extract the value.
    pub fn take_typed<T: Any + Send + Sync>(&mut self, key: &str) -> Option<T> {
        self.typed_slots
            .remove(key)
            .and_then(|s| s.downcast::<T>().ok())
    }

    /// Check if a key exists in either slot map.
    pub fn contains_any(&self, key: &str) -> bool {
        self.slots.contains_key(key) || self.typed_slots.contains_key(key)
    }

    /// Total number of slots (bytes + typed).
    pub fn total_len(&self) -> usize {
        self.slots.len() + self.typed_slots.len()
    }

    // --- Phase recording (used by Phase<'a>) ---

    /// Record a phase start in the trace.
    pub(crate) fn record_phase_start(&mut self, phase_name: &str) {
        self.trace.push(format!(">>phase:{}", phase_name));
    }

    /// Record a phase end in the trace.
    pub(crate) fn record_phase_end(&mut self, phase_name: &str, elapsed_ms: i64) {
        self.trace
            .push(format!("<<phase:{}:{}ms", phase_name, elapsed_ms));
    }

    // --- Cleanup ---

    /// Remove a slot by key.
    pub fn remove(&mut self, key: &str) -> Option<BlackboardSlot> {
        self.slots.remove(key)
    }

    /// Remove a typed slot by key.
    pub fn remove_typed(&mut self, key: &str) -> Option<TypedSlot> {
        self.typed_slots.remove(key)
    }

    /// Clear all slots and trace.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.typed_slots.clear();
        self.trace.clear();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blackboard_new() {
        let bb = Blackboard::new();
        assert!(bb.is_empty());
        assert_eq!(bb.len(), 0);
    }

    #[test]
    fn test_blackboard_put_get() {
        let mut bb = Blackboard::new();
        bb.put("step:0", serde_json::json!({"msg": "hello"}), "channel", "oc.channel.receive");

        assert!(bb.contains("step:0"));
        assert_eq!(bb.len(), 1);

        let value = bb.get_value("step:0");
        assert!(value.is_some());
        assert_eq!(value.unwrap()["msg"], "hello");
    }

    #[test]
    fn test_blackboard_put_bytes() {
        let mut bb = Blackboard::new();
        bb.put_bytes("raw:0", bytes::Bytes::from_static(b"raw data"), "test", "oc.browser.action");

        let slot = bb.get("raw:0").unwrap();
        assert_eq!(slot.as_str(), Some("raw data"));
    }

    #[test]
    fn test_blackboard_trace() {
        let mut bb = Blackboard::new();
        bb.put("a:0", serde_json::json!(1), "s", "oc.channel.receive");
        bb.put("b:1", serde_json::json!(2), "s", "oc.agent.think");
        bb.put("c:2", serde_json::json!(3), "s", "oc.channel.send");

        assert_eq!(bb.trace(), &["a:0", "b:1", "c:2"]);
        assert_eq!(bb.recent_trace(2), &["b:1", "c:2"]);
    }

    #[test]
    fn test_blackboard_slots_by_prefix() {
        let mut bb = Blackboard::new();
        bb.put("s:0", serde_json::json!(1), "ch", "oc.channel.receive");
        bb.put("s:1", serde_json::json!(2), "ag", "crew.agent");
        bb.put("s:2", serde_json::json!(3), "ch", "oc.channel.send");

        let oc_slots = bb.slots_by_prefix("oc.");
        assert_eq!(oc_slots.len(), 2);

        let crew_slots = bb.slots_by_prefix("crew.");
        assert_eq!(crew_slots.len(), 1);
    }

    #[test]
    fn test_blackboard_envelope_roundtrip() {
        let mut bb = Blackboard::new();
        bb.put("s:0", serde_json::json!({"result": "done"}), "agent", "crew.agent");

        let envelope = bb.to_envelope("s:0").unwrap();
        assert_eq!(envelope.data["result"], "done");
        assert_eq!(envelope.metadata.source_step, "crew.agent");

        bb.from_envelope("imported:0", &envelope);
        let value = bb.get_value("imported:0").unwrap();
        assert_eq!(value["result"], "done");
    }

    #[test]
    fn test_blackboard_remove_clear() {
        let mut bb = Blackboard::new();
        bb.put("a:0", serde_json::json!(1), "s", "t");
        bb.put("b:1", serde_json::json!(2), "s", "t");

        assert_eq!(bb.len(), 2);

        let removed = bb.remove("a:0");
        assert!(removed.is_some());
        assert_eq!(bb.len(), 1);

        bb.clear();
        assert!(bb.is_empty());
        assert!(bb.trace().is_empty());
    }

    #[test]
    fn test_blackboard_missing_key() {
        let mut bb = Blackboard::new();
        assert!(bb.get("missing").is_none());
        assert!(bb.get_value("missing").is_none());
        assert!(!bb.contains("missing"));
    }

    // --- Typed slot tests ---

    #[derive(Debug, PartialEq)]
    struct AgentOutput {
        text: String,
        confidence: f64,
    }

    #[test]
    fn test_blackboard_typed_put_get() {
        let mut bb = Blackboard::new();
        bb.put_typed(
            "resp:0",
            AgentOutput { text: "hello".into(), confidence: 0.95 },
            "agent",
            "oc.agent.think",
        );

        let out = bb.get_typed::<AgentOutput>("resp:0").unwrap();
        assert_eq!(out.text, "hello");
        assert_eq!(out.confidence, 0.95);
    }

    #[test]
    fn test_blackboard_typed_mut() {
        let mut bb = Blackboard::new();
        bb.put_typed("counter:0", 0u64, "test", "step");

        *bb.get_typed_mut::<u64>("counter:0").unwrap() += 1;
        assert_eq!(*bb.get_typed::<u64>("counter:0").unwrap(), 1);
    }

    #[test]
    fn test_blackboard_take_typed() {
        let mut bb = Blackboard::new();
        bb.put_typed("data:0", vec![1, 2, 3], "test", "step");

        let v = bb.take_typed::<Vec<i32>>("data:0").unwrap();
        assert_eq!(v, vec![1, 2, 3]);
        // Slot is consumed
        assert!(bb.get_typed::<Vec<i32>>("data:0").is_none());
    }

    #[test]
    fn test_blackboard_mixed_slots() {
        let mut bb = Blackboard::new();
        bb.put("json:0", serde_json::json!({"a": 1}), "s", "t");
        bb.put_typed("typed:0", 42u32, "s", "t");

        assert_eq!(bb.len(), 1);           // bytes slots
        assert_eq!(bb.total_len(), 2);     // bytes + typed
        assert!(bb.contains_any("json:0"));
        assert!(bb.contains_any("typed:0"));
    }

    // --- A2A registry tests ---

    #[test]
    fn test_blackboard_a2a_registry() {
        let mut bb = Blackboard::new();

        bb.a2a.register("agent-1", "Researcher", "research", vec!["search".into()]);
        bb.a2a.register("agent-2", "Writer", "writing", vec!["write".into()]);

        assert_eq!(bb.a2a.len(), 2);

        let searchers = bb.a2a.by_capability("search");
        assert_eq!(searchers.len(), 1);
        assert_eq!(searchers[0].name, "Researcher");
    }

    // --- Phase recording tests ---

    #[test]
    fn test_blackboard_phase_recording() {
        let mut bb = Blackboard::new();

        bb.record_phase_start("channel.receive");
        bb.put("msg:0", serde_json::json!("hi"), "ch", "oc.channel.receive");
        bb.record_phase_end("channel.receive", 5);

        let trace = bb.trace();
        assert_eq!(trace[0], ">>phase:channel.receive");
        assert_eq!(trace[1], "msg:0");
        assert!(trace[2].starts_with("<<phase:channel.receive:"));
    }
}
