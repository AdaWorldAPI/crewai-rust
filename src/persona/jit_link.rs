//! JIT Template Link — AgentCard → ThinkingStyle → compiled scan kernel.
//!
//! When an agent card (ModuleDef) is activated, its thinking style configuration
//! is resolved to a set of ThinkingStyle variants, which map to τ (tau) macro
//! addresses.  These addresses serve as JIT template identifiers that n8n-rs
//! compiles to native code via jitson/Cranelift.
//!
//! # Pipeline
//!
//! ```text
//! AgentCard (YAML)
//!   │  thinking_style: [f32; 10]    ← 10-layer cognitive stack
//!   │  persona: PersonaProfile       ← volition, affect, inner-loop
//!   ▼
//! JitProfile::from_module()
//!   │  Resolves 10-axis → dominant ThinkingStyle variants
//!   │  Maps styles → τ addresses → JIT template parameters
//!   ▼
//! JitProfile { templates: Vec<JitTemplate> }
//!   │  Each template has: τ address, scan params, priority, cluster
//!   ▼
//! n8n-rs CompiledStyleRegistry::compile(jit_profile)
//!   │  Cranelift compiles τ addresses → native ScanKernels
//!   ▼
//! Agent executes with compiled thinking textures
//!   │  No HashMap lookups — indexed dispatch by τ address
//!   ▼
//! fn ptrs called directly during agent reasoning loop
//! ```
//!
//! # Agent Cards as JIT Anchors
//!
//! An agent card's identity (role, goal, thinking style) is stable across
//! sessions.  This means its JIT templates can be **pre-compiled at startup**
//! and cached for the lifetime of the process — no recompilation unless
//! the agent's cognitive profile changes.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::thinking_style::{StyleCluster, ThinkingStyle, STYLE_TO_TAU, STYLE_VECTORS};

// ============================================================================
// JIT Template types
// ============================================================================

/// A single JIT template derived from a ThinkingStyle.
///
/// Represents a compiled-or-compilable cognitive kernel identified by its
/// τ (tau) macro address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitTemplate {
    /// The thinking style this template represents.
    pub style: ThinkingStyle,
    /// τ (tau) macro address — the JIT compilation key.
    ///
    /// n8n-rs CompiledStyleRegistry uses this to look up or compile
    /// the corresponding ScanKernel.
    pub tau: u8,
    /// Weight of this template in the agent's cognitive blend (0.0–1.0).
    pub weight: f32,
    /// The cluster this style belongs to.
    pub cluster: StyleCluster,
    /// Scan parameters derived from the style's 23D sparse vector.
    ///
    /// These become Cranelift immediates when compiled:
    /// - `threshold` → CMP immediate
    /// - `top_k` → loop bound
    /// - `prefetch_ahead` → PREFETCHT0 offset
    pub scan_params: JitScanParams,
}

/// Scan parameters extracted from a thinking style's 23D vector.
///
/// These values are known at activation time (deploy-time-known) and
/// can be baked as Cranelift immediates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitScanParams {
    /// Resonance threshold for Hamming similarity search.
    /// Derived from the `depth` dimension.
    pub threshold: u32,
    /// Top-K results to return from each scan.
    /// Derived from the `creative` + `analytical` dimensions.
    pub top_k: u32,
    /// Prefetch-ahead distance for PREFETCHT0.
    /// Derived from the `flow` + `vitality` dimensions.
    pub prefetch_ahead: u32,
    /// Record size (bytes per fingerprint, typically 2048).
    pub record_size: u32,
    /// Filter mask derived from domain affinity dimensions.
    /// Each bit corresponds to a BindSpace prefix.
    pub filter_mask: u64,
}

// ============================================================================
// JitProfile — the complete compiled profile for an agent
// ============================================================================

/// A complete JIT profile for an agent card.
///
/// Contains all the JIT templates needed for the agent's cognitive processing,
/// derived from its ModuleDef thinking style configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitProfile {
    /// Agent card identifier (module ID).
    pub card_id: String,
    /// The resolved JIT templates, sorted by weight (descending).
    pub templates: Vec<JitTemplate>,
    /// The dominant cluster for this agent.
    pub dominant_cluster: StyleCluster,
    /// Hash of the source configuration — if this changes, recompile.
    pub config_hash: u64,
    /// 10-axis thinking style vector from the module definition.
    pub module_style: [f32; 10],
}

impl JitProfile {
    /// Resolve a JIT profile from a module definition's thinking style.
    ///
    /// Maps the 10-axis cognitive stack vector to dominant ThinkingStyle
    /// variants, then derives JIT scan parameters from each style's 23D vector.
    ///
    /// # Arguments
    ///
    /// * `card_id` - Module identifier.
    /// * `thinking_style` - 10-axis cognitive stack vector from ModuleDef.
    pub fn from_module(card_id: impl Into<String>, thinking_style: [f32; 10]) -> Self {
        let card_id = card_id.into();

        // Map 10-axis to dominant styles via affinity scoring
        let affinities = compute_cluster_affinities(&thinking_style);

        // Select top styles from each activated cluster
        let mut templates = Vec::new();
        for (cluster, affinity) in &affinities {
            if *affinity < 0.2 {
                continue; // skip weakly-activated clusters
            }

            // Find the best-matching style in this cluster
            let cluster_styles = ThinkingStyle::ALL
                .iter()
                .filter(|s| s.cluster() == *cluster);

            let best = cluster_styles
                .map(|&style| {
                    let score = style_affinity(&style, &thinking_style);
                    (style, score)
                })
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            if let Some((style, score)) = best {
                let sparse_vec = style.vector();
                let scan_params = sparse_vec_to_scan_params(sparse_vec);

                templates.push(JitTemplate {
                    style,
                    tau: style.tau(),
                    weight: score * affinity,
                    cluster: *cluster,
                    scan_params,
                });
            }
        }

        // Sort by weight descending
        templates.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());

        // Determine dominant cluster
        let dominant_cluster = templates
            .first()
            .map(|t| t.cluster)
            .unwrap_or(StyleCluster::Analytical);

        // Config hash for cache invalidation
        let config_hash = hash_style(&thinking_style);

        Self {
            card_id,
            templates,
            dominant_cluster,
            config_hash,
            module_style: thinking_style,
        }
    }

    /// Get all τ addresses for this profile (for batch compilation).
    pub fn tau_addresses(&self) -> Vec<u8> {
        self.templates.iter().map(|t| t.tau).collect()
    }

    /// Get the primary (highest-weight) template.
    pub fn primary(&self) -> Option<&JitTemplate> {
        self.templates.first()
    }

    /// Get templates for a specific cluster.
    pub fn by_cluster(&self, cluster: StyleCluster) -> Vec<&JitTemplate> {
        self.templates
            .iter()
            .filter(|t| t.cluster == cluster)
            .collect()
    }

    /// Check if recompilation is needed (config hash changed).
    pub fn needs_recompile(&self, new_style: &[f32; 10]) -> bool {
        hash_style(new_style) != self.config_hash
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Map the 10-axis cognitive stack to cluster affinities.
///
/// The 10 axes correspond to the ladybug-rs cognitive stack layers:
/// ```text
/// [0] recognition     → Analytical + Exploratory
/// [1] resonance       → Empathic + Creative
/// [2] appraisal       → Analytical + Meta
/// [3] routing         → Direct + Analytical
/// [4] execution       → Direct + Creative
/// [5] delegation      → Meta + Direct
/// [6] contingency     → Exploratory + Creative
/// [7] integration     → Meta + Empathic
/// [8] validation      → Analytical + Meta
/// [9] crystallization → Meta + Exploratory
/// ```
fn compute_cluster_affinities(style: &[f32; 10]) -> Vec<(StyleCluster, f32)> {
    let analytical =
        (style[0] * 0.3 + style[2] * 0.25 + style[3] * 0.15 + style[8] * 0.2 + style[9] * 0.1)
            .min(1.0);

    let creative = (style[1] * 0.3 + style[4] * 0.25 + style[6] * 0.25 + style[7] * 0.2).min(1.0);

    let empathic = (style[1] * 0.4 + style[7] * 0.35 + style[5] * 0.25).min(1.0);

    let direct = (style[3] * 0.3 + style[4] * 0.35 + style[5] * 0.35).min(1.0);

    let exploratory = (style[0] * 0.2 + style[6] * 0.4 + style[9] * 0.4).min(1.0);

    let meta =
        (style[2] * 0.15 + style[5] * 0.2 + style[7] * 0.2 + style[8] * 0.2 + style[9] * 0.25)
            .min(1.0);

    vec![
        (StyleCluster::Analytical, analytical),
        (StyleCluster::Creative, creative),
        (StyleCluster::Empathic, empathic),
        (StyleCluster::Direct, direct),
        (StyleCluster::Exploratory, exploratory),
        (StyleCluster::Meta, meta),
    ]
}

/// Score how well a ThinkingStyle matches a 10-axis module vector.
///
/// Uses the style's 23D sparse vector projected onto the 10-axis space.
fn style_affinity(style: &ThinkingStyle, module_style: &[f32; 10]) -> f32 {
    let vec = style.vector();

    // Project 23D → 10-axis via dimension affinities
    let recognition =
        *vec.get("analytical").unwrap_or(&0.0) * 0.5 + *vec.get("intuitive").unwrap_or(&0.0) * 0.5;
    let resonance =
        *vec.get("emotional").unwrap_or(&0.0) * 0.5 + *vec.get("relational").unwrap_or(&0.0) * 0.5;
    let appraisal =
        *vec.get("dialectic").unwrap_or(&0.0) * 0.5 + *vec.get("cognitive").unwrap_or(&0.0) * 0.5;
    let routing = *vec.get("instrumental").unwrap_or(&0.0) * 0.7
        + *vec.get("analytical").unwrap_or(&0.0) * 0.3;
    let execution =
        *vec.get("instrumental").unwrap_or(&0.0) * 0.5 + *vec.get("creative").unwrap_or(&0.0) * 0.5;
    let delegation =
        *vec.get("meta").unwrap_or(&0.0) * 0.5 + *vec.get("autonomy").unwrap_or(&0.0) * 0.5;
    let contingency = *vec.get("creative").unwrap_or(&0.0) * 0.4
        + *vec.get("spontaneity").unwrap_or(&0.0) * 0.3
        + *vec.get("existential").unwrap_or(&0.0) * 0.3;
    let integration =
        *vec.get("receptivity").unwrap_or(&0.0) * 0.5 + *vec.get("meta").unwrap_or(&0.0) * 0.5;
    let validation =
        *vec.get("analytical").unwrap_or(&0.0) * 0.5 + *vec.get("meta").unwrap_or(&0.0) * 0.5;
    let crystallization =
        *vec.get("transcendent").unwrap_or(&0.0) * 0.5 + *vec.get("meta").unwrap_or(&0.0) * 0.5;

    let projected = [
        recognition,
        resonance,
        appraisal,
        routing,
        execution,
        delegation,
        contingency,
        integration,
        validation,
        crystallization,
    ];

    // Dot product (cosine-like affinity)
    let dot: f32 = projected
        .iter()
        .zip(module_style.iter())
        .map(|(a, b)| a * b)
        .sum();

    let mag_a: f32 = projected
        .iter()
        .map(|x| x * x)
        .sum::<f32>()
        .sqrt()
        .max(0.001);
    let mag_b: f32 = module_style
        .iter()
        .map(|x| x * x)
        .sum::<f32>()
        .sqrt()
        .max(0.001);

    dot / (mag_a * mag_b)
}

/// Convert a 23D sparse vector to JIT scan parameters.
fn sparse_vec_to_scan_params(vec: &HashMap<&str, f32>) -> JitScanParams {
    let depth = *vec.get("depth").unwrap_or(&0.5);
    let creative = *vec.get("creative").unwrap_or(&0.0);
    let analytical = *vec.get("analytical").unwrap_or(&0.0);
    let flow = *vec.get("flow").unwrap_or(&0.0);
    let vitality = *vec.get("vitality").unwrap_or(&0.0);
    let instrumental = *vec.get("instrumental").unwrap_or(&0.0);
    let cognitive = *vec.get("cognitive").unwrap_or(&0.0);

    // threshold: higher depth = lower threshold (more selective search)
    let threshold = (2000.0 - depth * 1900.0) as u32;

    // top_k: creative styles want more results; analytical want fewer, precise ones
    let top_k = (8.0 + creative * 60.0 + analytical * 20.0).min(128.0) as u32;

    // prefetch_ahead: flow + vitality = how aggressively to prefetch
    let prefetch_ahead = (4.0 + (flow + vitality) * 16.0).min(32.0) as u32;

    // filter_mask: which BindSpace prefixes to search
    // instrumental → surface prefixes, cognitive → node prefixes
    let mut filter_mask: u64 = 0xFFFF_FFFF_FFFF_FFFF; // all by default
    if instrumental > 0.7 {
        // Focus on surface + fluid zones
        filter_mask = 0x0000_FFFF_FFFF_FFFF;
    }
    if cognitive > 0.8 {
        // Focus on node zone (deep memory)
        filter_mask = 0xFFFF_FFFF_0000_0000;
    }

    JitScanParams {
        threshold,
        top_k,
        prefetch_ahead,
        record_size: 2048,
        filter_mask,
    }
}

/// Hash a 10-axis style vector for cache invalidation.
fn hash_style(style: &[f32; 10]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    // Quantize to 3 decimal places for stable hashing
    for &v in style {
        let quantized = (v * 1000.0).round() as i32;
        quantized.hash(&mut h);
    }
    h.finish()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_profile_from_module() {
        // SOC analyst: high recognition, appraisal, validation
        let style = [0.9, 0.2, 0.8, 0.5, 0.7, 0.95, 0.6, 0.85, 0.9, 0.75];
        let profile = JitProfile::from_module("soc:incident_response", style);

        assert_eq!(profile.card_id, "soc:incident_response");
        assert!(!profile.templates.is_empty());
        // Should have multiple clusters activated
        assert!(profile.templates.len() >= 2);
        // All templates should have valid τ addresses
        for t in &profile.templates {
            assert!(t.tau > 0);
            assert!(t.weight > 0.0);
        }
    }

    #[test]
    fn test_jit_profile_analytical_dominant() {
        // Pure analytical: high recognition, appraisal, validation
        let style = [1.0, 0.1, 0.9, 0.8, 0.3, 0.1, 0.1, 0.1, 0.9, 0.2];
        let profile = JitProfile::from_module("test:analytical", style);

        assert_eq!(profile.dominant_cluster, StyleCluster::Analytical);
    }

    #[test]
    fn test_jit_profile_creative_dominant() {
        // Creative: high resonance, execution, contingency
        let style = [0.1, 0.9, 0.1, 0.1, 0.9, 0.1, 0.9, 0.1, 0.1, 0.1];
        let profile = JitProfile::from_module("test:creative", style);

        assert_eq!(profile.dominant_cluster, StyleCluster::Creative);
    }

    #[test]
    fn test_tau_addresses() {
        let style = [0.5; 10];
        let profile = JitProfile::from_module("test:balanced", style);
        let taus = profile.tau_addresses();

        assert!(!taus.is_empty());
        // All τ addresses should be in valid ranges
        for &tau in &taus {
            assert!(
                (0x20..=0x25).contains(&tau)
                    || (0x40..=0x45).contains(&tau)
                    || (0x60..=0x65).contains(&tau)
                    || (0x80..=0x85).contains(&tau)
                    || (0xA0..=0xA5).contains(&tau)
                    || (0xC0..=0xC5).contains(&tau),
                "Invalid τ address: {:#x}",
                tau,
            );
        }
    }

    #[test]
    fn test_scan_params_depth_sensitivity() {
        let shallow_vec: HashMap<&str, f32> = [("depth", 0.1)].into_iter().collect();
        let deep_vec: HashMap<&str, f32> = [("depth", 0.9)].into_iter().collect();

        let shallow = sparse_vec_to_scan_params(&shallow_vec);
        let deep = sparse_vec_to_scan_params(&deep_vec);

        // Deeper depth = lower threshold (more selective)
        assert!(deep.threshold < shallow.threshold);
    }

    #[test]
    fn test_config_hash_stability() {
        let style = [0.5; 10];
        let h1 = hash_style(&style);
        let h2 = hash_style(&style);
        assert_eq!(h1, h2);

        let mut style2 = style;
        style2[0] = 0.5004; // Tiny change (rounds to same quantized value: 500)
        let h3 = hash_style(&style2);
        assert_eq!(h1, h3); // Same after quantization

        style2[0] = 0.6; // Significant change
        let h4 = hash_style(&style2);
        assert_ne!(h1, h4);
    }

    #[test]
    fn test_needs_recompile() {
        let style = [0.5; 10];
        let profile = JitProfile::from_module("test", style);

        assert!(!profile.needs_recompile(&style));

        let mut changed = style;
        changed[0] = 0.9;
        assert!(profile.needs_recompile(&changed));
    }
}
