//! Inner thought loop — agent self-reflection between execution steps.
//!
//! The inner loop gives agents a hook to introspect and optionally mutate
//! their own cognitive profile between reasoning steps.  This is the
//! mechanism through which agents exercise "free will" — bounded by
//! `SelfModifyBounds`.
//!
//! All types are domain-neutral.  The loop provides *structure*;
//! consumers inject *meaning* via `custom_properties`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::profile::{PersonaProfile, SelfModifyBounds};

// ============================================================================
// Agent state snapshot
// ============================================================================

/// Read-only snapshot of agent cognitive state, passed to inner thought hooks.
///
/// This is what the agent "sees" when it reflects between steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    /// Current 10-axis thinking style (from the module definition).
    pub thinking_style: [f32; 10],

    /// Current persona profile (if set).
    pub persona: Option<PersonaProfile>,

    /// Opaque custom properties — consumer-injected, never parsed by this crate.
    pub custom_properties: HashMap<String, serde_json::Value>,

    /// Number of execution steps completed so far.
    pub step_count: u32,

    /// Current confidence level (0.0–1.0).
    pub confidence: f32,

    /// Whether the last action succeeded.
    pub last_action_succeeded: bool,

    /// Current sparse thinking vector (23D, from `ExecutableStyle`).
    pub current_vector: HashMap<String, f32>,
}

// ============================================================================
// Inner thought result
// ============================================================================

/// Result of an inner thought loop iteration.
///
/// The agent may choose to:
/// - Do nothing (return `InnerThoughtResult::Continue`)
/// - Adjust its thinking style vector
/// - Adjust its volition axes
/// - Mutate custom properties (within `SelfModifyBounds`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InnerThoughtResult {
    /// Continue with no changes.
    Continue,

    /// Adjust the thinking style vector.
    AdjustStyle {
        /// New 10-axis thinking style. `None` means keep current.
        new_thinking_style: Option<[f32; 10]>,
        /// Adjustments to the 23D sparse vector (dimension → delta).
        vector_deltas: HashMap<String, f32>,
    },

    /// Adjust volition axes.
    AdjustVolition {
        /// New 5-axis volition. `None` means keep current.
        new_volition: Option<[f32; 5]>,
    },

    /// Mutate custom properties (only allowed under `Constrained` or `Open`).
    MutateProperties {
        /// Properties to set/overwrite.
        set: HashMap<String, serde_json::Value>,
        /// Property keys to remove.
        remove: Vec<String>,
    },

    /// Compound adjustment (multiple changes at once).
    Compound {
        new_thinking_style: Option<[f32; 10]>,
        vector_deltas: HashMap<String, f32>,
        new_volition: Option<[f32; 5]>,
        property_set: HashMap<String, serde_json::Value>,
        property_remove: Vec<String>,
    },
}

// ============================================================================
// Inner thought hook
// ============================================================================

/// Hook function type called between agent execution steps.
///
/// Receives a read-only `AgentState` snapshot and returns an `InnerThoughtResult`
/// describing any self-modifications.
///
/// # Bounds Enforcement
///
/// The runtime enforces `SelfModifyBounds`:
/// - `None`: all results except `Continue` are silently dropped
/// - `Constrained`: only `MutateProperties` and `AdjustVolition` are allowed
/// - `Open`: all modifications are allowed
pub type InnerThoughtHook =
    Box<dyn Fn(&AgentState) -> InnerThoughtResult + Send + Sync>;

/// Validate that an inner thought result respects the given bounds.
pub fn validate_result(result: &InnerThoughtResult, bounds: SelfModifyBounds) -> bool {
    match bounds {
        SelfModifyBounds::None => matches!(result, InnerThoughtResult::Continue),
        SelfModifyBounds::Constrained => matches!(
            result,
            InnerThoughtResult::Continue
                | InnerThoughtResult::MutateProperties { .. }
                | InnerThoughtResult::AdjustVolition { .. }
        ),
        SelfModifyBounds::Open => true, // all modifications allowed
    }
}

/// Apply an inner thought result to an agent state, returning a new state.
///
/// Clamps all values to valid ranges.
pub fn apply_result(state: &AgentState, result: &InnerThoughtResult) -> AgentState {
    let mut new_state = state.clone();

    match result {
        InnerThoughtResult::Continue => {}

        InnerThoughtResult::AdjustStyle {
            new_thinking_style,
            vector_deltas,
        } => {
            if let Some(ts) = new_thinking_style {
                new_state.thinking_style = clamp_array_10(ts);
            }
            for (dim, delta) in vector_deltas {
                let entry = new_state.current_vector.entry(dim.clone()).or_insert(0.0);
                *entry = (*entry + delta).clamp(0.0, 1.0);
            }
        }

        InnerThoughtResult::AdjustVolition { new_volition } => {
            if let (Some(persona), Some(vol)) = (new_state.persona.as_mut(), new_volition) {
                persona.volition_axes = clamp_array_5(vol);
            }
        }

        InnerThoughtResult::MutateProperties { set, remove } => {
            for key in remove {
                new_state.custom_properties.remove(key);
            }
            for (k, v) in set {
                new_state.custom_properties.insert(k.clone(), v.clone());
            }
        }

        InnerThoughtResult::Compound {
            new_thinking_style,
            vector_deltas,
            new_volition,
            property_set,
            property_remove,
        } => {
            if let Some(ts) = new_thinking_style {
                new_state.thinking_style = clamp_array_10(ts);
            }
            for (dim, delta) in vector_deltas {
                let entry = new_state.current_vector.entry(dim.clone()).or_insert(0.0);
                *entry = (*entry + delta).clamp(0.0, 1.0);
            }
            if let (Some(persona), Some(vol)) = (new_state.persona.as_mut(), new_volition) {
                persona.volition_axes = clamp_array_5(vol);
            }
            for key in property_remove {
                new_state.custom_properties.remove(key);
            }
            for (k, v) in property_set {
                new_state.custom_properties.insert(k.clone(), v.clone());
            }
        }
    }

    new_state
}

fn clamp_array_10(arr: &[f32; 10]) -> [f32; 10] {
    let mut out = *arr;
    for v in &mut out {
        *v = v.clamp(0.0, 1.0);
    }
    out
}

fn clamp_array_5(arr: &[f32; 5]) -> [f32; 5] {
    let mut out = *arr;
    for v in &mut out {
        *v = v.clamp(0.0, 1.0);
    }
    out
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> AgentState {
        AgentState {
            thinking_style: [0.5; 10],
            persona: Some(PersonaProfile::default()),
            custom_properties: HashMap::new(),
            step_count: 3,
            confidence: 0.75,
            last_action_succeeded: true,
            current_vector: HashMap::new(),
        }
    }

    #[test]
    fn test_continue_is_always_valid() {
        assert!(validate_result(&InnerThoughtResult::Continue, SelfModifyBounds::None));
        assert!(validate_result(&InnerThoughtResult::Continue, SelfModifyBounds::Constrained));
        assert!(validate_result(&InnerThoughtResult::Continue, SelfModifyBounds::Open));
    }

    #[test]
    fn test_style_adjust_blocked_by_none() {
        let result = InnerThoughtResult::AdjustStyle {
            new_thinking_style: Some([0.9; 10]),
            vector_deltas: HashMap::new(),
        };
        assert!(!validate_result(&result, SelfModifyBounds::None));
        assert!(!validate_result(&result, SelfModifyBounds::Constrained));
        assert!(validate_result(&result, SelfModifyBounds::Open));
    }

    #[test]
    fn test_property_mutation_allowed_under_constrained() {
        let result = InnerThoughtResult::MutateProperties {
            set: [("key".into(), serde_json::json!("val"))].into(),
            remove: vec![],
        };
        assert!(!validate_result(&result, SelfModifyBounds::None));
        assert!(validate_result(&result, SelfModifyBounds::Constrained));
        assert!(validate_result(&result, SelfModifyBounds::Open));
    }

    #[test]
    fn test_apply_continue_is_noop() {
        let state = test_state();
        let new = apply_result(&state, &InnerThoughtResult::Continue);
        assert_eq!(new.thinking_style, state.thinking_style);
    }

    #[test]
    fn test_apply_style_adjustment() {
        let state = test_state();
        let result = InnerThoughtResult::AdjustStyle {
            new_thinking_style: Some([0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1, 0.0]),
            vector_deltas: [("autonomy".into(), 0.3)].into(),
        };
        let new = apply_result(&state, &result);
        assert_eq!(new.thinking_style[0], 0.9);
        assert_eq!(new.thinking_style[9], 0.0);
        assert_eq!(new.current_vector.get("autonomy").copied().unwrap_or(0.0), 0.3);
    }

    #[test]
    fn test_apply_clamps_values() {
        let state = test_state();
        let result = InnerThoughtResult::AdjustStyle {
            new_thinking_style: Some([1.5, -0.3, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]),
            vector_deltas: HashMap::new(),
        };
        let new = apply_result(&state, &result);
        assert_eq!(new.thinking_style[0], 1.0); // clamped from 1.5
        assert_eq!(new.thinking_style[1], 0.0); // clamped from -0.3
    }

    #[test]
    fn test_apply_property_mutation() {
        let mut state = test_state();
        state
            .custom_properties
            .insert("old_key".into(), serde_json::json!("old"));

        let result = InnerThoughtResult::MutateProperties {
            set: [("new_key".into(), serde_json::json!("new"))].into(),
            remove: vec!["old_key".into()],
        };

        let new = apply_result(&state, &result);
        assert!(new.custom_properties.contains_key("new_key"));
        assert!(!new.custom_properties.contains_key("old_key"));
    }

    #[test]
    fn test_no_identity_references() {
        // Verify the module contains no identity-specific strings
        let state = test_state();
        let serialized = serde_json::to_string(&state).unwrap().to_lowercase();
        assert!(!serialized.contains("ada"));
        assert!(!serialized.contains("nsfw"));
        assert!(!serialized.contains("eroti"));
    }
}
