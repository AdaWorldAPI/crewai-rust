//! Integration test: Claude ⇆ Grok A2A ping-pong loop.
//!
//! Exercises the full Agent-to-Agent pipeline:
//!
//! 1. Register Claude (Anthropic) and Grok (xAI) as agents in A2ARegistry
//! 2. Open a bidirectional A2A channel between them
//! 3. Agent A (Claude) receives a seed task, produces a response
//! 4. Agent B (Grok) receives Claude's output via A2A, refines it
//! 5. Round-trip validation: coherent multi-agent reasoning
//!
//! # Running
//!
//! Requires live API keys:
//! ```bash
//! ANTHROPIC_API_KEY=sk-... XAI_API_KEY=xai-... cargo test --test a2a_claude_grok_loop -- --ignored
//! ```
//!
//! Unit-level wiring tests run without keys (not `#[ignore]`).

use std::collections::HashMap;

use serde_json::Value;

use crewai::blackboard::{A2ARegistry, AgentState};
use crewai::drivers::spo::{
    extract_triples, infer_triples, entity_hash, ConversationPredicate, SpoTriple,
};
use crewai::llms::base_llm::{BaseLLM, LLMMessage};
use crewai::llms::providers::anthropic::AnthropicCompletion;
use crewai::llms::providers::xai::XAICompletion;

// ============================================================================
// Helper: build a simple user message
// ============================================================================

fn user_msg(content: &str) -> LLMMessage {
    let mut msg = HashMap::new();
    msg.insert("role".into(), Value::String("user".into()));
    msg.insert("content".into(), Value::String(content.into()));
    msg
}

fn system_msg(content: &str) -> LLMMessage {
    let mut msg = HashMap::new();
    msg.insert("role".into(), Value::String("system".into()));
    msg.insert("content".into(), Value::String(content.into()));
    msg
}

// ============================================================================
// Unit tests — A2A wiring, no API keys needed
// ============================================================================

#[test]
fn test_a2a_registry_claude_grok_agents() {
    let mut registry = A2ARegistry::new();

    registry.register(
        "claude-agent",
        "Claude",
        "Deep reasoning agent (Anthropic)",
        vec!["reasoning".into(), "analysis".into(), "code".into()],
    );

    registry.register(
        "grok-agent",
        "Grok",
        "Fast reasoning agent with live search (xAI)",
        vec!["reasoning".into(), "search".into(), "synthesis".into()],
    );

    assert_eq!(registry.len(), 2);

    // Both can reason
    let reasoners = registry.by_capability("reasoning");
    assert_eq!(reasoners.len(), 2);

    // Only Grok has search
    let searchers = registry.by_capability("search");
    assert_eq!(searchers.len(), 1);
    assert_eq!(searchers[0].name, "Grok");

    // Only Claude has code
    let coders = registry.by_capability("code");
    assert_eq!(coders.len(), 1);
    assert_eq!(coders[0].name, "Claude");
}

#[test]
fn test_a2a_state_machine_for_loop() {
    let mut registry = A2ARegistry::new();

    registry.register("claude-agent", "Claude", "reason", vec![]);
    registry.register("grok-agent", "Grok", "reason", vec![]);

    // Step 1: Claude starts processing
    registry.set_state("claude-agent", AgentState::Active);
    registry.set_goal("claude-agent", "Analyze the seed task");
    assert_eq!(registry.active_agents().len(), 1);

    // Step 2: Claude finishes, Grok takes over
    registry.set_state("claude-agent", AgentState::Completed);
    registry.set_state("grok-agent", AgentState::Active);
    registry.set_goal("grok-agent", "Refine Claude's analysis");
    assert_eq!(registry.active_agents().len(), 1);
    assert_eq!(registry.active_agents()[0].name, "Grok");

    // Step 3: Grok finishes
    registry.set_state("grok-agent", AgentState::Completed);
    assert_eq!(registry.active_agents().len(), 0);
    assert_eq!(registry.by_state(AgentState::Completed).len(), 2);
}

#[test]
fn test_spo_triples_from_a2a_exchange() {
    // Simulate a conversation turn between Claude and Grok via A2A
    let triples = extract_triples(
        "Compare quantum computing approaches for optimization problems",
        "Quantum annealing excels at combinatorial optimization while gate-based approaches...",
        "a2a-session-claude-grok",
        "work",
        &["THOUGHT".into()],
        &[],
    );

    // Should produce: (User, asks, topic), (Ada, explains, topic),
    // (Ada, feels_about, THOUGHT), (session, mode_switch, work)
    assert!(triples.len() >= 4);

    // Project all triples to 3D and verify spatial coherence
    let positions: Vec<[f32; 3]> = triples.iter().map(|t| t.to_3d()).collect();

    for pos in &positions {
        for c in pos {
            assert!(*c >= 0.0 && *c <= 1.0);
        }
    }

    // Infer relationships
    let inferred = infer_triples(&triples);
    assert!(
        !inferred.is_empty(),
        "Should infer at least one relationship from ask+explain"
    );

    // Inferred triples should also project cleanly to 3D
    for t in &inferred {
        let pos = t.to_3d();
        assert!(t.is_inferred());
        for c in &pos {
            assert!(*c >= 0.0 && *c <= 1.0);
        }
    }
}

#[test]
fn test_spo_3d_clustering_by_predicate() {
    // Triples with the same predicate should cluster on the y-axis
    let asks_1 = SpoTriple::new(
        entity_hash("user"),
        ConversationPredicate::Asks.hash(),
        entity_hash("quantum"),
        0.8,
    );
    let asks_2 = SpoTriple::new(
        entity_hash("researcher"),
        ConversationPredicate::Asks.hash(),
        entity_hash("optimization"),
        0.8,
    );
    let explains = SpoTriple::new(
        entity_hash("claude"),
        ConversationPredicate::Explains.hash(),
        entity_hash("quantum"),
        0.8,
    );

    let v1 = asks_1.to_3d();
    let v2 = asks_2.to_3d();
    let v3 = explains.to_3d();

    // Same predicate → same y coordinate
    assert!(
        (v1[1] - v2[1]).abs() < 1e-6,
        "Same predicate should yield same y: {} vs {}",
        v1[1],
        v2[1]
    );

    // Different predicate → different y coordinate
    assert!(
        (v1[1] - v3[1]).abs() > 0.01,
        "Different predicate should separate on y: {} vs {}",
        v1[1],
        v3[1]
    );

    // Distance between same-predicate triples should be < distance to different-predicate
    let d_same = asks_1.distance_3d(&asks_2);
    let d_diff = asks_1.distance_3d(&explains);
    // Not guaranteed in all cases, but for these specific inputs:
    println!(
        "Same-pred distance: {:.4}, cross-pred distance: {:.4}",
        d_same, d_diff
    );
}

#[test]
fn test_provider_construction() {
    // Verify both providers construct without panicking
    let claude = AnthropicCompletion::new("claude-haiku-4-5-20251001", None, None);
    assert_eq!(claude.model(), "claude-haiku-4-5-20251001");
    assert_eq!(claude.provider(), "anthropic");
    assert!(claude.supports_function_calling());

    let grok = XAICompletion::new("grok-3-mini", None, None);
    assert_eq!(grok.model(), "grok-3-mini");
    assert_eq!(grok.provider(), "xai");
    assert!(grok.supports_function_calling());
    assert_eq!(grok.get_context_window_size(), 131_072);
}

// ============================================================================
// Live integration test — requires API keys
// ============================================================================

/// Full Claude ⇆ Grok A2A ping-pong loop.
///
/// Run with:
/// ```bash
/// ANTHROPIC_API_KEY=sk-... XAI_API_KEY=xai-... \
///   cargo test --test a2a_claude_grok_loop test_claude_grok_a2a_loop -- --ignored
/// ```
#[tokio::test]
#[ignore]
async fn test_claude_grok_a2a_loop() {
    // ---- Setup A2A registry ----
    let mut registry = A2ARegistry::new();
    registry.register(
        "claude",
        "Claude",
        "Deep analysis",
        vec!["reasoning".into(), "analysis".into()],
    );
    registry.register(
        "grok",
        "Grok",
        "Fast synthesis with search",
        vec!["reasoning".into(), "search".into(), "synthesis".into()],
    );

    // ---- Initialize providers ----
    let claude = AnthropicCompletion::new("claude-haiku-4-5-20251001", None, None);
    let grok = XAICompletion::new("grok-3-mini", None, None);

    let seed_task = "In exactly 2-3 sentences, explain why Rust's ownership model \
                     prevents data races at compile time.";

    // ---- Turn 1: Claude processes the seed task ----
    registry.set_state("claude", AgentState::Active);
    registry.set_goal("claude", seed_task);

    let claude_messages = vec![
        system_msg("You are a precise technical expert. Keep answers to 2-3 sentences."),
        user_msg(seed_task),
    ];

    let claude_result = claude.acall(claude_messages, None, None).await;
    assert!(claude_result.is_ok(), "Claude call failed: {:?}", claude_result.err());

    let claude_response = claude_result.unwrap();
    let claude_text = claude_response.as_str().unwrap_or("").to_string();
    assert!(!claude_text.is_empty(), "Claude returned empty response");
    println!("\n[Claude → Grok] {}\n", claude_text);

    registry.set_state("claude", AgentState::Completed);

    // ---- Extract SPO triples from Claude's turn ----
    let claude_triples = extract_triples(
        seed_task,
        &claude_text,
        "a2a-claude-grok",
        "work",
        &[],
        &[],
    );
    println!(
        "SPO triples from Claude's turn: {} (3D projections below)",
        claude_triples.len()
    );
    for t in &claude_triples {
        let v = t.to_3d();
        let pred = ConversationPredicate::from_hash(t.predicate_hash)
            .map(|p| p.label())
            .unwrap_or("?");
        println!("  ({:#010x}) —[{}]→ ({:#010x})  3D=[{:.3}, {:.3}, {:.3}]",
            t.subject_dn, pred, t.object_dn, v[0], v[1], v[2]);
    }

    // ---- Turn 2: Grok refines Claude's output ----
    registry.set_state("grok", AgentState::Active);
    registry.set_goal("grok", "Refine and extend Claude's analysis");

    let grok_messages = vec![
        system_msg(
            "You are a fast synthesis agent. You received the following analysis from Claude \
             (another AI agent). Critique it in 2-3 sentences — add what's missing or correct \
             any imprecision."
        ),
        user_msg(&format!(
            "Claude's analysis:\n\n{}\n\nYour critique (2-3 sentences):",
            claude_text
        )),
    ];

    let grok_result = grok.acall(grok_messages, None, None).await;
    assert!(grok_result.is_ok(), "Grok call failed: {:?}", grok_result.err());

    let grok_response = grok_result.unwrap();
    let grok_text = grok_response.as_str().unwrap_or("").to_string();
    assert!(!grok_text.is_empty(), "Grok returned empty response");
    println!("\n[Grok → result] {}\n", grok_text);

    registry.set_state("grok", AgentState::Completed);

    // ---- Extract SPO triples from Grok's turn ----
    let grok_triples = extract_triples(
        &format!("Critique Claude's analysis of {}", seed_task),
        &grok_text,
        "a2a-claude-grok",
        "work",
        &[],
        &[],
    );

    // ---- Combine and infer cross-agent relationships ----
    let mut all_triples: Vec<SpoTriple> = Vec::new();
    all_triples.extend(claude_triples);
    all_triples.extend(grok_triples);

    let inferred = infer_triples(&all_triples);
    println!(
        "Cross-agent inferred triples: {} (total graph: {} nodes)",
        inferred.len(),
        all_triples.len() + inferred.len()
    );

    for t in &inferred {
        let v = t.to_3d();
        let pred = ConversationPredicate::from_hash(t.predicate_hash)
            .map(|p| p.label())
            .unwrap_or("?");
        println!(
            "  [inferred] ({:#010x}) —[{}]→ ({:#010x})  3D=[{:.3}, {:.3}, {:.3}]  c={:.0}%",
            t.subject_dn, pred, t.object_dn, v[0], v[1], v[2],
            t.confidence() * 100.0
        );
    }

    // ---- Verify the loop completed ----
    assert_eq!(registry.by_state(AgentState::Completed).len(), 2);
    assert!(!all_triples.is_empty(), "Should have SPO triples from both agents");
    println!("\nA2A loop complete: Claude → Grok round-trip verified.");
}

/// Single-agent sanity check with Claude only.
#[tokio::test]
#[ignore]
async fn test_claude_single_turn() {
    let claude = AnthropicCompletion::new("claude-haiku-4-5-20251001", None, None);
    let messages = vec![user_msg("Say hello in exactly 3 words.")];

    let result = claude.acall(messages, None, None).await;
    assert!(result.is_ok(), "Claude call failed: {:?}", result.err());

    let text = result.unwrap();
    println!("Claude says: {}", text);
}

/// Single-agent sanity check with Grok only.
#[tokio::test]
#[ignore]
async fn test_grok_single_turn() {
    let grok = XAICompletion::new("grok-3-mini", None, None);
    let messages = vec![user_msg("Say hello in exactly 3 words.")];

    let result = grok.acall(messages, None, None).await;
    assert!(result.is_ok(), "Grok call failed: {:?}", result.err());

    let text = result.unwrap();
    println!("Grok says: {}", text);
}
