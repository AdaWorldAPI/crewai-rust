# CrewAI-Rust Integration Execution Plan

> **Date**: 2026-02-15
> **Branch**: `claude/ada-rs-consolidation-6nvNm`
> **Scope**: What crewai-rust needs to build to integrate with the cognitive stack

---

## Role: CrewAI-Rust IS the Agency

CrewAI-rust is the planning and decision layer — strategy, inner council, agent spawning,
and A2A delegation. It uses ladybug-rs BindSpace as substrate and delegates execution
to n8n-rs workflows. It does NOT depend on ada-rs.

```
CrewAI-rust provides:
├── Strategy: Domain-agnostic planning (WhatIfTree)         ← PARTIALLY EXISTS
├── Inner Council: Balanced/Catalyst/Guardian voting          ← TO BUILD
├── Agent Spawning: Resonance-driven specialist creation      ← TO BUILD
├── thinking_style[10]: Frozen/Crystallized/Discovered        ← TO BUILD
├── MUL Integration: Smart gating for agent decisions          ← TO BUILD
├── Domain Codebooks: 5 pre-built domains (20 FPs each)       ← TO BUILD
├── A2A Protocol: Delegation between agents                    ← PARTIALLY EXISTS
└── Agent → GEL Delegation: Tasks compiled to frames           ← TO BUILD
```

---

## Phase 1: MUL Integration (Priority: HIGHEST)

Wire ladybug-rs MulSnapshot into strategy engine and agent decisions.

### New File: `src/mul_integration.rs` (~200 LOC)

```rust
/// Bridge between ladybug-rs MUL and crewai-rust agent decisions.
///
/// MulSnapshot modulates:
/// - Strategy: WhatIfTree pruning (moral risk), exploration depth (DK state)
/// - Inner Council: Vote weighting (modifier × vote_weight)
/// - Agent Spawning: Gate on creation (need sufficient agency)
/// - Style Updates: Delta bounds (max_delta × modifier)
pub struct MulIntegration;

impl MulIntegration {
    /// Modulate strategy tree depth based on MUL state
    pub fn max_tree_depth(base: usize, mul: Option<&MulSnapshot>) -> usize;

    /// Modulate style update delta
    pub fn max_style_delta(base: f32, mul: Option<&MulSnapshot>) -> f32;

    /// Check if agent spawning is allowed
    pub fn can_spawn_agent(mul: Option<&MulSnapshot>) -> bool;
}
```

### Wiring Points

1. **inner_loop.rs** (or agent loop): `max_delta = 0.1 * mul.modifier`
2. **Strategy pruning**: Prune branches where `mul.risk.1 > 0.7` (moral risk)
3. **Delegation**: MulSnapshot travels with DelegationRequest
4. **Spawn decision**: Block when MountStupid or modifier < 0.7

---

## Phase 2: Inner Council (Priority: HIGH)

### New File: `src/inner_council.rs` (~350 LOC)

```rust
/// Inner council: 3-facet decision protocol.
///
/// Balanced: Weighs evidence, produces measured assessment
/// Catalyst: Optimistic, pushes for action, values epistemic discovery
/// Guardian: Conservative, checks moral implications, can VETO
pub struct InnerCouncil {
    balanced: BalancedFacet,
    catalyst: CatalystFacet,
    guardian: GuardianFacet,
}

pub enum Vote {
    Approve(f32),      // Confidence in approval
    Disapprove(f32),   // Confidence in disapproval
    Abstain,           // Not enough information
    Veto(String),      // Guardian only: blocks regardless (with reason)
}

impl InnerCouncil {
    /// Submit a proposal and collect votes
    pub fn decide(
        &self,
        proposal: &Proposal,
        mul: Option<&MulSnapshot>,
    ) -> CouncilDecision;
}

pub struct CouncilDecision {
    pub approved: bool,
    pub balanced_vote: Vote,
    pub catalyst_vote: Vote,
    pub guardian_vote: Vote,
    pub reasoning: String,
}
```

### Decision Logic

```
1. Guardian evaluates moral risk → if VETO → block regardless
2. Catalyst evaluates epistemic value → if BOOST (value > 0.8) → add weight
3. Balanced evaluates evidence → measured vote
4. All votes weighted by MUL modifier
5. Majority of non-vetoed votes wins
```

---

## Phase 3: Domain Codebooks (Priority: HIGH)

### New File: `src/domain_codebook.rs` (~250 LOC)

```rust
/// Domain-specific fingerprint codebook.
///
/// Each domain has 20 concept fingerprints (deterministic, seeded from name).
/// Strategy engine uses codebook for resonance matching in any domain.
pub struct DomainCodebook {
    pub name: String,
    pub concepts: Vec<ConceptFingerprint>,  // Exactly 20
}

pub struct ConceptFingerprint {
    pub name: String,
    pub fingerprint: [u64; 128],  // Content words only (8192 bits)
    pub role: ConceptRole,
}

pub enum ConceptRole {
    Action,      // Something you DO (refactor, deploy, traverse)
    Entity,      // Something that EXISTS (class, node, server)
    Relation,    // How things CONNECT (inherits, causes, monitors)
    Quality,     // How good something IS (complex, fragile, robust)
}
```

### Pre-Built Domains

```rust
pub fn programming_codebook() -> DomainCodebook;    // 20 concepts
pub fn knowledge_graph_codebook() -> DomainCodebook; // 20 concepts
pub fn devops_codebook() -> DomainCodebook;          // 20 concepts
pub fn chess_codebook() -> DomainCodebook;           // 20 concepts (exists)
pub fn aiwar_codebook() -> DomainCodebook;           // 20 concepts (exists)
```

---

## Phase 4: Agent Spawning (Priority: MEDIUM)

### New File: `src/spawn_manager.rs` (~300 LOC)

```rust
/// Resonance-driven agent spawning.
///
/// When no existing agent matches a task (low resonance score),
/// the spawn manager creates a new specialist agent.
pub struct SpawnManager;

impl SpawnManager {
    /// Evaluate if a new agent is needed
    pub fn evaluate_gap(
        &self,
        task_fp: &[u64; 128],
        agents: &[AgentProfile],
    ) -> SpawnEvaluation;

    /// Compute frozen floor for new agent's thinking_style
    pub fn compute_frozen_floor(
        &self,
        task_fp: &[u64; 128],
        codebook: &DomainCodebook,
    ) -> [f32; 10];

    /// Generate agent configuration from discovered style
    pub fn generate_agent_config(
        &self,
        style: &[f32; 10],
        frozen_floor: &[f32; 10],
        domain: &str,
    ) -> AgentConfig;
}

pub struct SpawnEvaluation {
    pub best_match_score: f32,
    pub gap_severity: f32,       // How badly we need a new agent
    pub recommended_domain: String,
    pub recommended_style: [f32; 10],
    pub spawn_approved: bool,    // MUL-gated
}
```

---

## Phase 5: thinking_style Triangle (Priority: MEDIUM)

### Integration with existing style system

```rust
/// thinking_style[10] with Frozen/Crystallized/Discovered states.
///
/// Each dimension has a state:
/// - Frozen: Identity floor (set at creation, never drops below this)
/// - Crystallized: Learned expertise (promoted from Discovered via L10)
/// - Discovered: Emergent innovation (from cross-domain resonance)
///
/// One-way ratchet: Discovered → Crystallized → Frozen
pub struct ThinkingStyleTriangle {
    pub values: [f32; 10],
    pub states: [StyleState; 10],
    pub frozen_floor: [f32; 10],
}

pub enum StyleState {
    Frozen,        // Cannot be lowered below floor
    Crystallized,  // Stable, can be promoted to Frozen
    Discovered,    // Recent, needs validation before crystallizing
}

impl ThinkingStyleTriangle {
    /// Recover modulation from crystal: mod = crystal ⊕ content
    pub fn recover_modulation(
        crystal_fp: &[u64; 128],
        content_fp: &[u64; 128],
    ) -> [f32; 10];

    /// Crystallize a Discovered dimension (L10 promotion)
    pub fn crystallize(&mut self, dim: usize);

    /// Freeze a Crystallized dimension (identity promotion)
    pub fn freeze(&mut self, dim: usize, floor: f32);

    /// Update with MUL-bounded delta
    pub fn update(
        &mut self,
        deltas: &[f32; 10],
        mul: Option<&MulSnapshot>,
    );
}
```

---

## Phase 6: Agent → GEL Delegation (Priority: LOWER)

### New File: `src/gel_delegation.rs` (~300 LOC)

Compile crewai-rust delegation requests into GEL frames for Redis lane execution.

```rust
pub struct DelegationCompiler;

impl DelegationCompiler {
    /// Compile a delegation request into GEL frames
    pub fn compile_delegation(&self, req: &DelegationRequest) -> Vec<FireflyFrame>;

    /// Compile inner council into GEL fan-out (3 specialist lanes)
    pub fn compile_council(&self, proposal: &Proposal) -> Vec<FireflyFrame>;

    /// Compile WhatIfTree branches into parallel GEL lanes
    pub fn compile_whatif(&self, tree: &WhatIfTree) -> Vec<FireflyFrame>;
}
```

---

## Execution Timeline

```
Week 1: Phase 1 (MUL integration) + Phase 2 (Inner Council)
Week 2: Phase 3 (Domain Codebooks) + Phase 4 (Agent Spawning)
Week 3: Phase 5 (Style Triangle) + Phase 6 (GEL Delegation)
```

---

## Verification Checklist

- [ ] `cargo check` — clean compile
- [ ] `cargo test` — all existing tests pass (backward compatible)
- [ ] Strategy with `mul: None` → unchanged behavior
- [ ] Strategy with MUL → moral risk prunes branches correctly
- [ ] Inner council votes correctly with 3 facets
- [ ] Guardian VETO blocks regardless of other votes
- [ ] Domain codebooks produce deterministic 20-concept sets
- [ ] Spawn manager detects gaps and creates appropriate agent configs
- [ ] thinking_style triangle enforces one-way ratchet
- [ ] Delegation compiles to valid GEL frames
- [ ] No ada-rs dependency anywhere in crewai-rust

---

## Dependency Map

```
ladybug-rs (substrate):
├── MulSnapshot type             ← crewai-rust imports this
├── BindSpace resonance search   ← crewai-rust uses for gap analysis
├── CogRecord                    ← crewai-rust stores agent state as containers
├── DomainCodebook fingerprints  ← generated from BindSpace seed vectors
├── GEL FireflyFrame             ← crewai-rust compiles to this
└── Arrow Flight                 ← crewai-rust communicates via this

n8n-rs (orchestration):
├── Task execution               → crewai-rust delegates to n8n-rs workflows
├── Status reporting              ← crewai-rust receives progress updates
└── Result delivery               ← crewai-rust integrates workflow results
```
