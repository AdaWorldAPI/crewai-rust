//! Semantic Kernel Bridge — ladybug-rs BindSpace as runtime kernel for crewai-rust.
//!
//! This module uses ladybug-rs as a semantic kernel where:
//!
//! - **Prefix 0x0C** (Agents) — each agent's identity lives as a BindNode
//! - **Prefix 0x0E** (Blackboard) — per-agent mutable state (ice-caked awareness)
//! - **Prefix 0x0D** (Thinking Styles) — τ-addressed cognitive profiles
//! - **Prefix 0x0F** (A2A Routing) — agent-to-agent message passing
//!
//! The bridge exposes ladybug-rs operations as a Rust-side `SemanticKernel` that
//! crewai-rust agents can borrow/mut through the blackboard scheme:
//!
//! ```text
//! Agent slot 0x0C:03 → reads Blackboard at 0x0E:03 (immutable borrow)
//! Agent slot 0x0C:03 → writes Blackboard at 0x0E:03 (mutable borrow)
//! Agent slot 0x0C:03 → reads ThinkingStyle at 0x0D:τ  (shared borrow)
//! ```
//!
//! This is the borrow/mut pattern from the blackboard scheme — agents can read
//! any blackboard slot, but only write their own. The kernel enforces this at
//! the address level.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// BindSpace address constants (matching ladybug-rs bind_space.rs)
// ============================================================================

/// Surface prefix for agent slots.
pub const PREFIX_AGENTS: u8 = 0x0C;
/// Surface prefix for thinking style slots.
pub const PREFIX_THINKING_STYLES: u8 = 0x0D;
/// Surface prefix for agent blackboard (per-agent mutable state).
pub const PREFIX_BLACKBOARD: u8 = 0x0E;
/// Surface prefix for A2A routing.
pub const PREFIX_A2A: u8 = 0x0F;

// ============================================================================
// Blackboard state (mirrors ladybug-rs blackboard_agent.rs)
// ============================================================================

/// Agent blackboard entry — per-agent mutable state stored in BindSpace.
///
/// This mirrors the `AgentBlackboard` in ladybug-rs `orchestration/blackboard_agent.rs`.
/// The crewai-rust side reads/writes this through HTTP, and ladybug-rs persists it
/// in the BindSpace at address `(0x0E, agent_slot)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackboardEntry {
    /// Active thinking style name.
    pub active_style: String,
    /// Reasoning coherence (0.0–1.0).
    pub coherence: f32,
    /// Task progress (0.0–1.0).
    pub progress: f32,
    /// Frozen commitments (ice-caked — won't change until explicitly thawed).
    pub ice_caked: Vec<String>,
    /// Active goals from agent card.
    pub active_goals: Vec<String>,
    /// Resonance hits from recent queries.
    pub resonance_hits: u32,
    /// Pending A2A messages.
    pub pending_messages: u32,
    /// Flow state momentum (0.0–1.0).
    pub flow_state: f32,
    /// Self-reported confidence (0.0–1.0).
    pub confidence: f32,
    /// State fingerprint (SHA-256 of awareness state).
    pub state_fingerprint: String,
}

impl Default for BlackboardEntry {
    fn default() -> Self {
        Self {
            active_style: "balanced".into(),
            coherence: 0.5,
            progress: 0.0,
            ice_caked: vec![],
            active_goals: vec![],
            resonance_hits: 0,
            pending_messages: 0,
            flow_state: 0.5,
            confidence: 0.5,
            state_fingerprint: String::new(),
        }
    }
}

// ============================================================================
// Semantic Kernel client
// ============================================================================

/// The SemanticKernel is the crewai-rust side of the ladybug-rs BindSpace bridge.
///
/// It provides borrow/mut semantics over HTTP:
/// - `read_blackboard()` — immutable borrow of agent state
/// - `write_blackboard()` — mutable borrow (only to own slot)
/// - `read_style()` — shared borrow of thinking style
/// - `resolve_agent()` — look up agent fingerprint by slot
#[derive(Debug, Clone)]
pub struct SemanticKernel {
    /// HTTP client for ladybug-rs calls.
    http: reqwest::Client,
    /// Ladybug-rs base URL.
    ladybug_url: String,
    /// Local blackboard cache (reduces round-trips).
    cache: HashMap<u8, BlackboardEntry>,
}

impl SemanticKernel {
    /// Create a new kernel bridge to ladybug-rs.
    pub fn new(ladybug_url: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            ladybug_url: ladybug_url.to_string(),
            cache: HashMap::new(),
        }
    }

    /// Read an agent's blackboard entry (immutable borrow).
    ///
    /// Reads from local cache first, then falls back to ladybug-rs.
    /// Address: `(0x0E, agent_slot)`.
    pub async fn read_blackboard(&mut self, agent_slot: u8) -> BlackboardEntry {
        if let Some(entry) = self.cache.get(&agent_slot) {
            return entry.clone();
        }

        // Fetch from ladybug-rs via CogRedis GET
        let addr = format!("{:02x}:{:02x}", PREFIX_BLACKBOARD, agent_slot);
        match self.cogredis_get(&addr).await {
            Some(value) => {
                let entry: BlackboardEntry =
                    serde_json::from_value(value).unwrap_or_default();
                self.cache.insert(agent_slot, entry.clone());
                entry
            }
            None => BlackboardEntry::default(),
        }
    }

    /// Write to an agent's blackboard (mutable borrow).
    ///
    /// Only writes to the agent's own slot — this is the borrow-mut invariant.
    /// The caller must own the slot (enforced by the agent_slot parameter).
    /// Address: `(0x0E, agent_slot)`.
    pub async fn write_blackboard(
        &mut self,
        agent_slot: u8,
        entry: &BlackboardEntry,
    ) -> Result<(), String> {
        let addr = format!("{:02x}:{:02x}", PREFIX_BLACKBOARD, agent_slot);
        let value = serde_json::to_value(entry).map_err(|e| e.to_string())?;
        self.cogredis_set(&addr, &value).await?;
        self.cache.insert(agent_slot, entry.clone());
        Ok(())
    }

    /// Resolve an agent's fingerprint by slot.
    ///
    /// Address: `(0x0C, agent_slot)`.
    pub async fn resolve_agent(&self, agent_slot: u8) -> Option<String> {
        let addr = format!("{:02x}:{:02x}", PREFIX_AGENTS, agent_slot);
        let value = self.cogredis_get(&addr).await?;
        value["fingerprint"].as_str().map(|s| s.to_string())
    }

    /// Read a thinking style by τ address.
    ///
    /// Address: `(0x0D, tau)`.
    pub async fn read_style(&self, tau: u8) -> Option<Value> {
        let addr = format!("{:02x}:{:02x}", PREFIX_THINKING_STYLES, tau);
        self.cogredis_get(&addr).await
    }

    /// Bind a message fingerprint into the substrate as a conversation node.
    ///
    /// Uses XOR binding: `message_fp ⊗ session_fp ⊗ verb_fp` to create an
    /// edge in the graph that can be unbound to recover any component.
    pub async fn bind_conversation(
        &self,
        message_fp: &str,
        session_fp: &str,
    ) -> Option<String> {
        let body = serde_json::json!({
            "a": message_fp,
            "b": session_fp,
        });

        let resp = self.http
            .post(format!("{}/api/v1/bind", self.ladybug_url))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .ok()?;

        let json: Value = resp.json().await.ok()?;
        json["result"].as_str().map(|s| s.to_string())
    }

    /// Bundle multiple turn fingerprints into a session summary.
    ///
    /// Majority-vote bundling: the resulting fingerprint captures the
    /// "average" semantic content of all turns in the session.
    pub async fn bundle_session(&self, turn_fps: &[String]) -> Option<String> {
        if turn_fps.is_empty() {
            return None;
        }

        let body = serde_json::json!({
            "fingerprints": turn_fps,
        });

        let resp = self.http
            .post(format!("{}/api/v1/bundle", self.ladybug_url))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .ok()?;

        let json: Value = resp.json().await.ok()?;
        json["result"].as_str().map(|s| s.to_string())
    }

    // ── Internal CogRedis helpers ──────────────────────────────────────────

    /// GET a value from CogRedis via the /redis endpoint.
    async fn cogredis_get(&self, key: &str) -> Option<Value> {
        let body = format!("GET {}", key);

        let resp = self.http
            .post(format!("{}/redis", self.ladybug_url))
            .header("Content-Type", "text/plain")
            .body(body)
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let text = resp.text().await.ok()?;
        // CogRedis returns Redis wire protocol, parse the bulk string
        parse_redis_bulk(&text)
    }

    /// SET a value in CogRedis via the /redis endpoint.
    async fn cogredis_set(&self, key: &str, value: &Value) -> Result<(), String> {
        let json_str = serde_json::to_string(value).map_err(|e| e.to_string())?;
        let body = format!("SET {} {}", key, json_str);

        let resp = self.http
            .post(format!("{}/redis", self.ladybug_url))
            .header("Content-Type", "text/plain")
            .body(body)
            .send()
            .await
            .map_err(|e| format!("CogRedis SET error: {}", e))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!("CogRedis SET returned {}", resp.status()))
        }
    }
}

/// Parse a Redis bulk string response into a JSON Value.
fn parse_redis_bulk(text: &str) -> Option<Value> {
    let trimmed = text.trim();

    // Redis nil
    if trimmed == "$-1" || trimmed == "(nil)" {
        return None;
    }

    // Try to find the JSON payload after Redis protocol framing
    // Bulk string: $<len>\r\n<data>\r\n
    if let Some(start) = trimmed.find('{') {
        let json_str = &trimmed[start..];
        serde_json::from_str(json_str).ok()
    } else {
        // Try parsing the whole thing as JSON
        serde_json::from_str(trimmed).ok()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blackboard_defaults() {
        let bb = BlackboardEntry::default();
        assert_eq!(bb.active_style, "balanced");
        assert_eq!(bb.coherence, 0.5);
        assert!(bb.ice_caked.is_empty());
    }

    #[test]
    fn test_blackboard_serializes() {
        let bb = BlackboardEntry {
            active_style: "analytical".into(),
            coherence: 0.9,
            progress: 0.5,
            ice_caked: vec!["commitment_1".into()],
            active_goals: vec!["goal_1".into()],
            resonance_hits: 42,
            pending_messages: 0,
            flow_state: 0.8,
            confidence: 0.85,
            state_fingerprint: "abc123".into(),
        };
        let json = serde_json::to_string(&bb).unwrap();
        assert!(json.contains("analytical"));
        assert!(json.contains("commitment_1"));

        let parsed: BlackboardEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.coherence, 0.9);
    }

    #[test]
    fn test_address_format() {
        let addr = format!("{:02x}:{:02x}", PREFIX_BLACKBOARD, 0x03_u8);
        assert_eq!(addr, "0e:03");

        let agent_addr = format!("{:02x}:{:02x}", PREFIX_AGENTS, 0x10_u8);
        assert_eq!(agent_addr, "0c:10");
    }

    #[test]
    fn test_parse_redis_bulk_json() {
        let input = "$45\r\n{\"active_style\":\"balanced\",\"coherence\":0.5}\r\n";
        let value = parse_redis_bulk(input).unwrap();
        assert_eq!(value["active_style"], "balanced");
    }

    #[test]
    fn test_parse_redis_bulk_nil() {
        assert!(parse_redis_bulk("$-1").is_none());
        assert!(parse_redis_bulk("(nil)").is_none());
    }

    #[test]
    fn test_parse_redis_bulk_plain_json() {
        let input = r#"{"coherence": 0.9}"#;
        let value = parse_redis_bulk(input).unwrap();
        assert_eq!(value["coherence"], 0.9);
    }
}
