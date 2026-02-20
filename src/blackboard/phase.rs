//! Phase-based borrow discipline for the blackboard.
//!
//! Execution flows through typed phases. During each phase, exactly one
//! subsystem holds `&mut Blackboard`. The `Phase` type provides scoped
//! access with automatic trace logging.
//!
//! # Phase Ordering
//!
//! A typical openclaw execution flows:
//!
//! 1. `channel.receive` — inbound message arrives
//! 2. `memory.recall` — fetch relevant context
//! 3. `agent.think` — LLM inference
//! 4. `agent.tool` — tool execution (optional, may loop)
//! 5. `memory.store` — persist new knowledge
//! 6. `channel.send` — deliver response
//!
//! Rust's borrow checker guarantees that phases cannot overlap.

use super::view::Blackboard;

/// A scoped phase that provides `&mut Blackboard` access.
///
/// When dropped, records the phase completion in the blackboard trace.
/// This provides automatic audit logging of which subsystem touched
/// the blackboard and when.
///
/// # Example
///
/// ```
/// use crewai::blackboard::{Blackboard, Phase};
///
/// let mut bb = Blackboard::new();
///
/// {
///     let mut phase = Phase::begin(&mut bb, "channel.receive");
///     phase.bb().put("msg:0", serde_json::json!({"text": "hi"}), "discord", "oc.channel.receive");
/// } // phase dropped — trace entry recorded
///
/// {
///     let mut phase = Phase::begin(&mut bb, "agent.think");
///     let _msg = phase.bb().get_value("msg:0");
///     phase.bb().put("resp:1", serde_json::json!({"text": "hello!"}), "agent", "oc.agent.think");
/// }
///
/// assert_eq!(bb.trace().len(), 6); // 2 data entries + 2 phase starts + 2 phase ends
/// ```
pub struct Phase<'a> {
    blackboard: &'a mut Blackboard,
    name: String,
    start_epoch: i64,
}

impl<'a> Phase<'a> {
    /// Begin a named phase, taking `&mut` access to the blackboard.
    pub fn begin(blackboard: &'a mut Blackboard, name: impl Into<String>) -> Self {
        let name = name.into();
        let start_epoch = chrono::Utc::now().timestamp_millis();

        // Record phase start in the trace
        blackboard.record_phase_start(&name);

        Self {
            blackboard,
            name,
            start_epoch,
        }
    }

    /// Get mutable access to the blackboard within this phase.
    pub fn bb(&mut self) -> &mut Blackboard {
        self.blackboard
    }

    /// Get immutable access to the blackboard within this phase.
    pub fn bb_ref(&self) -> &Blackboard {
        self.blackboard
    }

    /// Get the phase name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the phase start epoch.
    pub fn start_epoch(&self) -> i64 {
        self.start_epoch
    }

    /// Get elapsed time in milliseconds since phase started.
    pub fn elapsed_ms(&self) -> i64 {
        chrono::Utc::now().timestamp_millis() - self.start_epoch
    }
}

impl<'a> Drop for Phase<'a> {
    fn drop(&mut self) {
        let elapsed = self.elapsed_ms();
        self.blackboard.record_phase_end(&self.name, elapsed);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_basic() {
        let mut bb = Blackboard::new();

        {
            let mut phase = Phase::begin(&mut bb, "channel.receive");
            assert_eq!(phase.name(), "channel.receive");
            phase.bb().put(
                "msg:0",
                serde_json::json!({"text": "hello"}),
                "discord",
                "oc.channel.receive",
            );
        }

        assert!(bb.contains("msg:0"));
        // trace should have: phase_start marker, data entry, phase_end marker
        assert!(bb.trace().len() >= 2);
    }

    #[test]
    fn test_phase_sequential() {
        let mut bb = Blackboard::new();

        {
            let mut phase = Phase::begin(&mut bb, "phase_a");
            phase.bb().put("a:0", serde_json::json!(1), "s", "step.a");
        }

        {
            let mut phase = Phase::begin(&mut bb, "phase_b");
            // Can read what phase_a wrote
            let val = phase.bb().get_value("a:0");
            assert!(val.is_some());
            phase.bb().put("b:0", serde_json::json!(2), "s", "step.b");
        }

        assert_eq!(bb.len(), 2);
    }

    #[test]
    fn test_phase_immutable_access() {
        let mut bb = Blackboard::new();
        bb.put("pre:0", serde_json::json!("existing"), "s", "t");

        {
            let phase = Phase::begin(&mut bb, "read_only");
            assert!(phase.bb_ref().contains("pre:0"));
            assert_eq!(phase.bb_ref().len(), 1);
        }
    }
}
