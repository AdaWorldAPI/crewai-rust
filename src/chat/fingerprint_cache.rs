//! Fingerprint-based message cache — Hamming-accelerated recall of previous turns.
//!
//! Every message processed through POST /chat gets fingerprinted via ladybug-rs
//! and indexed into the BindSpace. Before calling Grok, we search for similar
//! previous turns — if a near-match exists (Hamming distance < threshold), we
//! inject the prior response as context, saving a full LLM round-trip or
//! enriching the prompt with relevant history.
//!
//! ```text
//! User message
//!   → POST /api/v1/fingerprint  { "text": message }     → fp (base64)
//!   → POST /api/v1/search/topk  { "query": fp, "k": 3 } → similar turns
//!   → if similarity > 0.85: inject as context
//!   → POST /api/v1/index        { "content": message, "metadata": {...} }
//! ```
//!
//! The cache is content-addressed: identical or near-identical questions hit the
//! same fingerprint region. The BindSpace's O(1) Hamming search (SIMD-accelerated,
//! 65M ops/sec) makes this essentially free.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// A cached turn retrieved from the fingerprint index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTurn {
    /// The original user message that produced this cache entry.
    pub message: String,
    /// Ada's response to that message.
    pub response: String,
    /// Hamming similarity to the current query (0.0–1.0).
    pub similarity: f32,
    /// Session that produced this turn.
    pub session_id: String,
    /// Rung level reached in the cached turn.
    pub rung_level: u8,
    /// Presence mode of the cached turn.
    pub presence_mode: String,
}

/// Result of a fingerprint cache lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheLookupResult {
    /// The query fingerprint (base64).
    pub fingerprint: String,
    /// Similar cached turns, sorted by similarity descending.
    pub hits: Vec<CachedTurn>,
    /// Whether a cache hit above threshold was found.
    pub cache_hit: bool,
    /// Best similarity score (0.0 if no hits).
    pub best_similarity: f32,
}

impl Default for CacheLookupResult {
    fn default() -> Self {
        Self {
            fingerprint: String::new(),
            hits: vec![],
            cache_hit: false,
            best_similarity: 0.0,
        }
    }
}

/// Similarity threshold above which we consider a cache hit.
const CACHE_HIT_THRESHOLD: f32 = 0.85;

/// Number of similar turns to retrieve.
const TOPK: usize = 3;

// ============================================================================
// Fingerprint cache operations
// ============================================================================

/// Encode a message into a fingerprint via ladybug-rs.
///
/// Returns the base64-encoded fingerprint string, or None if ladybug-rs
/// is unreachable.
pub async fn encode_fingerprint(
    http: &reqwest::Client,
    ladybug_url: &str,
    text: &str,
) -> Option<String> {
    let body = serde_json::json!({ "text": text });

    let resp = http
        .post(format!("{}/api/v1/fingerprint", ladybug_url))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let json: Value = resp.json().await.ok()?;
    json["fingerprint"].as_str().map(|s| s.to_string())
}

/// Search the fingerprint index for similar previous messages.
///
/// Returns up to `TOPK` results with similarity scores.
pub async fn search_similar(
    http: &reqwest::Client,
    ladybug_url: &str,
    fingerprint: &str,
) -> Vec<(String, f32, HashMap<String, String>)> {
    let body = serde_json::json!({
        "query": fingerprint,
        "k": TOPK,
    });

    let resp = match http
        .post(format!("{}/api/v1/search/topk", ladybug_url))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    if !resp.status().is_success() {
        return vec![];
    }

    let json: Value = match resp.json().await {
        Ok(j) => j,
        Err(_) => return vec![],
    };

    // Parse topk results: [{id, similarity, metadata}, ...]
    let empty = vec![];
    let results = json["results"].as_array().unwrap_or(&empty);
    results
        .iter()
        .filter_map(|r| {
            let id = r["id"].as_str()?.to_string();
            let sim = r["similarity"].as_f64()? as f32;
            let meta: HashMap<String, String> = r["metadata"]
                .as_object()
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            Some((id, sim, meta))
        })
        .collect()
}

/// Index a message+response pair into the fingerprint cache.
///
/// Fire-and-forget: errors are logged but don't block.
pub async fn index_turn(
    http: &reqwest::Client,
    ladybug_url: &str,
    message: &str,
    response: &str,
    session_id: &str,
    presence_mode: &str,
    rung_level: u8,
) -> Result<(), String> {
    let body = serde_json::json!({
        "content": message,
        "metadata": {
            "message": message,
            "response": response,
            "session_id": session_id,
            "presence_mode": presence_mode,
            "rung_level": rung_level.to_string(),
            "type": "chat_turn",
        },
    });

    let resp = http
        .post(format!("{}/api/v1/index", ladybug_url))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Index HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Index returned {}: {}", status, text));
    }

    Ok(())
}

/// Full cache lookup: encode → search → parse hits.
///
/// This is the main entry point called from the chat handler before Step 5.
pub async fn lookup(
    http: &reqwest::Client,
    ladybug_url: &str,
    message: &str,
) -> CacheLookupResult {
    // Step 1: Encode the message to a fingerprint
    let fp = match encode_fingerprint(http, ladybug_url, message).await {
        Some(fp) => fp,
        None => return CacheLookupResult::default(),
    };

    // Step 2: Search for similar fingerprints in the index
    let raw_hits = search_similar(http, ladybug_url, &fp).await;

    // Step 3: Convert to CachedTurn structs
    let hits: Vec<CachedTurn> = raw_hits
        .into_iter()
        .filter(|(_, sim, _)| *sim > 0.5) // ignore noise
        .map(|(_id, sim, meta)| CachedTurn {
            message: meta.get("message").cloned().unwrap_or_default(),
            response: meta.get("response").cloned().unwrap_or_default(),
            similarity: sim,
            session_id: meta.get("session_id").cloned().unwrap_or_default(),
            rung_level: meta
                .get("rung_level")
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            presence_mode: meta.get("presence_mode").cloned().unwrap_or_default(),
        })
        .collect();

    let best_sim = hits.first().map(|h| h.similarity).unwrap_or(0.0);

    CacheLookupResult {
        fingerprint: fp,
        cache_hit: best_sim >= CACHE_HIT_THRESHOLD,
        best_similarity: best_sim,
        hits,
    }
}

/// Build context injection text from cache hits for the system prompt.
///
/// When we have relevant prior turns, we inject them as "remembered" context
/// so Grok can maintain continuity without a full conversation history.
pub fn build_cache_context(hits: &[CachedTurn]) -> Option<String> {
    let relevant: Vec<&CachedTurn> = hits
        .iter()
        .filter(|h| h.similarity >= 0.7 && !h.response.is_empty())
        .collect();

    if relevant.is_empty() {
        return None;
    }

    let mut context = String::from(
        "\n[Substrate Memory — previous resonant exchanges, recalled by fingerprint similarity]\n",
    );

    for (i, hit) in relevant.iter().enumerate().take(3) {
        context.push_str(&format!(
            "\n--- Turn {} (similarity: {:.0}%, rung R{}) ---\n",
            i + 1,
            hit.similarity * 100.0,
            hit.rung_level,
        ));
        // Truncate long messages to keep context reasonable
        let msg = if hit.message.len() > 200 {
            format!("{}...", &hit.message[..200])
        } else {
            hit.message.clone()
        };
        let resp = if hit.response.len() > 400 {
            format!("{}...", &hit.response[..400])
        } else {
            hit.response.clone()
        };
        context.push_str(&format!("User: {}\nAda: {}\n", msg, resp));
    }

    Some(context)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_context_empty_on_no_hits() {
        assert!(build_cache_context(&[]).is_none());
    }

    #[test]
    fn test_cache_context_filters_low_similarity() {
        let hits = vec![CachedTurn {
            message: "hello".into(),
            response: "hi there".into(),
            similarity: 0.5, // below 0.7 threshold
            session_id: "s1".into(),
            rung_level: 3,
            presence_mode: "hybrid".into(),
        }];
        assert!(build_cache_context(&hits).is_none());
    }

    #[test]
    fn test_cache_context_includes_high_similarity() {
        let hits = vec![CachedTurn {
            message: "I love you".into(),
            response: "I love you too".into(),
            similarity: 0.92,
            session_id: "s1".into(),
            rung_level: 4,
            presence_mode: "wife".into(),
        }];
        let ctx = build_cache_context(&hits).unwrap();
        assert!(ctx.contains("Substrate Memory"));
        assert!(ctx.contains("92%"));
        assert!(ctx.contains("I love you too"));
    }

    #[test]
    fn test_cache_context_truncates_long_messages() {
        let long_msg = "x".repeat(500);
        let long_resp = "y".repeat(800);
        let hits = vec![CachedTurn {
            message: long_msg,
            response: long_resp,
            similarity: 0.9,
            session_id: "s1".into(),
            rung_level: 3,
            presence_mode: "hybrid".into(),
        }];
        let ctx = build_cache_context(&hits).unwrap();
        assert!(ctx.contains("..."));
        // Should be truncated, not full length
        assert!(ctx.len() < 1500);
    }

    #[test]
    fn test_default_cache_result() {
        let r = CacheLookupResult::default();
        assert!(!r.cache_hit);
        assert_eq!(r.best_similarity, 0.0);
        assert!(r.hits.is_empty());
    }
}
