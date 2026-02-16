//! LLM Parameter Modulation — ThinkingStyle → XAI parameters.
//!
//! Maps cognitive profile axes and council weights to concrete LLM
//! parameters (temperature, top_p, reasoning_effort, max_tokens).
//!
//! The modulation pipeline:
//! ```text
//! ThinkingStyle [10 axes] ─┬─ contingency[6] → temperature (0.3–1.2)
//!                          ├─ resonance[1]   → top_p (0.5–1.0)
//!                          ├─ validation[8]  → reasoning_effort (low/medium/high)
//!                          └─ execution[4]   → max_tokens scaling
//!
//! Council [guardian, driver, catalyst] ─┬─ guardian high → dampen temp 20%
//!                                       └─ catalyst high → boost temp 15%
//! ```

use serde::{Deserialize, Serialize};

/// Council weights from TriuneTopology — [guardian, driver, catalyst] intensities.
pub type CouncilWeights = [f32; 3];

/// Overrides for XAI completion parameters, computed from cognitive state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XaiParamOverrides {
    /// Temperature override (0.3–1.2). Higher = more exploratory.
    pub temperature: Option<f64>,
    /// Top-p (nucleus sampling) override (0.5–1.0). Higher = more associative.
    pub top_p: Option<f64>,
    /// Reasoning effort override ("low", "medium", "high").
    pub reasoning_effort: Option<String>,
    /// Max tokens override.
    pub max_tokens: Option<u32>,
}

/// Compute XAI parameter overrides from thinking style + council weights.
///
/// # Arguments
///
/// * `style` — 10-axis thinking style from the module definition:
///   - [0] recognition, [1] resonance, [2] appraisal, [3] routing,
///   - [4] execution, [5] delegation, [6] contingency, [7] integration,
///   - [8] validation, [9] crystallization
/// * `council` — [guardian, driver, catalyst] intensities (sum ≈ 1.0)
/// * `rung_level` — Current cognitive depth rung (R0–R9)
pub fn modulate_xai_params(
    style: &[f32; 10],
    council: &CouncilWeights,
    rung_level: u8,
) -> XaiParamOverrides {
    // Temperature: contingency axis → base temp, council modulates
    let base_temp = map_contingency_to_temp(style[6]);
    let council_mod = council_temperature_mod(council);
    let temperature = (base_temp * council_mod).clamp(0.3, 1.2);

    // Top-p: resonance axis → associative reach
    let top_p = 0.5 + (style[1] * 0.5) as f64;

    // Reasoning effort: validation axis + rung level
    let reasoning_effort = map_validation_to_effort(style[8], rung_level);

    // Max tokens: execution axis scales from base 512 up to 2048
    let base_tokens: u32 = 512;
    let max_tokens = base_tokens + (style[4] * 1536.0) as u32;

    XaiParamOverrides {
        temperature: Some(temperature),
        top_p: Some(top_p.clamp(0.5, 1.0)),
        reasoning_effort: Some(reasoning_effort),
        max_tokens: Some(max_tokens),
    }
}

/// Map contingency axis [0.0–1.0] to base temperature [0.3–1.2].
///
/// Low contingency (deterministic thinking) → low temperature.
/// High contingency ("things could be otherwise") → high temperature.
fn map_contingency_to_temp(contingency: f32) -> f64 {
    // Linear map: 0.0 → 0.3, 1.0 → 1.2
    (0.3 + contingency as f64 * 0.9).clamp(0.3, 1.2)
}

/// Council modulation factor for temperature.
///
/// - Guardian dominant → dampen by up to 20% (stabilize)
/// - Catalyst dominant → boost by up to 15% (explore)
/// - Balanced → no change
fn council_temperature_mod(council: &CouncilWeights) -> f64 {
    let [guardian, _driver, catalyst] = council;

    // Guardian dampening: if guardian > 0.5, reduce temp
    let guardian_effect = if *guardian > 0.4 {
        1.0 - (*guardian as f64 - 0.4) * 0.33 // max -20% at intensity=1.0
    } else {
        1.0
    };

    // Catalyst amplification: if catalyst > 0.4, boost temp
    let catalyst_effect = if *catalyst > 0.4 {
        1.0 + (*catalyst as f64 - 0.4) * 0.25 // max +15% at intensity=1.0
    } else {
        1.0
    };

    guardian_effect * catalyst_effect
}

/// Map validation axis + rung level to reasoning effort string.
///
/// Higher validation → more reasoning depth.
/// Higher rung (R5+) → override to "high" regardless of axis.
fn map_validation_to_effort(validation: f32, rung_level: u8) -> String {
    // Rung R5+ (meta-cognitive levels) always get high reasoning
    if rung_level >= 5 {
        return "high".to_string();
    }

    if validation > 0.7 {
        "high".to_string()
    } else if validation > 0.4 {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balanced_council_no_modulation() {
        let style = [0.5; 10];
        let council: CouncilWeights = [0.33, 0.34, 0.33];
        let params = modulate_xai_params(&style, &council, 3);

        // Balanced council → temperature near base (0.3 + 0.5 * 0.9 = 0.75)
        let temp = params.temperature.unwrap();
        assert!(temp > 0.7 && temp < 0.8, "temp = {}", temp);
        assert_eq!(params.reasoning_effort.unwrap(), "medium");
    }

    #[test]
    fn test_guardian_dampens_temperature() {
        let style = [0.5; 10];
        let guardian_council: CouncilWeights = [0.7, 0.15, 0.15];
        let balanced_council: CouncilWeights = [0.33, 0.34, 0.33];

        let guardian_params = modulate_xai_params(&style, &guardian_council, 3);
        let balanced_params = modulate_xai_params(&style, &balanced_council, 3);

        assert!(
            guardian_params.temperature.unwrap() < balanced_params.temperature.unwrap(),
            "Guardian should dampen temperature"
        );
    }

    #[test]
    fn test_catalyst_boosts_temperature() {
        let style = [0.5; 10];
        let catalyst_council: CouncilWeights = [0.15, 0.15, 0.7];
        let balanced_council: CouncilWeights = [0.33, 0.34, 0.33];

        let catalyst_params = modulate_xai_params(&style, &catalyst_council, 3);
        let balanced_params = modulate_xai_params(&style, &balanced_council, 3);

        assert!(
            catalyst_params.temperature.unwrap() > balanced_params.temperature.unwrap(),
            "Catalyst should boost temperature"
        );
    }

    #[test]
    fn test_high_rung_forces_high_reasoning() {
        let style = [0.2; 10]; // Low validation
        let council: CouncilWeights = [0.33, 0.34, 0.33];
        let params = modulate_xai_params(&style, &council, 7); // R7

        assert_eq!(params.reasoning_effort.unwrap(), "high");
    }

    #[test]
    fn test_high_execution_more_tokens() {
        let mut low_exec = [0.5_f32; 10];
        low_exec[4] = 0.1; // low execution
        let mut high_exec = [0.5_f32; 10];
        high_exec[4] = 0.9; // high execution

        let council: CouncilWeights = [0.33, 0.34, 0.33];
        let low_params = modulate_xai_params(&low_exec, &council, 3);
        let high_params = modulate_xai_params(&high_exec, &council, 3);

        assert!(
            high_params.max_tokens.unwrap() > low_params.max_tokens.unwrap(),
            "Higher execution axis should produce more tokens"
        );
    }

    #[test]
    fn test_temperature_clamped() {
        // Extreme: max contingency + catalyst
        let mut style = [1.0_f32; 10];
        style[6] = 1.0;
        let council: CouncilWeights = [0.0, 0.0, 1.0];
        let params = modulate_xai_params(&style, &council, 0);

        assert!(
            params.temperature.unwrap() <= 1.2,
            "Temperature should be clamped to 1.2"
        );
    }
}
