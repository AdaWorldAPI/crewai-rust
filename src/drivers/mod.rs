//! Drivers — the interface between agents and the cognitive substrate.
//!
//! Drivers provide pure-function access to cognitive primitives. They own no
//! IO, no HTTP, no state — they transform Blackboard TypedSlot values through
//! well-defined inference rules that any agent (internal or external via
//! MCP/REST) can invoke.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     ┌─────────────────┐     ┌────────────────┐
//! │  Agents      │────▶│  Blackboard      │────▶│  BindSpace     │
//! │ (any origin) │     │  TypedSlots      │     │  (ladybug-rs)  │
//! │              │◀────│                  │◀────│                │
//! └─────────────┘     └────────┬────────┘     └───────┬────────┘
//!                              │                      │
//!                     ┌────────▼────────┐     ┌───────▼────────┐
//!                     │   Drivers        │     │   rustynum     │
//!                     │   (this module)  │     │   (HW accel)   │
//!                     │   nars · spo     │     │   SIMD · BF16  │
//!                     └─────────────────┘     └────────────────┘
//! ```
//!
//! # Design Principles
//!
//! 1. **No bridges** — Drivers are not adapters between mismatched interfaces.
//!    They are the canonical interface itself: the types they define ARE the
//!    Blackboard contract.
//!
//! 2. **Protocol-agnostic** — The same types work as TypedSlots (zero-serde,
//!    in-process) and as JSON (for MCP inbound / REST outbound). Internal and
//!    external agents share one typing surface.
//!
//! 3. **Pure functions** — All inference is `(&input) -> output`. No side
//!    effects, no IO. The Blackboard mediates all state.
//!
//! 4. **Hardware-transparent** — Drivers don't know about SIMD or BF16.
//!    BindSpace and rustynum handle acceleration underneath. Drivers operate
//!    on the semantic level.

pub mod barrier_stack;
pub mod markov_barrier;
pub mod nars;
pub mod spo;

// Re-exports for ergonomic Blackboard access.
pub use barrier_stack::{
    BarrierStack, StackDecision, BlockingLayer, MulBlockReason, MulInput, Nudge,
    LayerVerdicts, MulVerdict,
};
pub use markov_barrier::{
    MarkovBarrier, SemanticTransaction, XorBudget, GateDecision, CallMeta, BarrierStats,
};
pub use nars::{
    AwarenessFrame, AwarenessMatch, AwarenessSummary,
    NarsTruth, NarsRule, NarsAxisInference, NarsSemanticState,
    CausalInference, SimilarityJudgment,
};
pub use spo::{
    SpoTriple, ConversationPredicate,
};
