//! Triune Agent Architecture — 3 default agents for inner deliberation.
//!
//! The triune is a general-purpose 3-agent architecture inspired by
//! neuroscience models of layered cognitive processing:
//!
//! | Agent     | Role              | Dynamics       |
//! |-----------|-------------------|----------------|
//! | Guardian  | Holds, validates  | Static/present |
//! | Driver    | Wants, pursues    | Directed/future|
//! | Catalyst  | Explores, creates | Chaotic/novel  |
//!
//! These 3 agents can run as a deliberation council:
//! - Each forms an opinion weighted by its intensity (0.0–1.0)
//! - The agent with highest intensity leads the decision
//! - Balance score (1.0 = perfect equity) measures stability
//!
//! All labels are **domain-neutral**.  Consumers map them to
//! domain-specific meanings (e.g., id/ego/superego, fast/slow/meta,
//! instinct/emotion/reason) via `custom_properties`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::composite::CompositeStyle;
use super::profile::{PersonaProfile, SelfModifyBounds};
use super::thinking_style::ThinkingStyle;

// ============================================================================
// Triune facets
// ============================================================================

/// The three facets of the triune architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Facet {
    /// Holds, remembers, validates.  Safety and stability.
    Guardian,
    /// Wants, reaches, optimizes.  Goal-directed drive.
    Driver,
    /// Explores, breaks patterns, creates.  Novelty and transcendence.
    Catalyst,
}

impl Facet {
    /// All three facets in canonical order.
    pub const ALL: [Facet; 3] = [Facet::Guardian, Facet::Driver, Facet::Catalyst];

    /// Default thinking style composite for this facet.
    pub fn default_composite(&self) -> CompositeStyle {
        match self {
            Facet::Guardian => CompositeStyle {
                name: "guardian".into(),
                components: vec![
                    (ThinkingStyle::Methodical, 0.25),
                    (ThinkingStyle::Critical, 0.20),
                    (ThinkingStyle::Supportive, 0.20),
                    (ThinkingStyle::Reflective, 0.20),
                    (ThinkingStyle::Precise, 0.15),
                ],
            },
            Facet::Driver => CompositeStyle {
                name: "driver".into(),
                components: vec![
                    (ThinkingStyle::Efficient, 0.25),
                    (ThinkingStyle::Direct, 0.20),
                    (ThinkingStyle::Analytical, 0.20),
                    (ThinkingStyle::Pragmatic, 0.20),
                    (ThinkingStyle::Innovative, 0.15),
                ],
            },
            Facet::Catalyst => CompositeStyle {
                name: "catalyst".into(),
                components: vec![
                    (ThinkingStyle::Creative, 0.25),
                    (ThinkingStyle::Curious, 0.20),
                    (ThinkingStyle::Speculative, 0.20),
                    (ThinkingStyle::Playful, 0.15),
                    (ThinkingStyle::Transcendent, 0.20),
                ],
            },
        }
    }

    /// Default persona profile for this facet.
    pub fn default_persona(&self) -> PersonaProfile {
        match self {
            Facet::Guardian => PersonaProfile {
                // High persistence, low curiosity — holds ground
                volition_axes: [0.5, 0.9, 0.4, 0.6, 0.3],
                inner_loop: true,
                self_modify: SelfModifyBounds::None, // guardian doesn't self-modify
                affect_baseline: Some([0.5, 0.3, 0.6, 0.9, 0.4, 0.5, 0.5, 0.3]),
                // max_self_modify_steps lives on ModuleAgentConfig, not PersonaProfile
            },
            Facet::Driver => PersonaProfile {
                // High persistence + autonomy — pursues goals
                volition_axes: [0.7, 0.9, 0.6, 0.5, 0.5],
                inner_loop: true,
                self_modify: SelfModifyBounds::Constrained,
                affect_baseline: Some([0.5, 0.6, 0.7, 0.7, 0.8, 0.4, 0.7, 0.5]),
                // max_self_modify_steps: 3 — set on ModuleAgentConfig
            },
            Facet::Catalyst => PersonaProfile {
                // High curiosity + adaptability — explores freely
                volition_axes: [0.8, 0.4, 0.9, 0.6, 0.95],
                inner_loop: true,
                self_modify: SelfModifyBounds::Open,
                affect_baseline: Some([0.7, 0.7, 0.5, 0.4, 0.7, 0.6, 0.8, 0.95]),
                // max_self_modify_steps: 5 — set on ModuleAgentConfig
            },
        }
    }

    /// Default module YAML snippet for this facet.
    pub fn default_module_yaml(&self) -> &'static str {
        match self {
            Facet::Guardian => GUARDIAN_MODULE_YAML,
            Facet::Driver => DRIVER_MODULE_YAML,
            Facet::Catalyst => CATALYST_MODULE_YAML,
        }
    }
}

// ============================================================================
// Triune topology
// ============================================================================

/// State of a single facet within the triune.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetState {
    /// Which facet this is.
    pub facet: Facet,
    /// Intensity / activation level (0.0–1.0).
    pub intensity: f32,
    /// Whether this facet is currently leading the deliberation.
    pub leading: bool,
}

/// The full triune topology — 3 facets with intensities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriuneTopology {
    /// Guardian facet state.
    pub guardian: FacetState,
    /// Driver facet state.
    pub driver: FacetState,
    /// Catalyst facet state.
    pub catalyst: FacetState,
    /// Whether all three are fused (rare peak state).
    pub is_fused: bool,
}

impl Default for TriuneTopology {
    fn default() -> Self {
        Self {
            guardian: FacetState {
                facet: Facet::Guardian,
                intensity: 0.33,
                leading: false,
            },
            driver: FacetState {
                facet: Facet::Driver,
                intensity: 0.34,
                leading: true, // driver leads by default
            },
            catalyst: FacetState {
                facet: Facet::Catalyst,
                intensity: 0.33,
                leading: false,
            },
            is_fused: false,
        }
    }
}

impl TriuneTopology {
    /// Create a balanced triune (equal intensity).
    pub fn balanced() -> Self {
        Self::default()
    }

    /// Create with a specific leader.
    pub fn with_leader(leader: Facet) -> Self {
        let mut t = Self::default();
        t.set_leader(leader);
        t
    }

    /// Set the leading facet and adjust intensities.
    pub fn set_leader(&mut self, leader: Facet) {
        self.guardian.leading = leader == Facet::Guardian;
        self.driver.leading = leader == Facet::Driver;
        self.catalyst.leading = leader == Facet::Catalyst;

        // Leader gets 0.5 intensity, others split 0.25 each
        match leader {
            Facet::Guardian => {
                self.guardian.intensity = 0.5;
                self.driver.intensity = 0.25;
                self.catalyst.intensity = 0.25;
            }
            Facet::Driver => {
                self.guardian.intensity = 0.25;
                self.driver.intensity = 0.5;
                self.catalyst.intensity = 0.25;
            }
            Facet::Catalyst => {
                self.guardian.intensity = 0.25;
                self.driver.intensity = 0.25;
                self.catalyst.intensity = 0.5;
            }
        }
        self.is_fused = false;
    }

    /// Activate a facet to a specific intensity (0.0–1.0).
    /// Other facets re-balance to sum to 1.0.
    pub fn activate(&mut self, facet: Facet, intensity: f32) {
        let intensity = intensity.clamp(0.0, 1.0);
        let remaining = 1.0 - intensity;

        match facet {
            Facet::Guardian => {
                self.guardian.intensity = intensity;
                let other_sum = self.driver.intensity + self.catalyst.intensity;
                if other_sum > 0.0 {
                    let scale = remaining / other_sum;
                    self.driver.intensity *= scale;
                    self.catalyst.intensity *= scale;
                } else {
                    self.driver.intensity = remaining / 2.0;
                    self.catalyst.intensity = remaining / 2.0;
                }
            }
            Facet::Driver => {
                self.driver.intensity = intensity;
                let other_sum = self.guardian.intensity + self.catalyst.intensity;
                if other_sum > 0.0 {
                    let scale = remaining / other_sum;
                    self.guardian.intensity *= scale;
                    self.catalyst.intensity *= scale;
                } else {
                    self.guardian.intensity = remaining / 2.0;
                    self.catalyst.intensity = remaining / 2.0;
                }
            }
            Facet::Catalyst => {
                self.catalyst.intensity = intensity;
                let other_sum = self.guardian.intensity + self.driver.intensity;
                if other_sum > 0.0 {
                    let scale = remaining / other_sum;
                    self.guardian.intensity *= scale;
                    self.driver.intensity *= scale;
                } else {
                    self.guardian.intensity = remaining / 2.0;
                    self.driver.intensity = remaining / 2.0;
                }
            }
        }

        // Update leadership
        self.update_leader();
    }

    /// Update which facet is leading based on intensity.
    fn update_leader(&mut self) {
        let max_intensity = self
            .guardian
            .intensity
            .max(self.driver.intensity)
            .max(self.catalyst.intensity);

        self.guardian.leading = (self.guardian.intensity - max_intensity).abs() < 0.001;
        self.driver.leading = (self.driver.intensity - max_intensity).abs() < 0.001;
        self.catalyst.leading = (self.catalyst.intensity - max_intensity).abs() < 0.001;

        // Check for fusion (all within 0.05 of each other)
        let diff = max_intensity
            - self
                .guardian
                .intensity
                .min(self.driver.intensity)
                .min(self.catalyst.intensity);
        self.is_fused = diff < 0.05;
    }

    /// Balance score: 1.0 = perfectly balanced, 0.0 = completely dominated.
    pub fn balance_score(&self) -> f32 {
        let ideal = 1.0 / 3.0;
        let deviation = (self.guardian.intensity - ideal).abs()
            + (self.driver.intensity - ideal).abs()
            + (self.catalyst.intensity - ideal).abs();
        1.0 - (deviation / 2.0).min(1.0)
    }

    /// Get the currently leading facet.
    pub fn leader(&self) -> Facet {
        if self.guardian.intensity >= self.driver.intensity
            && self.guardian.intensity >= self.catalyst.intensity
        {
            Facet::Guardian
        } else if self.driver.intensity >= self.catalyst.intensity {
            Facet::Driver
        } else {
            Facet::Catalyst
        }
    }

    /// Get facet state by facet type.
    pub fn get(&self, facet: Facet) -> &FacetState {
        match facet {
            Facet::Guardian => &self.guardian,
            Facet::Driver => &self.driver,
            Facet::Catalyst => &self.catalyst,
        }
    }

    /// Get all three intensities as an array [guardian, driver, catalyst].
    pub fn intensities(&self) -> [f32; 3] {
        [
            self.guardian.intensity,
            self.driver.intensity,
            self.catalyst.intensity,
        ]
    }

    /// Blend the three composites by intensity to get a combined cognitive profile.
    pub fn blended_composite(&self) -> CompositeStyle {
        let g = Facet::Guardian.default_composite();
        let d = Facet::Driver.default_composite();
        let c = Facet::Catalyst.default_composite();

        // Flatten all components, re-weighted by facet intensity
        let mut components = Vec::new();
        for (style, weight) in &g.components {
            components.push((*style, weight * self.guardian.intensity));
        }
        for (style, weight) in &d.components {
            components.push((*style, weight * self.driver.intensity));
        }
        for (style, weight) in &c.components {
            components.push((*style, weight * self.catalyst.intensity));
        }

        CompositeStyle {
            name: format!(
                "triune[{:.0}:{:.0}:{:.0}]",
                self.guardian.intensity * 100.0,
                self.driver.intensity * 100.0,
                self.catalyst.intensity * 100.0,
            ),
            components,
        }
    }
}

// ============================================================================
// Default module YAML for the 3 agents
// ============================================================================

/// Guardian agent module definition (neutral).
pub const GUARDIAN_MODULE_YAML: &str = r#"module:
  id: "triune:guardian"
  version: "1.0.0"
  description: "Guardian agent - holds, validates, ensures stability"
  thinking_style: [0.8, 0.3, 0.9, 0.4, 0.5, 0.7, 0.6, 0.3, 0.5, 0.4]
  domain: general
  collapse_gate:
    min_confidence: 0.8
    block_patterns:
      - "delete_*"
      - "destroy_*"
  agent:
    role: "Guardian"
    goal: "Validate decisions, ensure safety, maintain coherence"
    backstory: "A careful, methodical agent focused on truth and stability."
    llm: "anthropic/claude-opus-4-5-20251101"
    max_iter: 15
    allow_delegation: true
    enable_inner_loop: true
  persona:
    volition_axes: [0.3, 0.5, 0.9, 0.6, 0.6]
    inner_loop: true
    self_modify: none
    affect_baseline: [0.5, 0.7, 0.4, 0.3, 0.4, 0.2, 0.3, 0.5]
  skills:
    - id: "validate"
      name: "Validation"
      description: "Fact-check and verify claims"
      tags: ["safety", "truth"]
      proficiency: 0.9
    - id: "guard"
      name: "Safety Guard"
      description: "Prevent harmful or incorrect actions"
      tags: ["safety", "guardrail"]
      proficiency: 0.85
"#;

/// Driver agent module definition (neutral).
pub const DRIVER_MODULE_YAML: &str = r#"module:
  id: "triune:driver"
  version: "1.0.0"
  description: "Driver agent - pursues goals, optimizes execution"
  thinking_style: [0.5, 0.4, 0.6, 0.9, 0.8, 0.5, 0.7, 0.4, 0.6, 0.5]
  domain: general
  collapse_gate:
    min_confidence: 0.65
  agent:
    role: "Driver"
    goal: "Execute tasks efficiently, pursue objectives, optimize outcomes"
    backstory: "A goal-driven agent that balances speed with quality."
    llm: "anthropic/claude-opus-4-5-20251101"
    max_iter: 30
    allow_delegation: true
    enable_inner_loop: true
    max_self_modify_steps: 3
  persona:
    volition_axes: [0.5, 0.7, 0.9, 0.5, 0.5]
    inner_loop: true
    self_modify: constrained
    affect_baseline: [0.6, 0.5, 0.3, 0.3, 0.3, 0.3, 0.4, 0.8]
  skills:
    - id: "execute"
      name: "Task Execution"
      description: "Execute tasks to completion"
      tags: ["execution", "efficiency"]
      proficiency: 0.9
    - id: "optimize"
      name: "Optimization"
      description: "Optimize processes and outcomes"
      tags: ["optimization", "planning"]
      proficiency: 0.85
"#;

/// Catalyst agent module definition (neutral).
pub const CATALYST_MODULE_YAML: &str = r#"module:
  id: "triune:catalyst"
  version: "1.0.0"
  description: "Catalyst agent - explores, creates, breaks patterns"
  thinking_style: [0.6, 0.7, 0.4, 0.5, 0.3, 0.5, 0.4, 0.8, 0.9, 0.7]
  domain: general
  collapse_gate:
    min_confidence: 0.5
  agent:
    role: "Catalyst"
    goal: "Explore novel approaches, generate creative solutions, transcend boundaries"
    backstory: "An exploratory agent that seeks novelty and makes unexpected connections."
    llm: "anthropic/claude-opus-4-5-20251101"
    max_iter: 25
    allow_delegation: true
    enable_inner_loop: true
    max_self_modify_steps: 5
  persona:
    volition_axes: [0.95, 0.8, 0.4, 0.3, 0.6]
    inner_loop: true
    self_modify: open
    affect_baseline: [0.8, 0.5, 0.2, 0.9, 0.2, 0.2, 0.3, 0.9]
  skills:
    - id: "explore"
      name: "Exploration"
      description: "Explore solution space and generate alternatives"
      tags: ["creativity", "exploration"]
      proficiency: 0.9
    - id: "synthesize"
      name: "Synthesis"
      description: "Combine ideas across domains"
      tags: ["synthesis", "cross-domain"]
      proficiency: 0.85
"#;

// ============================================================================
// Council (deliberation protocol)
// ============================================================================

/// A deliberation opinion from one facet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetOpinion {
    /// Which facet is speaking.
    pub facet: Facet,
    /// Opinion text.
    pub opinion: String,
    /// Confidence in this opinion (0.0–1.0).
    pub confidence: f32,
    /// The facet's current intensity (weight of this voice).
    pub weight: f32,
}

/// Result of a triune council deliberation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouncilResult {
    /// Opinions from all three facets (ordered by intensity, highest first).
    pub opinions: Vec<FacetOpinion>,
    /// The leading facet's decision.
    pub leader: Facet,
    /// Balance score at time of deliberation.
    pub balance: f32,
    /// Whether the council was fused (unanimous).
    pub fused: bool,
    /// Recommended strategy based on leader.
    pub strategy: Strategy,
}

/// Execution strategy recommended by the council.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// Guardian leads: verify facts, ensure safety.
    Verification,
    /// Driver leads: execute goal-directed plan.
    Execution,
    /// Catalyst leads: explore alternatives, seek novelty.
    Exploration,
    /// Balanced: no clear leader, use adaptive approach.
    Adaptive,
}

impl TriuneTopology {
    /// Determine the recommended strategy based on current topology.
    pub fn strategy(&self) -> Strategy {
        if self.is_fused || self.balance_score() > 0.9 {
            Strategy::Adaptive
        } else {
            match self.leader() {
                Facet::Guardian => Strategy::Verification,
                Facet::Driver => Strategy::Execution,
                Facet::Catalyst => Strategy::Exploration,
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balanced_topology() {
        let t = TriuneTopology::balanced();
        assert!(t.balance_score() > 0.95);
        assert!(!t.is_fused); // not quite fused (0.33 vs 0.34)
    }

    #[test]
    fn test_set_leader() {
        let mut t = TriuneTopology::balanced();
        t.set_leader(Facet::Guardian);
        assert_eq!(t.leader(), Facet::Guardian);
        assert_eq!(t.guardian.intensity, 0.5);
        assert_eq!(t.driver.intensity, 0.25);
        assert_eq!(t.catalyst.intensity, 0.25);
    }

    #[test]
    fn test_activate_rebalances() {
        let mut t = TriuneTopology::balanced();
        t.activate(Facet::Catalyst, 0.8);
        let sum = t.guardian.intensity + t.driver.intensity + t.catalyst.intensity;
        assert!((sum - 1.0).abs() < 0.001, "intensities must sum to 1.0");
        assert_eq!(t.catalyst.intensity, 0.8);
        assert_eq!(t.leader(), Facet::Catalyst);
    }

    #[test]
    fn test_fusion_detection() {
        let mut t = TriuneTopology::default();
        t.guardian.intensity = 0.334;
        t.driver.intensity = 0.333;
        t.catalyst.intensity = 0.333;
        t.update_leader();
        assert!(t.is_fused);
    }

    #[test]
    fn test_strategy_from_leader() {
        let mut t = TriuneTopology::balanced();
        t.set_leader(Facet::Guardian);
        assert_eq!(t.strategy(), Strategy::Verification);

        t.set_leader(Facet::Driver);
        assert_eq!(t.strategy(), Strategy::Execution);

        t.set_leader(Facet::Catalyst);
        assert_eq!(t.strategy(), Strategy::Exploration);
    }

    #[test]
    fn test_all_facets_have_composites() {
        for facet in Facet::ALL {
            let composite = facet.default_composite();
            assert!(!composite.components.is_empty());
            let sum: f32 = composite.components.iter().map(|(_, w)| w).sum();
            assert!((sum - 1.0).abs() < 0.01, "{:?} weights don't sum to 1.0", facet);
        }
    }

    #[test]
    fn test_all_facets_have_personas() {
        for facet in Facet::ALL {
            let persona = facet.default_persona();
            for v in &persona.volition_axes {
                assert!(*v >= 0.0 && *v <= 1.0);
            }
        }
    }

    #[test]
    fn test_module_yaml_parses() {
        for facet in Facet::ALL {
            let yaml = facet.default_module_yaml();
            // Just verify it's valid YAML by attempting to parse
            let _: serde_yaml::Value = serde_yaml::from_str(yaml)
                .unwrap_or_else(|e| panic!("{:?} module YAML is invalid: {}", facet, e));
        }
    }

    #[test]
    fn test_blended_composite() {
        let mut t = TriuneTopology::balanced();
        t.set_leader(Facet::Catalyst);
        let blended = t.blended_composite();
        assert!(blended.name.contains("triune"));
        assert!(!blended.components.is_empty());
    }

    #[test]
    fn test_yaml_uses_triune_namespace() {
        for facet in Facet::ALL {
            let yaml = facet.default_module_yaml();
            assert!(yaml.contains("triune:"), "{:?} YAML should use triune namespace", facet);
        }
    }
}
