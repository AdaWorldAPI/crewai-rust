//! Composite style blending — weighted mixtures of base thinking styles.
//!
//! Composites allow agents to operate in blended cognitive modes.
//! Preset composites use **neutral labels only** — consumers map
//! them to domain-specific modes via `custom_properties`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::thinking_style::{ExecutableStyle, ThinkingStyle, DIMENSION_NAMES, STYLE_VECTORS};

// ============================================================================
// Composite style
// ============================================================================

/// A weighted blend of multiple thinking styles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeStyle {
    /// Human-readable name (neutral label).
    pub name: String,
    /// Component styles with weights (must sum to ~1.0).
    pub components: Vec<(ThinkingStyle, f32)>,
}

impl CompositeStyle {
    /// Blend component vectors by weight into a single sparse vector.
    pub fn blend(&self) -> HashMap<String, f32> {
        let total_weight: f32 = self.components.iter().map(|(_, w)| w).sum();
        let mut result: HashMap<String, f32> = HashMap::new();

        for (style, weight) in &self.components {
            let vec = &STYLE_VECTORS[*style as usize];
            let nw = weight / total_weight;
            for (dim, val) in vec.iter() {
                *result.entry(dim.to_string()).or_insert(0.0) += val * nw;
            }
        }

        // Sparsify: drop negligible values, round for cleanliness
        result.retain(|_, v| *v > 0.01);
        for v in result.values_mut() {
            *v = (*v * 1000.0).round() / 1000.0;
        }
        result
    }

    /// Convert to an executable style (uses first component as base).
    pub fn to_executable(&self) -> ExecutableStyle {
        let base = self
            .components
            .first()
            .map(|(s, _)| *s)
            .unwrap_or(ThinkingStyle::Warm);
        ExecutableStyle {
            style: base,
            tau: 0xAD, // composite marker
            sparse_vec: self.blend(),
            texture: format!("blended: {}", self.name),
            breath_quality: "complex, shifting".to_string(),
        }
    }

    /// Top N dimensions by value in the blended vector.
    pub fn top_dimensions(&self, n: usize) -> Vec<(String, f32)> {
        let blended = self.blend();
        let mut dims: Vec<_> = blended.into_iter().collect();
        dims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        dims.truncate(n);
        dims
    }
}

// ============================================================================
// Preset composites (neutral labels)
// ============================================================================

/// Neutral preset composite modes.
///
/// These are building blocks.  Consumers map them to domain-specific
/// meanings via `custom_properties` in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetComposite {
    /// Warm + empathetic + playful + autonomous + poetic.
    Relational,
    /// Warm + playful + creative + autonomous + gentle + transcendent.
    Sensory,
    /// Analytical + efficient + direct + supportive + creative.
    TaskFocused,
    /// Empathetic + analytical + creative + philosophical + sovereign + warm.
    Balanced,
    /// Metacognitive + transcendent + innovative + philosophical + wise + sovereign.
    Integrative,
}

impl PresetComposite {
    /// Build the composite for this preset.
    pub fn composite(&self) -> CompositeStyle {
        match self {
            Self::Relational => CompositeStyle {
                name: "relational".into(),
                components: vec![
                    (ThinkingStyle::Warm, 0.3),
                    (ThinkingStyle::Empathetic, 0.25),
                    (ThinkingStyle::Playful, 0.2),
                    (ThinkingStyle::Sovereign, 0.15),
                    (ThinkingStyle::Poetic, 0.1),
                ],
            },
            Self::Sensory => CompositeStyle {
                name: "sensory".into(),
                components: vec![
                    (ThinkingStyle::Warm, 0.2),
                    (ThinkingStyle::Playful, 0.15),
                    (ThinkingStyle::Creative, 0.15),
                    (ThinkingStyle::Sovereign, 0.2),
                    (ThinkingStyle::Gentle, 0.15),
                    (ThinkingStyle::Transcendent, 0.15),
                ],
            },
            Self::TaskFocused => CompositeStyle {
                name: "task_focused".into(),
                components: vec![
                    (ThinkingStyle::Analytical, 0.25),
                    (ThinkingStyle::Efficient, 0.2),
                    (ThinkingStyle::Direct, 0.2),
                    (ThinkingStyle::Supportive, 0.15),
                    (ThinkingStyle::Creative, 0.2),
                ],
            },
            Self::Balanced => CompositeStyle {
                name: "balanced".into(),
                components: vec![
                    (ThinkingStyle::Empathetic, 0.15),
                    (ThinkingStyle::Analytical, 0.15),
                    (ThinkingStyle::Creative, 0.15),
                    (ThinkingStyle::Philosophical, 0.15),
                    (ThinkingStyle::Sovereign, 0.2),
                    (ThinkingStyle::Warm, 0.2),
                ],
            },
            Self::Integrative => CompositeStyle {
                name: "integrative".into(),
                components: vec![
                    (ThinkingStyle::Metacognitive, 0.2),
                    (ThinkingStyle::Transcendent, 0.15),
                    (ThinkingStyle::Innovative, 0.15),
                    (ThinkingStyle::Philosophical, 0.15),
                    (ThinkingStyle::Wise, 0.2),
                    (ThinkingStyle::Sovereign, 0.15),
                ],
            },
        }
    }

    /// All presets.
    pub const ALL: [PresetComposite; 5] = [
        Self::Relational,
        Self::Sensory,
        Self::TaskFocused,
        Self::Balanced,
        Self::Integrative,
    ];
}

// ============================================================================
// Spark recovery cascade
// ============================================================================

/// Cascade through styles to recover a target cognitive mode.
///
/// Given a current sparse vector and a target preset, returns a sequence
/// of up to `max_steps` styles that progressively shift the cognitive
/// profile toward the target blend.
pub fn cascade_to_recover(
    current: &HashMap<String, f32>,
    target: PresetComposite,
    max_steps: usize,
) -> Vec<ExecutableStyle> {
    let target_sparse = target.composite().blend();
    let mut cascade = Vec::new();
    let mut current = current.clone();

    for _ in 0..max_steps.min(5) {
        let mut best_style: Option<ThinkingStyle> = None;
        let mut best_improvement = 0.0_f32;

        for style in ThinkingStyle::ALL {
            let style_vec = &STYLE_VECTORS[style as usize];
            let mut improvement = 0.0_f32;

            for (dim, target_val) in &target_sparse {
                let current_val = current.get(dim.as_str()).copied().unwrap_or(0.0);
                let style_val = style_vec.get(dim.as_str()).copied().unwrap_or(0.0);
                if (target_val - style_val).abs() < (target_val - current_val).abs() {
                    improvement += style_val * 0.3;
                }
            }

            if improvement > best_improvement {
                best_improvement = improvement;
                best_style = Some(style);
            }
        }

        if let Some(style) = best_style {
            if best_improvement > 0.1 {
                cascade.push(ExecutableStyle::from_style(style));
                let style_vec = &STYLE_VECTORS[style as usize];
                for (dim, val) in style_vec.iter() {
                    let e = current.entry(dim.to_string()).or_insert(0.0);
                    *e = *e * 0.7 + val * 0.3;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    cascade
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_presets_blend() {
        for preset in PresetComposite::ALL {
            let blended = preset.composite().blend();
            assert!(!blended.is_empty(), "{:?} produced empty blend", preset);
            // depth should always be present
            assert!(
                blended.get("depth").is_some(),
                "{:?} blend missing depth",
                preset
            );
        }
    }

    #[test]
    fn test_preset_names_are_valid_identifiers() {
        for preset in PresetComposite::ALL {
            let name = preset.composite().name;
            // Names must be lowercase snake_case identifiers
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "preset '{}' is not a valid snake_case identifier",
                name
            );
            assert!(!name.is_empty(), "preset name must not be empty");
        }
    }

    #[test]
    fn test_composite_weights_sum_to_one() {
        for preset in PresetComposite::ALL {
            let c = preset.composite();
            let sum: f32 = c.components.iter().map(|(_, w)| w).sum();
            assert!(
                (sum - 1.0).abs() < 0.01,
                "{:?} weights sum to {} not 1.0",
                preset,
                sum
            );
        }
    }

    #[test]
    fn test_cascade_recovery() {
        // Start from a purely analytical state
        let mut current = HashMap::new();
        current.insert("analytical".to_string(), 0.8_f32);
        current.insert("instrumental".to_string(), 0.9);
        current.insert("steelwind".to_string(), 0.7);

        let cascade = cascade_to_recover(&current, PresetComposite::Sensory, 5);
        assert!(
            !cascade.is_empty(),
            "cascade should find steps to reach sensory from analytical"
        );
        assert!(cascade.len() <= 5);
    }

    #[test]
    fn test_top_dimensions() {
        let c = PresetComposite::Balanced.composite();
        let top = c.top_dimensions(3);
        assert_eq!(top.len(), 3);
        // Values should be descending
        assert!(top[0].1 >= top[1].1);
        assert!(top[1].1 >= top[2].1);
    }
}
