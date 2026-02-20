//! Typed blackboard slot — zero-serialization in-process data sharing.
//!
//! When all crates compile into one binary, subsystems can share Rust
//! objects directly through `Box<dyn Any + Send + Sync>` without any
//! serde overhead. This module provides the typed slot abstraction.

use std::any::{Any, TypeId};

use super::slot::SlotMeta;

/// A typed slot that holds any `Send + Sync + 'static` value.
///
/// Unlike [`BlackboardSlot`](super::slot::BlackboardSlot) which stores
/// bytes and optional JSON, `TypedSlot` stores native Rust values.
/// This enables zero-copy, zero-serde data sharing between subsystems
/// compiled into the same binary.
///
/// # Example
///
/// ```
/// use crewai::blackboard::TypedSlot;
///
/// #[derive(Debug)]
/// struct AgentResponse { text: String, confidence: f64 }
///
/// let slot = TypedSlot::new(
///     AgentResponse { text: "Hello".into(), confidence: 0.95 },
///     "agent", "oc.agent.think",
/// );
///
/// let resp = slot.downcast_ref::<AgentResponse>().unwrap();
/// assert_eq!(resp.text, "Hello");
/// ```
pub struct TypedSlot {
    /// The stored value (type-erased).
    value: Box<dyn Any + Send + Sync>,
    /// The TypeId for fast type checking.
    type_id: TypeId,
    /// Slot metadata (same as BlackboardSlot).
    pub meta: SlotMeta,
}

impl TypedSlot {
    /// Create a new typed slot.
    pub fn new<T: Any + Send + Sync>(
        value: T,
        source: impl Into<String>,
        step_type: impl Into<String>,
    ) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            value: Box::new(value),
            meta: SlotMeta {
                source: source.into(),
                step_type: step_type.into(),
                epoch: chrono::Utc::now().timestamp_millis(),
                confidence: 1.0,
            },
        }
    }

    /// Check if this slot holds a value of type `T`.
    pub fn is<T: Any>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    /// Try to get a reference to the stored value as type `T`.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.value.downcast_ref::<T>()
    }

    /// Try to get a mutable reference to the stored value as type `T`.
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.value.downcast_mut::<T>()
    }

    /// Consume the slot and try to extract the value as type `T`.
    pub fn downcast<T: Any + Send + Sync>(self) -> Result<T, Self> {
        if self.is::<T>() {
            // Safety: we just checked the type
            let boxed = self.value.downcast::<T>().unwrap();
            Ok(*boxed)
        } else {
            Err(Self {
                value: self.value,
                type_id: self.type_id,
                meta: self.meta,
            })
        }
    }

    /// Get the type name for debugging.
    pub fn type_name(&self) -> &'static str {
        // TypeId doesn't expose the name, but we can use the value's Any impl
        self.value.as_ref().type_id();
        // Use std::any::type_name if we store it
        "dyn Any + Send + Sync"
    }

    /// Set confidence on this slot (builder pattern).
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.meta.confidence = confidence;
        self
    }

    /// Set epoch on this slot (builder pattern).
    pub fn with_epoch(mut self, epoch: i64) -> Self {
        self.meta.epoch = epoch;
        self
    }
}

impl std::fmt::Debug for TypedSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedSlot")
            .field("type_id", &self.type_id)
            .field("meta", &self.meta)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct TestMessage {
        text: String,
        score: f64,
    }

    #[derive(Debug, PartialEq)]
    struct OtherType(i32);

    #[test]
    fn test_typed_slot_new_and_downcast() {
        let slot = TypedSlot::new(
            TestMessage { text: "hello".into(), score: 0.9 },
            "agent",
            "oc.agent.think",
        );

        assert!(slot.is::<TestMessage>());
        assert!(!slot.is::<OtherType>());

        let msg = slot.downcast_ref::<TestMessage>().unwrap();
        assert_eq!(msg.text, "hello");
        assert_eq!(msg.score, 0.9);
    }

    #[test]
    fn test_typed_slot_downcast_mut() {
        let mut slot = TypedSlot::new(
            TestMessage { text: "hello".into(), score: 0.5 },
            "agent",
            "oc.agent.think",
        );

        let msg = slot.downcast_mut::<TestMessage>().unwrap();
        msg.score = 0.99;

        assert_eq!(slot.downcast_ref::<TestMessage>().unwrap().score, 0.99);
    }

    #[test]
    fn test_typed_slot_consume() {
        let slot = TypedSlot::new(
            TestMessage { text: "consumed".into(), score: 1.0 },
            "agent",
            "oc.agent.think",
        );

        let msg = slot.downcast::<TestMessage>().unwrap();
        assert_eq!(msg.text, "consumed");
    }

    #[test]
    fn test_typed_slot_wrong_type() {
        let slot = TypedSlot::new(42i32, "test", "test.step");
        assert!(slot.downcast_ref::<String>().is_none());
        assert!(slot.is::<i32>());
    }

    #[test]
    fn test_typed_slot_consume_wrong_type() {
        let slot = TypedSlot::new(42i32, "test", "test.step");
        let err = slot.downcast::<String>();
        assert!(err.is_err());
        // The slot is returned back on error
        let slot = err.unwrap_err();
        assert!(slot.is::<i32>());
    }

    #[test]
    fn test_typed_slot_confidence() {
        let slot = TypedSlot::new("data", "test", "oc.memory.recall")
            .with_confidence(0.75);
        assert_eq!(slot.meta.confidence, 0.75);
    }

    #[test]
    fn test_typed_slot_with_vec() {
        let data: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        let slot = TypedSlot::new(data, "test", "crew.step");

        let v = slot.downcast_ref::<Vec<String>>().unwrap();
        assert_eq!(v.len(), 3);
    }
}
