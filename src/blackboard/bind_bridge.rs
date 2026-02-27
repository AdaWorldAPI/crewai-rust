//! BindSpace ↔ Blackboard bridge — shared typed state for the single binary.
//!
//! When compiled into one binary, ladybug-rs (BindSpace) and crewai-rust
//! (Blackboard) share native Rust types via this bridge.  No HTTP.  No JSON.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────┐
//! │                    Single Binary (ladybug-rs)                      │
//! │                                                                    │
//! │   BindSpace              BindBridge               Blackboard       │
//! │   ═════════              ══════════               ══════════       │
//! │   65K addresses   ◄──►  SubstrateView   ◄──►   TypedSlots         │
//! │   O(1) lookup           trait impl              zero-serde         │
//! │                                                                    │
//! │   Surfaces 0x04    hydrate()──────►  awareness:frame               │
//! │   (NARS prefix)                      AwarenessFrame TypedSlot      │
//! │                                                                    │
//! │   Meta words 4-7   ◄────writeback()  awareness:nars                │
//! │   (truth values)                     NarsSemanticState TypedSlot   │
//! │                                                                    │
//! │   Fluid zone       ◄────writeback()  awareness:spo_triples         │
//! │   (working mem)                      Vec<SpoTriple> TypedSlot     │
//! │                                                                    │
//! │   XOR budget gate   barrier()──────►  External API calls           │
//! │   (Markov chain)                      (xAI, grok-3, etc.)         │
//! └────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The bridge is generic over `SubstrateView` — ladybug-rs provides the
//! concrete implementation.  crewai-rust never imports BindSpace directly.

use std::collections::HashMap;

use crate::blackboard::Blackboard;
use crate::drivers::nars::{AwarenessFrame, AwarenessMatch, AwarenessSummary, NarsSemanticState};

#[cfg(feature = "ladybug")]
use ladybug_contract::container::Container;
#[cfg(feature = "ladybug")]
use ladybug_contract::nars::TruthValue;

// ============================================================================
// SubstrateView — the trait that BindSpace implements
// ============================================================================

/// Abstraction over BindSpace for crewai-rust.
///
/// ladybug-rs implements this trait for its actual `BindSpace`.  In test or
/// standalone mode, a stub implementation can be used.
///
/// The trait uses 16-bit addresses (8-bit prefix : 8-bit slot) matching
/// BindSpace's O(1) array indexing model.
pub trait SubstrateView: Send + Sync {
    /// Read a fingerprint at the given address.
    ///
    /// Returns None if the slot is empty.
    fn read_fingerprint(&self, addr: u16) -> Option<[u64; 256]>;

    /// Read the label at the given address.
    fn read_label(&self, addr: u16) -> Option<String>;

    /// Read NARS truth value from meta words 4-7 at the given address.
    fn read_truth(&self, addr: u16) -> Option<(f32, f32)>;

    /// Write NARS truth value to meta words 4-7 at the given address.
    fn write_truth(&mut self, addr: u16, frequency: f32, confidence: f32);

    /// Hamming similarity search across a zone.
    ///
    /// Returns matches sorted by similarity (descending), up to `top_k`.
    /// `query` is the 256-word fingerprint to search against.
    /// `prefix_range` is the (start_prefix, end_prefix) to search.
    fn hamming_search(
        &self,
        query: &[u64; 256],
        prefix_range: (u8, u8),
        top_k: usize,
        threshold: f32,
    ) -> Vec<SubstrateMatch>;

    /// Write a fingerprint at the given address.
    ///
    /// Returns false if the address is out of range.
    fn write_fingerprint(&mut self, addr: u16, fingerprint: [u64; 256]) -> bool;

    /// XOR-delta write: XOR the fingerprint at `addr` with `delta`.
    ///
    /// This is the atomic writeback operation — no full overwrite, just
    /// the changed bits.
    fn xor_delta(&mut self, addr: u16, delta: [u64; 256]);

    /// Get the noise floor for a given prefix range.
    ///
    /// Returns the average similarity of the bottom 10% of non-empty slots.
    fn noise_floor(&self, prefix_range: (u8, u8)) -> f32;

    /// Read the NARS surface compartment (prefix 0x04).
    ///
    /// Returns all non-empty slots as (slot, label, truth_freq, truth_conf).
    fn nars_surface(&self) -> Vec<(u8, String, f32, f32)>;
}

/// A match result from Hamming similarity search.
#[derive(Clone, Debug)]
pub struct SubstrateMatch {
    /// Address of the matched node.
    pub addr: u16,
    /// Hamming similarity (0.0 = orthogonal, 1.0 = identical).
    pub similarity: f32,
    /// Label of the matched node (if any).
    pub label: Option<String>,
    /// NARS truth value (frequency, confidence) from meta words.
    pub truth: Option<(f32, f32)>,
}

// ============================================================================
// BindBridge — the glue between SubstrateView and Blackboard
// ============================================================================

/// Bridge between BindSpace (via SubstrateView) and Blackboard.
///
/// Provides hydration (BindSpace → Blackboard) and writeback (Blackboard → BindSpace)
/// with zero-serde data flow.
pub struct BindBridge<S: SubstrateView> {
    /// The substrate view (BindSpace implementation).
    substrate: S,
    /// Address mappings: slot key → BindSpace address.
    addr_map: HashMap<String, u16>,
    /// XOR delta accumulator for deferred writeback.
    pending_deltas: Vec<(u16, [u64; 256])>,
    /// Noise floor cache (refreshed per hydration cycle).
    cached_noise_floor: f32,
}

impl<S: SubstrateView> BindBridge<S> {
    /// Create a new bridge with the given substrate view.
    pub fn new(substrate: S) -> Self {
        Self {
            substrate,
            addr_map: HashMap::new(),
            pending_deltas: Vec::new(),
            cached_noise_floor: 0.0,
        }
    }

    /// Get a reference to the underlying substrate.
    pub fn substrate(&self) -> &S {
        &self.substrate
    }

    /// Get a mutable reference to the underlying substrate.
    pub fn substrate_mut(&mut self) -> &mut S {
        &mut self.substrate
    }

    // --- Hydration: BindSpace → Blackboard ---

    /// Hydrate an AwarenessFrame from the substrate and write it to the Blackboard.
    ///
    /// This is the core inbound path: reads BindSpace Hamming matches,
    /// classifies them into crystallized/tensioned/uncertain bins, and
    /// writes the resulting AwarenessFrame as a TypedSlot.
    ///
    /// # Arguments
    ///
    /// * `bb` - The blackboard to hydrate into.
    /// * `query_fingerprint` - The query fingerprint to search against.
    /// * `presence_mode` - Current presence mode (e.g., "work", "personal").
    /// * `session_id` - Current session identifier.
    /// * `current_axes` - Current meaning axis values (for tension detection).
    pub fn hydrate(
        &mut self,
        bb: &mut Blackboard,
        query_fingerprint: &[u64; 256],
        presence_mode: &str,
        session_id: &str,
        current_axes: &HashMap<String, f32>,
    ) {
        // 1. Refresh noise floor for the nodes zone (0x80-0xFF)
        self.cached_noise_floor = self.substrate.noise_floor((0x80, 0xFF));

        // 2. Hamming search across the nodes zone
        let matches = self.substrate.hamming_search(
            query_fingerprint,
            (0x80, 0xFF),
            32,                             // top_k
            self.cached_noise_floor + 0.05, // threshold = noise_floor + margin
        );

        // 3. Classify matches into crystallized / tensioned / uncertain
        let mut crystallized = Vec::new();
        let mut tensioned = Vec::new();
        let mut uncertain = Vec::new();

        for m in &matches {
            let awareness_match = AwarenessMatch {
                similarity: m.similarity,
                presence_mode: presence_mode.to_string(),
                rung_level: ((m.addr >> 8) & 0x07) as u8, // derive rung from prefix
                session_id: session_id.to_string(),
                divergent_axes: HashMap::new(), // filled below for tensioned
            };

            if m.similarity > 0.85 {
                // High similarity = crystallized (settled)
                crystallized.push(awareness_match);
            } else if m.similarity > 0.60 {
                // Medium similarity — check for axis tension
                let has_tension = if let Some((freq, conf)) = m.truth {
                    // Tension = high confidence but moderate similarity
                    conf > 0.5 && m.similarity < 0.75
                } else {
                    false
                };

                if has_tension {
                    let mut am = awareness_match;
                    // Compute divergent axes from truth values
                    if let Some((freq, _conf)) = m.truth {
                        for (axis, &current_val) in current_axes {
                            let cached_val = (freq * 2.0) - 1.0; // freq → [-1,1]
                            if (cached_val - current_val).abs() > 0.3 {
                                am.divergent_axes
                                    .insert(axis.clone(), (cached_val, current_val));
                            }
                        }
                    }
                    tensioned.push(am);
                } else {
                    uncertain.push(awareness_match);
                }
            } else if m.similarity > self.cached_noise_floor + 0.05 {
                // Above noise floor = uncertain
                uncertain.push(awareness_match);
            }
            // Below noise floor: discard
        }

        // 4. Build summary
        let total = crystallized.len() + tensioned.len() + uncertain.len();
        let best_sim = matches.first().map(|m| m.similarity).unwrap_or(0.0);
        let summary = AwarenessSummary {
            total_retrieved: total,
            crystallized_count: crystallized.len(),
            tensioned_count: tensioned.len(),
            uncertain_count: uncertain.len(),
            best_similarity: best_sim,
        };

        // 5. Write AwarenessFrame to Blackboard as TypedSlot
        let frame = AwarenessFrame {
            crystallized,
            tensioned,
            uncertain,
            noise_floor: self.cached_noise_floor,
            summary,
        };

        bb.put_typed(
            crate::drivers::nars::SLOT_AWARENESS_FRAME,
            frame,
            "lb.bind_bridge",
            "awareness.hydrate",
        );
    }

    // --- Writeback: Blackboard → BindSpace ---

    /// Write NARS state back to BindSpace.
    ///
    /// Reads the NarsSemanticState from Blackboard and updates BindSpace
    /// truth values (meta words 4-7) at the appropriate NARS surface addresses.
    pub fn writeback_nars(&mut self, bb: &Blackboard) {
        let nars_state: Option<&NarsSemanticState> =
            bb.get_typed(crate::drivers::nars::SLOT_NARS_STATE);

        let Some(state) = nars_state else { return };

        // Write overall truth to the NARS surface (prefix 0x04, slot 0x00)
        self.substrate.write_truth(
            0x0400,
            state.overall_truth.frequency,
            state.overall_truth.confidence,
        );

        // Write per-axis truths to NARS surface slots
        for (i, (axis, inference)) in state.axis_truths.iter().enumerate() {
            let slot = (i + 1) as u8; // slots 1..N
            let addr = 0x0400 | slot as u16;
            self.substrate
                .write_truth(addr, inference.truth.frequency, inference.truth.confidence);
        }
    }

    /// Queue an XOR delta for deferred writeback.
    ///
    /// Deltas are accumulated and flushed in batch via `flush_deltas()`.
    pub fn queue_delta(&mut self, addr: u16, delta: [u64; 256]) {
        self.pending_deltas.push((addr, delta));
    }

    /// Flush all pending XOR deltas to the substrate.
    ///
    /// Returns the number of deltas flushed.
    pub fn flush_deltas(&mut self) -> usize {
        let count = self.pending_deltas.len();
        for (addr, delta) in self.pending_deltas.drain(..) {
            self.substrate.xor_delta(addr, delta);
        }
        count
    }

    // --- Address mapping ---

    /// Register a Blackboard slot key → BindSpace address mapping.
    pub fn map_slot(&mut self, key: impl Into<String>, addr: u16) {
        self.addr_map.insert(key.into(), addr);
    }

    /// Look up the BindSpace address for a Blackboard slot key.
    pub fn addr_for(&self, key: &str) -> Option<u16> {
        self.addr_map.get(key).copied()
    }

    /// Project a single BindSpace address into the Blackboard as a TypedSlot.
    ///
    /// Reads the fingerprint and truth value at `addr` and writes them
    /// as a typed tuple into the Blackboard.
    #[cfg(feature = "ladybug")]
    pub fn project_addr(&self, bb: &mut Blackboard, key: impl Into<String>, addr: u16) {
        if let Some(fp) = self.substrate.read_fingerprint(addr) {
            let truth = self.substrate.read_truth(addr);
            let label = self.substrate.read_label(addr);

            // Pack as a BindProjection TypedSlot
            let projection = BindProjection {
                addr,
                fingerprint: fp,
                label,
                truth_freq: truth.map(|(f, _)| f).unwrap_or(0.5),
                truth_conf: truth.map(|(_, c)| c).unwrap_or(0.0),
            };

            bb.put_typed(key, projection, "lb.bind_bridge", "bind.project");
        }
    }
}

/// A projected BindSpace node, stored as a Blackboard TypedSlot.
///
/// Zero-serde: lives as a native Rust value in the Blackboard.
#[derive(Debug, Clone)]
pub struct BindProjection {
    /// BindSpace address (prefix:slot).
    pub addr: u16,
    /// Full 256-word fingerprint.
    pub fingerprint: [u64; 256],
    /// Human-readable label (if any).
    pub label: Option<String>,
    /// NARS truth frequency.
    pub truth_freq: f32,
    /// NARS truth confidence.
    pub truth_conf: f32,
}

// ============================================================================
// Stub implementation for standalone / test mode
// ============================================================================

/// A no-op substrate view for when ladybug-rs is not linked.
///
/// All reads return None; all writes are discarded.
pub struct StubSubstrate;

impl SubstrateView for StubSubstrate {
    fn read_fingerprint(&self, _addr: u16) -> Option<[u64; 256]> {
        None
    }
    fn read_label(&self, _addr: u16) -> Option<String> {
        None
    }
    fn read_truth(&self, _addr: u16) -> Option<(f32, f32)> {
        None
    }
    fn write_truth(&mut self, _addr: u16, _frequency: f32, _confidence: f32) {}
    fn hamming_search(
        &self,
        _query: &[u64; 256],
        _prefix_range: (u8, u8),
        _top_k: usize,
        _threshold: f32,
    ) -> Vec<SubstrateMatch> {
        Vec::new()
    }
    fn write_fingerprint(&mut self, _addr: u16, _fingerprint: [u64; 256]) -> bool {
        false
    }
    fn xor_delta(&mut self, _addr: u16, _delta: [u64; 256]) {}
    fn noise_floor(&self, _prefix_range: (u8, u8)) -> f32 {
        0.0
    }
    fn nars_surface(&self) -> Vec<(u8, String, f32, f32)> {
        Vec::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory substrate for testing.
    struct MemSubstrate {
        nodes: HashMap<u16, ([u64; 256], Option<String>, Option<(f32, f32)>)>,
    }

    impl MemSubstrate {
        fn new() -> Self {
            Self {
                nodes: HashMap::new(),
            }
        }

        fn insert(&mut self, addr: u16, fp: [u64; 256], label: &str, freq: f32, conf: f32) {
            self.nodes
                .insert(addr, (fp, Some(label.to_string()), Some((freq, conf))));
        }
    }

    impl SubstrateView for MemSubstrate {
        fn read_fingerprint(&self, addr: u16) -> Option<[u64; 256]> {
            self.nodes.get(&addr).map(|(fp, _, _)| *fp)
        }
        fn read_label(&self, addr: u16) -> Option<String> {
            self.nodes.get(&addr).and_then(|(_, l, _)| l.clone())
        }
        fn read_truth(&self, addr: u16) -> Option<(f32, f32)> {
            self.nodes.get(&addr).and_then(|(_, _, t)| *t)
        }
        fn write_truth(&mut self, addr: u16, freq: f32, conf: f32) {
            if let Some(entry) = self.nodes.get_mut(&addr) {
                entry.2 = Some((freq, conf));
            }
        }
        fn hamming_search(
            &self,
            query: &[u64; 256],
            prefix_range: (u8, u8),
            top_k: usize,
            threshold: f32,
        ) -> Vec<SubstrateMatch> {
            let mut results: Vec<SubstrateMatch> = self
                .nodes
                .iter()
                .filter(|(&addr, _)| {
                    let prefix = (addr >> 8) as u8;
                    prefix >= prefix_range.0 && prefix <= prefix_range.1
                })
                .map(|(&addr, (fp, label, truth))| {
                    // Simple hamming similarity: count matching words
                    let matching: u32 = fp
                        .iter()
                        .zip(query.iter())
                        .map(|(a, b)| (a ^ b).count_zeros())
                        .sum();
                    let total_bits = 256 * 64;
                    let similarity = matching as f32 / total_bits as f32;
                    SubstrateMatch {
                        addr,
                        similarity,
                        label: label.clone(),
                        truth: *truth,
                    }
                })
                .filter(|m| m.similarity >= threshold)
                .collect();
            results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
            results.truncate(top_k);
            results
        }
        fn write_fingerprint(&mut self, addr: u16, fingerprint: [u64; 256]) -> bool {
            self.nodes
                .entry(addr)
                .or_insert(([0u64; 256], None, None))
                .0 = fingerprint;
            true
        }
        fn xor_delta(&mut self, addr: u16, delta: [u64; 256]) {
            if let Some(entry) = self.nodes.get_mut(&addr) {
                for (w, d) in entry.0.iter_mut().zip(delta.iter()) {
                    *w ^= d;
                }
            }
        }
        fn noise_floor(&self, _prefix_range: (u8, u8)) -> f32 {
            0.3
        }
        fn nars_surface(&self) -> Vec<(u8, String, f32, f32)> {
            Vec::new()
        }
    }

    #[test]
    fn test_bridge_hydrate_empty() {
        let substrate = StubSubstrate;
        let mut bridge = BindBridge::new(substrate);
        let mut bb = Blackboard::new();

        bridge.hydrate(&mut bb, &[0u64; 256], "test", "sess-1", &HashMap::new());

        let frame = bb.get_typed::<AwarenessFrame>("awareness:frame").unwrap();
        assert_eq!(frame.summary.total_retrieved, 0);
    }

    #[test]
    fn test_bridge_hydrate_with_matches() {
        let mut substrate = MemSubstrate::new();
        // Insert a high-similarity node (crystallized)
        let mut fp = [0u64; 256];
        fp[0] = 0xFFFF_FFFF_FFFF_FFFF; // some content
        substrate.insert(0x8001, fp, "high_match", 0.9, 0.8);
        // Insert a medium-similarity node (uncertain)
        let mut fp2 = [0u64; 256];
        fp2[0] = 0xAAAA_AAAA_AAAA_AAAA;
        substrate.insert(0x8002, fp2, "medium_match", 0.5, 0.3);

        let mut bridge = BindBridge::new(substrate);
        let mut bb = Blackboard::new();

        let query = [0u64; 256]; // all zeros query
        bridge.hydrate(&mut bb, &query, "work", "sess-1", &HashMap::new());

        let frame = bb.get_typed::<AwarenessFrame>("awareness:frame").unwrap();
        assert!(frame.summary.total_retrieved > 0);
    }

    #[test]
    fn test_bridge_xor_delta_flush() {
        let mut substrate = MemSubstrate::new();
        let fp = [1u64; 256];
        substrate.insert(0x8001, fp, "node", 0.5, 0.5);

        let mut bridge = BindBridge::new(substrate);

        // Queue a delta
        let delta = [1u64; 256]; // XOR with all 1s should zero out
        bridge.queue_delta(0x8001, delta);

        assert_eq!(bridge.flush_deltas(), 1);

        // Verify the fingerprint was XORed
        let result = bridge.substrate().read_fingerprint(0x8001).unwrap();
        assert_eq!(result[0], 0); // 1 ^ 1 = 0
    }

    #[test]
    fn test_bridge_addr_mapping() {
        let bridge = BindBridge::new(StubSubstrate);
        let mut bridge = bridge;
        bridge.map_slot("awareness:frame", 0x0400);
        assert_eq!(bridge.addr_for("awareness:frame"), Some(0x0400));
        assert_eq!(bridge.addr_for("missing"), None);
    }
}
