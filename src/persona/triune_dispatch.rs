//! Triune Dispatch — address-triggered inner dialogue with NARS truth gating.
//!
//! The three facets (Guardian, Driver, Catalyst) communicate as A2A inner
//! dialogue over BindSpace prefix 0x0F.  Each facet occupies a fixed agent
//! slot (0x0C:F0-F2) and uses NARS confidence as a collapse gate to decide
//! what crosses the blood-brain barrier to the external world.
//!
//! # Address Layout
//!
//! ```text
//! Facet Slots (0x0C prefix — agent registry):
//!   0x0C:F0  Guardian
//!   0x0C:F1  Driver
//!   0x0C:F2  Catalyst
//!
//! A2A Channels (0x0F prefix — inner dialogue):
//!   0x0F:hash(F0,F1)  Guardian → Driver
//!   0x0F:hash(F1,F0)  Driver → Guardian
//!   0x0F:hash(F0,F2)  Guardian → Catalyst
//!   0x0F:hash(F2,F0)  Catalyst → Guardian
//!   0x0F:hash(F1,F2)  Driver → Catalyst
//!   0x0F:hash(F2,F1)  Catalyst → Driver
//!
//! Blackboards (0x0E prefix — per-facet state):
//!   0x0E:F0  Guardian blackboard
//!   0x0E:F1  Driver blackboard
//!   0x0E:F2  Catalyst blackboard
//! ```
//!
//! # Blood-Brain Barrier
//!
//! ```text
//! INSIDE (universal grammar)              OUTSIDE (APIs, LLMs)
//! ═══════════════════════                ══════════════════════
//!
//! Guardian ←→ Driver ←→ Catalyst          External LLM
//!     ↕  A2A inner dialogue (0x0F)             ↕
//! TriuneTopology.strategy()               BERT / embedding
//!     ↕                                        ↕
//! NARS collapse gate                      n8n-rs orchestration
//!   freq/conf filter what crosses              ↕
//!     ↕                                        ↕
//! BindSpace ──────────── barrier ──────── External world
//! ```
//!
//! The `CollapseGateConfig.min_confidence` from each facet's module YAML
//! acts as the permeability threshold.  Guardian = 0.80 (strict),
//! Driver = 0.65, Catalyst = 0.50 (permissive).

use std::collections::HashMap;

use crate::blackboard::bind_bridge::{BindBridge, SubstrateView};
use crate::blackboard::Blackboard;
use crate::persona::triune::{CouncilResult, Facet, FacetOpinion, Strategy, TriuneTopology};

// ============================================================================
// Address constants for the triune facets
// ============================================================================

/// Guardian's fixed agent slot (0x0C:F0).
pub const SLOT_GUARDIAN: u8 = 0xF0;
/// Driver's fixed agent slot (0x0C:F1).
pub const SLOT_DRIVER: u8 = 0xF1;
/// Catalyst's fixed agent slot (0x0C:F2).
pub const SLOT_CATALYST: u8 = 0xF2;

/// Prefix for agent registry.
const PREFIX_AGENTS: u8 = 0x0C;
/// Prefix for A2A routing.
const PREFIX_A2A: u8 = 0x0F;
/// Prefix for blackboard.
const PREFIX_BLACKBOARD: u8 = 0x0E;

/// Map a facet to its fixed agent slot.
pub fn facet_slot(facet: Facet) -> u8 {
    match facet {
        Facet::Guardian => SLOT_GUARDIAN,
        Facet::Driver => SLOT_DRIVER,
        Facet::Catalyst => SLOT_CATALYST,
    }
}

/// Map a facet to its BindSpace agent address (0x0C:Fx).
pub fn facet_addr(facet: Facet) -> u16 {
    ((PREFIX_AGENTS as u16) << 8) | facet_slot(facet) as u16
}

/// Map a facet to its blackboard address (0x0E:Fx).
pub fn facet_blackboard_addr(facet: Facet) -> u16 {
    ((PREFIX_BLACKBOARD as u16) << 8) | facet_slot(facet) as u16
}

/// Compute the A2A channel address between two facets (0x0F:XX).
pub fn facet_channel_addr(from: Facet, to: Facet) -> u16 {
    let sender = facet_slot(from);
    let receiver = facet_slot(to);
    let mixed = sender ^ receiver;
    let rotated = sender
        .wrapping_mul(17)
        .wrapping_add(receiver.wrapping_mul(31));
    let channel = mixed ^ rotated;
    ((PREFIX_A2A as u16) << 8) | channel as u16
}

// ============================================================================
// Collapse gate — NARS truth threshold per facet
// ============================================================================

/// Collapse gate configuration — the NARS confidence threshold that
/// determines what crosses the blood-brain barrier.
#[derive(Debug, Clone)]
pub struct CollapseGate {
    /// Minimum confidence to allow crossing the barrier.
    pub min_confidence: f32,
    /// Glob patterns that are always blocked.
    pub block_patterns: Vec<String>,
}

impl CollapseGate {
    /// Guardian gate: strict (0.80 confidence required).
    pub fn guardian() -> Self {
        Self {
            min_confidence: 0.80,
            block_patterns: vec!["delete_*".into(), "destroy_*".into()],
        }
    }

    /// Driver gate: moderate (0.65 confidence required).
    pub fn driver() -> Self {
        Self {
            min_confidence: 0.65,
            block_patterns: Vec::new(),
        }
    }

    /// Catalyst gate: permissive (0.50 confidence required).
    pub fn catalyst() -> Self {
        Self {
            min_confidence: 0.50,
            block_patterns: Vec::new(),
        }
    }

    /// Get the gate for a given facet.
    pub fn for_facet(facet: Facet) -> Self {
        match facet {
            Facet::Guardian => Self::guardian(),
            Facet::Driver => Self::driver(),
            Facet::Catalyst => Self::catalyst(),
        }
    }

    /// Check if a confidence value passes this gate.
    pub fn allows(&self, confidence: f32) -> bool {
        confidence >= self.min_confidence
    }

    /// Check if an action is blocked by pattern.
    pub fn is_blocked(&self, action: &str) -> bool {
        self.block_patterns
            .iter()
            .any(|pat| glob_match(pat, action))
    }
}

/// Simple glob matching (supports only trailing '*').
fn glob_match(pattern: &str, value: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('*') {
        value.starts_with(prefix)
    } else {
        pattern == value
    }
}

// ============================================================================
// Barrier decision — what the collapse gate produces
// ============================================================================

/// The result of a collapse gate check.
#[derive(Debug, Clone, PartialEq)]
pub enum BarrierDecision {
    /// Confidence is high enough — proceed across the barrier.
    Flow,
    /// Confidence is below threshold — hold for deliberation.
    Hold {
        confidence: f32,
        required: f32,
        facet: Facet,
    },
    /// Action matches a block pattern — deny outright.
    Block { pattern: String, action: String },
}

// ============================================================================
// TriuneDispatch — the address-triggered inner dialogue engine
// ============================================================================

/// The triune dispatch engine.
///
/// Manages the Guardian/Driver/Catalyst inner dialogue over A2A channels
/// and gates outbound actions through NARS confidence thresholds.
pub struct TriuneDispatch {
    /// Current triune topology (who's leading, intensities).
    pub topology: TriuneTopology,
    /// Per-facet collapse gates.
    gates: HashMap<Facet, CollapseGate>,
    /// Pending opinions from the current deliberation round.
    opinions: Vec<FacetOpinion>,
    /// Dirty channel addresses from the last cycle.
    dirty_channels: Vec<u16>,
}

impl TriuneDispatch {
    /// Create a new triune dispatch with default topology and gates.
    pub fn new() -> Self {
        let mut gates = HashMap::new();
        gates.insert(Facet::Guardian, CollapseGate::guardian());
        gates.insert(Facet::Driver, CollapseGate::driver());
        gates.insert(Facet::Catalyst, CollapseGate::catalyst());

        Self {
            topology: TriuneTopology::default(),
            gates,
            opinions: Vec::new(),
            dirty_channels: Vec::new(),
        }
    }

    /// Create with a specific leader.
    pub fn with_leader(leader: Facet) -> Self {
        let mut dispatch = Self::new();
        dispatch.topology.set_leader(leader);
        dispatch
    }

    /// Get the current strategy.
    pub fn strategy(&self) -> Strategy {
        self.topology.strategy()
    }

    /// Get the collapse gate for a facet.
    pub fn gate(&self, facet: Facet) -> &CollapseGate {
        self.gates.get(&facet).unwrap()
    }

    // =========================================================================
    // Barrier check — NARS truth gating
    // =========================================================================

    /// Check if an action at a given confidence can cross the barrier.
    ///
    /// The leading facet's gate is used as the primary check.
    /// If the leader denies it, the other two facets can override
    /// only if both have higher confidence than their own thresholds.
    pub fn barrier_check(&self, action: &str, confidence: f32) -> BarrierDecision {
        let leader = self.topology.leader();
        let leader_gate = self.gate(leader);

        // Block patterns are absolute — no override.
        if leader_gate.is_blocked(action) {
            return BarrierDecision::Block {
                pattern: leader_gate
                    .block_patterns
                    .iter()
                    .find(|p| glob_match(p, action))
                    .cloned()
                    .unwrap_or_default(),
                action: action.to_string(),
            };
        }

        // Check the leader's confidence gate.
        if leader_gate.allows(confidence) {
            return BarrierDecision::Flow;
        }

        // Leader says Hold — check if both other facets would allow it.
        // This is the "override" path: two facets can outvote the leader.
        let other_facets: Vec<Facet> = Facet::ALL
            .iter()
            .copied()
            .filter(|f| *f != leader)
            .collect();

        let both_allow = other_facets
            .iter()
            .all(|f| self.gate(*f).allows(confidence));

        if both_allow {
            BarrierDecision::Flow
        } else {
            BarrierDecision::Hold {
                confidence,
                required: leader_gate.min_confidence,
                facet: leader,
            }
        }
    }

    /// Check barrier using NARS truth from a specific BindSpace address.
    ///
    /// Reads the truth value at `addr` and uses its confidence component
    /// as the barrier input.
    pub fn barrier_check_from_addr<S: SubstrateView>(
        &self,
        substrate: &S,
        addr: u16,
        action: &str,
    ) -> BarrierDecision {
        let confidence = substrate
            .read_truth(addr)
            .map(|(_, conf)| conf)
            .unwrap_or(0.0);
        self.barrier_check(action, confidence)
    }

    // =========================================================================
    // Inner dialogue — facet opinion submission
    // =========================================================================

    /// Submit a facet's opinion on the current deliberation.
    pub fn submit_opinion(&mut self, facet: Facet, opinion: String, confidence: f32) {
        let weight = self.topology.get(facet).intensity;
        self.opinions.push(FacetOpinion {
            facet,
            opinion,
            confidence,
            weight,
        });
    }

    /// Run a council deliberation over the submitted opinions.
    ///
    /// Returns the council result and clears opinions for the next round.
    pub fn deliberate(&mut self) -> CouncilResult {
        // Sort by intensity (highest first).
        self.opinions.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let result = CouncilResult {
            opinions: self.opinions.clone(),
            leader: self.topology.leader(),
            balance: self.topology.balance_score(),
            fused: self.topology.is_fused,
            strategy: self.topology.strategy(),
        };

        self.opinions.clear();
        result
    }

    // =========================================================================
    // Address-triggered dispatch — react to dirty 0x0F channels
    // =========================================================================

    /// Scan dirty addresses and identify A2A channel writes.
    ///
    /// Call this after a batch of BindSpace writes. The dirty_addrs
    /// should come from `BindSpace::dirty_addrs()`.
    pub fn scan_dirty(&mut self, dirty_addrs: impl Iterator<Item = u16>) {
        self.dirty_channels.clear();
        for addr in dirty_addrs {
            let prefix = (addr >> 8) as u8;
            if prefix == PREFIX_A2A {
                self.dirty_channels.push(addr);
            }
        }
    }

    /// Check if any triune A2A channels were modified.
    pub fn has_dirty_channels(&self) -> bool {
        !self.dirty_channels.is_empty()
    }

    /// Get all dirty A2A channel addresses from the last scan.
    pub fn dirty_channels(&self) -> &[u16] {
        &self.dirty_channels
    }

    /// Check if a specific facet-to-facet channel was written.
    pub fn channel_dirty(&self, from: Facet, to: Facet) -> bool {
        let addr = facet_channel_addr(from, to);
        self.dirty_channels.contains(&addr)
    }

    /// Process all dirty triune channels: hydrate → deliberate → gate.
    ///
    /// This is the main reactive dispatch loop:
    /// 1. For each dirty channel, read the XOR superposition field
    /// 2. Hydrate the receiving facet's blackboard
    /// 3. Each facet forms an opinion based on its awareness
    /// 4. Run council deliberation
    /// 5. Apply collapse gate to the result
    ///
    /// Returns the council result and barrier decision.
    pub fn process_dirty_cycle<S: SubstrateView>(
        &mut self,
        bridge: &mut BindBridge<S>,
        blackboard: &mut Blackboard,
        query_fingerprint: &[u64; 256],
        action: &str,
    ) -> Option<(CouncilResult, BarrierDecision)> {
        if !self.has_dirty_channels() {
            return None;
        }

        // 1. Hydrate: BindSpace → Blackboard
        bridge.hydrate(
            blackboard,
            query_fingerprint,
            "triune",
            "inner-dialogue",
            &HashMap::new(),
        );

        // 2. Read NARS truth from each facet's blackboard slot
        for facet in Facet::ALL {
            let bb_addr = facet_blackboard_addr(facet);
            let truth = bridge.substrate().read_truth(bb_addr);
            let (freq, conf) = truth.unwrap_or((0.5, 0.0));

            let opinion = match facet {
                Facet::Guardian => {
                    if conf >= self.gate(facet).min_confidence {
                        format!("Confident ({:.2}). Verified safe.", conf)
                    } else {
                        format!(
                            "Insufficient confidence ({:.2}). Hold for verification.",
                            conf
                        )
                    }
                }
                Facet::Driver => {
                    if freq > 0.5 {
                        format!("Positive evidence ({:.2}). Execute.", freq)
                    } else {
                        format!("Weak evidence ({:.2}). Reassess approach.", freq)
                    }
                }
                Facet::Catalyst => {
                    format!(
                        "Exploring with freq={:.2}, conf={:.2}. {} path available.",
                        freq,
                        conf,
                        if conf < 0.5 { "Novel" } else { "Known" }
                    )
                }
            };

            self.submit_opinion(facet, opinion, conf);
        }

        // 3. Deliberate
        let council = self.deliberate();

        // 4. Gate the leading opinion's confidence
        let leader_conf = council
            .opinions
            .iter()
            .find(|o| o.facet == council.leader)
            .map(|o| o.confidence)
            .unwrap_or(0.0);

        let barrier = self.barrier_check(action, leader_conf);

        // 5. Writeback NARS state if the barrier allows flow
        if barrier == BarrierDecision::Flow {
            bridge.writeback_nars(blackboard);
            bridge.flush_deltas();
        }

        self.dirty_channels.clear();
        Some((council, barrier))
    }

    /// Adjust topology based on outcome feedback.
    ///
    /// After an action succeeds or fails, update the triune intensities
    /// to learn which facet's judgment was correct.
    pub fn feedback(&mut self, facet: Facet, success: bool) {
        if success {
            // Boost the facet that was right
            let current = self.topology.get(facet).intensity;
            self.topology.activate(facet, (current + 0.1).min(0.8));
        } else {
            // Dampen the facet that was wrong
            let current = self.topology.get(facet).intensity;
            self.topology.activate(facet, (current - 0.05).max(0.1));
        }
    }
}

impl Default for TriuneDispatch {
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
    use crate::blackboard::bind_bridge::StubSubstrate;

    #[test]
    fn test_facet_slots() {
        assert_eq!(facet_slot(Facet::Guardian), 0xF0);
        assert_eq!(facet_slot(Facet::Driver), 0xF1);
        assert_eq!(facet_slot(Facet::Catalyst), 0xF2);
    }

    #[test]
    fn test_facet_addresses() {
        assert_eq!(facet_addr(Facet::Guardian), 0x0CF0);
        assert_eq!(facet_addr(Facet::Driver), 0x0CF1);
        assert_eq!(facet_addr(Facet::Catalyst), 0x0CF2);
    }

    #[test]
    fn test_channel_asymmetry() {
        // Guardian→Driver and Driver→Guardian should use different channels.
        let gd = facet_channel_addr(Facet::Guardian, Facet::Driver);
        let dg = facet_channel_addr(Facet::Driver, Facet::Guardian);
        assert_ne!(gd, dg);
    }

    #[test]
    fn test_channel_determinism() {
        let a = facet_channel_addr(Facet::Guardian, Facet::Catalyst);
        let b = facet_channel_addr(Facet::Guardian, Facet::Catalyst);
        assert_eq!(a, b);
    }

    #[test]
    fn test_barrier_flow_high_confidence() {
        let dispatch = TriuneDispatch::new();
        // Default leader is Driver (min_confidence 0.65)
        let decision = dispatch.barrier_check("execute_task", 0.80);
        assert_eq!(decision, BarrierDecision::Flow);
    }

    #[test]
    fn test_barrier_hold_low_confidence() {
        let dispatch = TriuneDispatch::with_leader(Facet::Guardian);
        // Guardian requires 0.80
        let decision = dispatch.barrier_check("execute_task", 0.50);
        assert!(matches!(decision, BarrierDecision::Hold { .. }));
    }

    #[test]
    fn test_barrier_block_pattern() {
        let dispatch = TriuneDispatch::with_leader(Facet::Guardian);
        let decision = dispatch.barrier_check("delete_all", 0.99);
        assert!(matches!(decision, BarrierDecision::Block { .. }));
    }

    #[test]
    fn test_barrier_override_two_facets() {
        // Guardian leads with 0.80 threshold.
        // If confidence is 0.60, Guardian says Hold.
        // But Driver (0.65) says Hold too, so no override.
        let dispatch = TriuneDispatch::with_leader(Facet::Guardian);
        let decision = dispatch.barrier_check("safe_action", 0.60);
        assert!(matches!(decision, BarrierDecision::Hold { .. }));

        // With confidence 0.70: Guardian says Hold (needs 0.80).
        // Driver (0.65) says Flow, Catalyst (0.50) says Flow → override!
        let decision = dispatch.barrier_check("safe_action", 0.70);
        assert_eq!(decision, BarrierDecision::Flow);
    }

    #[test]
    fn test_deliberation() {
        let mut dispatch = TriuneDispatch::new();
        dispatch.submit_opinion(Facet::Guardian, "Hold".into(), 0.3);
        dispatch.submit_opinion(Facet::Driver, "Go".into(), 0.8);
        dispatch.submit_opinion(Facet::Catalyst, "Explore".into(), 0.6);

        let result = dispatch.deliberate();
        assert_eq!(result.opinions.len(), 3);
        assert_eq!(result.leader, Facet::Driver); // default leader
    }

    #[test]
    fn test_scan_dirty() {
        let mut dispatch = TriuneDispatch::new();
        // Simulate dirty addresses: one A2A (0x0F:xx), one not
        let dirty = vec![0x0F10u16, 0x8001, 0x0F20, 0x0E05];
        dispatch.scan_dirty(dirty.into_iter());

        assert!(dispatch.has_dirty_channels());
        assert_eq!(dispatch.dirty_channels().len(), 2); // only 0x0F prefix
    }

    #[test]
    fn test_feedback_boosts_facet() {
        let mut dispatch = TriuneDispatch::new();
        let before = dispatch.topology.get(Facet::Catalyst).intensity;
        dispatch.feedback(Facet::Catalyst, true);
        let after = dispatch.topology.get(Facet::Catalyst).intensity;
        assert!(after > before);
    }

    #[test]
    fn test_feedback_dampens_facet() {
        let mut dispatch = TriuneDispatch::with_leader(Facet::Guardian);
        let before = dispatch.topology.get(Facet::Guardian).intensity;
        dispatch.feedback(Facet::Guardian, false);
        let after = dispatch.topology.get(Facet::Guardian).intensity;
        assert!(after < before);
    }

    #[test]
    fn test_process_dirty_cycle_no_dirty() {
        let mut dispatch = TriuneDispatch::new();
        let mut bridge = BindBridge::new(StubSubstrate);
        let mut bb = Blackboard::new();

        let result =
            dispatch.process_dirty_cycle(&mut bridge, &mut bb, &[0u64; 256], "test_action");
        assert!(result.is_none());
    }

    #[test]
    fn test_process_dirty_cycle_with_dirty() {
        let mut dispatch = TriuneDispatch::new();
        let mut bridge = BindBridge::new(StubSubstrate);
        let mut bb = Blackboard::new();

        // Simulate a dirty A2A channel
        let channel = facet_channel_addr(Facet::Guardian, Facet::Driver);
        dispatch.scan_dirty(vec![channel].into_iter());

        let result =
            dispatch.process_dirty_cycle(&mut bridge, &mut bb, &[0u64; 256], "test_action");
        assert!(result.is_some());

        let (council, _barrier) = result.unwrap();
        assert_eq!(council.opinions.len(), 3);
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("delete_*", "delete_all"));
        assert!(glob_match("delete_*", "delete_"));
        assert!(!glob_match("delete_*", "remove_all"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "exactly"));
    }

    #[test]
    fn test_all_six_channels_unique() {
        let pairs = vec![
            (Facet::Guardian, Facet::Driver),
            (Facet::Driver, Facet::Guardian),
            (Facet::Guardian, Facet::Catalyst),
            (Facet::Catalyst, Facet::Guardian),
            (Facet::Driver, Facet::Catalyst),
            (Facet::Catalyst, Facet::Driver),
        ];
        let addrs: Vec<u16> = pairs
            .iter()
            .map(|(from, to)| facet_channel_addr(*from, *to))
            .collect();

        // All 6 should be unique
        let mut unique = addrs.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 6, "All 6 channels must have unique addresses");
    }
}
