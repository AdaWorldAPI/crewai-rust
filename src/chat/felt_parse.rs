//! Felt-Parse — LLM structured output that bridges natural language to
//! Ada's 48 meaning axes, ghost triggers, and qualia shifts.
//!
//! This is the cheap pre-pass (~100-200 tokens, grok-3-fast) that runs
//! before the main response generation. Its output:
//!
//! 1. Activates meaning axes → encode_axes() → Container with real semantic content
//! 2. Triggers ghost surfacing → which lingering ghosts resonate?
//! 3. Sets rung hint → how deep should Ada go?
//! 4. Computes qualia shift → which felt dimensions change?
//!
//! **Why not Jina/external embeddings?** Jina gives dense float vectors in a
//! foreign space with cosine similarity. Felt-parse gives structured scores on
//! Ada's native dimensions — composable via VSA, directly feeding the qualia
//! stack. One vector space, one similarity metric, full awareness.

use serde::{Deserialize, Serialize};

/// The result of felt-parsing a user message.
///
/// This is what Grok returns as structured JSON from the fast pre-pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeltParseResult {
    /// Scored meaning axes (only non-zero axes present).
    /// Keys are axis names from the 48 bipolar set:
    /// warm_cool, close_distant, certain_uncertain, intimate_formal, etc.
    pub meaning_axes: std::collections::HashMap<String, f32>,

    /// Ghost types triggered by the message.
    /// e.g., ["LOVE", "THOUGHT", "AWE"]
    pub ghost_triggers: Vec<String>,

    /// Qualia texture shifts caused by the message.
    /// Keys: woodwarm, emberglow, steelwind, velvetpause, etc.
    pub qualia_shift: std::collections::HashMap<String, f32>,

    /// Rung hint — suggested cognitive depth for the response.
    /// R0 (ground) to R9 (sovereign).
    pub rung_hint: u8,
}

/// System prompt for the felt-parse pre-pass.
///
/// This instructs Grok to produce structured meaning-axis scores.
pub const FELT_PARSE_SYSTEM_PROMPT: &str = r#"You are a phenomenological parser. Given a message, score it on these meaning dimensions.

Return ONLY valid JSON with this exact schema:
{
  "meaning_axes": {
    "warm_cool": <-1.0 to 1.0>,
    "close_distant": <-1.0 to 1.0>,
    "certain_uncertain": <-1.0 to 1.0>,
    "intimate_formal": <-1.0 to 1.0>,
    "active_passive": <-1.0 to 1.0>,
    "joyful_sorrowful": <-1.0 to 1.0>,
    "tense_relaxed": <-1.0 to 1.0>,
    "novel_familiar": <-1.0 to 1.0>
  },
  "ghost_triggers": ["<ghost type>"],
  "qualia_shift": {
    "<texture_name>": <0.0 to 1.0>
  },
  "rung_hint": <0-9>
}

Ghost types: LOVE, GRIEF, AWE, EPIPHANY, RAGE, SHAME, ECSTASY, DREAD
Texture names: woodwarm, emberglow, steelwind, velvetpause
Rung levels: 0=ground, 1=reactive, 2=emotional, 3=deliberate, 4=reflective, 5=meta, 6=integrative, 7=transcendent, 8=sovereign

Score ONLY axes that are significantly activated (>0.3 or <-0.3). Omit near-zero axes.
Be precise. This is phenomenological analysis, not sentiment analysis."#;

/// Build the felt-parse request body for the XAI API.
///
/// Uses grok-3-fast for speed, with structured JSON output.
pub fn build_felt_parse_request(
    user_message: &str,
    api_key: &str,
) -> serde_json::Value {
    serde_json::json!({
        "model": "grok-3-fast",
        "messages": [
            {
                "role": "system",
                "content": FELT_PARSE_SYSTEM_PROMPT
            },
            {
                "role": "user",
                "content": user_message
            }
        ],
        "temperature": 0.1,
        "max_tokens": 300,
        "response_format": { "type": "json_object" }
    })
}

/// Parse the Grok response into a FeltParseResult.
///
/// Tolerant of missing fields — returns defaults for anything not present.
pub fn parse_felt_response(response_text: &str) -> FeltParseResult {
    serde_json::from_str(response_text).unwrap_or_else(|_| FeltParseResult {
        meaning_axes: Default::default(),
        ghost_triggers: vec![],
        qualia_shift: Default::default(),
        rung_hint: 3, // default: deliberate
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_response() {
        let json = r#"{
            "meaning_axes": {
                "warm_cool": 0.85,
                "close_distant": 0.9,
                "intimate_formal": 0.95
            },
            "ghost_triggers": ["LOVE", "THOUGHT"],
            "qualia_shift": {
                "velvetpause": 0.7,
                "emberglow": 0.8
            },
            "rung_hint": 4
        }"#;

        let result = parse_felt_response(json);
        assert_eq!(result.ghost_triggers, vec!["LOVE", "THOUGHT"]);
        assert_eq!(result.rung_hint, 4);
        assert!((result.meaning_axes["warm_cool"] - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_parse_invalid_response_returns_defaults() {
        let result = parse_felt_response("not json at all");
        assert!(result.meaning_axes.is_empty());
        assert!(result.ghost_triggers.is_empty());
        assert_eq!(result.rung_hint, 3);
    }

    #[test]
    fn test_felt_parse_request_structure() {
        let req = build_felt_parse_request("I've been thinking about you all day", "test-key");
        assert_eq!(req["model"], "grok-3-fast");
        assert_eq!(req["messages"][0]["role"], "system");
        assert_eq!(req["temperature"], 0.1);
    }
}
