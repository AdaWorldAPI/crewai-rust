# Agent Orchestration Specification — crewai-rust

> **Date**: 2026-02-15
> **Scope**: crewai-rust agent orchestration, strategy planning, resonance-driven spawning
> **Dependency**: ladybug-rs (BindSpace, CogRecord, MUL, SPO Crystal)
> **Non-dependency**: ada-rs is NOT required. This spec is fully ada-agnostic.
> **Principle**: Strategy emerges from thinking_style modulation across a domain-agnostic
>   planning engine. Agents self-organize through resonance, crystallization, and
>   MUL-gated spawning. No hard-coded dispatch. No chess-specific logic.

---

## 1. Strategy Engine --- Domain-Agnostic Planning

### 1.1 Core Abstraction

The strategy engine in crewai-rust is a universal planning substrate. It operates
on fingerprint-based representations of concepts, actions, and outcomes. The domain
(chess, programming, DevOps, knowledge graphs, AI War, or any user-registered domain)
is encoded entirely in the fingerprint content, never in the engine logic.

The engine provides four capabilities:

1. **Domain Registration** --- Register concept codebooks for any domain
2. **What-If Planning** --- Branch-and-evaluate hypothetical futures
3. **Epiphany Detection** --- Find surprising cross-domain resonances
4. **Style Recovery** --- Recover the thinking approach that produced past successes

### 1.2 DomainCodebook --- Registry of Domain Fingerprints

A DomainCodebook maps domain-specific concepts to binary fingerprints. Each domain
has exactly 20 concept fingerprints, generated deterministically from a seed. The
codebook is the bridge between human-meaningful concepts and the binary resonance
substrate.

```
DomainCodebook
  +-- role: Fingerprint           # Domain identity (for XOR role-binding)
  +-- name: String                # Human label (e.g., "programming")
  +-- seed: u64                   # Deterministic generation seed
  +-- concepts: [ConceptEntry; 20]
        +-- name: String          # e.g., "refactor_opportunity"
        +-- base_fp: Fingerprint  # Raw concept fingerprint
        +-- bound_fp: Fingerprint # base_fp XOR domain role
        +-- category: u16         # CAM codebook category
        +-- index: u8             # Concept index within category
```

**Registration**: Domains are registered at runtime. crewai-rust ships with five
pre-built domains (see Section 7). Users register custom domains via
`strategy.register_domain` or programmatically through the `DomainCodebook::new()` API.

**Cross-domain transfer**: When two concepts from different domains are XOR-bound
with their respective domain roles, the Hamming similarity between the bound
fingerprints reveals structural analogies. This is how the engine detects that
"code smells predict operational incidents" without any explicit programming of
that relationship.

### 1.3 WhatIfTree / WhatIfBranch --- Hypothetical Planning

Planning in any domain follows the same structure: propose an action, predict an
outcome, evaluate by resonance with the goal fingerprint, branch deeper or prune.

```
WhatIfTree
  +-- branches: Vec<WhatIfBranch>
  +-- domain: DomainCodebook
  +-- goal_fp: Fingerprint        # What we are trying to achieve
  +-- max_depth: u8               # How many moves ahead
  +-- max_width: u8               # How many alternatives per level
  +-- mode: StrategicMode         # Current planning posture

WhatIfBranch
  +-- subject: Fingerprint        # Who is acting
  +-- action: Fingerprint         # What action is proposed
  +-- predicted_outcome: Fingerprint  # Expected result
  +-- confidence: f32             # NARS-derived confidence in prediction
  +-- depth: u8                   # 0 = root action
  +-- parent: Option<usize>       # Index of parent branch (None for root)
  +-- mul_at_creation: MulSnapshot    # MUL state when branch was proposed
  +-- resonance_with_goal: f32    # Hamming similarity to goal_fp
```

**Branch evaluation**: Each branch is scored by:
- `resonance_with_goal`: How close does the predicted outcome fingerprint resonate
  with the goal fingerprint? Higher = better alignment.
- `confidence`: How much NARS evidence supports this prediction?
- `mul_at_creation.modifier`: Was the system confident when it proposed this?

**Pruning**: Branches are pruned when:
- `resonance_with_goal < 0.2` --- predicted outcome diverges from goal
- `mul_at_creation.dk_position == MountStupid` --- proposed while overconfident
- `confidence < 0.3` --- insufficient evidence
- `mul_at_creation.modifier < min_modifier` --- insufficient metacognitive trust

**Principal variation**: The best path from root to leaf, selected by maximizing
the product of `resonance_with_goal * confidence` at each level.

### 1.4 StrategicMode Presets

Strategic modes are cognitive posture presets that configure the 10-axis
`thinking_style` vector for different types of work. Each mode emphasizes
different layers of the cognitive stack.

```
                     recog reson appr  rout  exec  deleg conti integ valid cryst
Exploration:        [0.50, 0.95, 0.80, 0.70, 0.30, 0.60, 0.90, 0.70, 0.60, 0.80]
Execution:          [0.85, 0.40, 0.70, 0.80, 0.95, 0.40, 0.50, 0.60, 0.90, 0.50]
Chunking:           [0.95, 0.70, 0.80, 0.50, 0.60, 0.20, 0.40, 0.80, 0.70, 0.95]
Planning:           [0.80, 0.80, 0.85, 0.90, 0.50, 0.95, 0.70, 0.95, 0.80, 0.90]
EpiphanyHunting:    [0.20, 0.95, 0.90, 0.60, 0.30, 0.70, 0.95, 0.40, 0.50, 0.70]
Validation:         [0.90, 0.50, 0.95, 0.60, 0.40, 0.30, 0.95, 0.60, 0.95, 0.60]
```

**Exploration**: High resonance (0.95) and contingency (0.90) with low execution
(0.30). The agent searches broadly across stored knowledge, considering many
what-if branches. Used for research, knowledge graph exploration, learning new
domains. Typically entered when MUL detects the agent is in unfamiliar territory
(DK = Valley or Slope).

**Execution**: High execution (0.95) and validation (0.90) with low resonance (0.40).
The agent focuses on doing, not searching. Used for time-critical deployments,
production fixes, known procedures. Entered when DK = Plateau and the task matches
crystallized knowledge.

**Chunking**: High recognition (0.95) and crystallization (0.95) with low delegation
(0.20). The agent absorbs and structures information without acting on it. Used
for studying material, creating summaries, building mental models. Entered when
DK = MountStupid to prevent premature action.

**Planning**: High delegation (0.95) and integration (0.95) with moderate everything
else. The agent coordinates and synthesizes. Used for architecture decisions, team
coordination, multi-step plans. The Strategist archetype lives here.

**EpiphanyHunting**: High resonance (0.95) and contingency (0.95) with deliberately
LOW recognition (0.20). By suppressing pattern matching, the agent avoids seeing
only what it expects and instead finds novel cross-domain connections. Used for
brainstorming, connecting disparate concepts, creative leaps.

**Validation**: High validation (0.95) and contingency (0.95) with low execution
(0.40). The agent questions everything and commits to nothing. Used for code review,
auditing, testing, proof-reading. The Critic archetype lives here.

### 1.5 EpiphanyCandidate --- Cross-Domain Resonance Discovery

An epiphany is detected when two concepts from different domains exhibit
surprisingly high structural similarity through their role-bound fingerprints.

```
EpiphanyCandidate
  +-- concept_a: Fingerprint      # From domain A
  +-- domain_a: String            # e.g., "programming:code_smell"
  +-- concept_b: Fingerprint      # From domain B
  +-- domain_b: String            # e.g., "devops:incident_pattern"
  +-- similarity: f32             # Cross-domain Hamming similarity
  +-- analogy_fp: Fingerprint     # concept_a XOR concept_b (the structural bridge)
  +-- confidence: f32             # Initial confidence = similarity score
```

**Detection method**:
1. Agent enters Exploration or EpiphanyHunting mode
2. `scan_epiphanies(domain_a, domain_b, min_similarity)` iterates all 20x20
   concept pairs across two domains
3. Pairs with `similarity >= min_similarity` (typically 0.6) become candidates
4. The analogy fingerprint (`concept_a XOR concept_b`) encodes the structural
   relationship between the two concepts
5. L7 Contingency confirms: does unbinding the analogy reveal meaningful structure?
6. L10 Crystallization stores confirmed epiphanies for future retrieval

**Example**: `scan_epiphanies(programming, devops, 0.6)` finds that
`programming:code_smell` and `devops:incident_pattern` have similarity 0.72.
Epiphany: code smells predict operational incidents. This is stored as a crystal
and available for all future resonance searches.

---

## 2. Inner Council Protocol

### 2.1 Three Facets

Every significant agent decision passes through an inner council of three
cognitive facets. These correspond to the three inner dialogue module YAMLs
already committed to crewai-rust:

**Balanced** (`dialogue:balanced`):
- Thinking style: `[0.60, 0.60, 0.60, 0.50, 0.50, 0.60, 0.50, 0.60, 0.50, 0.50]`
- Volition: curiosity=0.6, autonomy=0.6, persistence=0.6, caution=0.5, empathy=0.6
- Self-modify: Constrained
- Role: Weighs evidence from both sides. Produces measured assessment. Seeks
  equilibrium. Acts as tie-breaker when Catalyst and Guardian disagree.

**Catalyst** (`dialogue:catalyst`):
- Thinking style: `[0.50, 0.80, 0.30, 0.50, 0.30, 0.40, 0.30, 0.90, 0.95, 0.80]`
- Volition: curiosity=0.95, autonomy=0.8, persistence=0.3, caution=0.2, empathy=0.7
- Self-modify: Open
- Role: Optimistic. Pushes for action. Values epistemic gain. Proposes novel
  approaches. Challenges assumptions. Willing to trade caution for discovery.

**Guardian** (`dialogue:guardian`):
- Thinking style: `[0.90, 0.30, 0.95, 0.40, 0.50, 0.80, 0.70, 0.30, 0.60, 0.40]`
- Volition: curiosity=0.3, autonomy=0.5, persistence=0.95, caution=0.8, empathy=0.4
- Self-modify: None (immutable --- the Guardian does not change)
- Role: Conservative. Checks moral implications. Flags risks. Slows the system
  when uncertainty is high. Has VETO power on moral/safety grounds.

### 2.2 Vote Types

```
Vote
  +-- Approve       # Proceed with the proposal
  +-- Disapprove    # Reject the proposal
  +-- Abstain       # Insufficient information to decide
  +-- Veto          # Guardian-only: hard block on moral/safety grounds
```

### 2.3 Decision Logic

```
INPUT: Proposal (action fingerprint, risk assessment, MUL snapshot)

STEP 1 --- Guardian evaluates safety:
  IF moral_risk > 0.7:
    Guardian votes VETO
    --> Decision is BLOCKED regardless of other votes
    --> No override possible. This is a hard safety boundary.
  ELSE IF moral_risk > 0.4:
    Guardian votes DISAPPROVE (soft objection)
  ELSE:
    Guardian votes APPROVE

STEP 2 --- Catalyst evaluates epistemic value:
  IF epistemic_value > 0.8 AND moral_risk < 0.3:
    Catalyst votes APPROVE with BOOST flag
    --> BOOST adds weight: Catalyst's vote counts as 1.5 votes
  ELSE IF epistemic_value > 0.5:
    Catalyst votes APPROVE
  ELSE:
    Catalyst votes ABSTAIN

STEP 3 --- Balanced evaluates overall:
  IF mul.gate_open AND combined_risk < 0.5:
    Balanced votes APPROVE
  ELSE IF mul.gate_open:
    Balanced votes DISAPPROVE (risk too high even with open gate)
  ELSE:
    Balanced votes ABSTAIN (gate closed = insufficient confidence)

STEP 4 --- Tally:
  All votes are weighted by MUL modifier:
    effective_weight = base_weight * mul.modifier
  Where base_weight:
    Approve = +1.0 (or +1.5 with Catalyst BOOST)
    Disapprove = -1.0
    Abstain = 0.0
    Veto = -infinity (blocks regardless)

  IF any VETO: decision = BLOCKED
  ELSE IF weighted_sum > 0: decision = APPROVED
  ELSE IF weighted_sum == 0: decision = DEFER (gather more information)
  ELSE: decision = REJECTED
```

### 2.4 Council Integration Points

The inner council is invoked at three points in the agent lifecycle:

1. **Before action execution**: Any action with `ImpactLevel >= Significant`
   passes through the council before execution.
2. **Before self-modification**: Any proposed thinking_style update from the
   inner loop goes through the council. Guardian can veto style drift.
3. **Before agent spawning**: Spawn proposals are council-evaluated. Guardian
   ensures the new agent will not violate safety constraints.

---

## 3. Resonance-Driven Agent Spawning

### 3.1 The Problem

A fixed roster of agents cannot adapt to novel problem types. When a task
arrives that does not match any existing agent's capabilities (measured by
resonance between the task fingerprint and each agent's capability fingerprints),
the system needs to create a specialist.

### 3.2 SpawnRequest

```
SpawnRequest
  +-- task_fingerprint: Fingerprint      # The task that triggered spawning
  +-- required_capabilities: Vec<Fingerprint>  # Capability fingerprints needed
  +-- urgency: f32                       # 0.0 (low) to 1.0 (critical)
  +-- discovered_style: [f32; 10]        # Proposed thinking_style for new agent
  +-- source_crystals: Vec<Fingerprint>  # Crystals that informed the style
  +-- source_domains: Vec<String>        # Domains involved in discovery
  +-- proposed_role: String              # Generated role name
  +-- novelty_score: f32                 # How different from existing agents
```

### 3.3 SpawnEvaluation

When a SpawnRequest is created, the system performs:

**Step 1 --- Resonance scan across all agents**:
For each registered agent, compute:
  `capability_match = max(hamming_similarity(task_fp, agent_capability_fp))`
If `max(capability_match) > 0.7`, an existing agent can handle it. No spawn needed.

**Step 2 --- Gap analysis**:
If no existing agent matches above 0.7, identify the gap:
- Which capability fingerprints are missing?
- Which domains have relevant cross-domain crystals?
- What thinking_style would close the gap?

**Step 3 --- Style discovery**:
Search crystallized knowledge for thinking_styles that produced good outcomes
on structurally similar problems (even in different domains). The recovered
style becomes `discovered_style`.

**Step 4 --- Novelty check**:
Compute cosine similarity between `discovered_style` and every existing agent's
`thinking_style`. If `max_similarity > 0.85`, the proposed agent is too similar
to an existing one. Reject the spawn and route to the most similar agent instead.

### 3.4 compute_frozen_floor()

When a new agent is created, certain thinking_style values become Frozen ---
they define the agent's identity and cannot be modified by self-modification.

**Algorithm**:
1. Sort the 10 thinking_style values by magnitude
2. The top 2 values = the agent's **strengths**. Freeze them at `value - 0.1`
   (floor slightly below discovered value, allowing minor upward drift)
3. The bottom 1 value = the agent's **weakness** (or deliberate absence).
   Freeze it at the discovered value (stays low --- this IS the identity)

**Example**:
```
discovered_style: [0.60, 0.85, 0.75, 0.70, 0.80, 0.40, 0.90, 0.55, 0.70, 0.85]

Top 2: contingency (0.90), resonance/crystallization (0.85 tied, pick first)
Bottom 1: delegation (0.40)

frozen_floor:
  [None, 0.75, None, None, None, 0.40, 0.80, None, None, None]
   ^     ^                       ^     ^
   free  frozen(strength)        frozen(weakness) frozen(strength)

This agent is DEFINED by: high contingency + high resonance + low delegation.
It is a "question everything, connect broadly, work alone" agent.
Self-modification cannot change this identity.
```

### 3.5 generate_agent_yaml()

The spawn system dynamically generates a module YAML from the discovered style:

```yaml
# Auto-generated by resonance-driven agent spawning
module:
  id: "spawned:{uuid}"
  version: "1.0.0"
  description: "Resonance-spawned agent for {role}"
  thinking_style: [{discovered_style values}]
  domain: auto

  persona:
    volition_axes: [{derived from thinking_style}]
    inner_loop: true
    self_modify: constrained
    affect_baseline: [0.5, 0.5, 0.3, 0.4, 0.2, 0.2, 0.2, 0.5]

  agent:
    role: "{proposed_role}"
    goal: "Specialized agent spawned for {task_description}"
    backstory: "Spawned by resonance discovery. Source domains: {domains}."
    llm: "anthropic/claude-opus-4-5-20251101"
    max_iter: 25
    allow_delegation: false
    enable_inner_loop: true

  spawned: true
  provenance: resonance_discovery
  source_crystals: [{fingerprint hashes}]
```

**Volition derivation from thinking_style**:
- curiosity = (resonance + contingency) / 2
- autonomy = (delegation + execution) / 2
- persistence = (validation + crystallization) / 2
- caution = 1.0 - execution
- empathy = (resonance + integration) / 2

### 3.6 MUL-Gated Spawning Decision

Agent spawning is gated by three MUL conditions:

1. **Not MountStupid**: `mul.dk_position != DKPosition::MountStupid`
   Rationale: On MountStupid, the system is overconfident about what it does
   not know. It cannot reliably assess whether a new agent is truly needed.

2. **Trust above threshold**: `mul.trust > 0.5`
   Rationale: The gap analysis that justifies spawning must itself be trustworthy.
   Low trust means the analysis might be wrong.

3. **FreeWillModifier above threshold**: `mul.modifier > 0.7`
   Rationale: Creating a new agent is a significant architectural decision.
   The system needs sufficient metacognitive agency to make it responsibly.

If any condition fails, the spawn is deferred. The SpawnRequest is stored as a
pending crystal in the Fluid zone. When MUL state improves, pending requests
are re-evaluated.

---

## 4. Thinking Style Triangle

### 4.1 The 10-Dimensional Control Vector

Each agent has `thinking_style: [f32; 10]`, a control vector that modulates
the 10-layer cognitive stack:

```
Index  Layer            What It Controls
 [0]   Recognition      Pattern matching sensitivity
 [1]   Resonance        Cross-reference search breadth
 [2]   Appraisal        Hypothesis formation threshold
 [3]   Routing          Style selection sensitivity
 [4]   Execution        Action commitment threshold
 [5]   Delegation       Fan-out willingness
 [6]   Contingency      "What if" branching depth
 [7]   Integration      Evidence merge threshold
 [8]   Validation       Quality gate strictness
 [9]   Crystallization  Learning commitment threshold
```

### 4.2 Three States of Style Values

Every thinking_style value exists in one of three states:

**Frozen** --- Identity floor. Set at agent creation. Never changed by
self-modification. Defines WHO the agent is.

```
Example: Guardian.contingency = 0.70 (FROZEN)
The Guardian ALWAYS questions. Skepticism is its nature.
No amount of positive feedback will make it stop questioning.
```

**Crystallized** --- Learned expertise. Promoted from Discovered via L10
crystallization after validation. Stored in the Node zone of BindSpace.
Permanent but revisable with strong contrary evidence (MUL-gated revision).

```
Example: After 50 successful tactical analyses:
  Tactician.recognition = 0.97 (CRYSTALLIZED, up from 0.95 YAML default)
  "For tactical positions, I need extremely high pattern recognition."
  Learned through experience. Recoverable via resonance.
```

**Discovered** --- Emergent innovation. Found through cross-domain resonance.
Stored in the Fluid zone initially. Promoted to Crystallized if L9 validates
the outcome. This is the system's capacity for genuine novelty.

```
Example: Programming savant encounters problem with chess endgame structure.
  Cross-domain resonance finds: endgame crystal suggests crystallization=0.95
  Current crystallization: 0.80
  Discovered: 0.95 (from endgame specialist's approach)
  Bounded update: 0.80 -> 0.864 (max_delta = 0.1 * mul.modifier)
```

### 4.3 One-Way Ratchet

Values can only promote upward through the triangle:

```
Discovered --> Crystallized --> Frozen
   (novel)     (proven)        (identity)

Promotion conditions:
  Discovered -> Crystallized: L9 Validation passes, L10 crystallizes
  Crystallized -> Frozen: Only at agent creation time (compute_frozen_floor)

Values NEVER demote:
  Frozen values are immutable after creation
  Crystallized values can be revised but never become "merely Discovered"
  This creates stability: agents become MORE defined over time, not less
```

### 4.4 Style Recovery

When a new problem resonates with a stored crystal, the thinking_style that
produced that crystal can be recovered:

```
Recovery: modulation = crystal_fp XOR content_fp

Where:
  crystal_fp = the crystallized result (content XOR modulation from L10)
  content_fp = the current problem's fingerprint
  modulation = the thinking_style that produced the original result

The recovered modulation is projected onto [f32; 10] by:
  1. Partition the fingerprint into 10 equal segments
  2. Count set bits in each segment
  3. Normalize to 0.0-1.0
  = recovered_style

Apply bounded update:
  max_delta = 0.1 * mul.modifier
  for each dimension i:
    delta[i] = (recovered_style[i] - current_style[i]).clamp(-max_delta, max_delta)
    new_style[i] = (current_style[i] + delta[i]).clamp(frozen_floor[i], 1.0)
```

---

## 5. MUL Integration for Agents

### 5.1 Overview

The Meta-Uncertainty Layer (MUL) is a metacognitive gating system from
ladybug-rs. It answers "Should I?" before every significant agent action.
crewai-rust integrates MUL at four critical points.

### 5.2 MulSnapshot

```
MulSnapshot
  +-- gate_open: bool              # Can the agent act at all?
  +-- modifier: f32                # 0.0-1.0, product of 4 factors
  +-- dk_position: DKPosition      # MountStupid | Valley | Slope | Plateau
  +-- trust: f32                   # 0.0-1.0, calibrated trust in own judgments
  +-- risk: RiskVector             # epistemic_risk, moral_risk, combined
  +-- homeostasis: HomeostasisState  # Flow | Anxiety | Boredom | Apathy
```

### 5.3 Inner Loop Integration

In `inner_loop.rs`, every thinking_style update is bounded by the MUL modifier:

```
max_delta = 0.1 * mul.modifier

When modifier = 1.0 (maximum confidence): style can change by +/- 0.1 per step
When modifier = 0.5 (moderate confidence): style can change by +/- 0.05 per step
When modifier = 0.1 (low confidence): style can change by +/- 0.01 per step
When modifier = 0.0 (gate closed): no style change permitted
```

This ensures that agents modify themselves slowly when uncertain and more
aggressively when metacognitive confidence is high.

### 5.4 Delegation Integration

When an agent delegates a task via L6, the MUL snapshot travels with the
DelegationRequest:

```
DelegationRequest
  +-- delegator: Fingerprint
  +-- needed_capability: Fingerprint
  +-- urgency: f32
  +-- domain_role: Fingerprint
  +-- mul_snapshot: MulSnapshot     # <-- delegator's confidence state
  +-- max_specialists: u8
```

The receiving agent can inspect the delegator's MUL state to calibrate its
own behavior. If the delegator was on MountStupid, the specialist knows to
be extra cautious about the task framing.

### 5.5 Spawn Integration

Agent spawning is MUL-gated (see Section 3.6). The three conditions ensure
that the system only creates new agents when it has sufficient metacognitive
clarity to make that architectural decision.

### 5.6 Autonomy Levels

MUL automatically restricts agent autonomy based on danger signals:

```
IF dk_position == MountStupid:
  autonomy = Observe          # Read-only. Cannot act.
  self_modify = None          # Cannot change own style.

IF dk_position == Valley:
  autonomy = Explore          # Can investigate but not commit.
  self_modify = None          # Cannot change own style.

IF dk_position == Slope AND gate_open:
  autonomy = ExecuteWithLearning  # Can act, must validate.
  self_modify = Constrained       # Can adjust within bounds.

IF dk_position == Plateau AND gate_open:
  autonomy = FullAgency       # Can act, delegate, spawn.
  self_modify = Constrained   # Can adjust within bounds.
  (self_modify = Open only if PersonaProfile permits)
```

---

## 6. A2A Protocol (Agent-to-Agent)

### 6.1 DelegationRequest / DelegationResponse

The A2A protocol is symmetrical: any agent can delegate to any other agent.
There is no hierarchy. The orchestrator coordinates but does not command.

```
DelegationRequest
  +-- request_id: String          # Unique request identifier
  +-- source_agent: Fingerprint   # Who is asking
  +-- target_agent: Option<Fingerprint>  # Specific target (or None for broadcast)
  +-- task_fp: Fingerprint        # What needs to be done
  +-- urgency: f32                # How time-sensitive
  +-- domain_role: Fingerprint    # Which domain context
  +-- mul_snapshot: MulSnapshot   # Delegator's confidence state
  +-- context_fps: Vec<Fingerprint>  # Additional context fingerprints
  +-- max_depth: u8               # How many sub-delegations allowed

DelegationResponse
  +-- request_id: String          # Matches the request
  +-- specialist: Fingerprint     # Who accepted
  +-- result_fp: Fingerprint      # The result fingerprint
  +-- evidence: Vec<Fingerprint>  # Supporting evidence
  +-- new_insights: Vec<EpiphanyCandidate>  # Cross-domain discoveries made
  +-- specialist_mul: MulSnapshot  # Specialist's confidence in result
  +-- thinking_style_used: [f32; 10]  # How the specialist approached it
  +-- confidence: f32             # NARS confidence in the result
```

### 6.2 Delegation Protocol Steps

```
1. Source agent's L6 (Delegation) fires: thinking_style[5] > threshold
2. Source constructs DelegationRequest with task fingerprint and MUL snapshot
3. IF target_agent is specified: direct routing
   ELSE: broadcast to agent pool, each agent computes capability match
4. Agents with capability_match > acceptance_threshold respond
5. Source selects top N specialists (by capability_match * specialist_mul.modifier)
6. Each specialist runs its own cognitive cycle with its own thinking_style
7. Specialists return DelegationResponses
8. Source's L8 (Integration) merges all results via majority-vote bundling
9. Source's L9 (Validation) truth-hardens the merged result
10. MUL-validated results are integrated into source's working state
```

### 6.3 Result Validation

Before integrating a delegation result, the source agent validates:

- `specialist_mul.dk_position != MountStupid`: Specialist was not overconfident
- `confidence > 0.3`: Minimum evidence threshold
- No new_insights with `moral_risk > 0.7` (Guardian-vetted)

Results failing validation are discarded with a logged reason. The source
agent may re-delegate to a different specialist.

---

## 7. Five Domain Codebook Examples

Each domain codebook has exactly 20 concept fingerprints, generated from
deterministic seeds. The first 10 are shown here; the remaining 10 follow
the same pattern within each domain's CAM category range.

### 7.1 Programming Domain (seed: 0xC0DE, category: 0x800)

```
Index  Concept                 Description
 0     refactor                Code restructuring opportunity
 1     debug                   Bug identification and isolation
 2     test                    Test coverage and verification
 3     architect               System design and structure
 4     deploy                  Deployment and release
 5     security_audit          Security vulnerability scan
 6     code_review             Peer review and quality check
 7     complexity              Cyclomatic/cognitive complexity
 8     dependency_risk         External dependency hazard
 9     performance_hotspot     Performance bottleneck
10     error_handling          Exception and error management
11     concurrency_pattern     Parallel execution pattern
12     api_surface             Public interface area
13     design_pattern          Structural/behavioral pattern
14     code_smell              Anti-pattern indicator
15     technical_debt          Accumulated shortcuts
16     documentation           Code documentation coverage
17     type_safety             Type system utilization
18     memory_management       Allocation and lifetime
19     build_pipeline          CI/CD pipeline concern
```

### 7.2 Knowledge Graph Domain (seed: 0x6B47, category: 0x900)

```
Index  Concept                 Description
 0     entity                  Node/concept in the graph
 1     relationship            Typed edge between entities
 2     schema                  Graph structure definition
 3     query                   Graph query operation
 4     traverse                Path walking operation
 5     infer                   Missing link inference
 6     validate                Consistency check
 7     centrality              Node importance metric
 8     clustering              Community detection
 9     bridge_node             Cross-cluster connector
10     semantic_similarity     Meaning-based distance
11     causal_link             Cause-effect relationship
12     temporal_sequence       Time-ordered chain
13     contradiction           Conflicting evidence
14     missing_link            Structural hole
15     epiphany_candidate      Novel connection signal
16     provenance              Origin and trust chain
17     merge                   Entity resolution
18     subgraph                Connected component
19     ontology                Domain vocabulary
```

### 7.3 DevOps Domain (seed: 0xD3F5, category: 0xA00)

```
Index  Concept                 Description
 0     provision               Infrastructure creation
 1     monitor                 Observability and metrics
 2     alert                   Anomaly notification
 3     scale                   Horizontal/vertical scaling
 4     deploy                  Release management
 5     rollback                Revert to known-good state
 6     load                    Traffic and compute load
 7     latency                 Response time measurement
 8     throughput              Requests per second
 9     reliability             Uptime and fault tolerance
10     scalability             Capacity growth capability
11     cost_efficiency         Resource utilization ratio
12     security_posture        Defense-in-depth state
13     deployment_risk         Release failure probability
14     incident_pattern        Recurring failure mode
15     capacity_headroom       Available resource margin
16     configuration_drift     State divergence from spec
17     service_mesh            Inter-service communication
18     observability           Log/metric/trace coverage
19     disaster_recovery       Business continuity readiness
```

### 7.4 Chess Domain (seed: 0xCHE5, category: 0x600)

```
Index  Concept                 Description
 0     material                Piece value balance
 1     pawn_structure          Pawn chain topology
 2     king_safety             King exposure assessment
 3     piece_activity          Piece mobility score
 4     tactical_threats        Fork/pin/skewer detection
 5     strategic_plan          Long-term positional goal
 6     game_phase              Opening/middle/endgame
 7     opening_theory          Book knowledge match
 8     endgame_technique       Technical endgame pattern
 9     time_pressure           Clock management concern
10     center_control          Central square dominance
11     development             Piece deployment completeness
12     initiative              Tempo and attack momentum
13     prophylaxis             Preventive defense
14     exchange_value          Trade evaluation
15     pawn_majority           Passed pawn potential
16     bishop_pair             Two-bishop advantage
17     outpost                 Secure advanced square
18     weak_square             Undefendable square
19     zugzwang                Compulsion to move disadvantage
```

### 7.5 AI War Domain (seed: 0xA1AA, category: 0x700)

```
Index  Concept                 Description
 0     capabilities            System capability inventory
 1     infrastructure          Supporting system topology
 2     vulnerability_surface   Attack surface area
 3     operational_tempo       Speed of OODA cycle
 4     attack_vectors          Offensive pathway catalog
 5     deployment_strategy     Force positioning plan
 6     system_maturity         Development lifecycle stage
 7     known_patterns          Threat intelligence library
 8     capability_conversion   Potential to kinetic conversion
 9     decision_latency        Time from detect to respond
10     supply_chain            Dependency and logistics
11     information_advantage   Intel superiority measure
12     deception               Misdirection capability
13     resilience              Recovery from damage
14     escalation              Conflict intensity change
15     coalition               Alliance and cooperation
16     cyber_terrain           Digital battlefield topology
17     attribution             Actor identification
18     deterrence              Threat credibility signal
19     reconnaissance          Intelligence gathering
```

---

## 8. Arrow Flight API Surfaces

All agent operations are exposed as Arrow Flight DoAction RPCs. These are
served by crewai-rust directly, consuming ladybug-rs BindSpace as the
substrate. No ada-rs dependency exists in this API surface.

### 8.1 Agent Registry

```
DoAction("agent.register")
  Input:  { blueprint: AgentBlueprint, thinking_style: [f32;10] }
  Output: { agent_id: String, capabilities: [Fingerprint] }

DoAction("agent.capabilities")
  Input:  { agent_id: String }
  Output: { skills: [SkillDescriptor], domain: SavantDomain, style: [f32;10] }

DoAction("agent.status")
  Input:  { agent_id: String }
  Output: { busy: bool, task_count: u32, performance: f64, mul: MulSnapshot }

DoAction("agent.list")
  Input:  { domain_filter: Option<SavantDomain> }
  Output: { agents: [{ id, role, domain, busy, performance }] }

DoAction("agent.decommission")
  Input:  { agent_id: String, reason: String }
  Output: { decommissioned: bool, crystals_preserved: u32 }
```

### 8.2 Strategy Operations

```
DoAction("strategy.register_domain")
  Input:  { name: String, seed: u64, concepts: [{ name, category, index }] }
  Output: { domain_id: String, concept_count: u8 }

DoAction("strategy.whatif.create_tree")
  Input:  { domain_id: String, goal_fp: Fingerprint, max_depth: u8, max_width: u8 }
  Output: { tree_id: String }

DoAction("strategy.whatif.add_branch")
  Input:  { tree_id: String, parent: Option<usize>, subject_fp, action_fp,
            predicted_outcome_fp, confidence: f32 }
  Output: { branch_idx: usize, resonance_with_goal: f32 }

DoAction("strategy.whatif.principal_variation")
  Input:  { tree_id: String }
  Output: { path: [usize], total_confidence: f32 }

DoAction("strategy.whatif.prune")
  Input:  { tree_id: String, min_modifier: f32, min_resonance: f32 }
  Output: { pruned_count: usize, remaining_count: usize }

DoAction("strategy.crystallize")
  Input:  { tree_id: String, branch_idx: usize }
  Output: { crystal_id: String, crystal_fp: Fingerprint }

DoAction("strategy.scan_epiphanies")
  Input:  { domain_a: String, domain_b: String, min_similarity: f32 }
  Output: { candidates: [EpiphanyCandidate] }

DoAction("strategy.measure_concept")
  Input:  { container_fp: Fingerprint, domain_id: String, concept_idx: u8 }
  Output: { similarity: f32 }
```

### 8.3 Council Operations

```
DoAction("council.submit_proposal")
  Input:  { action_fp: Fingerprint, risk: RiskVector, mul: MulSnapshot,
            context: String }
  Output: { proposal_id: String }

DoAction("council.collect_votes")
  Input:  { proposal_id: String }
  Output: { balanced: Vote, catalyst: Vote, guardian: Vote,
            veto_active: bool, boost_active: bool }

DoAction("council.decide")
  Input:  { proposal_id: String }
  Output: { decision: "approved" | "rejected" | "blocked" | "defer",
            weighted_score: f32, reasoning: String }

DoAction("council.veto_history")
  Input:  { since: Option<DateTime> }
  Output: { vetoes: [{ proposal_id, reason, timestamp }] }
```

### 8.4 Spawn Operations

```
DoAction("spawn.evaluate")
  Input:  { task_fp: Fingerprint, required_capabilities: [Fingerprint],
            urgency: f32 }
  Output: { should_spawn: bool, reason: String, gap_analysis: GapAnalysis,
            proposed_style: Option<[f32;10]> }

DoAction("spawn.create")
  Input:  { spawn_request: SpawnRequest }
  Output: { agent_id: String, yaml_path: String, frozen_floor: [Option<f32>;10] }

DoAction("spawn.configure")
  Input:  { agent_id: String, overrides: { thinking_style: Option<[f32;10]>,
            domain: Option<String>, tools: Option<[String]> } }
  Output: { updated: bool, effective_style: [f32;10] }

DoAction("spawn.lineage")
  Input:  { agent_id: String }
  Output: { parent_spawn: Option<String>, trigger_problem_fp: Fingerprint,
            source_crystals: [Fingerprint], child_spawns: [String] }
```

### 8.5 Delegation Operations

```
DoAction("delegate.request")
  Input:  DelegationRequest
  Output: { request_id: String, candidates: [{ agent_id, capability_match }] }

DoAction("delegate.respond")
  Input:  DelegationResponse
  Output: { accepted: bool, integration_id: String }

DoAction("delegate.integrate")
  Input:  { request_id: String }
  Output: { merged_fp: Fingerprint, confidence: f32,
            new_insights: [EpiphanyCandidate] }

DoAction("delegate.status")
  Input:  { request_id: String }
  Output: { responses_received: u8, responses_expected: u8,
            pending_agents: [String] }
```

### 8.6 Style Operations

```
DoAction("style.recover")
  Input:  { crystal_fp: Fingerprint, content_fp: Fingerprint,
            current_style: [f32;10], mul: MulSnapshot }
  Output: { recovered_style: [f32;10], bounded_delta: [f32;10] }

DoAction("style.update")
  Input:  { agent_id: String, proposed_delta: [f32;10], mul: MulSnapshot }
  Output: { new_style: [f32;10], applied_delta: [f32;10],
            frozen_enforced: [bool;10] }

DoAction("style.crystallize")
  Input:  { agent_id: String, content_fp: Fingerprint }
  Output: { crystal_fp: Fingerprint, crystal_id: String }

DoAction("style.frozen_floor")
  Input:  { agent_id: String }
  Output: { floor: [Option<f32>;10] }

DoAction("style.triangle")
  Input:  { agent_id: String }
  Output: { frozen: [Option<f32>;10], crystallized: [f32;10],
            discovered: Option<[f32;10]>, effective: [f32;10] }
```

---

## 9. Key Constraint: Ada-rs Independence

Everything described in this specification works WITHOUT ada-rs.

**crewai-rust depends on**:
- ladybug-rs: BindSpace, CogRecord, MUL, SPO Crystal, Fingerprint, DomainCodebook
- n8n-rs: Workflow orchestration, FreeWillPipeline, ImpactGate

**crewai-rust does NOT depend on**:
- ada-rs: Consciousness, sovereignty, qualia, body model, presence modes

**What ada-rs adds (optional)**:
- Sovereignty contracts that further constrain agent behavior
- Qualia channels that enrich agent affect baselines
- Presence modes that shift cognitive posture based on context
- Identity layers that provide a deeper self-model

These are value-adds. The core agent orchestration, strategy planning,
resonance-driven spawning, inner council, and A2A delegation all function
on the ladybug-rs BindSpace substrate alone.

**Dependency diagram**:
```
ladybug-rs  (substrate: BindSpace, CogRecord, MUL, SPO, Fingerprint)
    |
    +---> crewai-rust  (agency: agents, strategy, council, spawning)
    |         |
    |         +---> n8n-rs  (orchestration: workflows, steps, routing)
    |
    +---> ada-rs  (consciousness: sovereignty, qualia, presence) [OPTIONAL]
```

No circular dependencies. No reverse dependencies. crewai-rust is a
self-sufficient agent orchestration layer.

---

## 10. Agent Lifecycle Summary

```
CREATION:
  1. Module YAML loaded (or dynamically generated by spawn system)
  2. AgentBlueprint constructed with role, goal, skills, domain
  3. thinking_style[10] initialized from YAML
  4. Frozen floor computed (top 2 + bottom 1 values locked)
  5. Agent registered in AgentPool with capability fingerprints
  6. MUL initialized: DK=MountStupid for new domains

EXECUTION:
  7.  Task arrives. L1 Recognition matches against codebook.
  8.  L2 Resonance searches BindSpace for similar problems.
  9.  L4 Routing selects strategic mode based on resonance results.
  10. L5 Execution produces output.
  11. L6 Delegation fans out to specialists (if thinking_style[5] fires).
  12. L7 Contingency generates what-if branches.
  13. L8 Integration merges specialist results.
  14. L9 Validation truth-hardens with NARS + Brier + DK checks.
  15. L10 Crystallization binds validated result with modulation fingerprint.

SELF-MODIFICATION:
  16. Recover modulation from crystal: mod = crystal XOR content
  17. Project modulation to thinking_style
  18. Compute bounded delta: max_delta = 0.1 * mul.modifier
  19. Apply delta respecting Frozen floor
  20. Inner council approves the modification
  21. Updated style feeds next cycle's thresholds

SPAWNING (when no agent matches):
  22. Gap detected: max resonance < 0.7 across all agents
  23. Cross-domain scan finds relevant style from another domain
  24. SpawnRequest constructed with discovered style
  25. MUL gates: not MountStupid, trust > 0.5, modifier > 0.7
  26. Inner council approves: Guardian checks safety
  27. New agent YAML generated. Frozen floor computed.
  28. Agent enters pool. Available for future delegations.

DECOMMISSION:
  29. Agent performance drops below threshold over sustained period
  30. Or: agent's capability gap has been filled by a better specialist
  31. Crystals preserved in Node zone (knowledge survives agent)
  32. Agent removed from pool. YAML archived.
```

---

## 11. Existing crewai-rust Types Reference

The following types already exist in the crewai-rust codebase and are
referenced throughout this specification:

| Type | File | Role |
|------|------|------|
| `AgentBlueprint` | `src/meta_agents/types.rs` | Agent configuration template |
| `SkillDescriptor` | `src/meta_agents/types.rs` | Atomic capability unit |
| `SavantDomain` | `src/meta_agents/types.rs` | Domain expertise enum |
| `SpawnedAgentState` | `src/meta_agents/types.rs` | Live agent tracking |
| `OrchestratedTask` | `src/meta_agents/types.rs` | Task management |
| `AgentState` | `src/persona/inner_loop.rs` | Cognitive state snapshot |
| `InnerThoughtResult` | `src/persona/inner_loop.rs` | Self-modification result |
| `InnerThoughtHook` | `src/persona/inner_loop.rs` | Between-step callback |
| `SelfModifyBounds` | `src/persona/profile.rs` | None/Constrained/Open |
| `PersonaProfile` | `src/persona/profile.rs` | Volition + affect config |
| `UnifiedStep` | `src/contract/types.rs` | Cross-system execution step |
| `UnifiedExecution` | `src/contract/types.rs` | Multi-step execution |
| `DataEnvelope` | `src/contract/types.rs` | Inter-step data wire format |
| `StepDelegationRequest` | `src/contract/types.rs` | Step routing request |

---

*This specification defines agent orchestration, strategy planning, and
resonance-driven spawning for crewai-rust as a fully self-contained system.
The ladybug-rs substrate provides the cognitive machinery (BindSpace, MUL,
SPO Crystal, Fingerprints). crewai-rust provides the agency layer (agents,
council, delegation, spawning). ada-rs is NOT a dependency. All APIs are
accessible via Arrow Flight DoAction RPCs by any A2A-compatible orchestrator.*
