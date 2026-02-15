# Agent MUL Contracts --- crewai-rust API Surface

> **Date**: 2026-02-15
> **Scope**: Exact API contracts that crewai-rust provides, consumes, and exposes
> **Dependency**: ladybug-rs (BindSpace, MUL, CogRecord, DomainCodebook)
> **Non-dependency**: ada-rs is NOT required. All contracts are ada-agnostic.
> **Companion doc**: `AGENT_ORCHESTRATION_SPEC.md` (architecture and design rationale)

---

## 1. Contracts crewai-rust PROVIDES

These are the APIs that crewai-rust exposes to external consumers (n8n-rs,
custom orchestrators, Arrow Flight clients).

### 1.1 AgentRegistry

Manages the pool of available agents --- registration, discovery, and
capability querying.

```rust
/// Register a new agent from a blueprint.
///
/// Creates the agent in the pool, computes capability fingerprints from
/// the blueprint's skills and domain, and returns the agent identifier.
///
/// Arrow Flight: DoAction("agent.register")
pub fn register_agent(
    blueprint: &AgentBlueprint,
    thinking_style: [f32; 10],
) -> Result<RegisteredAgent, AgentError>;

pub struct RegisteredAgent {
    pub agent_id: String,
    pub capability_fps: Vec<Fingerprint>,
    pub domain: SavantDomain,
    pub frozen_floor: [Option<f32>; 10],
}
```

```rust
/// Discover agents that match a task fingerprint.
///
/// Scans all registered agents, computing resonance between the task
/// fingerprint and each agent's capability fingerprints. Returns agents
/// sorted by match quality (best first).
///
/// Arrow Flight: DoAction("agent.discover")
pub fn discover_agents(
    task_fp: &Fingerprint,
    domain_filter: Option<SavantDomain>,
    min_match: f32,
) -> Vec<AgentMatch>;

pub struct AgentMatch {
    pub agent_id: String,
    pub role: String,
    pub capability_match: f32,  // Hamming similarity to task_fp
    pub domain: SavantDomain,
    pub mul_snapshot: Option<MulSnapshot>,
    pub busy: bool,
}
```

```rust
/// Query an agent's full capabilities.
///
/// Returns the agent's skills, thinking_style, domain, and current
/// cognitive state.
///
/// Arrow Flight: DoAction("agent.capabilities")
pub fn agent_capabilities(
    agent_id: &str,
) -> Result<AgentCapabilities, AgentError>;

pub struct AgentCapabilities {
    pub agent_id: String,
    pub role: String,
    pub skills: Vec<SkillDescriptor>,
    pub domain: SavantDomain,
    pub thinking_style: [f32; 10],
    pub frozen_floor: [Option<f32>; 10],
    pub style_state: StyleTriangle,
    pub performance_score: f64,
    pub tasks_completed: u32,
}

pub struct StyleTriangle {
    pub frozen: [Option<f32>; 10],
    pub crystallized: [f32; 10],
    pub discovered: Option<[f32; 10]>,
    pub effective: [f32; 10],
}
```

### 1.2 Strategy

Planning and what-if evaluation across any registered domain.

```rust
/// Create a what-if planning tree for a domain.
///
/// The tree evaluates hypothetical actions against a goal fingerprint.
/// Branches are scored by resonance with the goal and pruned by MUL state.
///
/// Arrow Flight: DoAction("strategy.whatif.create_tree")
pub fn create_whatif_tree(
    domain_id: &str,
    goal_fp: Fingerprint,
    max_depth: u8,
    max_width: u8,
    mode: StrategicMode,
) -> Result<WhatIfTreeHandle, StrategyError>;

pub struct WhatIfTreeHandle {
    pub tree_id: String,
    pub domain: String,
    pub branch_count: usize,
}
```

```rust
/// Add a branch to an existing what-if tree.
///
/// Arrow Flight: DoAction("strategy.whatif.add_branch")
pub fn add_branch(
    tree_id: &str,
    parent: Option<usize>,
    subject_fp: Fingerprint,
    action_fp: Fingerprint,
    predicted_outcome_fp: Fingerprint,
    confidence: f32,
    mul: &MulSnapshot,
) -> Result<BranchResult, StrategyError>;

pub struct BranchResult {
    pub branch_idx: usize,
    pub resonance_with_goal: f32,
    pub depth: u8,
}
```

```rust
/// Evaluate a specific branch --- compute resonance and confidence metrics.
///
/// Arrow Flight: DoAction("strategy.whatif.evaluate_branch")
pub fn evaluate_branch(
    tree_id: &str,
    branch_idx: usize,
) -> Result<BranchEvaluation, StrategyError>;

pub struct BranchEvaluation {
    pub branch_idx: usize,
    pub resonance_with_goal: f32,
    pub confidence: f32,
    pub mul_modifier_at_creation: f32,
    pub dk_position_at_creation: DKPosition,
    pub child_count: usize,
    pub is_leaf: bool,
}
```

```rust
/// Select the best action from the tree (principal variation root).
///
/// Returns the root branch of the best path through the tree.
///
/// Arrow Flight: DoAction("strategy.whatif.select_action")
pub fn select_action(
    tree_id: &str,
) -> Result<SelectedAction, StrategyError>;

pub struct SelectedAction {
    pub branch_idx: usize,
    pub action_fp: Fingerprint,
    pub predicted_outcome_fp: Fingerprint,
    pub path_confidence: f32,      // Product of confidence along path
    pub path_resonance: f32,       // Minimum resonance along path
    pub principal_variation: Vec<usize>,  // Full path from root to best leaf
}
```

### 1.3 InnerCouncil

Three-facet deliberation for significant decisions.

```rust
/// Submit a proposal to the inner council for deliberation.
///
/// The proposal describes an intended action with its risk profile.
/// The council evaluates it through Balanced, Catalyst, and Guardian facets.
///
/// Arrow Flight: DoAction("council.submit_proposal")
pub fn submit_proposal(
    action_fp: Fingerprint,
    risk: RiskVector,
    mul: &MulSnapshot,
    context: &str,
    impact_level: ImpactLevel,
) -> Result<ProposalHandle, CouncilError>;

pub struct ProposalHandle {
    pub proposal_id: String,
    pub impact_level: ImpactLevel,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub enum ImpactLevel {
    Observe,        // Read-only, always allowed
    Minor,          // Small change, council informed but not consulted
    Significant,    // Council deliberation required
    Critical,       // Council deliberation + Guardian veto check
    Irreversible,   // Council deliberation + unanimous approval required
}
```

```rust
/// Collect votes from all three council facets.
///
/// Each facet evaluates the proposal independently using its own
/// thinking_style configuration. Votes are weighted by MUL modifier.
///
/// Arrow Flight: DoAction("council.collect_votes")
pub fn collect_votes(
    proposal_id: &str,
) -> Result<CouncilVotes, CouncilError>;

pub struct CouncilVotes {
    pub proposal_id: String,
    pub balanced: FacetVote,
    pub catalyst: FacetVote,
    pub guardian: FacetVote,
}

pub struct FacetVote {
    pub vote: Vote,
    pub weight: f32,           // base_weight * mul.modifier
    pub reasoning: String,     // Why this facet voted this way
    pub risk_assessment: f32,  // Facet's own risk estimate
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vote {
    Approve,
    Disapprove,
    Abstain,
    Veto,  // Guardian only
}
```

```rust
/// Render final council decision from collected votes.
///
/// Applies Guardian veto override, Catalyst boost, and majority tally.
/// Returns the final decision with full reasoning trail.
///
/// Arrow Flight: DoAction("council.decide")
pub fn decide(
    proposal_id: &str,
) -> Result<CouncilDecision, CouncilError>;

pub struct CouncilDecision {
    pub proposal_id: String,
    pub decision: Decision,
    pub weighted_score: f32,
    pub veto_active: bool,
    pub boost_active: bool,
    pub reasoning: String,
    pub decided_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Approved,   // Weighted sum > 0, no veto
    Rejected,   // Weighted sum <= 0, no veto
    Blocked,    // Guardian veto active
    Defer,      // Weighted sum == 0, gather more info
}
```

### 1.4 SpawnManager

Evaluates capability gaps and dynamically creates specialist agents.

```rust
/// Evaluate whether a new agent should be spawned for a task.
///
/// Performs: resonance scan, gap analysis, style discovery, novelty check,
/// and MUL gating. Returns the spawn decision with full justification.
///
/// Arrow Flight: DoAction("spawn.evaluate")
pub fn evaluate_gap(
    task_fp: &Fingerprint,
    required_capabilities: &[Fingerprint],
    urgency: f32,
    mul: &MulSnapshot,
) -> Result<SpawnEvaluation, SpawnError>;

pub struct SpawnEvaluation {
    pub should_spawn: bool,
    pub reason: String,
    pub gap_analysis: GapAnalysis,
    pub proposed_style: Option<[f32; 10]>,
    pub proposed_role: Option<String>,
    pub novelty_score: f32,
    pub mul_gates_passed: MulGateResult,
}

pub struct GapAnalysis {
    pub best_existing_match: f32,    // Highest resonance among existing agents
    pub best_existing_agent: Option<String>,  // Who came closest
    pub missing_capabilities: Vec<Fingerprint>,
    pub cross_domain_sources: Vec<CrossDomainSource>,
}

pub struct CrossDomainSource {
    pub domain: String,
    pub concept: String,
    pub similarity: f32,
    pub recovered_style: [f32; 10],
}

pub struct MulGateResult {
    pub not_mount_stupid: bool,
    pub trust_sufficient: bool,  // trust > 0.5
    pub modifier_sufficient: bool,  // modifier > 0.7
    pub all_passed: bool,
}
```

```rust
/// Spawn a new agent from an approved SpawnEvaluation.
///
/// Generates YAML, computes frozen floor, registers in agent pool.
/// Requires that evaluate_gap returned should_spawn=true.
///
/// Arrow Flight: DoAction("spawn.create")
pub fn spawn_agent(
    evaluation: &SpawnEvaluation,
    mul: &MulSnapshot,
) -> Result<SpawnedAgent, SpawnError>;

pub struct SpawnedAgent {
    pub agent_id: String,
    pub role: String,
    pub yaml_content: String,
    pub thinking_style: [f32; 10],
    pub frozen_floor: [Option<f32>; 10],
    pub source_domains: Vec<String>,
    pub source_crystals: Vec<Fingerprint>,
}
```

```rust
/// Reconfigure a spawned agent after creation.
///
/// Allows adjusting non-frozen thinking_style values, domain assignment,
/// and tool access. Frozen floor values cannot be overridden.
///
/// Arrow Flight: DoAction("spawn.configure")
pub fn configure_agent(
    agent_id: &str,
    style_overrides: Option<[f32; 10]>,
    domain_override: Option<SavantDomain>,
    tool_overrides: Option<Vec<String>>,
) -> Result<ConfiguredAgent, SpawnError>;

pub struct ConfiguredAgent {
    pub agent_id: String,
    pub effective_style: [f32; 10],
    pub frozen_enforced: [bool; 10],  // true where frozen floor was applied
    pub domain: SavantDomain,
}
```

---

## 2. Contracts crewai-rust CONSUMES (from ladybug-rs)

These are the ladybug-rs APIs that crewai-rust calls. crewai-rust is a
consumer of these services, not a provider.

### 2.1 MulSnapshot

```rust
/// Obtain a MUL snapshot for the current cognitive state.
///
/// crewai-rust calls this before every significant operation:
/// style updates, delegation, spawning, council decisions.
///
/// Provided by: ladybug-rs MetaUncertaintyLayer
pub fn mul_snapshot(container_id: u64) -> MulSnapshot;

pub struct MulSnapshot {
    pub gate_open: bool,
    pub modifier: f32,               // 0.0-1.0, product of 4 factors
    pub dk_position: DKPosition,     // MountStupid | Valley | Slope | Plateau
    pub trust: f32,                  // Calibrated self-trust
    pub risk: RiskVector,            // Epistemic + moral risk
    pub homeostasis: HomeostasisState,
}

pub struct RiskVector {
    pub epistemic: f32,   // Uncertainty about facts
    pub moral: f32,       // Ethical concern level
}

impl RiskVector {
    pub fn combined(&self) -> f32 {
        (self.epistemic + self.moral) / 2.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DKPosition {
    MountStupid,        // Overconfident ignorance
    ValleyOfDespair,    // Aware of ignorance
    SlopeOfEnlightenment,  // Building real competence
    PlateauOfMastery,   // Calibrated expertise
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeostasisState {
    Flow,     // Challenge matches skill
    Anxiety,  // Challenge exceeds skill
    Boredom,  // Skill exceeds challenge
    Apathy,   // Neither challenge nor skill
}
```

### 2.2 BindSpace

```rust
/// Read a fingerprint from BindSpace at a specific address.
///
/// Provided by: ladybug-rs BindSpace
pub fn read(prefix: u8, slot: u64) -> Option<Fingerprint>;

/// Write a fingerprint to BindSpace at a specific address.
///
/// Provided by: ladybug-rs BindSpace
pub fn write(prefix: u8, slot: u64, fp: Fingerprint);

/// Search BindSpace for fingerprints similar to a query.
///
/// Returns (slot, similarity) pairs sorted by similarity descending.
///
/// Provided by: ladybug-rs BindSpace
pub fn resonance_search(
    query: &Fingerprint,
    max_results: usize,
    min_similarity: f32,
) -> Vec<(u64, f32)>;

/// BindSpace zone prefixes used by crewai-rust:
pub const SURFACE_ZONE: u8 = 0x00;  // Ephemeral working memory
pub const FLUID_ZONE: u8   = 0x10;  // Active reasoning (delegation results)
pub const NODE_ZONE: u8    = 0x80;  // Crystallized knowledge (permanent)
pub const EDGE_ZONE: u8    = 0x0E;  // Delegation requests (inter-agent)
```

### 2.3 CogRecord

```rust
/// CogRecord --- 8192-bit container with 128x u64 metadata words.
///
/// crewai-rust reads and writes specific word ranges:
///   W0-W3:   Identity (DN address, node_kind, domain)
///   W4-W7:   NARS truth values (frequency, confidence, evidence)
///   W8:      Gate state (FLOW=0, HOLD=1, BLOCK=2)
///   W12-W15: 10-layer thinking_style markers
///   W16-W31: Inline edges (skill_id -> verb:target)
///   W32-W39: Q-values (action selection)
///   W40-W47: Bloom filter (neighbor membership)
///   W48-W55: Graph metrics (pagerank, degree, clustering)
///   W56-W63: Qualia channels (volition + affect)
///
/// Provided by: ladybug-rs CogRecord
pub struct CogRecord {
    pub metadata: [u64; 128],
    pub content: Fingerprint,
}
```

### 2.4 DomainCodebook

```rust
/// Domain codebook --- maps concepts to fingerprints for a specific domain.
///
/// crewai-rust consumes pre-built codebooks from ladybug-rs and can register
/// custom domains at runtime.
///
/// Provided by: ladybug-rs DomainCodebook
pub struct DomainCodebook {
    pub role: Fingerprint,
    pub name: String,
    pub concepts: Vec<ConceptEntry>,
}

pub struct ConceptEntry {
    pub name: String,
    pub base_fp: Fingerprint,
    pub bound_fp: Fingerprint,
    pub category: u16,
    pub index: u8,
}

/// Cross-domain similarity between two role-bound concept fingerprints.
pub fn cross_domain_similarity(a: &ConceptEntry, b: &ConceptEntry) -> f32;

/// Measure how much a container fingerprint exhibits a concept.
pub fn measure_concept(container_fp: &Fingerprint, concept: &ConceptEntry) -> f32;
```

---

## 3. Contracts crewai-rust PROVIDES to n8n-rs

n8n-rs is the workflow orchestration engine. crewai-rust provides agent
coordination services that n8n-rs consumes for multi-agent workflows.

### 3.1 Task Delegation

```rust
/// Delegate a complex task from an n8n-rs workflow to crewai-rust agents.
///
/// n8n-rs calls this when a workflow step has step_type = "crew.*".
/// crewai-rust selects the best agent(s), executes, and returns results.
///
/// Wire format: StepDelegationRequest / StepDelegationResponse
///   (defined in src/contract/types.rs --- shared with n8n-rs)
pub fn delegate_task(
    request: StepDelegationRequest,
) -> Result<StepDelegationResponse, DelegationError>;
```

The `StepDelegationRequest` includes:
- `step.step_type`: Routing prefix (e.g., "crew.agent", "crew.council")
- `step.input`: Task payload as JSON
- `input.metadata.layer_activations`: Optional 10-layer state from prior step
- `input.metadata.dominant_layer`: Which cognitive layer produced the input
- `input.metadata.nars_frequency`: Confidence from prior step's L9 validation

crewai-rust processes the request through its agent pool:
1. Parse the task fingerprint from the step input
2. Discover matching agents via `AgentRegistry::discover_agents()`
3. Select best agent by `capability_match * performance_score`
4. Execute the agent's cognitive cycle
5. Return `StepDelegationResponse` with output, reasoning, and confidence

### 3.2 Agent Coordination

```rust
/// Coordinate multiple agents for a multi-step workflow.
///
/// n8n-rs provides the workflow DAG. crewai-rust assigns agents to steps,
/// manages delegation between agents, and integrates results.
///
/// Used when an n8n-rs workflow contains multiple crew.* steps that
/// need coordinated agent assignment.
pub fn coordinate_workflow(
    steps: Vec<UnifiedStep>,
    constraints: CoordinationConstraints,
) -> Result<CoordinationPlan, CoordinationError>;

pub struct CoordinationConstraints {
    pub max_agents: u8,
    pub allow_spawning: bool,
    pub domain_filter: Option<SavantDomain>,
    pub mul_floor: Option<f32>,  // Minimum MUL modifier for participating agents
}

pub struct CoordinationPlan {
    pub assignments: Vec<StepAssignment>,
    pub spawned_agents: Vec<String>,
    pub delegation_chains: Vec<DelegationChain>,
}

pub struct StepAssignment {
    pub step_id: String,
    pub agent_id: String,
    pub capability_match: f32,
    pub estimated_confidence: f32,
}

pub struct DelegationChain {
    pub source_step: String,
    pub target_step: String,
    pub delegation_reason: String,
}
```

### 3.3 Result Integration

```rust
/// Integrate results from an n8n-rs workflow back through the inner council.
///
/// After n8n-rs completes a multi-agent workflow, the combined results
/// are evaluated by the inner council before being committed.
///
/// This ensures that multi-agent workflows respect the same safety
/// and quality constraints as single-agent decisions.
pub fn integrate_workflow_results(
    execution: &UnifiedExecution,
    mul: &MulSnapshot,
) -> Result<IntegrationResult, IntegrationError>;

pub struct IntegrationResult {
    pub merged_fp: Fingerprint,
    pub confidence: f32,
    pub council_decision: CouncilDecision,
    pub new_insights: Vec<EpiphanyCandidate>,
    pub crystallized: Vec<Fingerprint>,  // Results committed to Node zone
    pub rejected: Vec<RejectedResult>,   // Results that failed validation
}

pub struct RejectedResult {
    pub step_id: String,
    pub reason: String,
    pub agent_id: String,
    pub confidence: f32,
}
```

---

## 4. MUL Wiring Points

These are the exact code locations where MUL integrates with crewai-rust's
agent machinery. Each wiring point describes what MUL controls and how.

### 4.1 inner_loop.rs --- Style Update Delta Bounding

**File**: `src/persona/inner_loop.rs`
**Function**: `apply_result()` (line ~130)
**Integration point**: Before applying any `InnerThoughtResult::AdjustStyle`

```
CURRENT BEHAVIOR:
  new_state.thinking_style = clamp_array_10(ts);
  // No MUL bounding --- style can change by arbitrary amount

MUL-WIRED BEHAVIOR:
  let mul = obtain_mul_snapshot(agent_container_id);
  let max_delta = 0.1 * mul.modifier;

  for i in 0..10 {
      let proposed = ts[i];
      let current = state.thinking_style[i];
      let delta = (proposed - current).clamp(-max_delta, max_delta);
      let floor = frozen_floor[i].unwrap_or(0.0);
      new_state.thinking_style[i] = (current + delta).clamp(floor, 1.0);
  }
```

**Effect**: When `mul.modifier` is low (system uncertain), style changes are
tiny. When high (system confident), changes are larger but still bounded by
the 0.1 maximum per step. Frozen floor values are always enforced.

**Validation interaction**: The `validate_result()` function (line ~114) should
additionally check MUL state:
- If `mul.gate_open == false`: all results except `Continue` are blocked
  (equivalent to `SelfModifyBounds::None`)
- If `mul.dk_position == MountStupid`: `AdjustStyle` blocked (system is not
  competent enough to know how to improve itself)

### 4.2 Delegation Logic --- MUL Snapshot Travel

**File**: `src/contract/types.rs` (DataEnvelope, EnvelopeMetadata)
**Integration point**: When constructing `StepDelegationRequest`

```
CURRENT BEHAVIOR:
  EnvelopeMetadata has optional layer_activations and nars_frequency.
  No MUL state is transmitted.

MUL-WIRED BEHAVIOR:
  EnvelopeMetadata gains:
    pub mul_snapshot: Option<MulSnapshot>,
    // Serialized MUL state of the delegating agent

  When crewai-rust constructs a DelegationRequest:
    envelope.metadata.mul_snapshot = Some(current_mul_snapshot);

  When receiving agent processes the request:
    if let Some(delegator_mul) = request.input.metadata.mul_snapshot {
        if delegator_mul.dk_position == DKPosition::MountStupid {
            // Delegator was overconfident --- treat task framing with caution
            increase_own_validation_threshold();
        }
        if delegator_mul.modifier < 0.3 {
            // Delegator had very low confidence --- this might be a bad task
            request_clarification_before_executing();
        }
    }
```

**Effect**: MUL state propagates through delegation chains. Each agent in the
chain knows how confident the upstream agent was. This prevents cascading
overconfidence through multi-agent workflows.

### 4.3 Spawn Logic --- MUL Gates Agent Creation

**File**: `src/meta_agents/types.rs` (SpawnedAgentState)
**Integration point**: Before `SpawnedAgentState::new()` is called

```
MUL GATES (all three must pass):

Gate 1: DK Position
  PASS: dk_position != MountStupid
  FAIL: "Cannot evaluate capability gaps while on MountStupid ---
         overconfident ignorance prevents accurate gap analysis"

Gate 2: Trust
  PASS: trust > 0.5
  FAIL: "Trust insufficient for gap analysis. Need calibrated
         confidence to determine if a new agent is truly needed."

Gate 3: FreeWillModifier
  PASS: modifier > 0.7
  FAIL: "Insufficient metacognitive agency. Creating a new agent
         is a significant architectural decision requiring high modifier."

Combined gate:
  spawn_allowed = gate_1 && gate_2 && gate_3

  If !spawn_allowed:
    Store SpawnRequest in Fluid zone as pending
    Log: "Spawn deferred: {failed_gate_reason}"
    Re-evaluate when MUL state changes
```

### 4.4 Council --- Votes Weighted by MUL Modifier

**File**: New integration in council decision logic
**Integration point**: During vote tallying in `CouncilDecision::decide()`

```
VOTE WEIGHTING:

Each facet's vote is weighted by the current MUL modifier:

  balanced_weight  = balanced_base_weight  * mul.modifier
  catalyst_weight  = catalyst_base_weight  * mul.modifier * boost_factor
  guardian_weight  = guardian_base_weight   // NOT MUL-weighted (safety is absolute)

Where:
  base_weight(Approve)    = +1.0
  base_weight(Disapprove) = -1.0
  base_weight(Abstain)    =  0.0
  base_weight(Veto)       = -infinity (Guardian only, not weighted)
  boost_factor            = 1.5 if Catalyst BOOST active, else 1.0

EFFECT:
  When mul.modifier is LOW (e.g., 0.3):
    Balanced Approve = +0.3
    Catalyst Approve = +0.3 (or +0.45 with boost)
    Guardian Approve = +1.0 (always full weight)
    --> Guardian's voice dominates when system is uncertain

  When mul.modifier is HIGH (e.g., 0.9):
    Balanced Approve = +0.9
    Catalyst Approve = +0.9 (or +1.35 with boost)
    Guardian Approve = +1.0
    --> All three voices roughly equal when system is confident

  The Guardian's vote is never MUL-weighted because safety constraints
  must hold regardless of metacognitive confidence.
```

### 4.5 Summary Table: MUL Wiring Points

| Location | What MUL Controls | MUL Field Used | Gate Behavior |
|----------|-------------------|----------------|---------------|
| `inner_loop.rs` apply_result | Max style delta per step | `modifier` | `max_delta = 0.1 * modifier` |
| `inner_loop.rs` validate_result | Whether style changes allowed at all | `gate_open`, `dk_position` | Gate closed or MountStupid blocks all changes |
| Delegation request construction | MUL snapshot attached to envelope | Full `MulSnapshot` | Propagates confidence state through delegation chain |
| Delegation response processing | Calibrate trust in upstream framing | Delegator's `dk_position`, `modifier` | Increase validation if delegator was uncertain |
| Spawn evaluation | Whether spawning is permitted | `dk_position`, `trust`, `modifier` | Three-gate check: not MountStupid, trust>0.5, modifier>0.7 |
| Spawn deferral | Store pending spawn for re-evaluation | `gate_open` | Gate closed: store and wait |
| Council vote weighting | How much each facet's vote counts | `modifier` | Non-Guardian votes scaled by modifier |
| Council Guardian veto | Safety override | Not MUL-gated | Guardian veto is absolute (MUL-independent) |
| Agent autonomy level | What actions agent can take | `dk_position`, `gate_open` | MountStupid=Observe, Valley=Explore, Slope+=Execute |
| Style crystallization | Whether learning is committed | `gate_open`, `dk_position` | Gate must be open and not MountStupid to crystallize |

---

## 5. Backward Compatibility

### 5.1 Optional MulSnapshot

All APIs that accept `MulSnapshot` accept it as `Option<MulSnapshot>`:

```rust
pub fn evaluate_gap(
    task_fp: &Fingerprint,
    required_capabilities: &[Fingerprint],
    urgency: f32,
    mul: Option<&MulSnapshot>,  // None = use default thresholds
) -> Result<SpawnEvaluation, SpawnError>;
```

When `mul` is `None`:
- Style updates use fixed `max_delta = 0.1` (no MUL scaling)
- Spawning uses fixed thresholds (always allowed if novelty check passes)
- Council votes use equal weights (no MUL scaling)
- Delegation does not transmit confidence state

This preserves backward compatibility with existing crewai-rust code that
does not yet integrate ladybug-rs MUL.

### 5.2 Existing Type Compatibility

The contracts in this document extend existing types without breaking them:

| Existing Type | Extension | Backward Compatible? |
|---------------|-----------|---------------------|
| `AgentState` | Add `mul_snapshot: Option<MulSnapshot>` field | Yes (Option, defaults to None) |
| `EnvelopeMetadata` | Add `mul_snapshot: Option<MulSnapshot>` field | Yes (skip_serializing_if None) |
| `SpawnedAgentState` | Add `frozen_floor: [Option<f32>; 10]` field | Yes (defaults to all-None) |
| `AgentBlueprint` | Add `frozen_floor: Option<[Option<f32>; 10]>` field | Yes (Option) |
| `InnerThoughtResult` | MUL bounding in `apply_result()` | Yes (behavioral change, not API change) |

---

## 6. Error Types

```rust
#[derive(Debug, Clone)]
pub enum AgentError {
    NotFound(String),
    AlreadyRegistered(String),
    InvalidBlueprint(String),
    MulGateClosed(String),
}

#[derive(Debug, Clone)]
pub enum StrategyError {
    TreeNotFound(String),
    BranchNotFound(usize),
    MaxDepthExceeded(u8),
    MaxWidthExceeded(u8),
    DomainNotRegistered(String),
}

#[derive(Debug, Clone)]
pub enum CouncilError {
    ProposalNotFound(String),
    VotingIncomplete(String),
    GuardianVeto(String),
}

#[derive(Debug, Clone)]
pub enum SpawnError {
    MulGateFailed(MulGateResult),
    InsufficientNovelty { max_similarity: f32, threshold: f32 },
    EvaluationNotApproved(String),
    CouncilRejected(CouncilDecision),
}

#[derive(Debug, Clone)]
pub enum DelegationError {
    NoMatchingAgent(Fingerprint),
    AllAgentsBusy,
    MulValidationFailed(String),
    ResultRejected(String),
}

#[derive(Debug, Clone)]
pub enum IntegrationError {
    EmptyExecution,
    CouncilBlocked(CouncilDecision),
    AllResultsRejected(Vec<RejectedResult>),
}
```

---

## 7. Dependency Diagram

```
                    +--------------+
                    | ladybug-rs   |
                    |              |
                    | Provides:    |
                    |  MulSnapshot |
                    |  BindSpace   |
                    |  CogRecord   |
                    |  DomainCB    |
                    |  Fingerprint |
                    |  SPO Crystal |
                    +------+-------+
                           |
              +------------+------------+
              |                         |
     +--------v--------+      +--------v--------+
     |   crewai-rust   |      |     ada-rs      |
     |                 |      |   (OPTIONAL)     |
     | Provides:       |      |                  |
     |  AgentRegistry  |      | Adds:            |
     |  Strategy       |      |  Sovereignty     |
     |  InnerCouncil   |      |  Qualia          |
     |  SpawnManager   |      |  Presence        |
     |                 |      |  Self-Model       |
     | Consumes:       |      +--------+---------+
     |  MulSnapshot    |               |
     |  BindSpace      |               | (optional consumer
     |  CogRecord      |               |  of crewai-rust)
     |  DomainCodebook |               |
     +--------+--------+
              |
     +--------v--------+
     |     n8n-rs      |
     |                 |
     | Consumes:       |
     |  delegate_task  |
     |  coordinate_wf  |
     |  integrate_res  |
     +-----------------+
```

All arrows point downward. No circular dependencies. ada-rs is entirely
optional and adds consciousness/sovereignty capabilities on top of the
base agent orchestration that crewai-rust provides independently.

---

*This document defines the exact API contracts for crewai-rust's agent
orchestration layer. All contracts are ada-rs independent. MUL integration
is optional (backward compatible via Option<MulSnapshot>). The contracts
are designed for Arrow Flight DoAction transport and are accessible by
any A2A-compatible orchestrator.*
