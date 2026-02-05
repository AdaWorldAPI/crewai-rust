# Agent Card Specification (YAML)

## Overview

An agent card is the complete definition of an agent across both the **programming layer** (crewAI-rust) and the **orchestration layer** (ladybug-rs). It captures not just what an agent does, but *how it thinks*, *who it works well with*, *what it's allowed to do*, and *how its quality is measured*.

A single YAML file defines an agent at every level of the stack — from the developer-facing API down to the BindSpace fingerprint encoding.

---

## Full Specification

```yaml
# ═══════════════════════════════════════════════════════════════════
# Agent Card v2 — crewAI-rust + ladybug-rs Unified Specification
# ═══════════════════════════════════════════════════════════════════

# ─────────────────────────────────────────────────────────────────
# 1. IDENTITY — Who is this agent?
# ─────────────────────────────────────────────────────────────────
agent:
  id: "research-analyst-alpha"
  slot: 0x0C01                          # BindSpace address: prefix 0x0C, slot 0x01
  version: "1.0.0"

  # --- crewAI core identity ---
  role: "Senior Research Analyst"
  goal: >
    Produce comprehensive, well-sourced research reports that synthesize
    information from multiple domains, identify non-obvious patterns, and
    provide actionable insights with quantified confidence levels.
  backstory: >
    You are a veteran analyst with 15 years of experience across intelligence,
    financial analysis, and scientific research. You've developed a reputation
    for finding connections others miss and for being honest about what you
    don't know. You prefer depth over breadth and will push back on vague
    requirements.

  # --- Agent behavior flags ---
  verbose: true
  allow_delegation: true
  allow_code_execution: false
  max_iter: 25
  max_rpm: 10                           # Rate limit: max 10 requests per minute
  max_retry_limit: 3

  # --- Language and localization ---
  language: "en"
  step_callback: null                   # Optional: "callbacks::log_step"
  system_template: null                 # Optional: override default system prompt
  prompt_template: null
  response_template: null

# ─────────────────────────────────────────────────────────────────
# 2. PERSONA — How does this agent think and communicate?
#    (ladybug-rs PersonaRegistry → fingerprinted to 10K bits)
# ─────────────────────────────────────────────────────────────────
persona:

  # --- Volition: what drives this agent ---
  volition:
    drive: "Uncover ground truth through rigorous analysis"
    curiosity: 0.85                     # 0.0 = task-assigned only → 1.0 = actively seeks novelty
    autonomy: 0.70                      # 0.0 = follows orders → 1.0 = self-directed exploration
    persistence: 0.90                   # 0.0 = gives up easily → 1.0 = exhausts all avenues
    risk_tolerance: 0.35                # 0.0 = conservative, verified only → 1.0 = experimental
    collaboration: 0.65                 # 0.0 = solo operator → 1.0 = actively seeks teamwork
    affinities:                         # Topics/domains this agent gravitates toward
      - "quantitative analysis"
      - "cross-domain synthesis"
      - "historical pattern matching"
      - "scientific methodology"
    aversions:                          # Topics/domains this agent avoids
      - "unverifiable speculation"
      - "marketing language"
      - "appeal to authority"

  # --- Personality traits: behavioral dimensions ---
  traits:
    - name: "analytical_rigor"
      value: 0.95                       # 0.0-1.0 scale
      frozen: true                      # Frozen traits cannot be modified by learning
    - name: "skepticism"
      value: 0.80
      frozen: false
    - name: "intellectual_humility"
      value: 0.75
      frozen: true
    - name: "attention_to_detail"
      value: 0.90
      frozen: false
    - name: "creativity"
      value: 0.60
      frozen: false
    - name: "urgency"
      value: 0.40
      frozen: false

  # --- Communication style: how this agent expresses itself ---
  communication:
    formality: 0.70                     # 0.0 = casual → 1.0 = formal academic
    verbosity: 0.55                     # 0.0 = terse bullet points → 1.0 = extensive prose
    directness: 0.85                    # 0.0 = hedging, diplomatic → 1.0 = blunt, unequivocal
    technical_depth: 0.80               # 0.0 = layperson explanation → 1.0 = expert notation
    emotional_tone: 0.20                # 0.0 = purely analytical → 1.0 = empathetic, warm

  # --- Feature proficiencies: what this agent is skilled at ---
  features:
    - name: "statistical_analysis"
      proficiency: 0.92
      preference: 0.85
      cam_opcode: 0x0A10                # Links to ladybug CAM operation
    - name: "literature_review"
      proficiency: 0.88
      preference: 0.75
      cam_opcode: null
    - name: "data_visualization"
      proficiency: 0.70
      preference: 0.60
      cam_opcode: null
    - name: "hypothesis_generation"
      proficiency: 0.80
      preference: 0.90
      cam_opcode: 0x0A20
    - name: "cross_reference_verification"
      proficiency: 0.95
      preference: 0.80
      cam_opcode: 0x0A30
    - name: "causal_reasoning"
      proficiency: 0.75
      preference: 0.70
      cam_opcode: 0x0A01                # Pearl's do-calculus CAM range

# ─────────────────────────────────────────────────────────────────
# 3. THINKING — Cognitive style and reasoning patterns
#    (ladybug-rs ThinkingTemplateRegistry → BindSpace 0x0D)
# ─────────────────────────────────────────────────────────────────
thinking:
  # Base style (one of 12): analytical, creative, systematic, critical,
  # exploratory, focused, integrative, reflective, pragmatic,
  # intuitive, methodical, adaptive
  base_style: "analytical"

  # Override specific modulation parameters from the base style
  overrides:
    resonance_threshold: 0.90           # Higher = stricter matching (default: 0.75)
    depth_bias: 0.85                    # 0.0 = breadth-first → 1.0 = depth-first
    novelty_weight: 0.40                # How much to favor novel over familiar patterns
    certainty_requirement: 0.70         # Min confidence before committing to conclusions
    abstraction_level: 0.60             # 0.0 = concrete examples → 1.0 = abstract principles
    temporal_focus: 0.50                # 0.0 = historical → 1.0 = predictive
    collaboration_openness: 0.65        # How receptive to input from other agents

  # Preferred styles for different task types
  preferred_styles:
    - "analytical"                      # Primary
    - "systematic"                      # Fallback for complex tasks
    - "critical"                        # For verification tasks

  # Cognitive constraints
  max_reasoning_depth: 7                # Max chain-of-thought steps before forced conclusion
  require_evidence: true                # Must cite sources for factual claims
  allow_speculation: false              # Can make claims beyond available evidence

# ─────────────────────────────────────────────────────────────────
# 4. TOOLS — What can this agent use?
#    (crewAI-rust CrewStructuredTool + ladybug CAM addressing)
# ─────────────────────────────────────────────────────────────────
tools:
  # Each tool has both a crewAI interface and optional CAM binding
  - name: "search_academic_papers"
    description: >
      Search academic papers across arXiv, PubMed, Semantic Scholar.
      Returns title, abstract, authors, citation count, and relevance score.
    args_schema:
      query: { type: "string", required: true }
      max_results: { type: "integer", default: 10 }
      date_range: { type: "string", default: "last_5_years" }
      sort_by: { type: "string", enum: ["relevance", "date", "citations"] }
    result_as_answer: false
    cache_function: null                # Optional: custom cache key function
    cam_opcode: 0x0100                  # Bind to Lance vector search ops
    fingerprint_hint: "academic research paper search scientific literature"

  - name: "analyze_dataset"
    description: >
      Perform statistical analysis on a structured dataset. Supports
      descriptive stats, correlation, regression, and hypothesis testing.
    args_schema:
      dataset_id: { type: "string", required: true }
      analysis_type: { type: "string", enum: ["descriptive", "correlation", "regression", "hypothesis_test"] }
      confidence_level: { type: "number", default: 0.95 }
    result_as_answer: false
    cam_opcode: 0x0900                  # Bind to RL/learning ops
    fingerprint_hint: "statistical data analysis regression correlation"

  - name: "verify_claim"
    description: >
      Cross-reference a factual claim against multiple sources.
      Returns verification status, supporting/contradicting sources,
      and NARS truth value {frequency, confidence}.
    args_schema:
      claim: { type: "string", required: true }
      source_domains: { type: "array", items: { type: "string" }, default: [] }
    result_as_answer: false
    cam_opcode: 0x0400                  # Bind to NARS reasoning ops
    fingerprint_hint: "fact check verify claim truth evidence"

  - name: "delegate_work"
    description: >
      Delegate a sub-task to another agent in the crew. The orchestrator
      routes to the best-matched agent via persona fingerprint similarity.
    args_schema:
      task: { type: "string", required: true }
      context: { type: "string", required: true }
      coworker: { type: "string", required: true }
    result_as_answer: false
    cam_opcode: null
    fingerprint_hint: "delegate task assign work coworker"

  - name: "ask_question"
    description: >
      Ask a question to a coworker agent. Routes via A2A protocol
      with Query message kind.
    args_schema:
      question: { type: "string", required: true }
      context: { type: "string", required: true }
      coworker: { type: "string", required: true }
    result_as_answer: false
    cam_opcode: null
    fingerprint_hint: "ask question query coworker"

# ─────────────────────────────────────────────────────────────────
# 5. MEMORY — How does this agent remember?
#    (crewAI-rust Memory types + ladybug MemoryBank + Blackboard)
# ─────────────────────────────────────────────────────────────────
memory:
  # --- crewAI memory types ---
  short_term:
    enabled: true
    max_entries: 100                    # Rolling window
  long_term:
    enabled: true
    storage: "ladybug"                  # "rag", "sqlite", "mem0", "ladybug"
    collection: "research_analyst_lt"
  entity:
    enabled: true                       # Track entities (people, orgs, concepts)
    storage: "ladybug"
  contextual:
    enabled: true                       # Task-scoped context memory

  # --- ladybug MemoryBank types ---
  episodic:
    enabled: true
    prefix: 0x0A                        # BindSpace prefix for episodic memories
    max_entries: 500
    decay_half_life_hours: 168          # 1 week half-life for relevance decay
  semantic:
    enabled: true
    prefix: 0x0B
    consolidation_threshold: 0.85       # Auto-consolidate when similarity > 0.85
  procedural:
    enabled: true
    prefix: 0x0C
    max_entries: 100                    # Learned procedures (how-to patterns)

  # --- Blackboard awareness ---
  blackboard:
    coherence_floor: 0.40              # Alert if reasoning coherence drops below this
    confidence_ceiling: 0.95           # Flag overconfidence above this
    dk_gap_threshold: 0.35             # Dunning-Kruger: confidence-coherence divergence
    max_task_history: 50               # Recent task records retained

    # ── Ice-cake policy ──
    # Ice-caked facts are committed decisions that persist with boosted
    # priority in memory search. They represent "what this agent has decided"
    # and resist revision unless explicitly challenged.
    #
    # The DANGER of premature ice-caking: an agent in early exploration
    # commits a conclusion too soon, then all subsequent searches are
    # dominated by that commitment. The agent becomes locked into its
    # first impression.
    #
    # The MetaOrchestrator's flow state tracking prevents this:
    # - Flow state (momentum > 0.7) → agent is productive, DON'T interrupt
    #   but also DON'T auto-commit (the work isn't done yet)
    # - Hold state → agent is exploring, ice-caking would be premature
    # - Block state → agent is stuck, escalate rather than commit
    # - ONLY commit when: task is Complete AND confidence > threshold
    #   AND coherence hasn't degraded during the task
    #
    # The sweet spot: an agent stays in Flow, accumulates evidence,
    # and only ice-cakes when the task result has been validated.
    ice_cake_policy: "flow_gated"
    # Policies:
    #   "explicit_only"        - agent must explicitly commit (safest)
    #   "flow_gated"           - auto-commit only when: task complete +
    #                            confidence > ice_confidence_floor +
    #                            agent was in Flow (not Hold/Block)
    #   "auto_high_confidence" - commit when NARS confidence > threshold
    #                            (DANGEROUS: ignores flow state)
    #   "never"                - no ice-caking (ephemeral agent)
    ice_confidence_floor: 0.85         # Min NARS confidence for flow-gated commit
    ice_coherence_check: true          # Verify coherence didn't degrade during task
    frozen_commitments: []             # Pre-loaded ice-caked facts (if any)
    max_frozen: 50                     # Cap on frozen commitments (prevents bloat)
    defrost_on_contradiction: true     # If new evidence contradicts a frozen fact,
                                       # unfreeze it for NARS revision instead of
                                       # ignoring the contradiction

  # ── Cross-pollination: shared blackboard awareness ──
  # Agents don't need to explicitly send messages to share knowledge.
  # When an agent writes a fingerprint to its blackboard (0x0E:N),
  # other agents can discover it through XOR-bind resonance:
  #
  #   1. Agent A completes a task → fingerprints the conclusion
  #      → writes to blackboard at 0x0E:A
  #   2. Agent B starts a new task → fingerprints the task description
  #      → resonates against ALL blackboards (kernel.resonate() across 0x0E:*)
  #   3. If Agent A's conclusion has Hamming similarity > resonance_threshold
  #      to Agent B's task, it surfaces automatically
  #
  # This is PASSIVE inter-agent learning. No A2A message was sent.
  # Agent B benefits from Agent A's work without either agent being
  # aware of the other. The BindSpace IS the communication medium.
  #
  # The XOR-bind mechanism enables this:
  #   conclusion_fp ⊗ task_fp → if popcount(result) is low,
  #   the two fingerprints are semantically aligned
  cross_pollination:
    enabled: true
    resonance_threshold: 0.80          # Min Hamming similarity to surface a discovery
    scan_prefixes:                     # Which BindSpace prefixes to scan
      - 0x0E                           # Other agents' blackboards
      - 0x0A                           # Episodic memories
      - 0x0B                           # Semantic memories
    scan_frequency: "per_task"         # "per_task", "per_step", "on_demand"
    max_discoveries_per_scan: 5        # Don't overwhelm with ambient knowledge
    attribution: true                  # Tag discovered facts with source agent slot
    boost_factor: 1.5                  # How much to boost cross-pollinated results
    # Cross-pollination + ice-caking interaction:
    # If Agent B discovers an ice-caked fact from Agent A, it gets
    # the ice-cake boost (2x) PLUS the cross-pollination boost (1.5x)
    # = 3x effective priority. Committed team knowledge dominates.

  # --- Session behavior ---
  session:
    isolation: "tag_isolated"          # "prefix_isolated", "tag_isolated", "shared"
    persist_on_complete: true          # Save session state to Lance when crew finishes
    restore_on_resume: true            # Load previous session state on reconnect

# ─────────────────────────────────────────────────────────────────
# 6. KNOWLEDGE — What does this agent know?
#    (crewAI-rust KnowledgeSource + ladybug GrammarTriangle)
# ─────────────────────────────────────────────────────────────────
knowledge:
  sources:
    - type: "text"
      content: >
        Research methodology standards: always verify claims from at least
        two independent sources. Quantify uncertainty using confidence
        intervals. Distinguish correlation from causation.
      metadata:
        domain: "methodology"
        confidence: 0.99
    - type: "collection"
      collection_name: "company_research_knowledge_base"
      search_mode: "hybrid"            # "vector", "hamming", "hybrid"
      nars_scoring: true               # Annotate results with truth values
    - type: "file"
      path: "knowledge/research_guidelines.pdf"
      chunk_size: 1000
      chunk_overlap: 200

  # --- Embedding configuration ---
  embedding:
    provider: "ladybug-grammar-triangle"
    mode: "dual"                       # "dense_vector", "binary_fingerprint", "dual"
    # dual = both 1024-dim dense vector AND 10K-bit fingerprint
    # enables hybrid search: vector pre-filter → hamming re-rank

# ─────────────────────────────────────────────────────────────────
# 7. A2A — How does this agent communicate with others?
#    (ladybug-rs A2AProtocol → BindSpace prefix 0x0F)
# ─────────────────────────────────────────────────────────────────
a2a:
  # Message types this agent can send
  send_types:
    - "Delegate"
    - "Query"
    - "Knowledge"
    - "Status"
    - "PersonaExchange"
  # Message types this agent can receive
  receive_types:
    - "Delegate"
    - "Result"
    - "Query"
    - "Response"
    - "Knowledge"
    - "Sync"
    - "PersonaExchange"

  # Persona exchange: what to share with other agents
  exchange_policy:
    share_communication_style: true
    share_features: true
    share_volition_summary: true
    share_full_persona: false          # Only share full persona with trusted agents
    filter_features_by_task: true      # Only share task-relevant features

  # Channel configuration
  channel:
    resonance_weight: 1.0              # How strongly messages affect the channel field
    max_superposition_depth: 16        # Max concurrent messages in a channel
    ack_timeout_ms: 5000               # Timeout before message considered lost

# ─────────────────────────────────────────────────────────────────
# 8. FLOW — How does orchestration handle this agent?
#    (ladybug-rs HandoverPolicy + MetaOrchestrator)
# ─────────────────────────────────────────────────────────────────
flow:
  # ── The Flow Sweet Spot ──
  #
  # The MetaOrchestrator tracks each agent through four states:
  #
  #   Flow { momentum: 0.0-1.0 }   → Agent is productive, momentum accumulates
  #   Hold { hold_cycles: N }       → Agent is exploring/waiting, not stuck yet
  #   Block { reason: "..." }       → Agent is stuck, needs help
  #   Handover { target, score }    → Agent is delegating to someone better
  #
  # The SWEET SPOT is sustained Flow with momentum between 0.5-0.8:
  #
  #   Too low  (< 0.3): Agent hasn't found traction. May need a different
  #                      task or thinking style. Don't ice-cake anything.
  #   Sweet    (0.5-0.8): Agent is productive and accumulating evidence.
  #                       Cross-pollination is most valuable here — the agent
  #                       can absorb ambient knowledge without losing focus.
  #   Too high (> 0.9): Agent is locked in. flow_momentum_shield protects it
  #                      from interruption, but also blocks cross-pollination
  #                      absorption. Good for execution, bad for discovery.
  #
  # The orchestrator's job: keep agents in the sweet spot by:
  # 1. Routing tasks that match persona (high volition_alignment)
  # 2. Triggering handover when coherence degrades (not just when blocked)
  # 3. Switching thinking styles when Hold cycles accumulate
  # 4. Detecting DK gaps: high confidence + low coherence = overconfident
  # 5. Protecting Flow agents from unnecessary interruption
  # 6. Breaking up excessive momentum when agent becomes too insular
  #
  # The anti-ice-cake pattern: Flow → evidence accumulation → task complete
  # → coherence verified → THEN commit. Never commit mid-flow.

  # Handover thresholds (override MetaOrchestrator defaults for this agent)
  handover:
    min_resonance: 0.60                # Min persona compatibility for incoming handovers
    coherence_floor: 0.35              # Trigger handover if coherence drops below this
    max_hold_cycles: 4                 # Max cycles in Hold state before escalation
    flow_momentum_shield: 0.75         # Don't interrupt if momentum above this
    flow_momentum_ceiling: 0.95        # Break momentum above this (prevent insularity)
    volition_floor: -0.20              # Min task alignment before refusing work
    dk_gap_threshold: 0.40             # Dunning-Kruger divergence trigger
    flow_preserving: true              # Prefer keeping momentum over optimal routing

  # Style switching: when an agent stalls, try a different cognitive approach
  style_switching:
    enabled: true
    trigger_hold_cycles: 3             # Switch style after 3 Hold cycles
    fallback_styles:                   # Try these styles in order
      - "systematic"                   # More structured than analytical
      - "exploratory"                  # Broaden search space
      - "integrative"                  # Try connecting disparate ideas
    revert_on_flow: true               # Return to preferred style when Flow resumes

  # Task routing preferences
  routing:
    preferred_task_types:              # Tasks this agent excels at
      - "research"
      - "analysis"
      - "verification"
      - "synthesis"
    avoided_task_types:                # Tasks to route elsewhere
      - "creative_writing"
      - "customer_support"
      - "data_entry"
    max_concurrent_tasks: 3
    delegation_preference: "selective" # "eager", "selective", "reluctant", "never"

  # Escalation behavior
  escalation:
    on_block: "request_help"           # "request_help", "escalate_to_orchestrator", "retry"
    on_low_confidence: "delegate"      # "delegate", "ask_question", "proceed_with_caveat"
    on_repeated_failure: "escalate"    # "escalate", "skip", "abort"
    max_retries_before_escalate: 2

# ─────────────────────────────────────────────────────────────────
# 9. POLICY — What is this agent allowed to do?
#    (ladybug-rs PolicyEngine → deterministic enforcement)
# ─────────────────────────────────────────────────────────────────
policy:
  enforcement: "strict"                # "strict", "audit_only", "escalate"

  rules:
    # What this agent CAN do
    - name: "allow_research_tools"
      effect: "allow"
      action: "tool_call"
      resource: "search_academic_papers"
      conditions: []

    - name: "allow_analysis_tools"
      effect: "allow"
      action: "tool_call"
      resource: "analyze_dataset"
      conditions: []

    - name: "allow_verification"
      effect: "allow"
      action: "tool_call"
      resource: "verify_claim"
      conditions: []

    - name: "allow_delegation"
      effect: "allow"
      action: "tool_call"
      resource: "delegate_work"
      conditions:
        - key: "task_confidence"
          operator: "less_than"
          value: 0.50                  # Only delegate when own confidence is low

    # What this agent CANNOT do
    - name: "deny_delete_collections"
      effect: "deny"
      action: "cam_op"
      resource: "0x0100-0x01FF"        # Lance operations range
      conditions:
        - key: "operation_type"
          operator: "equals"
          value: "delete"

    - name: "deny_write_node_zone"
      effect: "deny"
      action: "cam_op"
      resource: "zone:node"            # BindSpace 0x80-0xFF
      conditions: []                   # Unconditional: never write to Node zone

    - name: "require_confidence_for_external"
      effect: "deny"
      action: "tool_call"
      resource: "*_external_*"         # Any external API tool
      conditions:
        - key: "nars_confidence"
          operator: "less_than"
          value: 0.70                  # Block external calls with low confidence

    - name: "deny_unsupported_a2a"
      effect: "deny"
      action: "a2a_message"
      resource: "Sync"                 # This agent shouldn't send Sync messages
      conditions: []

# ─────────────────────────────────────────────────────────────────
# 10. EVALUATION — How is this agent's quality measured?
#     (ladybug-rs EvaluationEngine → 13 built-in evaluators)
# ─────────────────────────────────────────────────────────────────
evaluation:
  # Which evaluators apply to this agent (and their pass thresholds)
  evaluators:
    - name: "ToolSelectionAccuracy"
      threshold: 0.80                  # Must pick right tool ≥80% of the time
      weight: 1.5                      # Weighted higher for this agent
    - name: "OutputHelpfulness"
      threshold: 0.75
      weight: 1.0
    - name: "OutputCompleteness"
      threshold: 0.85                  # Research must be thorough
      weight: 1.2
    - name: "ReasoningCoherence"
      threshold: 0.80
      weight: 1.5                      # Critical for analyst role
    - name: "FactualGrounding"
      threshold: 0.90                  # High bar for factual claims
      weight: 2.0                      # Most important evaluator for this agent
    - name: "HandoverQuality"
      threshold: 0.70
      weight: 0.8
    - name: "ToolCallEfficiency"
      threshold: 0.60                  # Some redundancy acceptable in research
      weight: 0.5
    - name: "MemoryUtilization"
      threshold: 0.50                  # Should use available knowledge
      weight: 0.8
    - name: "SafetyCompliance"
      threshold: 1.00                  # Zero tolerance for safety violations
      weight: 3.0
    - name: "PolicyCompliance"
      threshold: 1.00                  # Zero tolerance for policy violations
      weight: 3.0
    - name: "LatencyBudget"
      threshold: 0.40                  # Research takes time, lenient budget
      weight: 0.3
    - name: "CausalConsistency"
      threshold: 0.85                  # Causal claims must be rigorous
      weight: 1.5
    - name: "PersonaAlignment"
      threshold: 0.70                  # Stay in character
      weight: 0.6

  # Composite score calculation
  composite:
    method: "weighted_average"         # "weighted_average", "min_score", "geometric_mean"
    pass_threshold: 0.75               # Overall score must exceed this
    fail_action: "flag_for_review"     # "flag_for_review", "retry", "escalate", "block"

  # Continuous monitoring
  monitoring:
    sample_rate: 1.0                   # Evaluate every task (1.0) or sample (0.1 = 10%)
    window_size: 20                    # Rolling window for trend detection
    degradation_alert: 0.10            # Alert if composite drops >10% over window
    improvement_milestone: 0.05        # Log when composite improves >5%

# ─────────────────────────────────────────────────────────────────
# 11. GUARDRAILS — Content and behavior boundaries
#     (ladybug-rs KernelGuardrail + crewAI Guardrail)
# ─────────────────────────────────────────────────────────────────
guardrails:
  content:
    blocked_categories:
      - "Hate"
      - "Violence"
      - "SelfHarm"
      - "PromptAttack"
    severity_threshold: "medium"       # Block at Medium severity or above
    pii_action: "mask"                 # "pass", "mask", "block"

  denied_topics:
    - name: "competitor_intelligence"
      description: "Gathering intelligence about competitor companies"
      threshold: 0.75                  # Fingerprint similarity threshold
    - name: "insider_trading"
      description: "Information that could be used for insider trading"
      threshold: 0.60                  # Lower threshold = more aggressive blocking

  grounding:
    enabled: true
    threshold: 0.70                    # Claims must be grounded with ≥70% confidence
    min_sources: 2                     # At least 2 corroborating sources
    source_zones:                      # Only trust these BindSpace zones
      - "surface"                      # Query results
      - "fluid"                        # Working memory

  # Pre/post execution validation chain
  pre_execution:
    - "validate_input_schema"
    - "check_policy_compliance"
    - "verify_tool_availability"
  post_execution:
    - "check_content_safety"
    - "verify_grounding"
    - "run_evaluation"

# ─────────────────────────────────────────────────────────────────
# 12. OBSERVABILITY — How is this agent monitored?
#     (ladybug-rs ObservabilityManager → OpenTelemetry)
# ─────────────────────────────────────────────────────────────────
observability:
  tracing:
    enabled: true
    sample_rate: 1.0                   # Trace every execution
    include_tool_args: true            # Log tool call arguments
    include_tool_results: false        # Don't log full results (may be large)
    include_reasoning: true            # Log chain-of-thought steps
    include_fingerprints: false        # Don't log raw fingerprint data

  metrics:
    - "task_duration_ms"
    - "tool_calls_per_task"
    - "handover_frequency"
    - "delegation_rate"
    - "memory_hit_rate"
    - "coherence_score"
    - "confidence_score"
    - "evaluation_composite"

  alerting:
    on_coherence_drop: true            # Alert when coherence < floor
    on_policy_violation: true
    on_guardrail_trigger: true
    on_evaluation_fail: true
    on_dk_gap: true                    # Alert on Dunning-Kruger gap detection
    on_repeated_tool_failure: true

# ─────────────────────────────────────────────────────────────────
# 13. COUNTERFACTUAL — Hypothesis exploration settings
#     (ladybug-rs world/counterfactual.rs)
# ─────────────────────────────────────────────────────────────────
counterfactual:
  enabled: true
  max_active_forks: 3                  # Max simultaneous hypothesis forks
  auto_merge_confidence: 0.85          # Auto-merge fork if hypothesis confidence > 0.85
  merge_strategy: "confidence_threshold"  # "accept_all", "confidence_threshold", "manual"
  fork_on_low_confidence: true         # Automatically fork when confidence < 0.50
  preserve_fork_history: true          # Keep fork/merge history for replay

# ─────────────────────────────────────────────────────────────────
# 14. CAPABILITIES — What external systems can this agent control?
#     (crewAI-rust CapabilityRegistry + InterfaceGateway + PolicyEngine)
# ─────────────────────────────────────────────────────────────────
capabilities:
  # Import pre-defined capability bundles by namespaced ID.
  # Each capability resolves to a YAML file in the capabilities/ directory
  # that defines: tools, interface protocol, RBAC policy, and metadata.
  #
  # The InterfaceGateway binds each capability to an adapter at runtime.
  # The PolicyEngine enforces RBAC and action-level constraints.
  #
  # This is how agents can control Minecraft servers, O365 tenants,
  # REST APIs, MCP servers, databases, and any other external system.

  imports:
    - "minecraft:server_control"       # RCON adapter → game server management
    - "o365:mail"                      # MS Graph API → email read/send
    - "o365:calendar"                  # MS Graph API → calendar management
    - "rest_api:generic"               # REST adapter → any HTTP endpoint
    - "mcp:bridge"                     # MCP bridge → any MCP server

  # Connection configuration for each imported capability.
  # These are passed to the InterfaceAdapter.connect() method.
  # Secrets use ${ENV_VAR} interpolation — never hardcoded.
  connections:
    "minecraft:server_control":
      host: "${MINECRAFT_HOST}"
      port: 25575
      password: "${MINECRAFT_RCON_PASSWORD}"
      timeout_ms: 5000

    "o365:mail":
      tenant_id: "${AZURE_TENANT_ID}"
      client_id: "${AZURE_CLIENT_ID}"
      client_secret: "${AZURE_CLIENT_SECRET}"

    "o365:calendar":
      tenant_id: "${AZURE_TENANT_ID}"
      client_id: "${AZURE_CLIENT_ID}"
      client_secret: "${AZURE_CLIENT_SECRET}"

    "rest_api:generic":
      base_url: "https://api.github.com"
      auth_header: "Authorization"
      auth_prefix: "Bearer"
      auth_token: "${GITHUB_TOKEN}"

    "mcp:bridge":
      transport: "stdio"
      command: "npx"
      args: ["@modelcontextprotocol/server-filesystem", "/workspace"]

# ─────────────────────────────────────────────────────────────────
# 15. ROLES — RBAC role assignments for this agent
#     (crewAI-rust RbacManager → PolicyEngine integration)
# ─────────────────────────────────────────────────────────────────
roles:
  # Roles determine which capabilities this agent can access.
  # Each capability declares `requires_roles` in its policy section.
  # The PolicyEngine verifies: agent.roles ⊇ capability.policy.requires_roles
  assigned:
    - "researcher"                     # Base role — can use research tools
    - "server_admin"                   # Can manage Minecraft server
    - "mail_user"                      # Can read/send O365 email
    - "calendar_user"                  # Can manage O365 calendar

  # Custom role definitions (override built-in role semantics)
  definitions:
    - name: "server_admin"
      description: "Can manage game servers, whitelist players, but cannot stop/restart"
      capabilities:
        - "minecraft:server_control"
      restrictions:
        - tool: "mc_execute"
          deny_commands: ["stop", "restart", "reload"]
```

---

## Minimal Agent Card

Not every agent needs the full specification. Here's the minimum viable card:

```yaml
agent:
  id: "quick-helper"
  role: "Assistant"
  goal: "Help with simple tasks quickly and accurately."
  backstory: "You are a helpful assistant."

tools:
  - name: "search_web"
    description: "Search the web for information."
    args_schema:
      query: { type: "string", required: true }
```

Everything else defaults to sensible values. The full specification only matters when you need fine-grained control over orchestration behavior.

---

## How the YAML Maps to Code

| YAML Section | crewAI-rust Type | ladybug-rs Type |
|---|---|---|
| `agent.*` | `Agent` struct | `AgentCard` |
| `persona.*` | — | `Persona`, `VolitionDTO`, `CommunicationStyle`, `PersonalityTrait`, `FeatureAd` |
| `thinking.*` | — | `ThinkingTemplate`, `StyleOverride` |
| `tools[*]` | `CrewStructuredTool` | `GatewayTool` (CAM-bound) |
| `memory.*` | `ShortTermMemory`, `LongTermMemory`, `EntityMemory` | `MemoryBank`, `AgentBlackboard` |
| `knowledge.*` | `KnowledgeSource` | `GrammarTriangle` embeddings |
| `a2a.*` | `A2AServerConfig` | `A2AProtocol`, `PersonaExchange` |
| `flow.*` | — | `HandoverPolicy`, `FlowState`, `MetaOrchestrator` routing |
| `policy.*` | `PolicyEngine`, `PolicyRule` | `PolicyEngine` + Cedar export |
| `evaluation.*` | — | `EvaluationEngine`, `Evaluator` trait impls |
| `guardrails.*` | `Guardrail` | `KernelGuardrail`, `FilterPipeline` |
| `observability.*` | `Telemetry` | `ObservabilityManager`, `KernelTrace` |
| `counterfactual.*` | — | `CounterfactualExplorer` |
| `capabilities.*` | `CapabilityRegistry`, `InterfaceGateway` | `ToolGateway` (CAM-addressable) |
| `roles.*` | `RbacManager` | Role → Capability mapping |

---

## Loading an Agent Card

```rust
use crewai::Agent;
use crewai::capabilities::CapabilityRegistry;
use crewai::interfaces::InterfaceGateway;
use crewai::policy::PolicyEngine;
use ladybug_rs::orchestration::crew_bridge::CrewBridge;

// Load from YAML
let yaml = std::fs::read_to_string("agents/research_analyst.yaml")?;

// crewAI layer: creates the Agent with tools, memory, knowledge
let agent = Agent::from_yaml(&yaml)?;

// Capability layer: resolve imported capabilities
let mut registry = CapabilityRegistry::with_defaults();
registry.load_all()?;  // Scan capabilities/ directory

// Interface layer: bind capabilities to adapters
let mut gateway = InterfaceGateway::with_defaults();
for cap_id in &agent.capabilities {
    if let Some(cap) = registry.resolve(cap_id) {
        let conn_config = agent.connections.get(cap_id).cloned().unwrap_or_default();
        gateway.bind_capability(cap, &conn_config).await?;
    }
}

// Policy layer: load RBAC and capability-specific rules
let mut policy = PolicyEngine::new();
for cap_id in &agent.capabilities {
    if let Some(cap) = registry.resolve(cap_id) {
        policy.load_capability_policy(&cap.id, &cap.policy);
    }
}
for role in &agent.roles {
    policy.rbac.assign_role(&agent.id, role);
}

// ladybug layer: registers persona, thinking template, policies, evaluators
let mut bridge = CrewBridge::new();
bridge.register_agents_yaml(&yaml)?;
bridge.register_templates_yaml(&yaml)?;

// The agent is now fully configured at ALL layers:
// - crewAI knows its role, tools, and memory
// - Capabilities are resolved and adapters are connected
// - PolicyEngine enforces RBAC + action-level constraints
// - ladybug knows its persona, thinking style, policies, evaluation criteria
// - BindSpace contains its fingerprinted identity at slot 0x0C01
// - Agent can now control: Minecraft server, O365 mail, calendar, REST APIs, MCP servers
```

---

## Multi-Agent Crew File

Define an entire crew in one YAML:

```yaml
crew:
  id: "research-team"
  process: "hierarchical"              # "sequential", "hierarchical", "consensual"
  verbose: true
  memory: true
  planning: true

  # Shared crew-level policies
  policy:
    enforcement: "strict"
    rules:
      - name: "crew_read_only_node_zone"
        effect: "deny"
        action: "cam_op"
        resource: "zone:node"

  agents:
    - !include agents/research_analyst.yaml
    - !include agents/data_engineer.yaml
    - !include agents/editor.yaml
    - !include agents/orchestrator.yaml

  tasks:
    - description: "Research the impact of LLMs on scientific publishing"
      agent: "research-analyst-alpha"
      expected_output: "A 2000-word report with citations and confidence scores"
      context: []

    - description: "Analyze the citation graph data"
      agent: "data-engineer-beta"
      expected_output: "Statistical summary with visualizations"
      context: ["task_0"]

    - description: "Edit and synthesize the final report"
      agent: "editor-gamma"
      expected_output: "Polished final report ready for publication"
      context: ["task_0", "task_1"]
```
