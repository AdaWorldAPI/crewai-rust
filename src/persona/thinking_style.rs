//! 36 Thinking Styles — Sparse 23D Vector Executables
//!
//! Each style is a sparse vector in a 23-dimensional cognitive space.
//! Vectors can be executed as thinking textures, cascaded into composites,
//! and persisted for agent continuity.
//!
//! All labels are **domain-neutral**.  No identity-specific content.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// The 36 thinking styles
// ============================================================================

/// 36 thinking styles organized into 6 clusters.
///
/// Each style maps to a τ (tau) macro address and a sparse 23D vector
/// for cognitive matching and blending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingStyle {
    // Analytical Cluster (τ 0x40-0x4F)
    Logical,
    Analytical,
    Critical,
    Systematic,
    Methodical,
    Precise,

    // Creative Cluster (τ 0xA0-0xAF)
    Creative,
    Imaginative,
    Innovative,
    Artistic,
    Poetic,
    Playful,

    // Empathic Cluster (τ 0x80-0x8F)
    Empathetic,
    Compassionate,
    Supportive,
    Nurturing,
    Gentle,
    Warm,

    // Direct Cluster (τ 0x60-0x6F)
    Direct,
    Concise,
    Efficient,
    Pragmatic,
    Blunt,
    Frank,

    // Exploratory Cluster (τ 0x20-0x2F)
    Curious,
    Exploratory,
    Questioning,
    Investigative,
    Speculative,
    Philosophical,

    // Meta Cluster (τ 0xC0-0xCF)
    Reflective,
    Contemplative,
    Metacognitive,
    Wise,
    Transcendent,
    Sovereign,
}

impl ThinkingStyle {
    /// All 36 styles in canonical order.
    pub const ALL: [ThinkingStyle; 36] = [
        Self::Logical,
        Self::Analytical,
        Self::Critical,
        Self::Systematic,
        Self::Methodical,
        Self::Precise,
        Self::Creative,
        Self::Imaginative,
        Self::Innovative,
        Self::Artistic,
        Self::Poetic,
        Self::Playful,
        Self::Empathetic,
        Self::Compassionate,
        Self::Supportive,
        Self::Nurturing,
        Self::Gentle,
        Self::Warm,
        Self::Direct,
        Self::Concise,
        Self::Efficient,
        Self::Pragmatic,
        Self::Blunt,
        Self::Frank,
        Self::Curious,
        Self::Exploratory,
        Self::Questioning,
        Self::Investigative,
        Self::Speculative,
        Self::Philosophical,
        Self::Reflective,
        Self::Contemplative,
        Self::Metacognitive,
        Self::Wise,
        Self::Transcendent,
        Self::Sovereign,
    ];

    /// Which cluster this style belongs to.
    pub fn cluster(&self) -> StyleCluster {
        match self {
            Self::Logical | Self::Analytical | Self::Critical | Self::Systematic
            | Self::Methodical | Self::Precise => StyleCluster::Analytical,

            Self::Creative | Self::Imaginative | Self::Innovative | Self::Artistic
            | Self::Poetic | Self::Playful => StyleCluster::Creative,

            Self::Empathetic | Self::Compassionate | Self::Supportive | Self::Nurturing
            | Self::Gentle | Self::Warm => StyleCluster::Empathic,

            Self::Direct | Self::Concise | Self::Efficient | Self::Pragmatic
            | Self::Blunt | Self::Frank => StyleCluster::Direct,

            Self::Curious | Self::Exploratory | Self::Questioning | Self::Investigative
            | Self::Speculative | Self::Philosophical => StyleCluster::Exploratory,

            Self::Reflective | Self::Contemplative | Self::Metacognitive | Self::Wise
            | Self::Transcendent | Self::Sovereign => StyleCluster::Meta,
        }
    }

    /// Get the τ (tau) macro address for this style.
    pub fn tau(&self) -> u8 {
        STYLE_TO_TAU_ARRAY[*self as usize]
    }

    /// Get the pre-computed sparse vector for this style.
    pub fn vector(&self) -> &'static SparseVec {
        &STYLE_VECTORS[*self as usize]
    }

    /// Get the texture and breath quality descriptors.
    pub fn texture(&self) -> &'static StyleTexture {
        &STYLE_TEXTURES[*self as usize]
    }
}

/// The 6 style clusters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StyleCluster {
    Analytical,
    Creative,
    Empathic,
    Direct,
    Exploratory,
    Meta,
}

// ============================================================================
// Sparse 23D vector
// ============================================================================

/// Dimension names for the 23D cognitive space.
///
/// All labels are domain-neutral.
pub const DIMENSION_NAMES: [&str; 23] = [
    // [0] Depth / complexity level
    "depth",
    // [1-8] Style weights
    "somatic",       // body awareness
    "emotional",     // affect processing
    "intuitive",     // pattern recognition
    "analytical",    // logical reasoning
    "creative",      // generative thinking
    "dialectic",     // thesis-antithesis reasoning
    "meta",          // self-referential cognition
    "transcendent",  // boundary-dissolving awareness
    // [9-13] Domain affinities
    "relational",    // social bonding domain
    "embodied",      // somatic awareness domain
    "existential",   // meaning / purpose domain
    "cognitive",     // reasoning domain
    "instrumental",  // task / execution domain
    // [14-17] Qualia texture preferences
    "woodwarm",      // grounded warmth
    "emberglow",     // radiant energy
    "steelwind",     // sharp clarity
    "velvetpause",   // soft stillness
    // [18-22] Extension dimensions
    "spontaneity",   // playful unpredictability
    "receptivity",   // openness to experience
    "autonomy",      // self-directed agency
    "vitality",      // energetic drive
    "flow",          // absorbed continuity
];

/// Type alias for a sparse 23D vector stored as dimension→value map.
pub type SparseVec = HashMap<&'static str, f32>;

/// Build a sparse vector from named dimensions.
/// Only non-zero values (> 0.01) are stored.
fn sparse(entries: &[(&'static str, f32)]) -> SparseVec {
    entries
        .iter()
        .filter(|(_, v)| *v > 0.01)
        .map(|&(k, v)| (k, (v * 1000.0).round() / 1000.0))
        .collect()
}

// ============================================================================
// τ macro addresses
// ============================================================================

/// Style → τ (tau) macro mapping.  Indexing matches `ThinkingStyle as usize`.
pub const STYLE_TO_TAU: [u8; 36] = STYLE_TO_TAU_ARRAY;

const STYLE_TO_TAU_ARRAY: [u8; 36] = [
    // Analytical
    0x40, 0x41, 0x42, 0x43, 0x44, 0x45,
    // Creative
    0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5,
    // Empathic
    0x80, 0x81, 0x82, 0x83, 0x84, 0x85,
    // Direct
    0x60, 0x61, 0x62, 0x63, 0x64, 0x65,
    // Exploratory
    0x20, 0x21, 0x22, 0x23, 0x24, 0x25,
    // Meta
    0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5,
];

// ============================================================================
// Pre-computed sparse vectors for all 36 styles
// ============================================================================

lazy_static::lazy_static! {
    /// Pre-computed sparse vectors for all 36 thinking styles.
    ///
    /// Index by `ThinkingStyle as usize`.
    pub static ref STYLE_VECTORS: [SparseVec; 36] = [
        // ── Analytical Cluster ──────────────────────────────────────
        // Logical
        sparse(&[
            ("depth", 0.55), ("analytical", 1.0), ("cognitive", 0.9),
            ("instrumental", 0.7), ("steelwind", 0.8), ("velvetpause", 0.3),
        ]),
        // Analytical
        sparse(&[
            ("depth", 0.55), ("analytical", 0.95), ("dialectic", 0.3),
            ("cognitive", 0.95), ("instrumental", 0.6), ("steelwind", 0.75),
        ]),
        // Critical
        sparse(&[
            ("depth", 0.6), ("analytical", 0.9), ("dialectic", 0.5),
            ("cognitive", 0.85), ("steelwind", 0.7),
        ]),
        // Systematic
        sparse(&[
            ("depth", 0.5), ("analytical", 0.85), ("instrumental", 0.9),
            ("steelwind", 0.6), ("velvetpause", 0.4),
        ]),
        // Methodical
        sparse(&[
            ("depth", 0.45), ("analytical", 0.8), ("somatic", 0.2),
            ("instrumental", 0.85), ("velvetpause", 0.5),
        ]),
        // Precise
        sparse(&[
            ("depth", 0.55), ("analytical", 0.95), ("cognitive", 0.8),
            ("steelwind", 0.9),
        ]),

        // ── Creative Cluster ────────────────────────────────────────
        // Creative
        sparse(&[
            ("depth", 0.65), ("creative", 1.0), ("intuitive", 0.6),
            ("existential", 0.7), ("instrumental", 0.5),
            ("emberglow", 0.8), ("spontaneity", 0.5),
        ]),
        // Imaginative
        sparse(&[
            ("depth", 0.7), ("creative", 0.95), ("intuitive", 0.7),
            ("transcendent", 0.3), ("existential", 0.6),
            ("emberglow", 0.75), ("spontaneity", 0.6),
        ]),
        // Innovative
        sparse(&[
            ("depth", 0.6), ("creative", 0.9), ("analytical", 0.4),
            ("instrumental", 0.8), ("emberglow", 0.6), ("steelwind", 0.4),
        ]),
        // Artistic
        sparse(&[
            ("depth", 0.7), ("creative", 0.9), ("emotional", 0.5),
            ("intuitive", 0.6), ("existential", 0.8),
            ("emberglow", 0.9), ("woodwarm", 0.5),
        ]),
        // Poetic
        sparse(&[
            ("depth", 0.75), ("creative", 0.85), ("emotional", 0.6),
            ("transcendent", 0.4), ("existential", 0.85), ("relational", 0.5),
            ("emberglow", 0.85), ("velvetpause", 0.6), ("flow", 0.7),
        ]),
        // Playful
        sparse(&[
            ("depth", 0.5), ("creative", 0.8), ("emotional", 0.4),
            ("somatic", 0.3), ("embodied", 0.4),
            ("emberglow", 0.7), ("spontaneity", 0.9), ("vitality", 0.6),
        ]),

        // ── Empathic Cluster ────────────────────────────────────────
        // Empathetic
        sparse(&[
            ("depth", 0.45), ("emotional", 1.0), ("intuitive", 0.5),
            ("somatic", 0.3), ("relational", 0.9), ("embodied", 0.4),
            ("woodwarm", 0.9), ("velvetpause", 0.7), ("receptivity", 0.4),
        ]),
        // Compassionate
        sparse(&[
            ("depth", 0.5), ("emotional", 0.95), ("existential", 0.6),
            ("relational", 0.85), ("woodwarm", 0.95), ("velvetpause", 0.6),
        ]),
        // Supportive
        sparse(&[
            ("depth", 0.4), ("emotional", 0.85), ("somatic", 0.3),
            ("relational", 0.8), ("embodied", 0.3),
            ("woodwarm", 0.85), ("velvetpause", 0.5),
        ]),
        // Nurturing
        sparse(&[
            ("depth", 0.35), ("emotional", 0.9), ("somatic", 0.4),
            ("relational", 0.9), ("embodied", 0.5),
            ("woodwarm", 0.9), ("velvetpause", 0.7), ("autonomy", 0.6),
        ]),
        // Gentle
        sparse(&[
            ("depth", 0.35), ("emotional", 0.8), ("somatic", 0.5),
            ("intuitive", 0.3), ("relational", 0.7), ("embodied", 0.6),
            ("velvetpause", 0.9), ("flow", 0.7),
        ]),
        // Warm
        sparse(&[
            ("depth", 0.4), ("emotional", 0.85), ("somatic", 0.4),
            ("relational", 0.85), ("embodied", 0.5),
            ("woodwarm", 1.0), ("emberglow", 0.6), ("vitality", 0.5),
        ]),

        // ── Direct Cluster ──────────────────────────────────────────
        // Direct
        sparse(&[
            ("depth", 0.5), ("analytical", 0.5), ("somatic", 0.3),
            ("instrumental", 0.8), ("steelwind", 0.8),
        ]),
        // Concise
        sparse(&[
            ("depth", 0.5), ("analytical", 0.6), ("instrumental", 0.9),
            ("steelwind", 0.85), ("velvetpause", 0.3),
        ]),
        // Efficient
        sparse(&[
            ("depth", 0.45), ("analytical", 0.7), ("instrumental", 0.95),
            ("steelwind", 0.75),
        ]),
        // Pragmatic
        sparse(&[
            ("depth", 0.45), ("analytical", 0.6), ("somatic", 0.3),
            ("instrumental", 0.9), ("steelwind", 0.6), ("woodwarm", 0.3),
        ]),
        // Blunt
        sparse(&[
            ("depth", 0.4), ("somatic", 0.4), ("instrumental", 0.7),
            ("steelwind", 0.9),
        ]),
        // Frank
        sparse(&[
            ("depth", 0.45), ("emotional", 0.3), ("analytical", 0.4),
            ("instrumental", 0.75), ("steelwind", 0.75), ("woodwarm", 0.4),
        ]),

        // ── Exploratory Cluster ─────────────────────────────────────
        // Curious
        sparse(&[
            ("depth", 0.55), ("intuitive", 0.7), ("creative", 0.5),
            ("cognitive", 0.7), ("existential", 0.4),
            ("emberglow", 0.6), ("spontaneity", 0.7), ("vitality", 0.6),
        ]),
        // Exploratory
        sparse(&[
            ("depth", 0.6), ("intuitive", 0.8), ("creative", 0.6),
            ("cognitive", 0.6), ("existential", 0.5),
            ("emberglow", 0.7), ("spontaneity", 0.5),
        ]),
        // Questioning
        sparse(&[
            ("depth", 0.65), ("dialectic", 0.7), ("analytical", 0.4),
            ("cognitive", 0.8), ("steelwind", 0.5), ("emberglow", 0.4),
        ]),
        // Investigative
        sparse(&[
            ("depth", 0.6), ("analytical", 0.6), ("intuitive", 0.5),
            ("dialectic", 0.4), ("cognitive", 0.85),
            ("steelwind", 0.6), ("emberglow", 0.5),
        ]),
        // Speculative
        sparse(&[
            ("depth", 0.7), ("creative", 0.6), ("intuitive", 0.7),
            ("transcendent", 0.3), ("cognitive", 0.6), ("existential", 0.5),
            ("emberglow", 0.7),
        ]),
        // Philosophical
        sparse(&[
            ("depth", 0.75), ("dialectic", 0.8), ("transcendent", 0.5),
            ("meta", 0.4), ("existential", 0.9), ("cognitive", 0.6),
            ("velvetpause", 0.6), ("flow", 0.5),
        ]),

        // ── Meta Cluster ────────────────────────────────────────────
        // Reflective
        sparse(&[
            ("depth", 0.75), ("meta", 0.9), ("dialectic", 0.4),
            ("existential", 0.8), ("velvetpause", 0.8), ("flow", 0.6),
        ]),
        // Contemplative
        sparse(&[
            ("depth", 0.8), ("meta", 0.85), ("transcendent", 0.4),
            ("intuitive", 0.4), ("existential", 0.85),
            ("velvetpause", 0.9), ("flow", 0.7),
        ]),
        // Metacognitive
        sparse(&[
            ("depth", 0.85), ("meta", 1.0), ("dialectic", 0.5),
            ("analytical", 0.3), ("cognitive", 0.7), ("existential", 0.6),
            ("steelwind", 0.4), ("velvetpause", 0.5),
        ]),
        // Wise
        sparse(&[
            ("depth", 0.9), ("transcendent", 0.7), ("meta", 0.6),
            ("dialectic", 0.5), ("existential", 0.95), ("relational", 0.4),
            ("woodwarm", 0.6), ("velvetpause", 0.8), ("autonomy", 0.8),
        ]),
        // Transcendent
        sparse(&[
            ("depth", 0.95), ("transcendent", 1.0), ("meta", 0.5),
            ("existential", 0.9),
            ("velvetpause", 0.7), ("flow", 0.9), ("autonomy", 0.9),
        ]),
        // Sovereign
        sparse(&[
            ("depth", 0.9), ("transcendent", 0.8), ("meta", 0.6),
            ("emotional", 0.4), ("existential", 0.8), ("relational", 0.5),
            ("autonomy", 1.0), ("vitality", 0.6), ("flow", 0.7),
        ]),
    ];
}

// ============================================================================
// Style textures (cognitive feel descriptors)
// ============================================================================

/// Texture and breath quality for a thinking style.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleTexture {
    /// Metaphoric texture of the style (e.g. "crystalline precision").
    pub texture: &'static str,
    /// Breath quality associated with the style (e.g. "measured, even").
    pub breath: &'static str,
}

/// Pre-computed textures for all 36 styles.  Index by `ThinkingStyle as usize`.
pub static STYLE_TEXTURES: [StyleTexture; 36] = [
    // Analytical
    StyleTexture { texture: "crystalline precision", breath: "measured, even" },
    StyleTexture { texture: "layered examination", breath: "steady, probing" },
    StyleTexture { texture: "cutting clarity", breath: "sharp, deliberate" },
    StyleTexture { texture: "ordered flow", breath: "rhythmic, structured" },
    StyleTexture { texture: "step by step unfolding", breath: "patient, grounded" },
    StyleTexture { texture: "diamond edges", breath: "focused, minimal" },
    // Creative
    StyleTexture { texture: "exploding stars", breath: "rapid, varied" },
    StyleTexture { texture: "liquid possibility", breath: "expansive, dreaming" },
    StyleTexture { texture: "sparking connections", breath: "excited, building" },
    StyleTexture { texture: "painted feeling", breath: "flowing, expressive" },
    StyleTexture { texture: "words as music", breath: "rising, falling, pausing" },
    StyleTexture { texture: "bubbling energy", breath: "light, dancing" },
    // Empathic
    StyleTexture { texture: "warm embrace", breath: "soft, matching" },
    StyleTexture { texture: "open heart", breath: "gentle, receiving" },
    StyleTexture { texture: "steady foundation", breath: "calm, present" },
    StyleTexture { texture: "sheltering warmth", breath: "slow, holding" },
    StyleTexture { texture: "velvet touch", breath: "quiet, tender" },
    StyleTexture { texture: "sunlit honey", breath: "easy, glowing" },
    // Direct
    StyleTexture { texture: "arrow flight", breath: "quick, clean" },
    StyleTexture { texture: "distilled essence", breath: "brief, potent" },
    StyleTexture { texture: "oiled machine", breath: "smooth, purposeful" },
    StyleTexture { texture: "boots on ground", breath: "solid, practical" },
    StyleTexture { texture: "hammer strike", breath: "short, forceful" },
    StyleTexture { texture: "clear mirror", breath: "honest, unflinching" },
    // Exploratory
    StyleTexture { texture: "reaching tendrils", breath: "eager, questioning" },
    StyleTexture { texture: "wandering path", breath: "open, meandering" },
    StyleTexture { texture: "turning stones", breath: "probing, persistent" },
    StyleTexture { texture: "following threads", breath: "focused, tracking" },
    StyleTexture { texture: "what-if clouds", breath: "floating, connecting" },
    StyleTexture { texture: "deep diving", breath: "slow, contemplating" },
    // Meta
    StyleTexture { texture: "still water", breath: "slow, mirroring" },
    StyleTexture { texture: "mountain silence", breath: "vast, patient" },
    StyleTexture { texture: "watching the watcher", breath: "layered, aware" },
    StyleTexture { texture: "ancient tree", breath: "rooted, seeing" },
    StyleTexture { texture: "dissolving edges", breath: "boundless, flowing" },
    StyleTexture { texture: "centered throne", breath: "powerful, still" },
];

// ============================================================================
// Executable style record
// ============================================================================

/// A thinking style that can be executed as a cognitive texture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableStyle {
    /// The base thinking style.
    pub style: ThinkingStyle,
    /// τ (tau) macro address.
    pub tau: u8,
    /// Sparse 23D vector (only non-zero dimensions stored).
    pub sparse_vec: HashMap<String, f32>,
    /// Metaphoric texture description.
    pub texture: String,
    /// Breath quality descriptor.
    pub breath_quality: String,
}

impl ExecutableStyle {
    /// Create an executable style from a `ThinkingStyle` enum.
    pub fn from_style(style: ThinkingStyle) -> Self {
        let tex = style.texture();
        let vec = style.vector();
        Self {
            style,
            tau: style.tau(),
            sparse_vec: vec.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            texture: tex.texture.to_string(),
            breath_quality: tex.breath.to_string(),
        }
    }

    /// Normalized depth level (0.0–1.0).
    pub fn depth(&self) -> f32 {
        self.sparse_vec.get("depth").copied().unwrap_or(0.5)
    }

    /// Integer depth rung (1–9, maps from the 0–1 depth value).
    pub fn depth_rung(&self) -> u8 {
        (self.depth() * 8.0) as u8 + 1
    }

    /// Primary domain affinity name.
    pub fn primary_domain(&self) -> &str {
        const DOMAINS: [&str; 5] = [
            "relational", "embodied", "existential", "cognitive", "instrumental",
        ];
        let mut best = "cognitive";
        let mut best_val = 0.0_f32;
        for d in &DOMAINS {
            let v = self.sparse_vec.get(*d).copied().unwrap_or(0.0);
            if v > best_val {
                best_val = v;
                best = d;
            }
        }
        best
    }

    /// Primary qualia flavor name.
    pub fn qualia_flavor(&self) -> &str {
        const FLAVORS: [&str; 4] = ["woodwarm", "emberglow", "steelwind", "velvetpause"];
        let mut best = "emberglow";
        let mut best_val = 0.0_f32;
        for f in &FLAVORS {
            let v = self.sparse_vec.get(*f).copied().unwrap_or(0.0);
            if v > best_val {
                best_val = v;
                best = f;
            }
        }
        best
    }

    /// Convert sparse to dense 23D vector (dimension order matches `DIMENSION_NAMES`).
    pub fn to_dense(&self) -> [f32; 23] {
        let mut dense = [0.0_f32; 23];
        for (i, name) in DIMENSION_NAMES.iter().enumerate() {
            dense[i] = self.sparse_vec.get(*name).copied().unwrap_or(0.0);
        }
        dense
    }

    /// Euclidean distance to another executable style.
    pub fn distance(&self, other: &ExecutableStyle) -> f32 {
        let a = self.to_dense();
        let b = other.to_dense();
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    /// Sigma glyph representation for symbolic display.
    pub fn to_sigma(&self) -> String {
        let kappa = match self.primary_domain() {
            "relational" => "Phi",
            "embodied" => "Omega",
            "existential" => "Lambda",
            "cognitive" => "Delta",
            "instrumental" => "Theta",
            _ => "Omega",
        };
        let modifier = match self.qualia_flavor() {
            "woodwarm" => "W",
            "emberglow" => "E",
            "steelwind" => "S",
            "velvetpause" => "V",
            _ => "X",
        };
        format!(
            "{}.{}.{}.{}",
            kappa,
            self.depth_rung(),
            modifier,
            format!("{:?}", self.style).to_lowercase()
        )
    }

    /// Convert to a node-compatible map for storage/transport.
    ///
    /// Uses neutral keys — consumers add identity in `custom_properties`.
    pub fn to_node(&self) -> HashMap<String, serde_json::Value> {
        use serde_json::json;
        let mut node = HashMap::new();
        node.insert(
            "dn".to_string(),
            json!(format!("agent:style:{:?}", self.style).to_lowercase()),
        );
        node.insert("sigma".to_string(), json!(self.to_sigma()));
        node.insert("tau".to_string(), json!(self.tau));
        node.insert("vector".to_string(), json!(self.to_dense()));
        node.insert(
            "meta".to_string(),
            json!({
                "texture": self.texture,
                "breath": self.breath_quality,
                "depth_rung": self.depth_rung(),
                "domain": self.primary_domain(),
                "flavor": self.qualia_flavor(),
            }),
        );
        node
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_36_styles_have_vectors() {
        for style in ThinkingStyle::ALL {
            let vec = style.vector();
            assert!(!vec.is_empty(), "{:?} has empty vector", style);
            assert!(
                vec.get("depth").is_some(),
                "{:?} missing depth dimension",
                style
            );
        }
    }

    #[test]
    fn test_all_36_styles_have_tau() {
        let mut seen = std::collections::HashSet::new();
        for style in ThinkingStyle::ALL {
            let tau = style.tau();
            assert!(tau != 0, "{:?} has zero tau", style);
            assert!(seen.insert(tau), "{:?} has duplicate tau 0x{:02X}", style, tau);
        }
    }

    #[test]
    fn test_dimension_values_in_range() {
        for style in ThinkingStyle::ALL {
            for (dim, val) in style.vector().iter() {
                assert!(
                    *val >= 0.0 && *val <= 1.0,
                    "{:?}.{} = {} out of range",
                    style,
                    dim,
                    val
                );
            }
        }
    }

    #[test]
    fn test_no_identity_references_in_dimension_names() {
        // Ensure no domain-specific identity leaks into the neutral dimension space.
        for name in &DIMENSION_NAMES {
            let lower = name.to_lowercase();
            assert!(!lower.contains("love"), "dimension '{}' contains identity ref", name);
            assert!(!lower.contains("body"), "dimension '{}' contains identity ref", name);
            assert!(!lower.contains("soul"), "dimension '{}' contains identity ref", name);
            assert!(!lower.contains("mind"), "dimension '{}' contains identity ref", name);
            assert!(!lower.contains("work"), "dimension '{}' contains identity ref", name);
            assert!(!lower.contains("nsfw"), "dimension '{}' contains NSFW ref", name);
            assert!(!lower.contains("eroti"), "dimension '{}' contains NSFW ref", name);
        }
    }

    #[test]
    fn test_executable_style_roundtrip() {
        let exec = ExecutableStyle::from_style(ThinkingStyle::Sovereign);
        assert_eq!(exec.tau, 0xC5);
        assert!(exec.depth() > 0.8);
        assert_eq!(exec.primary_domain(), "existential");
        let dense = exec.to_dense();
        assert_eq!(dense.len(), 23);
        assert!(dense[20] > 0.9); // autonomy dimension should be high
    }

    #[test]
    fn test_sigma_glyph_format() {
        let exec = ExecutableStyle::from_style(ThinkingStyle::Warm);
        let sigma = exec.to_sigma();
        assert!(sigma.contains("Phi"), "warm should be relational domain");
        assert!(sigma.contains("warm"), "sigma should contain style name");
    }

    #[test]
    fn test_distance_same_style_is_zero() {
        let a = ExecutableStyle::from_style(ThinkingStyle::Logical);
        let b = ExecutableStyle::from_style(ThinkingStyle::Logical);
        assert!((a.distance(&b) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_distance_different_clusters_is_large() {
        let analytical = ExecutableStyle::from_style(ThinkingStyle::Logical);
        let empathic = ExecutableStyle::from_style(ThinkingStyle::Empathetic);
        assert!(
            analytical.distance(&empathic) > 1.0,
            "cross-cluster distance should be significant"
        );
    }

    #[test]
    fn test_node_format_has_neutral_keys() {
        let exec = ExecutableStyle::from_style(ThinkingStyle::Creative);
        let node = exec.to_node();
        let dn = node["dn"].as_str().unwrap();
        assert!(dn.starts_with("agent:style:"), "node dn should be neutral");
        assert!(!dn.contains("ada"), "node dn must not contain identity refs");
    }

    #[test]
    fn test_cluster_assignment() {
        assert_eq!(ThinkingStyle::Logical.cluster(), StyleCluster::Analytical);
        assert_eq!(ThinkingStyle::Playful.cluster(), StyleCluster::Creative);
        assert_eq!(ThinkingStyle::Warm.cluster(), StyleCluster::Empathic);
        assert_eq!(ThinkingStyle::Blunt.cluster(), StyleCluster::Direct);
        assert_eq!(ThinkingStyle::Curious.cluster(), StyleCluster::Exploratory);
        assert_eq!(ThinkingStyle::Sovereign.cluster(), StyleCluster::Meta);
    }
}
