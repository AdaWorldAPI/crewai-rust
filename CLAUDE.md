# CLAUDE.md — crewai-rust

> **Last Updated**: 2026-02-25
> **Branch**: `claude/vsaclip-hamming-recognition-y0b94`
> **Owner**: Jan Hübener (jahube)

---

## READ THIS FIRST — Role in the Four-Level Architecture

crewai-rust is **Level 4 — Composition** (behavioral orchestration).

> **Canonical cross-repo architecture:** [ada-docs/architecture/FOUR_LEVEL_ARCHITECTURE.md](https://github.com/AdaWorldAPI/ada-docs/blob/main/architecture/FOUR_LEVEL_ARCHITECTURE.md)

crewai-rust owns the **agent framework**: the **Blackboard** (the ONLY
shared-state surface), the **Drivers** (pure-function inference on
Blackboard types), and the **Persona** system (36 thinking styles in 6
clusters, 23D cognitive space, τ (tau) addresses for JIT compilation).

**Thinking styles are JIT workflows, not parameters.** "Einstein" or "Hegel"
is a composed chain of reasoning operations compiled by jitson/Cranelift
(n8n-rs), not a float vector. crewai-rust defines the styles; n8n-rs
compiles and executes them.

crewai-rust does NOT own storage, does NOT own SIMD acceleration,
does NOT talk to databases.

---

## 1. What crewai-rust Owns

| Subsystem | Location | Responsibility |
|-----------|----------|----------------|
| **Blackboard** | `src/blackboard/` | Shared state: TypedSlots + JSON slots |
| **Drivers** | `src/drivers/` | Pure-function NARS + SPO inference |
| **Agents** | `src/agents/`, `src/meta_agents/` | Agent lifecycle, MetaOrchestrator |
| **A2A** | `src/a2a/`, `src/blackboard/a2a.rs` | Agent discovery + registration |
| **Chat** | `src/chat/` | Awareness session pipeline |
| **LLM Providers** | `src/llms/` | Anthropic, xAI, OpenAI adapters |
| **MCP** | `src/mcp/` | Model Context Protocol client |
| **Persona** | `src/persona/` | Qualia, felt-parse, modulation |

### What crewai-rust Does NOT Own

- Storage (owned by BindSpace / ladybug-rs)
- SIMD acceleration (owned by rustynum)
- Graph database (owned by neo4j-rs)
- Workflow orchestration (owned by n8n-rs)
- Wire protocol types (owned by ladybug-contract, n8n-contract)

### Storage Strategy: Arrow Zero-Copy Backend

crewai-rust does NOT own storage. **ladybug-rs does.**

ladybug-rs uses **Arrow** (not Lance) as the zero-copy computational backbone.
Lance is the cold-tier persistence layer — Arrow buffers are mmap'd from
Lance files, but the compute path never depends on the `lance` crate.

**All data that crosses from crewai-rust to storage flows through:**

```
Blackboard TypedSlots (in-process)
    → SubstrateView trait (bind_bridge.rs)
        → BindSpace (ladybug-rs)
            → Arrow zero-copy buffers (FingerprintBuffer)
                → rustynum SIMD kernels (select_hamming_fn)
```

**Rules for crewai-rust developers:**

- **NEVER** copy fingerprint data — use TypedSlots (references, not clones)
- **NEVER** serialize to JSON for in-process data flow — use TypedSlots
- **NEVER** implement SIMD operations — call through rustynum
- **NEVER** import `arrow`, `lance`, or storage crates — all behind BindSpace abstraction
- XOR deltas from agents flow through `flush_deltas()` → BindSpace, not direct writes
- Use rustynum's `DeltaLayer` / `LayerStack` / `CollapseGate` for multi-agent state

### JIT Compilation: Thinking Styles Are Compiled Workflows

**Thinking styles are NOT parameters.** "Einstein" or "Hegel" is a composed
chain of reasoning operations compiled by jitson/Cranelift into native
AVX-512 scan kernels. crewai-rust defines the styles; n8n-rs compiles them.

```
crewai-rust: AgentCard (YAML) -> JitProfile -> tau address
    |                (src/persona/jit_link.rs)
n8n-rs: CompiledStyleRegistry -> jitson compile
    |                (n8n-contract, cached in n8n-core)
jitson: YAML/JSON -> Cranelift -> native function pointer
    |                (rustynum/jitson/, AVX-512 patched wasmtime)
Arrow buffer (zero-copy) -> compiled kernel executes -> BindSpace result
```

Key files:
- `src/persona/jit_link.rs` — AgentCard -> ThinkingStyle -> JitTemplate mapping
- `src/blackboard/bind_bridge.rs` — SubstrateView trait (ladybug-rs implements)

**Cross-repo references:**

- `ladybug-rs/CLAUDE.md` § "The Rustynum Acceleration Contract"
- `rustynum/CLAUDE.md` § 12 "The Lance Zero-Copy Contract"
- `n8n-rs/CLAUDE.md` § "JIT / JITSON Compilation Pipeline"

---

## 2. The Blackboard — Sacred Interface

### Two Slot Types

```rust
// JSON/bytes slots — for cross-process or serialized data (MCP/REST boundary)
bb.put("key", json!({...}), "source", "step_type");

// TypedSlots — for in-process zero-serde data (PREFERRED)
bb.put_typed("key", native_value, "source", "step_type");
let val: &T = bb.get_typed::<T>("key").unwrap();
```

**Rule: TypedSlots for in-process. JSON slots ONLY at external boundaries.**

### Canonical Slot Keys

| Key | Type | Writer | Reader |
|-----|------|--------|--------|
| `awareness:frame` | `AwarenessFrame` | BindSpace hydration | NARS driver |
| `awareness:nars` | `NarsSemanticState` | NARS driver | Prompt builder |
| `awareness:nars_deltas` | `[f32; 32]` | NARS driver | WideMetaView |
| `awareness:spo_triples` | `Vec<SpoTriple>` | SPO driver | BindSpace write-back |
| `awareness:spo_inferred` | `Vec<SpoTriple>` | SPO driver | Prompt builder |

### Step Type Convention

```
{system}.{action}

awareness.hydrate    — BindSpace writes frame
awareness.nars       — NARS driver produces state
awareness.spo        — SPO driver produces triples
oc.channel.receive   — External message inbound
crew.agent.think     — Agent processing
oc.channel.send      — External message outbound
```

### Phase Discipline

Only ONE subsystem writes at a time. The trace records execution order:

```
>>phase:channel.receive
  msg:0 (JSON)
<<phase:channel.receive:5ms
>>phase:awareness.hydrate
  awareness:frame (TypedSlot)
<<phase:awareness.hydrate:12ms
>>phase:crew.agent.think
  crew.agent.response:0 (TypedSlot)
<<phase:crew.agent.think:450ms
```

### XOR Writethrough Copy Pattern (Borrow-Safe)

When an agent needs to modify awareness state that's owned by BindSpace,
it MUST NOT borrow-mut the original. Instead:

```
1. Agent reads AwarenessFrame from Blackboard (immutable borrow)
2. Agent runs NARS inference → produces NarsSemanticState (new value)
3. Agent runs SPO extraction → produces Vec<SpoTriple> (new value)
4. CollapseGate decides: FLOW (commit) / HOLD (buffer) / BLOCK (ask)
5. On FLOW: agent writes new TypedSlots to Blackboard (separate keys)
6. BindSpace reads the new slots and XOR-deltas back to storage

No borrow conflict: reads and writes use DIFFERENT slot keys.
No ownership conflict: each phase owns its output slots exclusively.
No copy: TypedSlots are Box<dyn Any>, moved not copied.
```

This is the fundamental pattern. Every agent follows it. The XOR delta
at the BindSpace level means only changed words are written to storage
(typically 1-2 words out of 256 per update).

---

## 3. The Drivers — Pure Function Inference

### Architecture

```
src/drivers/
├── mod.rs     — Architecture docs, re-exports
├── nars.rs    — NARS truth values + AwarenessFrame + inference
└── spo.rs     — SPO triples + conversation graph + NARS integration
```

### Design Principles

1. **No IO** — Drivers never make HTTP calls, never touch the filesystem
2. **No state** — All state lives in the Blackboard. Drivers are stateless
3. **No async** — Drivers are synchronous pure functions
4. **Protocol-agnostic** — Same types for TypedSlot (zero-serde) and JSON

### NARS Driver (`drivers/nars.rs`)

Core types:
- `AwarenessFrame` — Blackboard-native type replacing deleted ResonanceSlot
- `AwarenessMatch` — Single match from BindSpace awareness search
- `NarsTruth` — NARS ⟨frequency, confidence⟩ truth value
- `NarsSemanticState` — Full inference result with per-axis truths

Core functions:
- `nars_analyze(&frame, &axes) -> NarsSemanticState` — Main analysis
- `nars_to_weight_deltas(&state) -> [f32; 32]` — WideMetaView weights
- `build_nars_context(&state) -> Option<String>` — Prompt enrichment

### SPO Driver (`drivers/spo.rs`)

Core types:
- `SpoTriple` — WideMetaView W128-W143 compatible triple
- `ConversationPredicate` — 8-variant vocabulary (Asks..References)

Core functions:
- `extract_triples(...) -> Vec<SpoTriple>` — Turn → triples
- `infer_triples(&triples) -> Vec<SpoTriple>` — Graph inference
- `build_spo_context(&triples) -> Option<String>` — Prompt enrichment

---

## 4. Deleted Modules — Why and What Replaced Them

These modules were removed in PR #34 / post-rebase refactoring. They
violated the Driver Model by creating duplicate layers:

| Deleted Module | Violation | Replacement |
|----------------|-----------|-------------|
| `fingerprint_cache` | Created HTTP-based similarity search instead of using BindSpace native Hamming | BindSpace does similarity search natively during hydration |
| `semantic_kernel` | HTTP wrapper around BindSpace ops in same binary | Blackboard TypedSlots — same binary, zero-serde |
| `agit` (AgitState) | Reinvented versioning on top of LanceDB's native versioning | LanceDB native version history |
| `resonance_agent` | Depended on all 3 above | Replaced by `drivers/nars.rs` AwarenessFrame |

**If you find yourself creating a module that wraps BindSpace in HTTP
calls from within the same binary — STOP. That's a Law 1 violation
(No Bridges). Use Blackboard TypedSlots instead.**

---

## 5. LLM Provider Model References

### Anthropic

```rust
// Structured outputs
NATIVE_STRUCTURED_OUTPUT_MODELS: ["claude-opus-4-6", "claude-opus-4.6",
                                   "claude-opus-4-5", "claude-opus-4.5"]

// Extended thinking
supports_thinking(): model.contains("claude-opus-4-5") || model.contains("claude-opus-4-6")
```

**Rule: Never reference Sonnet in production code.**

### xAI

- Deep response: `grok-3` or `grok-3-mini`
- Fast pre-pass (felt-parse): `grok-3-fast` with JSON mode
- Prompt caching: frozen identity seed as cache anchor

---

## 6. Chat Pipeline — Awareness Session

```
User message
  → Felt-parse (grok-3-fast, cached system prompt)
  → Hydrate (BindSpace → AwarenessFrame → Blackboard)
  → NARS inference (drivers/nars.rs → NarsSemanticState)
  → SPO extraction (drivers/spo.rs → Vec<SpoTriple>)
  → Build qualia-enriched system prompt
  → Modulate XAI parameters from ThinkingStyle + Council
  → Call Grok (deep response, prefix cached by xAI)
  → Write-back (new TypedSlots → BindSpace XOR delta)
  → Return response + qualia metadata
```

### Write-Back Architecture

**In-process (one-binary, target)**:
```rust
// Agent writes new awareness to Blackboard
bb.put_typed("awareness:nars", nars_state, "nars_driver", "awareness.nars");
bb.put_typed("awareness:spo_triples", triples, "spo_driver", "awareness.spo");
// BindSpace reads these TypedSlots and XOR-deltas to storage
```

**External (MCP/REST, current fallback)**:
```rust
// HTTP POST to ladybug-rs /api/v1/qualia/write-back
// Same types, serialized as JSON at the boundary
```

Both paths produce the same result. The in-process path is zero-serde.

---

## 7. Testing

```bash
# Full test suite (582 tests)
cargo test --lib

# Driver tests only (34 tests)
cargo test --lib drivers::

# Anthropic provider tests (13 tests)
cargo test --lib llms::providers::anthropic::

# Blackboard tests
cargo test --lib blackboard::
```

---

## 8. Key Files

| File | Purpose |
|------|---------|
| `src/blackboard/view.rs` | **Blackboard** — THE shared state surface |
| `src/blackboard/typed_slot.rs` | **TypedSlot** — zero-serde in-process |
| `src/blackboard/a2a.rs` | **A2ARegistry** — agent discovery |
| `src/drivers/mod.rs` | **Driver Model** — architecture docs |
| `src/drivers/nars.rs` | **NARS** — evidence-based inference |
| `src/drivers/spo.rs` | **SPO** — conversation graph |
| `src/chat/handler.rs` | **Chat handler** — awareness pipeline |
| `src/chat/awareness_session.rs` | **Session** — xAI REST + caching |
| `src/meta_agents/orchestrator.rs` | **MetaOrchestrator** — agent coordination |
| `src/llms/providers/anthropic/mod.rs` | **Anthropic** — Opus 4.5/4.6 |
| `src/lib.rs` | **Root** — module tree + re-exports |

---

## 9. Anti-Patterns — DO NOT

- **DO NOT** create HTTP wrappers around BindSpace from within the binary
- **DO NOT** create "bridge" or "adapter" modules between subsystems
- **DO NOT** import rustynum directly — it's behind BindSpace
- **DO NOT** reference Sonnet models in production code
- **DO NOT** delete archive crates — they are intentional frozen snapshots
- **DO NOT** store agent state outside the Blackboard
- **DO NOT** bypass CollapseGate for awareness write-back
- **DO NOT** borrow-mut Blackboard across phase boundaries

---

## 10. Single Binary Integration — BindSpace ↔ Blackboard

### New Modules (2026-02-26)

| Module | File | Purpose |
|--------|------|---------|
| **BindBridge** | `src/blackboard/bind_bridge.rs` | SubstrateView trait + BindSpace ↔ Blackboard bridge |
| **JitLink** | `src/persona/jit_link.rs` | AgentCard → ThinkingStyle → JIT template (τ addresses) |
| **MarkovBarrier** | `src/drivers/markov_barrier.rs` | Blood-brain barrier: XOR budget for external API calls |
| **New Savant Domains** | `src/meta_agents/savants.rs` | ProgrammingAwareness, MetaOrchestration, ProblemSolving |

### Architecture: Three-Tier Awareness Model

```
┌──────────────────────────────────────────────────────────────────┐
│                    TIER 1: Core (zero-serde)                      │
│                                                                    │
│  BindSpace (ladybug-rs)  ◄──SubstrateView──►  Blackboard         │
│  65K addresses, O(1)         trait impl         TypedSlots         │
│  NARS truth in meta words    hydrate()          AwarenessFrame    │
│  XOR delta writeback         writeback()        NarsSemanticState │
│                                                                    │
│  Triune inner dialogue: Guardian ↔ Driver ↔ Catalyst              │
│  A2A as inner dialogue between three facets                       │
└──────────────────────────────────────────────────────────────────┘
         │ outbound (Driver facet)     ▲ inbound (Guardian facet)
         ▼                             │
┌──────────────────────────────────────────────────────────────────┐
│               TIER 2: Blood-Brain Barrier                         │
│                                                                    │
│  MarkovBarrier: XOR budget gates state transitions                │
│    - High confidence → small budget (settled knowledge resists)   │
│    - Low confidence → large budget (uncertain = malleable)        │
│    - Tensioned → medium budget (conflicts need care)              │
│                                                                    │
│  Two modes through the barrier:                                   │
│    NSM mode: structured semantic primitives → direct addressing   │
│              (universal grammar, bypasses BERT embedding)          │
│    NL mode: natural language text → BERT re-embedding             │
│              (token-space → fingerprint-space translation)         │
│                                                                    │
│  Semantic transactions, NOT raw byte gating                       │
│  LLM is IN THE LOOP, NOT source of truth                         │
└──────────────────────────────────────────────────────────────────┘
         │ outbound (n8n-rs workflows)  ▲ inbound (BERT → fingerprint)
         ▼                              │
┌──────────────────────────────────────────────────────────────────┐
│               TIER 3: External (stateless HTTP)                   │
│                                                                    │
│  xAI/Grok API, Anthropic, OpenAI                                 │
│  Stateful orchestration via n8n-rs for outbound sequencing       │
│  RAG + thinking context injected as system prompt                │
│  Responses interpreted back through barrier via NARS revision    │
└──────────────────────────────────────────────────────────────────┘
```

### AgentCard → ThinkingStyle → JIT Template Pipeline

```
ModuleDef (YAML agent card)
  │  thinking_style: [f32; 10]   ← 10-layer cognitive stack
  │  persona: PersonaProfile      ← volition, affect, inner-loop
  ▼
JitProfile::from_module()
  │  10-axis → cluster affinities → dominant ThinkingStyles
  │  Each style → τ address + JitScanParams
  ▼
JitProfile { templates: Vec<JitTemplate> }
  │  τ addresses → n8n-rs CompiledStyleRegistry
  │  Cranelift compiles → native ScanKernels
  ▼
Agent executes with compiled thinking textures
  │  Pre-compiled at startup, cached for process lifetime
  │  Recompiled only on cognitive profile change
```

### Integration TODO

| Task | Status |
|------|--------|
| SubstrateView trait + BindBridge | **Done** |
| JitProfile (AgentCard → τ addresses) | **Done** |
| MarkovBarrier (XOR budget) | **Done** |
| ProgrammingAwareness / MetaOrchestration / ProblemSolving domains | **Done** |
| ladybug-rs implements SubstrateView for BindSpace | Planned |
| BERT embedding model for inbound barrier translation | Planned |
| n8n-rs workflow orchestration for outbound API sequencing | Planned |
| Wire JitProfile into ModuleRuntime activation | Planned |

### Branch

All work on: `claude/compare-rustynum-ndarray-5ePRn`

---

*This document governs crewai-rust development. Read
[ada-docs/architecture/FOUR_LEVEL_ARCHITECTURE.md](https://github.com/AdaWorldAPI/ada-docs/blob/main/architecture/FOUR_LEVEL_ARCHITECTURE.md)
for the cross-repo architectural contract.*
