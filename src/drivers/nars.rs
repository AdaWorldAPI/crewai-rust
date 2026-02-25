//! NARS driver — Non-Axiomatic Reasoning System for evidence-based inference.
//!
//! Provides principled evidence accumulation, revision, and causal inference
//! over awareness classifications from BindSpace. All operations are pure
//! functions on Blackboard-native types — no IO, no state.
//!
//! # Evidence Rules
//!
//! | Classification | Rule       | Effect                                      |
//! |----------------|------------|---------------------------------------------|
//! | Crystallized   | Revision   | Accumulate evidence → confidence ↑           |
//! | Tensioned      | Abduction  | Infer cause of contradiction (mode switch)   |
//! | Uncertain      | Comparison | Similarity judgment between weak signals     |
//!
//! # Blackboard Integration
//!
//! ```text
//! BindSpace writes → AwarenessFrame → Blackboard TypedSlot
//! NARS driver reads AwarenessFrame, produces → NarsSemanticState
//! Agents read NarsSemanticState for prompt enrichment + weight deltas
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Blackboard-native awareness types
// ============================================================================

/// An awareness frame — a snapshot of the cognitive substrate's recognition
/// state at a single point in time.
///
/// This is the canonical Blackboard type that replaces ad-hoc retrieval
/// result structs. BindSpace writes it during hydration; drivers read it
/// for inference. Both internal agents and external MCP/REST agents
/// produce and consume this same type.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AwarenessFrame {
    /// Matches classified as crystallized (settled, high confidence).
    pub crystallized: Vec<AwarenessMatch>,
    /// Matches classified as tensioned (contradictory evidence).
    pub tensioned: Vec<AwarenessMatch>,
    /// Matches classified as uncertain (weak/ambiguous signal).
    pub uncertain: Vec<AwarenessMatch>,
    /// Noise floor threshold for this frame.
    pub noise_floor: f32,
    /// Summary statistics.
    pub summary: AwarenessSummary,
}

/// A single match from BindSpace awareness search.
///
/// Protocol-agnostic: identical layout whether transmitted as a TypedSlot
/// (in-process, zero-serde) or as JSON (MCP/REST boundary).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AwarenessMatch {
    /// Hamming similarity score (0.0 = orthogonal, 1.0 = identical).
    pub similarity: f32,
    /// Presence mode at time of recording.
    pub presence_mode: String,
    /// Cognitive rung level (0–8).
    pub rung_level: u8,
    /// Session ID of the original recording.
    pub session_id: String,
    /// Divergent meaning axes: axis_name → (cached_value, current_value).
    /// Populated only for tensioned matches.
    #[serde(default)]
    pub divergent_axes: HashMap<String, (f32, f32)>,
}

/// Summary statistics for an awareness frame.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AwarenessSummary {
    /// Total matches retrieved from BindSpace.
    pub total_retrieved: usize,
    /// Count per classification.
    pub crystallized_count: usize,
    pub tensioned_count: usize,
    pub uncertain_count: usize,
    /// Best similarity score across all matches.
    pub best_similarity: f32,
}

// ============================================================================
// NARS Truth Value
// ============================================================================

/// NARS truth value: ⟨frequency, confidence⟩.
///
/// The atomic unit of evidence in Non-Axiomatic Logic. Frequency measures
/// the proportion of positive evidence; confidence measures how reliable
/// that frequency estimate is given the total evidence.
///
/// Wire-compatible with `ladybug_contract::nars::TruthValue`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NarsTruth {
    /// Proportion of positive evidence (0.0–1.0).
    pub frequency: f32,
    /// Reliability of the frequency estimate (0.0–1.0).
    pub confidence: f32,
}

impl NarsTruth {
    /// Evidential horizon parameter (matching ladybug-rs).
    pub const HORIZON: f32 = 1.0;

    pub fn new(frequency: f32, confidence: f32) -> Self {
        Self {
            frequency: frequency.clamp(0.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// No evidence: ⟨0.5, 0.0⟩.
    pub fn unknown() -> Self {
        Self {
            frequency: 0.5,
            confidence: 0.0,
        }
    }

    /// Create from positive/negative evidence counts.
    pub fn from_evidence(positive: f32, negative: f32) -> Self {
        let total = positive + negative;
        if total == 0.0 {
            return Self::unknown();
        }
        let frequency = positive / total;
        let confidence = total / (total + Self::HORIZON);
        Self { frequency, confidence }
    }

    /// Convert to evidence counts.
    pub fn to_evidence(&self) -> (f32, f32) {
        let c = self.confidence.min(1.0 - 1.0 / (Self::HORIZON + 1000.0));
        let w = Self::HORIZON * c / (1.0 - c);
        let w_pos = w * self.frequency;
        let w_neg = w * (1.0 - self.frequency);
        (w_pos, w_neg)
    }

    /// Expected value: c × (f − 0.5) + 0.5.
    pub fn expectation(&self) -> f32 {
        self.confidence * (self.frequency - 0.5) + 0.5
    }

    /// Revision: combine two truth values with independent evidence.
    pub fn revision(&self, other: &NarsTruth) -> NarsTruth {
        let (w1_pos, w1_neg) = self.to_evidence();
        let (w2_pos, w2_neg) = other.to_evidence();
        NarsTruth::from_evidence(w1_pos + w2_pos, w1_neg + w2_neg)
    }

    /// Negation: NOT.
    pub fn negation(&self) -> NarsTruth {
        NarsTruth {
            frequency: 1.0 - self.frequency,
            confidence: self.confidence,
        }
    }

    /// Deduction: A→B, B→C ⊢ A→C.
    pub fn deduction(&self, other: &NarsTruth) -> NarsTruth {
        let f = self.frequency * other.frequency;
        let c = self.confidence * other.confidence * f;
        NarsTruth { frequency: f, confidence: c }
    }

    /// Abduction: A→B, C→B ⊢ A→C.
    pub fn abduction(&self, other: &NarsTruth) -> NarsTruth {
        let f = self.frequency;
        let c = other.frequency * self.confidence * other.confidence;
        NarsTruth { frequency: f, confidence: c }
    }

    /// Comparison: A→B, C→B ⊢ A↔C.
    pub fn comparison(&self, other: &NarsTruth) -> NarsTruth {
        let f1f2 = self.frequency * other.frequency;
        let f = f1f2 / (self.frequency + other.frequency - f1f2).max(f32::EPSILON);
        let c = self.confidence * other.confidence * f;
        NarsTruth { frequency: f, confidence: c }
    }
}

impl Default for NarsTruth {
    fn default() -> Self {
        Self::unknown()
    }
}

// ============================================================================
// NARS inference types
// ============================================================================

/// Which NARS inference rule was applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NarsRule {
    /// Combined independent evidence from multiple matches.
    Revision,
    /// Inferred cause from mode switch (shared effect = axis tension).
    Abduction,
    /// Similarity judgment from two uncertain matches.
    Comparison,
    /// Direct evidence from a single match.
    Direct,
}

/// How NARS inference was applied to a single meaning axis.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NarsAxisInference {
    /// The meaning axis name (e.g., "warm_cool").
    pub axis: String,
    /// The revised truth value after combining all evidence.
    pub truth: NarsTruth,
    /// Which inference rule was applied.
    pub rule: NarsRule,
    /// Evidence count (total matches contributing).
    pub evidence_count: u32,
}

/// NARS-enriched semantic state derived from awareness classification.
///
/// This is the output of `nars_analyze()`: every meaning axis gets a
/// truth value with proper evidence-based confidence, and tensioned
/// axes get causal inference explaining the contradiction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NarsSemanticState {
    /// Per-axis NARS truth values, revised from all evidence.
    pub axis_truths: HashMap<String, NarsAxisInference>,
    /// Overall awareness truth: how "settled" is the substrate?
    pub overall_truth: NarsTruth,
    /// Causal inferences from tensioned matches.
    pub causal_inferences: Vec<CausalInference>,
    /// Similarity judgments from uncertain matches.
    pub similarity_judgments: Vec<SimilarityJudgment>,
}

/// A causal inference from tensioned awareness.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalInference {
    /// The axis where tension was detected.
    pub axis: String,
    /// Truth in the source context.
    pub source_truth: NarsTruth,
    /// Truth in the current context.
    pub current_truth: NarsTruth,
    /// Abducted inference about the cause.
    pub abducted: NarsTruth,
    /// Human-readable explanation.
    pub explanation: String,
}

/// A similarity judgment from uncertain matches.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimilarityJudgment {
    /// First match similarity.
    pub match_a_sim: f32,
    /// Second match similarity.
    pub match_b_sim: f32,
    /// NARS comparison truth value: how similar are A and B?
    pub comparison_truth: NarsTruth,
}

// ============================================================================
// Core analysis: AwarenessFrame → NarsSemanticState
// ============================================================================

/// Analyze an awareness frame with NARS inference rules.
///
/// Applies:
/// 1. **Revision** on crystallized matches — accumulate evidence for settled axes.
/// 2. **Abduction** on tensioned matches — infer causes for axis contradictions.
/// 3. **Comparison** on uncertain matches — compute similarity between weak signals.
///
/// `current_axes`: meaning axes from the current felt-parse.
pub fn nars_analyze(
    frame: &AwarenessFrame,
    current_axes: &HashMap<String, f32>,
) -> NarsSemanticState {
    let mut axis_truths: HashMap<String, NarsAxisInference> = HashMap::new();
    let mut causal_inferences = Vec::new();
    let mut similarity_judgments = Vec::new();

    // ── Phase 1: Crystallized matches → Revision ──────────────────────────
    for am in &frame.crystallized {
        let match_confidence = am.similarity.clamp(0.0, 1.0);

        for (axis, &_current_val) in current_axes {
            let positive = match_confidence;
            let negative = 1.0 - match_confidence;
            let match_truth = NarsTruth::from_evidence(positive, negative);

            axis_truths
                .entry(axis.clone())
                .and_modify(|existing| {
                    existing.truth = existing.truth.revision(&match_truth);
                    existing.evidence_count += 1;
                    existing.rule = NarsRule::Revision;
                })
                .or_insert_with(|| NarsAxisInference {
                    axis: axis.clone(),
                    truth: match_truth,
                    rule: NarsRule::Direct,
                    evidence_count: 1,
                });
        }
    }

    // ── Phase 2: Tensioned matches → Abduction ───────────────────────────
    for am in &frame.tensioned {
        for (axis, &(cached_val, current_val)) in &am.divergent_axes {
            let cached_truth = NarsTruth::new(
                (cached_val + 1.0) / 2.0,
                am.similarity.clamp(0.1, 0.9),
            );
            let current_truth = NarsTruth::new(
                (current_val + 1.0) / 2.0,
                0.8,
            );

            let abducted = cached_truth.abduction(&current_truth);

            let explanation = if abducted.confidence < 0.3 {
                format!(
                    "Strong mode divergence on '{}': cached={:.2} (mode={}) vs current={:.2}",
                    axis, cached_val, am.presence_mode, current_val
                )
            } else {
                format!(
                    "Weak divergence on '{}': contexts partially overlap",
                    axis
                )
            };

            causal_inferences.push(CausalInference {
                axis: axis.clone(),
                source_truth: cached_truth,
                current_truth,
                abducted,
                explanation,
            });

            let tension_truth = NarsTruth::from_evidence(
                0.3,
                0.7 * (1.0 - abducted.confidence),
            );

            axis_truths
                .entry(axis.clone())
                .and_modify(|existing| {
                    existing.truth = existing.truth.revision(&tension_truth);
                    existing.evidence_count += 1;
                })
                .or_insert_with(|| NarsAxisInference {
                    axis: axis.clone(),
                    truth: tension_truth,
                    rule: NarsRule::Abduction,
                    evidence_count: 1,
                });
        }
    }

    // ── Phase 3: Uncertain matches → Comparison ──────────────────────────
    let uncertains = &frame.uncertain;
    if uncertains.len() >= 2 {
        for i in 0..uncertains.len().min(3) {
            for j in (i + 1)..uncertains.len().min(4) {
                let a_truth = NarsTruth::new(
                    uncertains[i].similarity,
                    uncertains[i].similarity * 0.5,
                );
                let b_truth = NarsTruth::new(
                    uncertains[j].similarity,
                    uncertains[j].similarity * 0.5,
                );
                let comparison = a_truth.comparison(&b_truth);

                similarity_judgments.push(SimilarityJudgment {
                    match_a_sim: uncertains[i].similarity,
                    match_b_sim: uncertains[j].similarity,
                    comparison_truth: comparison,
                });
            }
        }
    }

    // ── Phase 4: Overall awareness truth ─────────────────────────────────
    let overall_truth = compute_overall_truth(&axis_truths, frame);

    NarsSemanticState {
        axis_truths,
        overall_truth,
        causal_inferences,
        similarity_judgments,
    }
}

/// Compute overall awareness truth from per-axis truths.
fn compute_overall_truth(
    axis_truths: &HashMap<String, NarsAxisInference>,
    frame: &AwarenessFrame,
) -> NarsTruth {
    if axis_truths.is_empty() {
        let total = frame.summary.crystallized_count
            + frame.summary.tensioned_count
            + frame.summary.uncertain_count;
        if total == 0 {
            return NarsTruth::unknown();
        }
        let positive = frame.summary.crystallized_count as f32;
        let negative = frame.summary.tensioned_count as f32;
        return NarsTruth::from_evidence(positive, negative);
    }

    let mut overall = NarsTruth::unknown();
    for inference in axis_truths.values() {
        overall = overall.revision(&inference.truth);
    }
    overall
}

// ============================================================================
// Context builder — NARS-enriched prompt injection
// ============================================================================

/// Build a NARS-aware context string for system prompt injection.
pub fn build_nars_context(state: &NarsSemanticState) -> Option<String> {
    if state.axis_truths.is_empty() && state.causal_inferences.is_empty() {
        return None;
    }

    let mut ctx = String::new();

    ctx.push_str(&format!(
        "\n[Substrate NARS Evidence: certainty {:.0}%, confidence {:.0}%]\n",
        state.overall_truth.frequency * 100.0,
        state.overall_truth.confidence * 100.0,
    ));

    let settled: Vec<_> = state
        .axis_truths
        .values()
        .filter(|ai| ai.truth.confidence > 0.6 && ai.rule == NarsRule::Revision)
        .collect();
    if !settled.is_empty() {
        ctx.push_str("Settled axes: ");
        let descs: Vec<String> = settled
            .iter()
            .map(|ai| {
                format!(
                    "{}={:.0}% (c={:.0}%, {}ev)",
                    ai.axis,
                    ai.truth.frequency * 100.0,
                    ai.truth.confidence * 100.0,
                    ai.evidence_count
                )
            })
            .collect();
        ctx.push_str(&descs.join(", "));
        ctx.push('\n');
    }

    if !state.causal_inferences.is_empty() {
        ctx.push_str("Tension causes: ");
        let causes: Vec<String> = state
            .causal_inferences
            .iter()
            .take(3)
            .map(|ci| ci.explanation.clone())
            .collect();
        ctx.push_str(&causes.join("; "));
        ctx.push('\n');
    }

    Some(ctx)
}

// ============================================================================
// Weight generation — NARS → hybrid weight deltas
// ============================================================================

/// Convert NARS axis truths to hybrid weight deltas for WideMetaView.
///
/// Maps the 8 bipolar meaning axes to weight slots 0–7.
/// Crystallized → positive boost. Tensioned → negative. Uncertain → small negative.
pub fn nars_to_weight_deltas(state: &NarsSemanticState) -> [f32; 32] {
    let mut deltas = [0.0f32; 32];

    let axis_slot_map: &[(&str, usize)] = &[
        ("warm_cool", 0),
        ("close_distant", 1),
        ("certain_uncertain", 2),
        ("intimate_formal", 3),
        ("active_passive", 4),
        ("joyful_sorrowful", 5),
        ("tense_relaxed", 6),
        ("novel_familiar", 7),
    ];

    for (axis_name, slot) in axis_slot_map {
        if let Some(inference) = state.axis_truths.get(*axis_name) {
            let expectation = inference.truth.expectation();
            deltas[*slot] = match inference.rule {
                NarsRule::Revision => {
                    0.05 + 0.05 * inference.truth.confidence
                }
                NarsRule::Abduction => {
                    -0.03 - 0.05 * (1.0 - inference.truth.confidence)
                }
                NarsRule::Comparison => {
                    -0.01
                }
                NarsRule::Direct => {
                    0.02 * (expectation - 0.5)
                }
            };
        }
    }

    deltas
}

// ============================================================================
// Blackboard slot key conventions
// ============================================================================

/// Standard Blackboard key for the current awareness frame.
pub const SLOT_AWARENESS_FRAME: &str = "awareness:frame";

/// Standard Blackboard key for the NARS semantic state.
pub const SLOT_NARS_STATE: &str = "awareness:nars";

/// Standard Blackboard key for NARS weight deltas.
pub const SLOT_NARS_DELTAS: &str = "awareness:nars_deltas";

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_match(sim: f32, mode: &str, rung: u8) -> AwarenessMatch {
        AwarenessMatch {
            similarity: sim,
            presence_mode: mode.into(),
            rung_level: rung,
            session_id: "s1".into(),
            divergent_axes: HashMap::new(),
        }
    }

    fn make_frame(
        crystallized: Vec<AwarenessMatch>,
        tensioned: Vec<AwarenessMatch>,
        uncertain: Vec<AwarenessMatch>,
    ) -> AwarenessFrame {
        let c_count = crystallized.len();
        let t_count = tensioned.len();
        let u_count = uncertain.len();
        let best = crystallized
            .first()
            .map(|m| m.similarity)
            .unwrap_or(0.0);
        AwarenessFrame {
            crystallized,
            tensioned,
            uncertain,
            noise_floor: 0.5,
            summary: AwarenessSummary {
                total_retrieved: c_count + t_count + u_count,
                crystallized_count: c_count,
                tensioned_count: t_count,
                uncertain_count: u_count,
                best_similarity: best,
            },
        }
    }

    // ── NarsTruth basic tests ───────────────────────────────────────────

    #[test]
    fn test_nars_truth_revision_increases_confidence() {
        let a = NarsTruth::from_evidence(3.0, 1.0);
        let b = NarsTruth::from_evidence(2.0, 0.0);
        let revised = a.revision(&b);
        assert!(
            revised.confidence > a.confidence,
            "Revision should increase confidence: {} > {}",
            revised.confidence,
            a.confidence
        );
    }

    #[test]
    fn test_nars_truth_abduction_low_confidence_on_divergence() {
        let source = NarsTruth::new(0.9, 0.8);
        let current = NarsTruth::new(0.1, 0.8);
        let abducted = source.abduction(&current);
        assert!(
            abducted.confidence < 0.3,
            "Divergent contexts should have low abduction confidence: {}",
            abducted.confidence
        );
    }

    #[test]
    fn test_nars_truth_comparison_similar_inputs() {
        let a = NarsTruth::new(0.8, 0.7);
        let b = NarsTruth::new(0.75, 0.65);
        let compared = a.comparison(&b);
        assert!(
            compared.frequency > 0.5,
            "Similar inputs should have high comparison frequency: {}",
            compared.frequency
        );
    }

    #[test]
    fn test_nars_truth_expectation() {
        let confident = NarsTruth::new(1.0, 0.9);
        assert!(confident.expectation() > 0.9);

        let uncertain = NarsTruth::unknown();
        assert!((uncertain.expectation() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_nars_truth_roundtrip_evidence() {
        let original = NarsTruth::from_evidence(5.0, 2.0);
        let (pos, neg) = original.to_evidence();
        let restored = NarsTruth::from_evidence(pos, neg);
        assert!((original.frequency - restored.frequency).abs() < 0.01);
        assert!((original.confidence - restored.confidence).abs() < 0.01);
    }

    #[test]
    fn test_nars_truth_deduction() {
        let a = NarsTruth::new(0.9, 0.8);
        let b = NarsTruth::new(0.8, 0.7);
        let deduced = a.deduction(&b);
        assert!(deduced.frequency < a.frequency);
        assert!(deduced.confidence < a.confidence);
    }

    // ── nars_analyze integration tests ──────────────────────────────────

    #[test]
    fn test_nars_analyze_crystallized_revision() {
        let frame = make_frame(
            vec![make_match(0.92, "wife", 4), make_match(0.88, "wife", 3)],
            vec![],
            vec![],
        );

        let mut axes = HashMap::new();
        axes.insert("warm_cool".into(), 0.85);
        axes.insert("intimate_formal".into(), 0.90);

        let state = nars_analyze(&frame, &axes);

        let warm = state.axis_truths.get("warm_cool").unwrap();
        assert!(
            warm.truth.confidence > 0.5,
            "Revised crystallized should have confidence > 0.5, got {}",
            warm.truth.confidence
        );
        assert_eq!(warm.evidence_count, 2);
        assert_eq!(warm.rule, NarsRule::Revision);
    }

    #[test]
    fn test_nars_analyze_tensioned_abduction() {
        let mut tensioned_match = make_match(0.72, "wife", 3);
        tensioned_match.divergent_axes.insert(
            "warm_cool".into(),
            (0.9, -0.6),
        );

        let frame = make_frame(vec![], vec![tensioned_match], vec![]);
        let axes = HashMap::new();
        let state = nars_analyze(&frame, &axes);

        assert_eq!(state.causal_inferences.len(), 1);
        assert_eq!(state.causal_inferences[0].axis, "warm_cool");
        assert!(state.causal_inferences[0].explanation.contains("warm_cool"));

        let warm = state.axis_truths.get("warm_cool").unwrap();
        assert!(
            warm.truth.confidence < 0.7,
            "Tensioned axis should have reduced confidence: {}",
            warm.truth.confidence,
        );
    }

    #[test]
    fn test_nars_analyze_uncertain_comparison() {
        let frame = make_frame(
            vec![],
            vec![],
            vec![
                make_match(0.55, "hybrid", 3),
                make_match(0.52, "hybrid", 2),
            ],
        );

        let axes = HashMap::new();
        let state = nars_analyze(&frame, &axes);

        assert_eq!(state.similarity_judgments.len(), 1);
        assert!(state.similarity_judgments[0].comparison_truth.frequency > 0.0);
    }

    #[test]
    fn test_nars_analyze_mixed_bins() {
        let mut tensioned_match = make_match(0.72, "work", 5);
        tensioned_match
            .divergent_axes
            .insert("tense_relaxed".into(), (-0.5, 0.6));

        let frame = make_frame(
            vec![make_match(0.91, "wife", 4)],
            vec![tensioned_match],
            vec![make_match(0.55, "hybrid", 3)],
        );

        let mut axes = HashMap::new();
        axes.insert("warm_cool".into(), 0.8);
        axes.insert("tense_relaxed".into(), 0.6);

        let state = nars_analyze(&frame, &axes);

        assert!(state.axis_truths.contains_key("warm_cool"));
        assert!(state.axis_truths.contains_key("tense_relaxed"));
        assert!(!state.causal_inferences.is_empty());
        assert!(state.overall_truth.confidence > 0.0);
    }

    #[test]
    fn test_nars_overall_truth_crystallized_is_confident() {
        let frame = make_frame(
            vec![
                make_match(0.95, "wife", 4),
                make_match(0.92, "wife", 4),
                make_match(0.89, "wife", 3),
            ],
            vec![],
            vec![],
        );

        let mut axes = HashMap::new();
        axes.insert("warm_cool".into(), 0.9);
        let state = nars_analyze(&frame, &axes);

        assert!(
            state.overall_truth.confidence > 0.5,
            "3 crystallized should give high overall confidence: {}",
            state.overall_truth.confidence
        );
        assert!(
            state.overall_truth.frequency > 0.6,
            "3 crystallized should give high frequency: {}",
            state.overall_truth.frequency
        );
    }

    // ── Context builder tests ───────────────────────────────────────────

    #[test]
    fn test_nars_context_builder_settled() {
        let mut axis_truths = HashMap::new();
        axis_truths.insert(
            "warm_cool".into(),
            NarsAxisInference {
                axis: "warm_cool".into(),
                truth: NarsTruth::new(0.85, 0.8),
                rule: NarsRule::Revision,
                evidence_count: 3,
            },
        );

        let state = NarsSemanticState {
            axis_truths,
            overall_truth: NarsTruth::new(0.85, 0.8),
            causal_inferences: vec![],
            similarity_judgments: vec![],
        };

        let ctx = build_nars_context(&state).unwrap();
        assert!(ctx.contains("warm_cool"));
        assert!(ctx.contains("certainty"));
    }

    #[test]
    fn test_nars_context_builder_empty() {
        let state = NarsSemanticState {
            axis_truths: HashMap::new(),
            overall_truth: NarsTruth::unknown(),
            causal_inferences: vec![],
            similarity_judgments: vec![],
        };

        assert!(build_nars_context(&state).is_none());
    }

    // ── Weight delta tests ──────────────────────────────────────────────

    #[test]
    fn test_nars_weight_deltas_crystallized_positive() {
        let mut axis_truths = HashMap::new();
        axis_truths.insert(
            "warm_cool".into(),
            NarsAxisInference {
                axis: "warm_cool".into(),
                truth: NarsTruth::new(0.9, 0.8),
                rule: NarsRule::Revision,
                evidence_count: 3,
            },
        );

        let state = NarsSemanticState {
            axis_truths,
            overall_truth: NarsTruth::new(0.9, 0.8),
            causal_inferences: vec![],
            similarity_judgments: vec![],
        };

        let deltas = nars_to_weight_deltas(&state);
        assert!(
            deltas[0] > 0.0,
            "warm_cool (slot 0) should have positive delta: {}",
            deltas[0]
        );
    }

    #[test]
    fn test_nars_weight_deltas_tensioned_negative() {
        let mut axis_truths = HashMap::new();
        axis_truths.insert(
            "warm_cool".into(),
            NarsAxisInference {
                axis: "warm_cool".into(),
                truth: NarsTruth::new(0.4, 0.3),
                rule: NarsRule::Abduction,
                evidence_count: 1,
            },
        );

        let state = NarsSemanticState {
            axis_truths,
            overall_truth: NarsTruth::new(0.4, 0.3),
            causal_inferences: vec![],
            similarity_judgments: vec![],
        };

        let deltas = nars_to_weight_deltas(&state);
        assert!(
            deltas[0] < 0.0,
            "Tensioned warm_cool should have negative delta: {}",
            deltas[0]
        );
    }

    // ── Serialization tests (protocol-agnostic) ────────────────────────

    #[test]
    fn test_awareness_frame_json_roundtrip() {
        let frame = make_frame(
            vec![make_match(0.92, "wife", 4)],
            vec![],
            vec![],
        );
        let json = serde_json::to_string(&frame).unwrap();
        let restored: AwarenessFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.crystallized.len(), 1);
        assert!((restored.crystallized[0].similarity - 0.92).abs() < 0.001);
    }

    #[test]
    fn test_nars_semantic_state_json_roundtrip() {
        let mut axis_truths = HashMap::new();
        axis_truths.insert("warm_cool".into(), NarsAxisInference {
            axis: "warm_cool".into(),
            truth: NarsTruth::new(0.9, 0.8),
            rule: NarsRule::Revision,
            evidence_count: 3,
        });

        let state = NarsSemanticState {
            axis_truths,
            overall_truth: NarsTruth::new(0.85, 0.7),
            causal_inferences: vec![],
            similarity_judgments: vec![],
        };

        let json = serde_json::to_string(&state).unwrap();
        let restored: NarsSemanticState = serde_json::from_str(&json).unwrap();
        assert!(restored.axis_truths.contains_key("warm_cool"));
        assert!((restored.overall_truth.frequency - 0.85).abs() < 0.001);
    }
}
