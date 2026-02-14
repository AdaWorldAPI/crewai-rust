//! Persona profile utilities — axis names, defaults, and validation.
//!
//! The canonical `PersonaProfile` and `SelfModifyBounds` types live in
//! [`crate::modules::module_def`].  This module re-exports them and adds
//! neutral axis name constants and convenience helpers.

// Re-export the canonical types from module_def
pub use crate::modules::module_def::{PersonaProfile, SelfModifyBounds};

/// Names for the 5 volition axes (neutral labels).
///
/// PR #9 defines these as `[curiosity, autonomy, persistence, caution, empathy]`.
/// These constants provide the canonical names for programmatic access.
pub const VOLITION_AXIS_NAMES: [&str; 5] = [
    "curiosity",   // drive to explore and learn
    "autonomy",    // self-directed agency
    "persistence", // drive to continue despite obstacles
    "caution",     // risk awareness and safety orientation
    "empathy",     // orientation toward collaboration
];

/// Names for the 8 affect baseline dimensions (Plutchik's wheel, neutral labels).
pub const AFFECT_DIMENSION_NAMES: [&str; 8] = [
    "joy",           // positive valence
    "trust",         // openness to others
    "fear",          // threat awareness
    "surprise",      // novelty response
    "sadness",       // loss awareness
    "disgust",       // rejection response
    "anger",         // boundary enforcement
    "anticipation",  // forward orientation
];

/// Create a default (neutral midpoint) persona profile.
pub fn default_persona() -> PersonaProfile {
    PersonaProfile {
        volition_axes: [0.5, 0.5, 0.5, 0.5, 0.5],
        inner_loop: false,
        self_modify: SelfModifyBounds::None,
        affect_baseline: None,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_persona_is_neutral() {
        let p = default_persona();
        assert_eq!(p.volition_axes, [0.5, 0.5, 0.5, 0.5, 0.5]);
        assert!(!p.inner_loop);
        assert_eq!(p.self_modify, SelfModifyBounds::None);
        assert!(p.affect_baseline.is_none());
    }

    #[test]
    fn test_persona_yaml_roundtrip() {
        let yaml = r#"
volition_axes: [0.9, 0.7, 0.8, 0.6, 0.95]
inner_loop: true
self_modify: constrained
affect_baseline: [0.5, 0.3, 0.7, 0.4, 0.6, 0.5, 0.3, 0.8]
"#;
        let p: PersonaProfile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(p.volition_axes[0], 0.9);
        assert!(p.inner_loop);
        assert_eq!(p.self_modify, SelfModifyBounds::Constrained);
        assert!(p.affect_baseline.is_some());

        let yaml_out = serde_yaml::to_string(&p).unwrap();
        let p2: PersonaProfile = serde_yaml::from_str(&yaml_out).unwrap();
        assert_eq!(p.volition_axes, p2.volition_axes);
        assert_eq!(p.self_modify, p2.self_modify);
    }

    #[test]
    fn test_self_modify_bounds_serde() {
        assert_eq!(
            serde_yaml::from_str::<SelfModifyBounds>("\"none\"").unwrap(),
            SelfModifyBounds::None
        );
        assert_eq!(
            serde_yaml::from_str::<SelfModifyBounds>("\"constrained\"").unwrap(),
            SelfModifyBounds::Constrained
        );
        assert_eq!(
            serde_yaml::from_str::<SelfModifyBounds>("\"open\"").unwrap(),
            SelfModifyBounds::Open
        );
    }

    #[test]
    fn test_no_identity_in_axis_names() {
        for name in &VOLITION_AXIS_NAMES {
            let lower = name.to_lowercase();
            assert!(!lower.contains("ada"), "axis '{}' leaks identity", name);
            assert!(!lower.contains("nsfw"), "axis '{}' leaks NSFW", name);
        }
        for name in &AFFECT_DIMENSION_NAMES {
            let lower = name.to_lowercase();
            assert!(!lower.contains("ada"), "affect '{}' leaks identity", name);
            assert!(!lower.contains("nsfw"), "affect '{}' leaks NSFW", name);
        }
    }
}
