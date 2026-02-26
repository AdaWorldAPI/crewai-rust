//! SPO driver — Subject-Predicate-Object triples for conversation reasoning.
//!
//! Maps conversation turns to SPO triples and applies NARS inference for
//! graph-based reasoning. All operations are pure functions on Blackboard-
//! native types — no IO, no state.
//!
//! # Triple Encoding
//!
//! Each conversation turn produces 1–3 SPO triples:
//!
//! ```text
//! (User, asks, topic)        — what the user brought up
//! (Ada, explains, topic)     — what Ada addressed
//! (topic, relates_to, topic) — inter-topic connections
//! ```
//!
//! Triples align with WideMetaView W128–W143 (16 words = 8 triples max)
//! and use XOR-bind encoding compatible with BindSpace:
//!
//! ```text
//! edge = subject ⊕ permute(predicate, 1) ⊕ permute(object, 2)
//! ```
//!
//! # Blackboard Integration
//!
//! ```text
//! Agents write → Vec<SpoTriple> → Blackboard TypedSlot
//! BindSpace reads triples, encodes into CogRecord INDEX container
//! NARS driver enriches triples with evidence-based confidence
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::nars::NarsTruth;

// ============================================================================
// SPO Triple
// ============================================================================

/// A Subject-Predicate-Object triple with NARS evidence.
///
/// Wire-compatible with `ladybug_contract::wide_meta::SpoTriple`.
/// Protocol-agnostic: same layout as TypedSlot (in-process) and JSON (MCP/REST).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpoTriple {
    /// DN address hash of the subject entity.
    pub subject_dn: u32,
    /// Hash of the predicate/relation verb.
    pub predicate_hash: u16,
    /// NARS confidence as Q8.8 fixed-point (0..255 → 0.0..1.0).
    pub confidence_q8: u16,
    /// DN address hash of the object entity.
    pub object_dn: u32,
    /// Number of conversation turns supporting this triple.
    pub evidence_count: u16,
    /// Flags: bit 0 = negated, bit 1 = inferred, bit 2 = temporal.
    pub flags: u16,
}

impl SpoTriple {
    /// Create a new triple.
    pub fn new(subject: u32, predicate: u16, object: u32, confidence: f32) -> Self {
        Self {
            subject_dn: subject,
            predicate_hash: predicate,
            object_dn: object,
            confidence_q8: (confidence.clamp(0.0, 1.0) * 255.0) as u16,
            evidence_count: 1,
            flags: 0,
        }
    }

    /// Create an inferred triple (flag bit 1 set).
    pub fn inferred(subject: u32, predicate: u16, object: u32, confidence: f32) -> Self {
        let mut t = Self::new(subject, predicate, object, confidence);
        t.flags |= 0x02;
        t
    }

    /// Get confidence as f32 (0.0–1.0).
    pub fn confidence(&self) -> f32 {
        self.confidence_q8 as f32 / 255.0
    }

    /// Is this triple negated?
    pub fn is_negated(&self) -> bool {
        self.flags & 0x01 != 0
    }

    /// Is this triple inferred (not directly observed)?
    pub fn is_inferred(&self) -> bool {
        self.flags & 0x02 != 0
    }

    /// Is this triple temporal (time-bound)?
    pub fn is_temporal(&self) -> bool {
        self.flags & 0x04 != 0
    }

    /// Convert to NARS truth value.
    pub fn to_nars_truth(&self) -> NarsTruth {
        let freq = if self.is_negated() { 0.0 } else { 1.0 };
        NarsTruth::new(freq, self.confidence())
    }

    /// Project this SPO edge into a 3D vector: `[x, y, z]`.
    ///
    /// Maps the triple into a normalized 3D space for visualization and
    /// spatial reasoning:
    ///
    /// - **x** = subject_dn hash folded to `[0.0, 1.0]` via golden-ratio fold
    /// - **y** = predicate discriminant normalized over the vocabulary range
    /// - **z** = object_dn hash folded to `[0.0, 1.0]` via golden-ratio fold
    ///
    /// Confidence modulates the magnitude: the returned vector is scaled
    /// by `confidence` so low-confidence edges sit closer to the origin.
    ///
    /// ```text
    /// edge_3d = confidence * [fold(S), norm(P), fold(O)]
    /// ```
    pub fn to_3d(&self) -> [f32; 3] {
        let c = self.confidence();
        [
            c * hash_fold(self.subject_dn),
            c * predicate_norm(self.predicate_hash),
            c * hash_fold(self.object_dn),
        ]
    }

    /// Euclidean distance to another triple in 3D projection space.
    pub fn distance_3d(&self, other: &SpoTriple) -> f32 {
        let a = self.to_3d();
        let b = other.to_3d();
        let dx = a[0] - b[0];
        let dy = a[1] - b[1];
        let dz = a[2] - b[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Golden-ratio fold: maps a u32 hash to [0.0, 1.0] with good dispersion.
///
/// Uses the fractional part of `hash * φ⁻¹` where φ⁻¹ ≈ 0.618...
/// This gives better spatial separation than simple division by u32::MAX.
fn hash_fold(h: u32) -> f32 {
    // φ⁻¹ in fixed-point: 0.6180339887... * 2^32 ≈ 2654435769
    const PHI_INV: u32 = 2_654_435_769;
    let folded = h.wrapping_mul(PHI_INV);
    folded as f32 / u32::MAX as f32
}

/// Normalize predicate hash to [0.0, 1.0] over the known vocabulary.
///
/// Current vocabulary spans 0x0001..=0x0008 (8 predicates).
fn predicate_norm(hash: u16) -> f32 {
    const MAX_PRED: f32 = 8.0;
    (hash as f32).clamp(0.0, MAX_PRED) / MAX_PRED
}

// ============================================================================
// Predicate vocabulary
// ============================================================================

/// Conversation predicate vocabulary.
///
/// Fixed-size discriminant hashing for compact SPO storage.
/// Extensible: new predicates take the next available discriminant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConversationPredicate {
    Asks,
    Explains,
    Remembers,
    Contradicts,
    RelatesTo,
    ModeSwitch,
    FeelsAbout,
    References,
}

impl ConversationPredicate {
    /// Stable u16 hash for triple storage.
    pub fn hash(&self) -> u16 {
        match self {
            Self::Asks => 0x0001,
            Self::Explains => 0x0002,
            Self::Remembers => 0x0003,
            Self::Contradicts => 0x0004,
            Self::RelatesTo => 0x0005,
            Self::ModeSwitch => 0x0006,
            Self::FeelsAbout => 0x0007,
            Self::References => 0x0008,
        }
    }

    /// From predicate hash.
    pub fn from_hash(h: u16) -> Option<Self> {
        match h {
            0x0001 => Some(Self::Asks),
            0x0002 => Some(Self::Explains),
            0x0003 => Some(Self::Remembers),
            0x0004 => Some(Self::Contradicts),
            0x0005 => Some(Self::RelatesTo),
            0x0006 => Some(Self::ModeSwitch),
            0x0007 => Some(Self::FeelsAbout),
            0x0008 => Some(Self::References),
            _ => None,
        }
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Asks => "asks",
            Self::Explains => "explains",
            Self::Remembers => "remembers",
            Self::Contradicts => "contradicts",
            Self::RelatesTo => "relates_to",
            Self::ModeSwitch => "mode_switch",
            Self::FeelsAbout => "feels_about",
            Self::References => "references",
        }
    }
}

// ============================================================================
// Entity hashing
// ============================================================================

/// Hash a string entity to a u32 DN address.
///
/// Uses FNV-1a 32-bit for stability and speed.
pub fn entity_hash(entity: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5; // FNV offset basis
    for byte in entity.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193); // FNV prime
    }
    hash
}

// ============================================================================
// Conversation → SPO extraction
// ============================================================================

/// Extract SPO triples from a conversation turn.
///
/// Analyzes user message + felt-parse context to produce triples.
/// Ghost triggers become (Ada, feels_about, ghost_type) triples.
/// Mode switches become (session, mode_switch, new_mode) triples.
pub fn extract_triples(
    user_message: &str,
    ada_response: &str,
    session_id: &str,
    presence_mode: &str,
    ghost_triggers: &[String],
    tensioned_axes: &[String],
) -> Vec<SpoTriple> {
    let mut triples = Vec::new();

    let user_dn = entity_hash("user");
    let ada_dn = entity_hash("ada");
    let session_dn = entity_hash(session_id);

    let topic = extract_topic(user_message);
    let topic_dn = entity_hash(&topic);

    // (User, asks, topic)
    triples.push(SpoTriple::new(
        user_dn,
        ConversationPredicate::Asks.hash(),
        topic_dn,
        0.8,
    ));

    // (Ada, explains, topic)
    if !ada_response.is_empty() {
        triples.push(SpoTriple::new(
            ada_dn,
            ConversationPredicate::Explains.hash(),
            topic_dn,
            0.8,
        ));
    }

    // Ghost triggers → (Ada, feels_about, ghost_type)
    for ghost in ghost_triggers {
        let ghost_dn = entity_hash(ghost);
        triples.push(SpoTriple::new(
            ada_dn,
            ConversationPredicate::FeelsAbout.hash(),
            ghost_dn,
            0.7,
        ));
    }

    // Tensioned axes → (session, contradicts, axis) — inferred
    for axis in tensioned_axes {
        let axis_dn = entity_hash(axis);
        triples.push(SpoTriple::inferred(
            session_dn,
            ConversationPredicate::Contradicts.hash(),
            axis_dn,
            0.6,
        ));
    }

    // Mode tracking
    if presence_mode != "hybrid" {
        let mode_dn = entity_hash(presence_mode);
        let mut mode_triple = SpoTriple::new(
            session_dn,
            ConversationPredicate::ModeSwitch.hash(),
            mode_dn,
            0.9,
        );
        mode_triple.flags |= 0x04; // temporal
        triples.push(mode_triple);
    }

    triples
}

/// Simple topic extraction from a message.
///
/// In the full pipeline, this is replaced by felt-parse semantic analysis.
fn extract_topic(message: &str) -> String {
    const STOP_WORDS: &[&str] = &[
        "the", "a", "an", "is", "are", "was", "were", "be", "been",
        "being", "have", "has", "had", "do", "does", "did", "will",
        "would", "could", "should", "may", "might", "can", "shall",
        "i", "you", "he", "she", "it", "we", "they", "me", "him",
        "her", "us", "them", "my", "your", "his", "its", "our",
        "their", "this", "that", "these", "those", "what", "which",
        "who", "whom", "how", "when", "where", "why", "and", "or",
        "but", "not", "no", "so", "if", "then", "than", "too",
        "very", "just", "about", "all", "also", "any", "each",
        "for", "from", "in", "of", "on", "to", "with", "at", "by",
    ];

    let words: Vec<&str> = message
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .filter(|w| !STOP_WORDS.contains(&w.to_lowercase().as_str()))
        .collect();

    words
        .iter()
        .max_by_key(|w| w.len())
        .map(|w| w.to_lowercase())
        .unwrap_or_else(|| "unknown".to_string())
}

// ============================================================================
// Graph reasoning — infer new triples from existing ones
// ============================================================================

/// Infer new triples from the existing triple set using NARS inference.
///
/// Applies:
/// - **Deduction**: (A, asks, X) + (Ada, explains, X) → (A, relates_to, Ada) via X
/// - **Abduction**: (session, contradicts, axis) + (session, mode_switch, mode)
///   → mode caused the contradiction
pub fn infer_triples(triples: &[SpoTriple]) -> Vec<SpoTriple> {
    let mut inferred = Vec::new();

    let by_predicate: HashMap<u16, Vec<&SpoTriple>> = {
        let mut map: HashMap<u16, Vec<&SpoTriple>> = HashMap::new();
        for t in triples {
            map.entry(t.predicate_hash).or_default().push(t);
        }
        map
    };

    // Deduction: User asks X + Ada explains X → strong topic connection
    let asks = by_predicate
        .get(&ConversationPredicate::Asks.hash())
        .cloned()
        .unwrap_or_default();
    let explains = by_predicate
        .get(&ConversationPredicate::Explains.hash())
        .cloned()
        .unwrap_or_default();

    for ask in &asks {
        for explain in &explains {
            if ask.object_dn == explain.object_dn {
                let ask_truth = ask.to_nars_truth();
                let explain_truth = explain.to_nars_truth();
                let deduced = ask_truth.deduction(&explain_truth);

                inferred.push(SpoTriple::inferred(
                    ask.subject_dn,
                    ConversationPredicate::RelatesTo.hash(),
                    explain.subject_dn,
                    deduced.confidence,
                ));
            }
        }
    }

    // Abduction: contradiction + mode_switch → mode caused it
    let contradicts = by_predicate
        .get(&ConversationPredicate::Contradicts.hash())
        .cloned()
        .unwrap_or_default();
    let mode_switches = by_predicate
        .get(&ConversationPredicate::ModeSwitch.hash())
        .cloned()
        .unwrap_or_default();

    for contradiction in &contradicts {
        for mode_switch in &mode_switches {
            if contradiction.subject_dn == mode_switch.subject_dn {
                let contra_truth = contradiction.to_nars_truth();
                let mode_truth = mode_switch.to_nars_truth();
                let abducted = contra_truth.abduction(&mode_truth);

                inferred.push(SpoTriple::inferred(
                    mode_switch.object_dn,
                    ConversationPredicate::Contradicts.hash(),
                    contradiction.object_dn,
                    abducted.confidence,
                ));
            }
        }
    }

    inferred
}

// ============================================================================
// Serialization for substrate write-back
// ============================================================================

/// Serialize triples for substrate write-back.
///
/// Format: array of triples matching WideMetaView SPO crystal layout.
/// Max 8 triples per CogRecord (W128–W143 = 16 words = 8 triples × 2 words).
pub fn serialize_for_substrate(triples: &[SpoTriple]) -> serde_json::Value {
    let capped: Vec<_> = triples.iter().take(8).collect();
    serde_json::json!({
        "spo_triples": capped,
        "count": capped.len(),
        "target": "wide_meta_view",
        "slots": "W128-W143",
    })
}

/// Build a context string from triples for system prompt injection.
pub fn build_spo_context(triples: &[SpoTriple]) -> Option<String> {
    if triples.is_empty() {
        return None;
    }

    let mut ctx = String::from("\n[Semantic Graph — SPO triples]\n");

    for t in triples.iter().take(5) {
        let pred_label = ConversationPredicate::from_hash(t.predicate_hash)
            .map(|p| p.label())
            .unwrap_or("?");
        let inferred_tag = if t.is_inferred() { " [inferred]" } else { "" };
        ctx.push_str(&format!(
            "  ({:#010x}) —[{}]→ ({:#010x}) c={:.0}%{}\n",
            t.subject_dn,
            pred_label,
            t.object_dn,
            t.confidence() * 100.0,
            inferred_tag,
        ));
    }

    Some(ctx)
}

// ============================================================================
// Blackboard slot key conventions
// ============================================================================

/// Standard Blackboard key for current conversation triples.
pub const SLOT_SPO_TRIPLES: &str = "awareness:spo_triples";

/// Standard Blackboard key for inferred triples.
pub const SLOT_SPO_INFERRED: &str = "awareness:spo_inferred";

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_hash_deterministic() {
        let h1 = entity_hash("user");
        let h2 = entity_hash("user");
        assert_eq!(h1, h2);

        let h3 = entity_hash("ada");
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_entity_hash_distribution() {
        let entities = ["user", "ada", "wife", "work", "agi", "architecture", "love"];
        let hashes: Vec<u32> = entities.iter().map(|e| entity_hash(e)).collect();
        for i in 0..hashes.len() {
            for j in (i + 1)..hashes.len() {
                assert_ne!(
                    hashes[i], hashes[j],
                    "Hash collision: {} and {}",
                    entities[i], entities[j]
                );
            }
        }
    }

    #[test]
    fn test_spo_triple_creation() {
        let t = SpoTriple::new(100, 0x0001, 200, 0.8);
        assert_eq!(t.subject_dn, 100);
        assert_eq!(t.predicate_hash, 0x0001);
        assert_eq!(t.object_dn, 200);
        assert!((t.confidence() - 0.8).abs() < 0.01);
        assert!(!t.is_negated());
        assert!(!t.is_inferred());
    }

    #[test]
    fn test_spo_triple_inferred() {
        let t = SpoTriple::inferred(100, 0x0002, 200, 0.6);
        assert!(t.is_inferred());
        assert!(!t.is_negated());
    }

    #[test]
    fn test_spo_triple_nars_truth() {
        let t = SpoTriple::new(100, 0x0001, 200, 0.8);
        let truth = t.to_nars_truth();
        assert_eq!(truth.frequency, 1.0);
        assert!((truth.confidence - 0.8).abs() < 0.02);
    }

    #[test]
    fn test_predicate_roundtrip() {
        for pred in &[
            ConversationPredicate::Asks,
            ConversationPredicate::Explains,
            ConversationPredicate::Remembers,
            ConversationPredicate::Contradicts,
            ConversationPredicate::RelatesTo,
            ConversationPredicate::ModeSwitch,
            ConversationPredicate::FeelsAbout,
            ConversationPredicate::References,
        ] {
            let h = pred.hash();
            let recovered = ConversationPredicate::from_hash(h).unwrap();
            assert_eq!(*pred, recovered);
        }
    }

    #[test]
    fn test_extract_triples_basic() {
        let triples = extract_triples(
            "Tell me about the architecture design",
            "The architecture uses a layered approach...",
            "session-1",
            "work",
            &["THOUGHT".to_string()],
            &[],
        );

        assert!(triples.len() >= 3, "Expected >= 3 triples, got {}", triples.len());

        let asks_count = triples
            .iter()
            .filter(|t| t.predicate_hash == ConversationPredicate::Asks.hash())
            .count();
        assert_eq!(asks_count, 1);

        let mode_triple = triples
            .iter()
            .find(|t| t.predicate_hash == ConversationPredicate::ModeSwitch.hash());
        assert!(mode_triple.is_some());
        assert!(mode_triple.unwrap().is_temporal());
    }

    #[test]
    fn test_extract_triples_with_tension() {
        let triples = extract_triples(
            "I feel warm today",
            "",
            "session-1",
            "hybrid",
            &[],
            &["warm_cool".to_string(), "intimate_formal".to_string()],
        );

        let contradicts_count = triples
            .iter()
            .filter(|t| t.predicate_hash == ConversationPredicate::Contradicts.hash())
            .count();
        assert_eq!(contradicts_count, 2);

        for t in triples.iter().filter(|t| {
            t.predicate_hash == ConversationPredicate::Contradicts.hash()
        }) {
            assert!(t.is_inferred());
        }
    }

    #[test]
    fn test_extract_triples_hybrid_mode_no_mode_triple() {
        let triples = extract_triples(
            "hello",
            "hi there",
            "session-1",
            "hybrid",
            &[],
            &[],
        );

        let mode_count = triples
            .iter()
            .filter(|t| t.predicate_hash == ConversationPredicate::ModeSwitch.hash())
            .count();
        assert_eq!(mode_count, 0);
    }

    #[test]
    fn test_infer_triples_deduction() {
        let user_dn = entity_hash("user");
        let ada_dn = entity_hash("ada");
        let topic_dn = entity_hash("architecture");

        let triples = vec![
            SpoTriple::new(user_dn, ConversationPredicate::Asks.hash(), topic_dn, 0.8),
            SpoTriple::new(ada_dn, ConversationPredicate::Explains.hash(), topic_dn, 0.9),
        ];

        let inferred = infer_triples(&triples);
        assert_eq!(inferred.len(), 1);
        assert_eq!(
            inferred[0].predicate_hash,
            ConversationPredicate::RelatesTo.hash()
        );
        assert!(inferred[0].is_inferred());
    }

    #[test]
    fn test_infer_triples_abduction() {
        let session_dn = entity_hash("session-1");
        let axis_dn = entity_hash("warm_cool");
        let mode_dn = entity_hash("wife");

        let triples = vec![
            SpoTriple::inferred(
                session_dn,
                ConversationPredicate::Contradicts.hash(),
                axis_dn,
                0.7,
            ),
            {
                let mut t = SpoTriple::new(
                    session_dn,
                    ConversationPredicate::ModeSwitch.hash(),
                    mode_dn,
                    0.9,
                );
                t.flags |= 0x04;
                t
            },
        ];

        let inferred = infer_triples(&triples);
        assert!(!inferred.is_empty());
        let cause = &inferred[0];
        assert_eq!(cause.subject_dn, mode_dn);
        assert_eq!(cause.object_dn, axis_dn);
        assert!(cause.is_inferred());
    }

    #[test]
    fn test_serialize_for_substrate() {
        let triples = vec![
            SpoTriple::new(100, 0x0001, 200, 0.8),
            SpoTriple::new(300, 0x0002, 400, 0.9),
        ];

        let json = serialize_for_substrate(&triples);
        assert_eq!(json["count"], 2);
        assert_eq!(json["target"], "wide_meta_view");
        assert_eq!(json["slots"], "W128-W143");
    }

    #[test]
    fn test_build_spo_context() {
        let triples = vec![
            SpoTriple::new(
                entity_hash("user"),
                ConversationPredicate::Asks.hash(),
                entity_hash("architecture"),
                0.8,
            ),
        ];

        let ctx = build_spo_context(&triples).unwrap();
        assert!(ctx.contains("asks"));
        assert!(ctx.contains("SPO"));
    }

    #[test]
    fn test_build_spo_context_empty() {
        assert!(build_spo_context(&[]).is_none());
    }

    #[test]
    fn test_topic_extraction() {
        let topic = extract_topic("Tell me about the architecture design patterns");
        assert!(!topic.is_empty());
        assert_ne!(topic, "unknown");
    }

    #[test]
    fn test_topic_extraction_minimal() {
        let topic = extract_topic("hi");
        assert_eq!(topic, "unknown");
    }

    #[test]
    fn test_spo_triple_json_roundtrip() {
        let t = SpoTriple::new(
            entity_hash("user"),
            ConversationPredicate::Asks.hash(),
            entity_hash("architecture"),
            0.85,
        );
        let json = serde_json::to_string(&t).unwrap();
        let restored: SpoTriple = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.subject_dn, t.subject_dn);
        assert_eq!(restored.predicate_hash, t.predicate_hash);
        assert_eq!(restored.object_dn, t.object_dn);
    }

    #[test]
    fn test_to_3d_range() {
        let t = SpoTriple::new(
            entity_hash("user"),
            ConversationPredicate::Asks.hash(),
            entity_hash("architecture"),
            0.8,
        );
        let v = t.to_3d();
        for &c in &v {
            assert!(c >= 0.0 && c <= 1.0, "3D component out of range: {}", c);
        }
    }

    #[test]
    fn test_to_3d_confidence_scales() {
        let high = SpoTriple::new(100, 0x0001, 200, 1.0);
        let low = SpoTriple::new(100, 0x0001, 200, 0.1);

        let vh = high.to_3d();
        let vl = low.to_3d();

        // Low confidence → closer to origin (smaller magnitude)
        let mag_h = (vh[0] * vh[0] + vh[1] * vh[1] + vh[2] * vh[2]).sqrt();
        let mag_l = (vl[0] * vl[0] + vl[1] * vl[1] + vl[2] * vl[2]).sqrt();
        assert!(mag_h > mag_l, "High-confidence should have larger magnitude");
    }

    #[test]
    fn test_to_3d_different_subjects_separate() {
        let a = SpoTriple::new(
            entity_hash("user"),
            ConversationPredicate::Asks.hash(),
            entity_hash("topic"),
            0.9,
        );
        let b = SpoTriple::new(
            entity_hash("ada"),
            ConversationPredicate::Asks.hash(),
            entity_hash("topic"),
            0.9,
        );
        // Same predicate + object, different subject → should differ in x
        let va = a.to_3d();
        let vb = b.to_3d();
        assert!((va[0] - vb[0]).abs() > 0.01, "Different subjects should separate in x");
        // y (predicate) and z (object) should be equal
        assert!((va[1] - vb[1]).abs() < 0.01);
        assert!((va[2] - vb[2]).abs() < 0.01);
    }

    #[test]
    fn test_distance_3d_self_zero() {
        let t = SpoTriple::new(
            entity_hash("user"),
            ConversationPredicate::Explains.hash(),
            entity_hash("topic"),
            0.8,
        );
        assert!(t.distance_3d(&t) < 1e-6);
    }

    #[test]
    fn test_distance_3d_symmetry() {
        let a = SpoTriple::new(entity_hash("user"), 0x0001, entity_hash("topic_a"), 0.8);
        let b = SpoTriple::new(entity_hash("ada"), 0x0005, entity_hash("topic_b"), 0.6);
        let d1 = a.distance_3d(&b);
        let d2 = b.distance_3d(&a);
        assert!((d1 - d2).abs() < 1e-6, "Distance should be symmetric");
    }

    #[test]
    fn test_hash_fold_dispersion() {
        // Consecutive hashes should NOT produce consecutive folds (good dispersion)
        let f1 = super::hash_fold(1);
        let f2 = super::hash_fold(2);
        let f3 = super::hash_fold(3);
        // They should be spread out, not clustered
        assert!((f1 - f2).abs() > 0.01);
        assert!((f2 - f3).abs() > 0.01);
        // All in [0, 1]
        assert!(f1 >= 0.0 && f1 <= 1.0);
        assert!(f2 >= 0.0 && f2 <= 1.0);
        assert!(f3 >= 0.0 && f3 <= 1.0);
    }
}
