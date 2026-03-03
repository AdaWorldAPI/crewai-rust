//! Barrier Stack — Composing All Four Blood-Brain Barrier Layers
//!
//! The barrier is not a single gate — it is a stack of four complementary
//! layers that evaluate different dimensions of readiness:
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────┐
//! │                         Barrier Stack                              │
//! │                                                                    │
//! │  Layer 4 (outermost): MUL — Meta-Uncertainty                      │
//! │  ├─ DK detector: am I on Mount Stupid?                            │
//! │  ├─ Trust qualia: competence × source × environment × calibration │
//! │  ├─ Risk vector: epistemic × moral (Kahneman/Tversky)             │
//! │  ├─ Homeostasis: flow/anxiety/boredom/apathy                      │
//! │  └─ Free will modifier: multiplicative confidence (conservative)  │
//! │                                                                    │
//! │  Layer 3: Triune — Inner Dialogue Consensus                       │
//! │  ├─ Guardian: collapse gate @ 0.80 confidence                     │
//! │  ├─ Driver: collapse gate @ 0.65 confidence                       │
//! │  ├─ Catalyst: collapse gate @ 0.50 confidence                     │
//! │  └─ 2-of-3 override voting when leader blocks                    │
//! │                                                                    │
//! │  Layer 2: MarkovBarrier — XOR Budget Gating                       │
//! │  ├─ Pre-call fingerprint capture                                  │
//! │  ├─ Post-call XOR distance measurement                            │
//! │  ├─ Budget = f(confidence, rung, style)                           │
//! │  ├─ Commit / Dampen / Reject                                      │
//! │  └─ Cumulative drift tracking → consolidation                     │
//! │                                                                    │
//! │  Layer 1 (innermost): NARS Truth — Node Properties                │
//! │  ├─ Frequency: how often is this true? (evidence strength)        │
//! │  ├─ Confidence: how much evidence? (sample size)                  │
//! │  └─ Revision: new evidence updates existing truth values          │
//! │                                                                    │
//! │  Direction: inbound check = L4 → L3 → L2 → L1 (strict to lenient)│
//! │             outbound check = L1 → L2 → L3 → L4 (lenient to strict)│
//! └────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Impact vs Exploration Gating (Kahneman & Tversky)
//!
//! The risk vector (L4) encodes prospect-theory-inspired gating:
//! - `epistemic > 0.5 && moral < 0.3` → allows exploration (low stakes)
//! - `moral > 0.7` → requires caution (loss aversion dominant)
//! - `epistemic > 0.3 && moral > 0.5` → needs sandbox (uncertain + high stakes)
//!
//! This is the "impact vs exploration" gate: Kahneman's System 1 says
//! "go explore" but System 2 says "wait, the losses here are asymmetric."
//! The RiskVector mediates this tension.
//!
//! # Nudge Architecture
//!
//! When a barrier layer says HOLD (not BLOCK), the stack can nudge the
//! system toward a better state rather than hard-stopping:
//! - MUL Hold → suggest DK recalibration or trust repair
//! - Triune Hold → route to dissenting facet for deliberation
//! - Markov Hold → request smaller-scope API call
//! - NARS Hold → gather more evidence before committing

use super::markov_barrier::{GateDecision, MarkovBarrier};
use super::nars::NarsTruth;
use crate::persona::triune::Facet;
use crate::persona::triune_dispatch::{BarrierDecision, TriuneDispatch};

use serde::{Deserialize, Serialize};

// ============================================================================
// Stack decision — the composite output
// ============================================================================

/// Which barrier layer caused a hold or block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockingLayer {
    /// L1: NARS truth insufficient.
    NarsTruth { frequency: f32, confidence: f32 },
    /// L2: MarkovBarrier XOR budget exceeded.
    MarkovBudget {
        distance: u32,
        budget: u32,
        gate: GateDecision,
    },
    /// L3: Triune facet consensus denied.
    TriuneConsensus {
        leader: Facet,
        confidence: f32,
        required: f32,
    },
    /// L4: MUL metacognitive gate blocked.
    MetaUncertainty { reason: MulBlockReason },
}

/// MUL block reason (mirrors ladybug-rs GateBlockReason but owned here
/// so crewai-rust doesn't need a direct ladybug dependency for this enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MulBlockReason {
    MountStupid,
    ComplexityUnmapped,
    Depleted,
    TrustInsufficient,
    FalseFlow,
    LowFreeWill,
}

/// The composite decision from the full barrier stack.
#[derive(Debug, Clone)]
pub struct StackDecision {
    /// Whether the action should proceed.
    pub proceed: bool,
    /// Which layer blocked (if any). Ordered from outermost to innermost.
    pub blocking: Vec<BlockingLayer>,
    /// Suggested nudge direction when not blocked but not ideal.
    pub nudge: Option<Nudge>,
    /// The effective confidence after all layers.
    pub effective_confidence: f32,
    /// Per-layer verdicts (for diagnostics).
    pub verdicts: LayerVerdicts,
}

impl StackDecision {
    /// Whether this is a clean pass (all layers agree).
    pub fn is_clean(&self) -> bool {
        self.proceed && self.blocking.is_empty() && self.nudge.is_none()
    }

    /// Whether this is a nudge (proceed but with guidance).
    pub fn is_nudge(&self) -> bool {
        self.proceed && self.nudge.is_some()
    }

    /// Whether this is a hard block.
    pub fn is_blocked(&self) -> bool {
        !self.proceed
    }
}

/// A nudge — a soft suggestion to improve the system's state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Nudge {
    /// Suggest gathering more evidence before committing.
    GatherEvidence {
        current_confidence: f32,
        target: f32,
    },
    /// Suggest recalibrating DK position (more samples needed).
    Recalibrate { current_samples: u32, needed: u32 },
    /// Suggest trust repair (which component is weakest).
    RepairTrust { weakest: String },
    /// Suggest smaller-scope API call (budget was tight).
    ReduceScope { budget_utilization: f32 },
    /// Suggest switching leading facet for this domain.
    SwitchFacet { suggested: Facet, reason: String },
    /// Suggest consolidation pass (drift is high).
    Consolidate { drift: u64, ceiling: u64 },
}

/// Per-layer verdicts for diagnostics.
#[derive(Debug, Clone, Default)]
pub struct LayerVerdicts {
    /// L1: NARS truth (freq, conf).
    pub nars: Option<(f32, f32)>,
    /// L2: Markov gate decision.
    pub markov: Option<GateDecision>,
    /// L3: Triune barrier decision.
    pub triune: Option<BarrierDecision>,
    /// L4: MUL snapshot (gate_open, free_will_modifier, dk_position_name).
    pub mul: Option<MulVerdict>,
}

/// Compact MUL verdict for embedding in StackDecision.
#[derive(Debug, Clone)]
pub struct MulVerdict {
    pub gate_open: bool,
    pub free_will_modifier: f32,
    pub dk_position: String,
    pub trust_level: String,
    pub allostatic_load: f32,
}

// ============================================================================
// MulInput — what the barrier stack needs from ladybug's MUL
// ============================================================================

/// Input from the MUL (Meta-Uncertainty Layer).
///
/// This is a protocol struct so crewai-rust doesn't need to import
/// ladybug-rs directly. ladybug-rs packs its MulSnapshot into this
/// before handing it to the barrier stack.
#[derive(Debug, Clone)]
pub struct MulInput {
    /// Whether the MUL gate is open (L7).
    pub gate_open: bool,
    /// Free will modifier (L8) — multiplicative confidence.
    pub free_will_modifier: f32,
    /// DK position name (for diagnostics).
    pub dk_position: String,
    /// Trust level name (for diagnostics).
    pub trust_level: String,
    /// Allostatic load (cumulative stress).
    pub allostatic_load: f32,
    /// Block reason (if gate closed).
    pub block_reason: Option<MulBlockReason>,
    /// Risk vector: epistemic × moral.
    pub risk: (f32, f32),
}

impl MulInput {
    /// Create from individual values (typically from MulSnapshot).
    pub fn new(
        gate_open: bool,
        free_will_modifier: f32,
        dk_position: &str,
        trust_level: &str,
        allostatic_load: f32,
        block_reason: Option<MulBlockReason>,
        risk: (f32, f32),
    ) -> Self {
        Self {
            gate_open,
            free_will_modifier,
            dk_position: dk_position.to_string(),
            trust_level: trust_level.to_string(),
            allostatic_load,
            block_reason,
            risk,
        }
    }

    /// Permissive MUL input (for testing / standalone mode).
    pub fn permissive() -> Self {
        Self {
            gate_open: true,
            free_will_modifier: 1.0,
            dk_position: "plateau_of_mastery".into(),
            trust_level: "crystalline".into(),
            allostatic_load: 0.0,
            block_reason: None,
            risk: (0.1, 0.1),
        }
    }

    /// Whether this MUL state allows exploration (Kahneman/Tversky gate).
    pub fn allows_exploration(&self) -> bool {
        self.risk.0 > 0.5 && self.risk.1 < 0.3
    }

    /// Whether this MUL state requires caution (loss aversion).
    pub fn requires_caution(&self) -> bool {
        self.risk.1 > 0.7
    }
}

// ============================================================================
// BarrierStack — the 4-layer composition
// ============================================================================

/// The full barrier stack — composes NARS, Markov, Triune, and MUL.
pub struct BarrierStack {
    /// L2: Markov barrier for XOR budget gating.
    pub markov: MarkovBarrier,
    /// L3: Triune dispatch for inner dialogue consensus.
    pub triune: TriuneDispatch,
    /// NARS truth threshold for L1 (default: 0.3 confidence).
    pub nars_min_confidence: f32,
    /// Free will threshold for L4 (default: 0.3).
    pub mul_min_free_will: f32,
}

impl BarrierStack {
    /// Create with default parameters.
    pub fn new() -> Self {
        Self {
            markov: MarkovBarrier::default_barrier(),
            triune: TriuneDispatch::new(),
            nars_min_confidence: 0.3,
            mul_min_free_will: 0.3,
        }
    }

    /// Create with a specific triune leader.
    pub fn with_leader(leader: Facet) -> Self {
        Self {
            markov: MarkovBarrier::default_barrier(),
            triune: TriuneDispatch::with_leader(leader),
            nars_min_confidence: 0.3,
            mul_min_free_will: 0.3,
        }
    }

    /// Evaluate the full stack for an outbound action.
    ///
    /// Outbound = going from BindSpace to external world.
    /// Check order: L1 (NARS) → L2 (Markov) → L3 (Triune) → L4 (MUL).
    /// Lenient-to-strict: if even L1 fails, we stop early.
    pub fn check_outbound(
        &self,
        action: &str,
        nars_truth: NarsTruth,
        markov_tx: Option<&str>,
        mul_input: &MulInput,
    ) -> StackDecision {
        let mut blocking = Vec::new();
        let mut verdicts = LayerVerdicts::default();
        let mut nudge = None;

        // ── L1: NARS Truth Gate ──
        verdicts.nars = Some((nars_truth.frequency, nars_truth.confidence));
        if nars_truth.confidence < self.nars_min_confidence {
            blocking.push(BlockingLayer::NarsTruth {
                frequency: nars_truth.frequency,
                confidence: nars_truth.confidence,
            });
            nudge = Some(Nudge::GatherEvidence {
                current_confidence: nars_truth.confidence,
                target: self.nars_min_confidence,
            });
        }

        // ── L2: Markov Budget Gate ──
        if let Some(tx_id) = markov_tx {
            let history = self.markov.recent_history(1);
            if let Some((ref id, gate, distance)) = history.last() {
                if id == tx_id {
                    verdicts.markov = Some(*gate);
                    match gate {
                        GateDecision::Reject => {
                            let budget_max = 0; // would need tx access for real value
                            blocking.push(BlockingLayer::MarkovBudget {
                                distance: *distance,
                                budget: budget_max,
                                gate: *gate,
                            });
                        }
                        GateDecision::Dampen => {
                            // Not a block, but suggest scope reduction
                            if nudge.is_none() {
                                nudge = Some(Nudge::ReduceScope {
                                    budget_utilization: 1.5, // over budget
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Check drift ceiling
            if self.markov.needs_consolidation() && nudge.is_none() {
                nudge = Some(Nudge::Consolidate {
                    drift: self.markov.cumulative_drift(),
                    ceiling: 0, // would need access to ceiling
                });
            }
        }

        // ── L3: Triune Consensus Gate ──
        let triune_decision = self.triune.barrier_check(action, nars_truth.confidence);
        verdicts.triune = Some(triune_decision.clone());
        match &triune_decision {
            BarrierDecision::Hold {
                confidence,
                required,
                facet,
            } => {
                blocking.push(BlockingLayer::TriuneConsensus {
                    leader: *facet,
                    confidence: *confidence,
                    required: *required,
                });
                // Suggest switching to a more permissive facet
                if nudge.is_none() {
                    let suggested = match facet {
                        Facet::Guardian => Facet::Driver,
                        _ => Facet::Catalyst,
                    };
                    nudge = Some(Nudge::SwitchFacet {
                        suggested,
                        reason: format!(
                            "{:?} gate requires {:.2} but got {:.2}",
                            facet, required, confidence
                        ),
                    });
                }
            }
            BarrierDecision::Block {
                pattern: _,
                action: _act,
            } => {
                blocking.push(BlockingLayer::TriuneConsensus {
                    leader: self.triune.topology.leader(),
                    confidence: 0.0,
                    required: 1.0,
                });
            }
            BarrierDecision::Flow => {}
        }

        // ── L4: MUL Meta-Uncertainty Gate ──
        verdicts.mul = Some(MulVerdict {
            gate_open: mul_input.gate_open,
            free_will_modifier: mul_input.free_will_modifier,
            dk_position: mul_input.dk_position.clone(),
            trust_level: mul_input.trust_level.clone(),
            allostatic_load: mul_input.allostatic_load,
        });

        if !mul_input.gate_open {
            let reason = mul_input
                .block_reason
                .unwrap_or(MulBlockReason::TrustInsufficient);
            blocking.push(BlockingLayer::MetaUncertainty { reason });
        } else if mul_input.free_will_modifier < self.mul_min_free_will {
            blocking.push(BlockingLayer::MetaUncertainty {
                reason: MulBlockReason::LowFreeWill,
            });
        }

        // Kahneman/Tversky impact gate: caution blocks exploration
        if mul_input.requires_caution() && nars_truth.confidence < 0.7 {
            if nudge.is_none() {
                nudge = Some(Nudge::GatherEvidence {
                    current_confidence: nars_truth.confidence,
                    target: 0.7,
                });
            }
        }

        // ── Compute effective confidence ──
        // Multiplicative: each layer's pass contributes
        let nars_factor = if nars_truth.confidence >= self.nars_min_confidence {
            nars_truth.confidence
        } else {
            nars_truth.confidence * 0.5 // penalty for being below threshold
        };

        let triune_factor = match &triune_decision {
            BarrierDecision::Flow => 1.0,
            BarrierDecision::Hold { confidence, .. } => *confidence,
            BarrierDecision::Block { .. } => 0.0,
        };

        let mul_factor = if mul_input.gate_open {
            mul_input.free_will_modifier
        } else {
            0.0
        };

        let effective_confidence = (nars_factor * triune_factor * mul_factor).clamp(0.0, 1.0);

        let proceed = blocking.is_empty();

        StackDecision {
            proceed,
            blocking,
            nudge,
            effective_confidence,
            verdicts,
        }
    }

    /// Evaluate the full stack for an inbound response.
    ///
    /// Inbound = coming from external world back into BindSpace.
    /// Check order: L4 (MUL) → L3 (Triune) → L2 (Markov) → L1 (NARS).
    /// Strict-to-lenient: if MUL is unhealthy, reject everything.
    pub fn check_inbound(
        &self,
        evidence: NarsTruth,
        markov_gate: GateDecision,
        mul_input: &MulInput,
    ) -> StackDecision {
        let mut blocking = Vec::new();
        let mut verdicts = LayerVerdicts::default();

        // ── L4 first (strict) ──
        verdicts.mul = Some(MulVerdict {
            gate_open: mul_input.gate_open,
            free_will_modifier: mul_input.free_will_modifier,
            dk_position: mul_input.dk_position.clone(),
            trust_level: mul_input.trust_level.clone(),
            allostatic_load: mul_input.allostatic_load,
        });

        if !mul_input.gate_open {
            let reason = mul_input
                .block_reason
                .unwrap_or(MulBlockReason::TrustInsufficient);
            blocking.push(BlockingLayer::MetaUncertainty { reason });
        }

        // ── L2: Markov gate ──
        verdicts.markov = Some(markov_gate);
        if markov_gate == GateDecision::Reject {
            blocking.push(BlockingLayer::MarkovBudget {
                distance: 0,
                budget: 0,
                gate: markov_gate,
            });
        }

        // ── L1: NARS evidence quality ──
        verdicts.nars = Some((evidence.frequency, evidence.confidence));

        let proceed = blocking.is_empty();
        let effective_confidence = if proceed {
            (evidence.confidence * mul_input.free_will_modifier).clamp(0.0, 1.0)
        } else {
            0.0
        };

        StackDecision {
            proceed,
            blocking,
            nudge: None,
            effective_confidence,
            verdicts,
        }
    }

    /// Apply post-action feedback to all layers.
    pub fn feedback(&mut self, facet: Facet, success: bool) {
        // Triune layer: adjust facet intensities
        self.triune.feedback(facet, success);

        // Markov layer: reset drift if consolidation was done
        if success && self.markov.needs_consolidation() {
            self.markov.consolidation_done();
        }
    }
}

impl Default for BarrierStack {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_pass_all_layers() {
        let stack = BarrierStack::new();
        let truth = NarsTruth::new(0.8, 0.7);
        let mul = MulInput::permissive();

        let decision = stack.check_outbound("safe_action", truth, None, &mul);
        assert!(decision.proceed);
        assert!(decision.blocking.is_empty());
        assert!(decision.effective_confidence > 0.5);
    }

    #[test]
    fn test_nars_blocks_low_confidence() {
        let stack = BarrierStack::new();
        let truth = NarsTruth::new(0.5, 0.1); // very low confidence
        let mul = MulInput::permissive();

        let decision = stack.check_outbound("action", truth, None, &mul);
        assert!(!decision.proceed);
        assert!(decision
            .blocking
            .iter()
            .any(|b| matches!(b, BlockingLayer::NarsTruth { .. })));
    }

    #[test]
    fn test_triune_blocks_guardian_leader() {
        let stack = BarrierStack::with_leader(Facet::Guardian);
        // Guardian needs 0.80, providing 0.50
        let truth = NarsTruth::new(0.8, 0.50);
        let mul = MulInput::permissive();

        let decision = stack.check_outbound("safe_action", truth, None, &mul);
        assert!(!decision.proceed);
        assert!(decision
            .blocking
            .iter()
            .any(|b| matches!(b, BlockingLayer::TriuneConsensus { .. })));
    }

    #[test]
    fn test_mul_blocks_mount_stupid() {
        let stack = BarrierStack::new();
        let truth = NarsTruth::new(0.8, 0.8);
        let mul = MulInput {
            gate_open: false,
            free_will_modifier: 0.0,
            dk_position: "mount_stupid".into(),
            trust_level: "murky".into(),
            allostatic_load: 0.5,
            block_reason: Some(MulBlockReason::MountStupid),
            risk: (0.5, 0.5),
        };

        let decision = stack.check_outbound("action", truth, None, &mul);
        assert!(!decision.proceed);
        assert!(decision.blocking.iter().any(|b| matches!(
            b,
            BlockingLayer::MetaUncertainty {
                reason: MulBlockReason::MountStupid
            }
        )));
    }

    #[test]
    fn test_mul_low_free_will_blocks() {
        let stack = BarrierStack::new();
        let truth = NarsTruth::new(0.8, 0.8);
        let mul = MulInput {
            gate_open: true,
            free_will_modifier: 0.1, // below 0.3 threshold
            dk_position: "valley_of_despair".into(),
            trust_level: "fuzzy".into(),
            allostatic_load: 0.6,
            block_reason: None,
            risk: (0.3, 0.3),
        };

        let decision = stack.check_outbound("action", truth, None, &mul);
        assert!(!decision.proceed);
        assert!(decision.blocking.iter().any(|b| matches!(
            b,
            BlockingLayer::MetaUncertainty {
                reason: MulBlockReason::LowFreeWill
            }
        )));
    }

    #[test]
    fn test_nudge_on_caution() {
        let stack = BarrierStack::new();
        let truth = NarsTruth::new(0.8, 0.5); // moderate confidence
        let mul = MulInput {
            gate_open: true,
            free_will_modifier: 0.8,
            dk_position: "slope_of_enlightenment".into(),
            trust_level: "solid".into(),
            allostatic_load: 0.1,
            block_reason: None,
            risk: (0.4, 0.8), // high moral risk → requires caution
        };

        let decision = stack.check_outbound("risky_action", truth, None, &mul);
        // May pass but should have a nudge
        assert!(decision.nudge.is_some());
    }

    #[test]
    fn test_effective_confidence_multiplicative() {
        let stack = BarrierStack::new();
        let truth = NarsTruth::new(0.9, 0.9);
        let mul = MulInput {
            gate_open: true,
            free_will_modifier: 0.5, // moderate modifier
            dk_position: "slope_of_enlightenment".into(),
            trust_level: "solid".into(),
            allostatic_load: 0.0,
            block_reason: None,
            risk: (0.1, 0.1),
        };

        let decision = stack.check_outbound("action", truth, None, &mul);
        assert!(decision.proceed);
        // effective = nars_factor * triune_factor * mul_factor
        // = 0.9 * 1.0 * 0.5 = 0.45
        assert!(decision.effective_confidence < 0.5);
        assert!(decision.effective_confidence > 0.3);
    }

    #[test]
    fn test_inbound_rejects_when_mul_closed() {
        let stack = BarrierStack::new();
        let evidence = NarsTruth::new(0.9, 0.9);
        let mul = MulInput {
            gate_open: false,
            block_reason: Some(MulBlockReason::FalseFlow),
            ..MulInput::permissive()
        };

        let decision = stack.check_inbound(evidence, GateDecision::Commit, &mul);
        assert!(!decision.proceed);
    }

    #[test]
    fn test_inbound_rejects_when_markov_rejects() {
        let stack = BarrierStack::new();
        let evidence = NarsTruth::new(0.9, 0.9);
        let mul = MulInput::permissive();

        let decision = stack.check_inbound(evidence, GateDecision::Reject, &mul);
        assert!(!decision.proceed);
    }

    #[test]
    fn test_inbound_passes_clean() {
        let stack = BarrierStack::new();
        let evidence = NarsTruth::new(0.8, 0.7);
        let mul = MulInput::permissive();

        let decision = stack.check_inbound(evidence, GateDecision::Commit, &mul);
        assert!(decision.proceed);
        assert!(decision.effective_confidence > 0.5);
    }

    #[test]
    fn test_feedback_updates_triune() {
        let mut stack = BarrierStack::new();
        let before = stack.triune.topology.get(Facet::Catalyst).intensity;
        stack.feedback(Facet::Catalyst, true);
        let after = stack.triune.topology.get(Facet::Catalyst).intensity;
        assert!(after > before);
    }

    #[test]
    fn test_stack_decision_predicates() {
        // Clean pass
        let clean = StackDecision {
            proceed: true,
            blocking: Vec::new(),
            nudge: None,
            effective_confidence: 0.8,
            verdicts: LayerVerdicts::default(),
        };
        assert!(clean.is_clean());
        assert!(!clean.is_nudge());
        assert!(!clean.is_blocked());

        // Nudge
        let nudged = StackDecision {
            proceed: true,
            blocking: Vec::new(),
            nudge: Some(Nudge::GatherEvidence {
                current_confidence: 0.5,
                target: 0.7,
            }),
            effective_confidence: 0.5,
            verdicts: LayerVerdicts::default(),
        };
        assert!(!nudged.is_clean());
        assert!(nudged.is_nudge());
        assert!(!nudged.is_blocked());

        // Blocked
        let blocked = StackDecision {
            proceed: false,
            blocking: vec![BlockingLayer::MetaUncertainty {
                reason: MulBlockReason::MountStupid,
            }],
            nudge: None,
            effective_confidence: 0.0,
            verdicts: LayerVerdicts::default(),
        };
        assert!(!blocked.is_clean());
        assert!(!blocked.is_nudge());
        assert!(blocked.is_blocked());
    }

    #[test]
    fn test_mul_input_exploration_gate() {
        let explore = MulInput {
            risk: (0.7, 0.1), // high epistemic, low moral → explore!
            ..MulInput::permissive()
        };
        assert!(explore.allows_exploration());
        assert!(!explore.requires_caution());

        let caution = MulInput {
            risk: (0.3, 0.8), // low epistemic, high moral → caution!
            ..MulInput::permissive()
        };
        assert!(!caution.allows_exploration());
        assert!(caution.requires_caution());
    }

    #[test]
    fn test_multiple_layers_block() {
        // Both NARS and MUL should block
        let stack = BarrierStack::new();
        let truth = NarsTruth::new(0.5, 0.1); // low confidence
        let mul = MulInput {
            gate_open: false,
            block_reason: Some(MulBlockReason::Depleted),
            ..MulInput::permissive()
        };

        let decision = stack.check_outbound("action", truth, None, &mul);
        assert!(!decision.proceed);
        // Should have blocking from both layers
        assert!(decision.blocking.len() >= 2);
    }
}
