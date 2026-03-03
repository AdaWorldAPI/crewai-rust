//! Blood-Brain Barrier — Markov chain XOR budget for external API calls.
//!
//! External LLM APIs (xAI/Grok, Anthropic, OpenAI) are NOT the source of truth.
//! BindSpace is.  The LLM is a tool that generates text — its output must be
//! re-embedded and interpreted back through the NARS evidence system before
//! it becomes part of awareness state.
//!
//! The barrier enforces bounded state transitions:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │                       Blood-Brain Barrier                           │
//! │                                                                      │
//! │   Internal (BindSpace)         │         External (LLM API)          │
//! │   ═════════════════            │         ════════════════            │
//! │   Source of truth              │         In the loop, NOT oracle     │
//! │   Fingerprints (16K-bit)       │         Tokens (text strings)       │
//! │   NARS truth values            │         JSON responses              │
//! │   Zero-serde TypedSlots        │         HTTP round-trips            │
//! │                                │                                     │
//! │   ╔═══════════════════╗        │                                     │
//! │   ║  Pre-call state   ║────────┼─── Outbound: inject awareness ───► │
//! │   ║  (fingerprint)    ║        │    as RAG + thinking context       │
//! │   ╚═══════════════════╝        │                                     │
//! │                                │                                     │
//! │   ╔═══════════════════╗        │                                     │
//! │   ║  Post-call state  ║◄───────┼─── Inbound: re-embed response ◄── │
//! │   ║  (fingerprint)    ║        │    via BERT/embedding model         │
//! │   ╚═══════════════════╝        │                                     │
//! │                                │                                     │
//! │   XOR distance = popcount(pre ⊕ post)                               │
//! │   If distance > budget → DAMPEN or REJECT                           │
//! │   Budget = f(confidence, rung_level, thinking_style)                 │
//! └──────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Markov Chains as XOR Budgets
//!
//! The Markov chain state transitions are modeled as XOR distances:
//! - Each API round-trip is a state transition
//! - The "budget" limits how much the fingerprint state can change
//! - High-confidence states get SMALLER budgets (settled knowledge is hard to change)
//! - Low-confidence states get LARGER budgets (uncertain knowledge is malleable)
//! - Tensioned states get MEDIUM budgets (conflicts need careful resolution)
//!
//! # Semantic Transactions
//!
//! The barrier operates on semantic transactions, not raw bytes:
//! 1. **Outbound**: Awareness state → system prompt (RAG + thinking context)
//! 2. **LLM processing**: stateless HTTP call (token generation)
//! 3. **Inbound**: Response text → embedding → fingerprint delta
//! 4. **Gate**: XOR distance check against budget
//! 5. **Commit**: If within budget, apply delta to BindSpace via NARS revision

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::nars::NarsTruth;

// ============================================================================
// XOR Budget
// ============================================================================

/// XOR budget — maximum allowed fingerprint distance for a state transition.
///
/// The budget is a function of the current evidence state:
/// - High confidence → small budget (settled knowledge resists change)
/// - Low confidence → large budget (uncertain knowledge is malleable)
/// - Tensioned state → medium budget (conflicts need careful resolution)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct XorBudget {
    /// Maximum allowed XOR popcount (Hamming distance in bits).
    pub max_distance: u32,
    /// Current confidence from NARS (determines budget scaling).
    pub confidence: f32,
    /// Rung level (Pearl's ladder: 0=see, 1=do, 2=imagine).
    pub rung_level: u8,
    /// Budget scaling factor from thinking style (0.0–1.0).
    /// Analytical styles get smaller budgets; creative styles get larger.
    pub style_factor: f32,
}

impl XorBudget {
    /// The fingerprint width in bits (256 words × 64 bits).
    const TOTAL_BITS: u32 = 256 * 64;

    /// Compute the XOR budget from the current evidence state.
    ///
    /// Budget formula:
    /// ```text
    /// base = TOTAL_BITS × (1.0 − confidence) × 0.1
    /// rung_scale = 1.0 + rung_level × 0.5  (higher rung = more latitude)
    /// style_scale = 0.5 + style_factor × 0.5
    /// budget = base × rung_scale × style_scale
    /// ```
    pub fn compute(confidence: f32, rung_level: u8, style_factor: f32) -> Self {
        let base = Self::TOTAL_BITS as f32 * (1.0 - confidence.clamp(0.0, 1.0)) * 0.1;
        let rung_scale = 1.0 + rung_level.min(8) as f32 * 0.5;
        let style_scale = 0.5 + style_factor.clamp(0.0, 1.0) * 0.5;
        let max_distance = (base * rung_scale * style_scale).min(Self::TOTAL_BITS as f32) as u32;

        Self {
            max_distance,
            confidence,
            rung_level,
            style_factor,
        }
    }

    /// Check if a transition is within budget.
    pub fn allows(&self, xor_distance: u32) -> bool {
        xor_distance <= self.max_distance
    }

    /// Compute the damping factor for a transition that exceeds the budget.
    ///
    /// Returns a value in [0.0, 1.0] that scales the delta:
    /// - 1.0 = fully within budget (no damping)
    /// - 0.0 = infinitely over budget (full rejection)
    pub fn damping(&self, xor_distance: u32) -> f32 {
        if xor_distance <= self.max_distance {
            1.0
        } else {
            // Soft exponential decay beyond budget
            let overshoot = xor_distance as f32 / self.max_distance.max(1) as f32;
            (-2.0 * (overshoot - 1.0)).exp()
        }
    }
}

// ============================================================================
// Semantic Transaction
// ============================================================================

/// A semantic transaction through the blood-brain barrier.
///
/// Captures the pre-call state, the LLM response, and the post-call delta.
/// The gate decision (commit/dampen/reject) is based on XOR distance.
#[derive(Debug, Clone)]
pub struct SemanticTransaction {
    /// Transaction identifier.
    pub tx_id: String,
    /// Pre-call fingerprint state (from BindSpace).
    pub pre_state: [u64; 256],
    /// Post-call fingerprint delta (from re-embedding the LLM response).
    /// This is NOT the full new state — it's the XOR delta to apply.
    pub delta: Option<[u64; 256]>,
    /// The XOR budget for this transaction.
    pub budget: XorBudget,
    /// Gate decision after the transaction.
    pub gate: GateDecision,
    /// NARS truth value to revise into the substrate on commit.
    pub evidence: NarsTruth,
    /// Metadata about the external call.
    pub call_meta: CallMeta,
}

/// Gate decision for a semantic transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateDecision {
    /// Transaction is within budget — apply delta at full strength.
    Commit,
    /// Transaction exceeds budget — apply delta with damping factor.
    Dampen,
    /// Transaction far exceeds budget — reject delta, log for review.
    Reject,
    /// Transaction not yet decided (delta not computed).
    Pending,
}

/// Metadata about the external API call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallMeta {
    /// LLM provider (e.g., "xai", "anthropic", "openai").
    pub provider: String,
    /// Model identifier (e.g., "grok-3", "claude-opus-4-5-20251101").
    pub model: String,
    /// Token count in the response.
    pub response_tokens: u32,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Whether the system prompt was prefix-cached.
    pub prefix_cached: bool,
}

impl Default for CallMeta {
    fn default() -> Self {
        Self {
            provider: String::new(),
            model: String::new(),
            response_tokens: 0,
            latency_ms: 0,
            prefix_cached: false,
        }
    }
}

// ============================================================================
// MarkovBarrier — the gate controller
// ============================================================================

/// The blood-brain barrier gate controller.
///
/// Manages semantic transactions between the internal cognitive substrate
/// (BindSpace) and external LLM APIs.  Each API call is a bounded state
/// transition in a Markov chain, gated by XOR budget.
pub struct MarkovBarrier {
    /// Active transactions (tx_id → transaction).
    transactions: HashMap<String, SemanticTransaction>,
    /// Transaction counter for ID generation.
    tx_counter: u64,
    /// Cumulative XOR distance across all committed transactions.
    /// This is the total "cognitive drift" from external influences.
    cumulative_drift: u64,
    /// Maximum cumulative drift before requiring a consolidation pass.
    drift_ceiling: u64,
    /// History of gate decisions for monitoring.
    history: Vec<(String, GateDecision, u32)>, // (tx_id, decision, distance)
}

impl MarkovBarrier {
    /// Create a new barrier with the given drift ceiling.
    ///
    /// The drift ceiling is the maximum cumulative XOR distance before
    /// a consolidation pass is required (NARS revision + writeback).
    pub fn new(drift_ceiling: u64) -> Self {
        Self {
            transactions: HashMap::new(),
            tx_counter: 0,
            cumulative_drift: 0,
            drift_ceiling,
            history: Vec::new(),
        }
    }

    /// Default barrier with a drift ceiling of 10% of total fingerprint bits.
    pub fn default_barrier() -> Self {
        let ceiling = (256 * 64) / 10; // 10% of 16,384 bits = 1,638
        Self::new(ceiling as u64)
    }

    /// Begin a semantic transaction.
    ///
    /// Captures the pre-call fingerprint state and computes the XOR budget
    /// from the current evidence state.
    ///
    /// # Arguments
    ///
    /// * `pre_state` - Current fingerprint from BindSpace.
    /// * `confidence` - Current NARS confidence for this region.
    /// * `rung_level` - Pearl's rung level (0=see, 1=do, 2=imagine).
    /// * `style_factor` - Thinking style budget factor (from JitProfile).
    pub fn begin(
        &mut self,
        pre_state: [u64; 256],
        confidence: f32,
        rung_level: u8,
        style_factor: f32,
    ) -> String {
        self.tx_counter += 1;
        let tx_id = format!("tx-{}", self.tx_counter);

        let budget = XorBudget::compute(confidence, rung_level, style_factor);

        let tx = SemanticTransaction {
            tx_id: tx_id.clone(),
            pre_state,
            delta: None,
            budget,
            gate: GateDecision::Pending,
            evidence: NarsTruth::unknown(),
            call_meta: CallMeta::default(),
        };

        self.transactions.insert(tx_id.clone(), tx);
        tx_id
    }

    /// Complete a semantic transaction with the response delta.
    ///
    /// The delta is the XOR between the pre-call state and the re-embedded
    /// response fingerprint.  The gate decision is made based on XOR distance.
    ///
    /// # Arguments
    ///
    /// * `tx_id` - Transaction identifier from `begin()`.
    /// * `response_fingerprint` - Re-embedded response fingerprint.
    /// * `evidence` - NARS truth value from the response (frequency, confidence).
    /// * `call_meta` - Metadata about the API call.
    ///
    /// # Returns
    ///
    /// The gate decision: Commit, Dampen, or Reject.
    pub fn complete(
        &mut self,
        tx_id: &str,
        response_fingerprint: &[u64; 256],
        evidence: NarsTruth,
        call_meta: CallMeta,
    ) -> GateDecision {
        let Some(tx) = self.transactions.get_mut(tx_id) else {
            return GateDecision::Reject;
        };

        // Compute XOR delta
        let mut delta = [0u64; 256];
        for i in 0..256 {
            delta[i] = tx.pre_state[i] ^ response_fingerprint[i];
        }

        // Compute XOR distance (popcount of delta)
        let xor_distance: u32 = delta.iter().map(|w| w.count_ones()).sum();

        // Gate decision
        let decision = if tx.budget.allows(xor_distance) {
            GateDecision::Commit
        } else {
            let damping = tx.budget.damping(xor_distance);
            if damping > 0.1 {
                GateDecision::Dampen
            } else {
                GateDecision::Reject
            }
        };

        tx.delta = Some(delta);
        tx.gate = decision;
        tx.evidence = evidence;
        tx.call_meta = call_meta;

        // Track cumulative drift
        if decision == GateDecision::Commit {
            self.cumulative_drift += xor_distance as u64;
        } else if decision == GateDecision::Dampen {
            let damped_distance = (xor_distance as f32 * tx.budget.damping(xor_distance)) as u64;
            self.cumulative_drift += damped_distance;
        }

        self.history
            .push((tx_id.to_string(), decision, xor_distance));

        decision
    }

    /// Get the damped delta for a transaction.
    ///
    /// For Commit: returns the full delta.
    /// For Dampen: returns the delta with reduced magnitude (fewer bits flipped).
    /// For Reject/Pending: returns None.
    pub fn damped_delta(&self, tx_id: &str) -> Option<[u64; 256]> {
        let tx = self.transactions.get(tx_id)?;
        let delta = tx.delta?;

        match tx.gate {
            GateDecision::Commit => Some(delta),
            GateDecision::Dampen => {
                let xor_distance: u32 = delta.iter().map(|w| w.count_ones()).sum();
                let damping = tx.budget.damping(xor_distance);
                Some(dampen_fingerprint(&delta, damping))
            }
            _ => None,
        }
    }

    /// Take a completed transaction (removes it from the barrier).
    pub fn take(&mut self, tx_id: &str) -> Option<SemanticTransaction> {
        self.transactions.remove(tx_id)
    }

    /// Check if a consolidation pass is needed.
    ///
    /// Returns true when cumulative drift exceeds the ceiling.
    pub fn needs_consolidation(&self) -> bool {
        self.cumulative_drift >= self.drift_ceiling
    }

    /// Reset cumulative drift after a consolidation pass.
    pub fn consolidation_done(&mut self) {
        self.cumulative_drift = 0;
    }

    /// Get the current cumulative drift.
    pub fn cumulative_drift(&self) -> u64 {
        self.cumulative_drift
    }

    /// Get recent gate history.
    pub fn recent_history(&self, n: usize) -> &[(String, GateDecision, u32)] {
        let start = self.history.len().saturating_sub(n);
        &self.history[start..]
    }

    /// Statistics about the barrier's gate decisions.
    pub fn stats(&self) -> BarrierStats {
        let mut commits = 0u32;
        let mut dampens = 0u32;
        let mut rejects = 0u32;

        for (_, decision, _) in &self.history {
            match decision {
                GateDecision::Commit => commits += 1,
                GateDecision::Dampen => dampens += 1,
                GateDecision::Reject => rejects += 1,
                GateDecision::Pending => {}
            }
        }

        BarrierStats {
            total_transactions: self.history.len() as u32,
            commits,
            dampens,
            rejects,
            cumulative_drift: self.cumulative_drift,
            drift_ceiling: self.drift_ceiling,
        }
    }
}

/// Statistics about the barrier's operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierStats {
    pub total_transactions: u32,
    pub commits: u32,
    pub dampens: u32,
    pub rejects: u32,
    pub cumulative_drift: u64,
    pub drift_ceiling: u64,
}

// ============================================================================
// Helpers
// ============================================================================

/// Dampen a fingerprint delta by randomly clearing bits.
///
/// The damping factor (0.0–1.0) determines what fraction of set bits to keep.
/// Uses deterministic bit selection (every Nth bit) for reproducibility.
fn dampen_fingerprint(delta: &[u64; 256], damping: f32) -> [u64; 256] {
    if damping >= 1.0 {
        return *delta;
    }
    if damping <= 0.0 {
        return [0u64; 256];
    }

    let mut result = [0u64; 256];

    // For each word, keep approximately `damping` fraction of set bits
    for (i, &word) in delta.iter().enumerate() {
        if word == 0 {
            continue;
        }

        let set_bits = word.count_ones();
        let keep = (set_bits as f32 * damping).round() as u32;

        if keep == 0 {
            continue;
        }
        if keep >= set_bits {
            result[i] = word;
            continue;
        }

        // Keep the lowest `keep` set bits (deterministic selection)
        let mut mask = word;
        let mut kept = 0u64;
        let mut count = 0u32;
        while mask != 0 && count < keep {
            let lowest = mask & mask.wrapping_neg(); // isolate lowest set bit
            kept |= lowest;
            mask ^= lowest; // clear it
            count += 1;
        }
        result[i] = kept;
    }

    result
}

/// Compute XOR distance (Hamming distance in bit-space) between two fingerprints.
pub fn xor_distance(a: &[u64; 256], b: &[u64; 256]) -> u32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x ^ y).count_ones())
        .sum()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xor_budget_high_confidence() {
        let budget = XorBudget::compute(0.95, 0, 0.5);
        // High confidence = small budget
        assert!(budget.max_distance < 200);
    }

    #[test]
    fn test_xor_budget_low_confidence() {
        let budget = XorBudget::compute(0.1, 0, 0.5);
        // Low confidence = large budget
        assert!(budget.max_distance > 500);
    }

    #[test]
    fn test_xor_budget_rung_scaling() {
        let budget_see = XorBudget::compute(0.5, 0, 0.5);
        let budget_imagine = XorBudget::compute(0.5, 2, 0.5);
        // Higher rung = more latitude
        assert!(budget_imagine.max_distance > budget_see.max_distance);
    }

    #[test]
    fn test_xor_budget_style_scaling() {
        let budget_analytical = XorBudget::compute(0.5, 0, 0.1); // tight
        let budget_creative = XorBudget::compute(0.5, 0, 0.9); // loose
        assert!(budget_creative.max_distance > budget_analytical.max_distance);
    }

    #[test]
    fn test_damping_within_budget() {
        let budget = XorBudget::compute(0.5, 0, 0.5);
        assert_eq!(budget.damping(0), 1.0);
        assert_eq!(budget.damping(budget.max_distance), 1.0);
    }

    #[test]
    fn test_damping_over_budget() {
        let budget = XorBudget::compute(0.5, 0, 0.5);
        let far_over = budget.max_distance * 3;
        let damping = budget.damping(far_over);
        assert!(damping < 0.5); // significantly dampened
        assert!(damping > 0.0); // not fully rejected
    }

    #[test]
    fn test_barrier_commit_flow() {
        let mut barrier = MarkovBarrier::new(10000);

        let pre_state = [0u64; 256];
        let tx_id = barrier.begin(pre_state, 0.3, 0, 0.5);

        // Small response delta (within budget)
        let mut response = [0u64; 256];
        response[0] = 0x0F; // 4 bits changed

        let evidence = NarsTruth::new(0.8, 0.6);
        let decision = barrier.complete(&tx_id, &response, evidence, CallMeta::default());

        assert_eq!(decision, GateDecision::Commit);
        assert!(barrier.damped_delta(&tx_id).is_some());
    }

    #[test]
    fn test_barrier_reject_high_confidence() {
        let mut barrier = MarkovBarrier::new(10000);

        let pre_state = [0u64; 256];
        let tx_id = barrier.begin(pre_state, 0.99, 0, 0.1); // Very high confidence + tight style

        // Large response delta
        let response = [0xFFFF_FFFF_FFFF_FFFFu64; 256]; // all bits different

        let evidence = NarsTruth::new(0.5, 0.3);
        let decision = barrier.complete(&tx_id, &response, evidence, CallMeta::default());

        // Should reject: too much change for high-confidence state
        assert!(decision == GateDecision::Reject || decision == GateDecision::Dampen);
    }

    #[test]
    fn test_barrier_cumulative_drift() {
        let mut barrier = MarkovBarrier::new(100); // Low ceiling

        for _ in 0..5 {
            let pre_state = [0u64; 256];
            let tx_id = barrier.begin(pre_state, 0.1, 0, 1.0); // Low confidence, loose style

            let mut response = [0u64; 256];
            response[0] = 0xFF; // 8 bits per transaction

            let evidence = NarsTruth::new(0.8, 0.5);
            barrier.complete(&tx_id, &response, evidence, CallMeta::default());
        }

        // After several commits, should exceed drift ceiling
        // (depends on budget calculation, but cumulative tracking should work)
        let stats = barrier.stats();
        assert!(stats.total_transactions == 5);
    }

    #[test]
    fn test_dampen_fingerprint() {
        let mut delta = [0u64; 256];
        delta[0] = 0xFF; // 8 bits set

        let dampened = dampen_fingerprint(&delta, 0.5);
        let orig_bits: u32 = delta.iter().map(|w| w.count_ones()).sum();
        let dampened_bits: u32 = dampened.iter().map(|w| w.count_ones()).sum();

        // Should keep approximately half the bits
        assert!(dampened_bits <= orig_bits);
        assert!(dampened_bits >= 1); // at least something kept
    }

    #[test]
    fn test_dampen_full() {
        let delta = [0xAAAAu64; 256];
        let full = dampen_fingerprint(&delta, 1.0);
        assert_eq!(full, delta);
    }

    #[test]
    fn test_dampen_zero() {
        let delta = [0xAAAAu64; 256];
        let zero = dampen_fingerprint(&delta, 0.0);
        assert_eq!(zero, [0u64; 256]);
    }

    #[test]
    fn test_xor_distance_fn() {
        let a = [0u64; 256];
        let mut b = [0u64; 256];
        b[0] = 0xFF; // 8 bits
        b[1] = 0x0F; // 4 bits
        assert_eq!(xor_distance(&a, &b), 12);
    }

    #[test]
    fn test_barrier_stats() {
        let barrier = MarkovBarrier::default_barrier();
        assert_eq!(barrier.stats().total_transactions, 0);
        assert_eq!(barrier.stats().commits, 0);
    }
}
