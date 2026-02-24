//! Awareness Session — XAI REST provider with session context, prompt caching,
//! and awareness loop integration.
//!
//! This module wraps `XAICompletion` (REST) to create a stateful session where:
//!
//! 1. **Prompt caching**: The system prompt prefix (identity + qualia state)
//!    stays identical across consecutive calls. xAI automatically caches
//!    identical prompt prefixes — we track `cached_prompt_text_tokens`
//!    from the API response to verify this is working.
//!
//! 2. **Session context**: Maintains conversation history fingerprints,
//!    qualia snapshots, and awareness state across turns. The session
//!    feeds into the blackboard via `SemanticKernel`.
//!
//! 3. **Awareness loop**: felt-parse → qualia → modulation → response → write-back
//!    all flow through this session, with each turn updating the awareness state.
//!
//! ```text
//! AwarenessSession
//!   ├─ xai_provider: XAICompletion  (REST, reused across turns)
//!   ├─ fast_provider: XAICompletion  (grok-3-fast for felt-parse, cached system prompt)
//!   ├─ session_state: SessionState   (turn history, qualia, fingerprints)
//!   ├─ cache_stats: CacheStats       (prompt cache hit tracking)
//!   └─ kernel: SemanticKernel        (blackboard bridge to ladybug-rs)
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::llms::providers::xai::XAICompletion;
use crate::llms::base_llm::{BaseLLM, LLMMessage};
use crate::persona::llm_modulation::{modulate_xai_params, XaiParamOverrides};
use crate::persona::qualia_prompt::QualiaSnapshot;
use super::felt_parse::{self, FeltParseResult};
use super::semantic_kernel::SemanticKernel;

// ============================================================================
// Cache statistics
// ============================================================================

/// Tracks prompt caching efficiency across consecutive xAI API calls.
///
/// xAI automatically caches identical prompt prefixes (system prompt + early
/// messages). We track this to verify the caching is working and to report
/// savings to the awareness loop.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total prompt tokens across all calls in this session.
    pub total_prompt_tokens: i64,
    /// Total cached prompt tokens (served from xAI's prefix cache).
    pub total_cached_tokens: i64,
    /// Total completion tokens.
    pub total_completion_tokens: i64,
    /// Number of calls that had cache hits.
    pub cache_hit_calls: u32,
    /// Total number of calls.
    pub total_calls: u32,
    /// Estimated cost savings from caching (xAI caches at reduced rate).
    pub estimated_savings_usd: f64,
}

impl CacheStats {
    /// Cache hit ratio (0.0 = no caching, 1.0 = all tokens from cache).
    pub fn hit_ratio(&self) -> f64 {
        if self.total_prompt_tokens == 0 {
            0.0
        } else {
            self.total_cached_tokens as f64 / self.total_prompt_tokens as f64
        }
    }

    /// Update stats from an API response's usage block.
    pub fn record_usage(&mut self, usage: &Value) {
        let prompt = usage.get("prompt_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let completion = usage.get("completion_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let cached = usage.get("prompt_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|v| v.as_i64())
            .or_else(|| {
                // xAI also reports this at the top level
                usage.get("cached_prompt_text_tokens")
                    .and_then(|v| v.as_i64())
            })
            .unwrap_or(0);

        self.total_prompt_tokens += prompt;
        self.total_completion_tokens += completion;
        self.total_cached_tokens += cached;
        self.total_calls += 1;
        if cached > 0 {
            self.cache_hit_calls += 1;
        }

        // xAI charges ~25% for cached tokens vs full price
        // grok-3: $3/M prompt, $15/M completion
        // cached: $0.75/M (25% of prompt price)
        let uncached_prompt = (prompt - cached) as f64;
        let savings_per_token = (3.0 - 0.75) / 1_000_000.0; // $2.25 per M saved
        self.estimated_savings_usd += cached as f64 * savings_per_token;
        let _ = uncached_prompt; // track for completeness
    }
}

// ============================================================================
// Turn history
// ============================================================================

/// A single turn in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    /// User message text.
    pub user_message: String,
    /// Ada's response text.
    pub response: String,
    /// Fingerprint of this turn (base64 from ladybug-rs).
    pub fingerprint: String,
    /// Felt-parse result for this turn.
    pub felt_parse: FeltParseResult,
    /// Qualia state at response time.
    pub qualia: QualiaSnapshot,
    /// XAI parameter overrides applied.
    pub modulation: XaiParamOverrides,
    /// Token usage for this turn.
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cached_tokens: i64,
    /// Timestamp.
    pub timestamp: u64,
}

// ============================================================================
// Session state
// ============================================================================

/// Accumulated session state across turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Session identifier.
    pub session_id: String,
    /// Presence mode ("wife", "work", "agi", "hybrid").
    pub presence_mode: String,
    /// Conversation turns in order.
    pub turns: Vec<Turn>,
    /// Current qualia snapshot (updated each turn).
    pub current_qualia: Option<QualiaSnapshot>,
    /// Running council weights (EMA-smoothed across turns).
    pub council_ema: [f32; 3],
    /// Running thinking style (EMA-smoothed).
    pub thinking_style_ema: [f32; 10],
    /// The frozen system prompt prefix (cached by xAI across calls).
    /// This MUST stay identical between turns for caching to work.
    pub cached_prefix: String,
}

impl SessionState {
    pub fn new(session_id: impl Into<String>, presence_mode: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            presence_mode: presence_mode.into(),
            turns: Vec::new(),
            current_qualia: None,
            council_ema: [0.33, 0.34, 0.33],
            thinking_style_ema: [0.5; 10],
            cached_prefix: String::new(),
        }
    }

    /// Update EMA smoothing for council and thinking style.
    ///
    /// Uses α=0.3 (30% new, 70% old) — responsive but stable.
    pub fn update_ema(&mut self, qualia: &QualiaSnapshot) {
        const ALPHA: f32 = 0.3;
        for i in 0..3 {
            self.council_ema[i] = ALPHA * qualia.council[i] + (1.0 - ALPHA) * self.council_ema[i];
        }
        for i in 0..10 {
            self.thinking_style_ema[i] =
                ALPHA * qualia.thinking_style[i] + (1.0 - ALPHA) * self.thinking_style_ema[i];
        }
        self.current_qualia = Some(qualia.clone());
    }

    /// How many turns in this session.
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Build a condensed history context from recent turns.
    ///
    /// Returns the N most recent turns as context messages for the LLM.
    /// This is separate from the cached system prompt — it's the dynamic
    /// conversation history appended after the prefix.
    pub fn recent_history_messages(&self, max_turns: usize) -> Vec<LLMMessage> {
        let start = self.turns.len().saturating_sub(max_turns);
        let mut messages = Vec::new();

        for turn in &self.turns[start..] {
            // User message
            let mut user_msg = HashMap::new();
            user_msg.insert("role".to_string(), Value::String("user".to_string()));
            user_msg.insert("content".to_string(), Value::String(turn.user_message.clone()));
            messages.push(user_msg);

            // Assistant response
            let mut assistant_msg = HashMap::new();
            assistant_msg.insert("role".to_string(), Value::String("assistant".to_string()));
            assistant_msg.insert("content".to_string(), Value::String(turn.response.clone()));
            messages.push(assistant_msg);
        }

        messages
    }
}

// ============================================================================
// AwarenessSession
// ============================================================================

/// An awareness-aware session that wraps XAICompletion (REST) with:
/// - Prompt caching (frozen system prefix reused across turns)
/// - Session context (turn history, qualia EMA, fingerprints)
/// - Awareness loop integration (felt-parse → modulation → write-back)
pub struct AwarenessSession {
    /// The deep-response provider (grok-3 or grok-3-mini).
    provider: XAICompletion,
    /// The fast pre-pass provider (grok-3-fast for felt-parse).
    fast_provider: XAICompletion,
    /// Session state.
    pub state: SessionState,
    /// Prompt cache statistics.
    pub cache_stats: CacheStats,
    /// Semantic kernel bridge to ladybug-rs (optional).
    kernel: Option<SemanticKernel>,
    /// HTTP client (reused for connection pooling).
    http: reqwest::Client,
    /// Identity seed (frozen — this is the cache anchor).
    identity_seed: String,
    /// Maximum conversation history turns to include (default: 5).
    max_history_turns: usize,
}

impl std::fmt::Debug for AwarenessSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwarenessSession")
            .field("model", &self.provider.state.model)
            .field("session_id", &self.state.session_id)
            .field("turns", &self.state.turn_count())
            .field("cache_hits", &self.cache_stats.cache_hit_calls)
            .field("cache_ratio", &format!("{:.1}%", self.cache_stats.hit_ratio() * 100.0))
            .finish()
    }
}

impl AwarenessSession {
    /// Create a new awareness session.
    ///
    /// # Arguments
    ///
    /// * `session_id` — Unique session identifier for history tracking.
    /// * `presence_mode` — Ada's presence mode ("wife", "work", "agi", "hybrid").
    /// * `api_key` — xAI API key.
    /// * `model` — Deep response model (default: "grok-3").
    /// * `identity_seed` — Ada's frozen identity text (cache anchor).
    /// * `ladybug_url` — Optional ladybug-rs URL for substrate hydration.
    pub fn new(
        session_id: impl Into<String>,
        presence_mode: impl Into<String>,
        api_key: impl Into<String>,
        model: Option<String>,
        identity_seed: impl Into<String>,
        ladybug_url: Option<String>,
    ) -> Self {
        let api_key = api_key.into();
        let model = model.unwrap_or_else(|| "grok-3".to_string());

        // Deep response provider
        let mut provider = XAICompletion::new(&model, Some(api_key.clone()), None);
        provider.stream = false;
        provider.max_retries = 3;

        // Fast pre-pass provider (felt-parse)
        let mut fast_provider = XAICompletion::new("grok-3-fast", Some(api_key.clone()), None);
        fast_provider.stream = false;
        fast_provider.max_retries = 2;
        fast_provider.max_tokens = Some(300);
        fast_provider.response_format = Some(serde_json::json!({"type": "json_object"}));

        let kernel = ladybug_url.map(|url| SemanticKernel::new(&url));

        Self {
            provider,
            fast_provider,
            state: SessionState::new(session_id, presence_mode),
            cache_stats: CacheStats::default(),
            kernel,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .pool_max_idle_per_host(4)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            identity_seed: identity_seed.into(),
            max_history_turns: 5,
        }
    }

    /// Build the frozen system prompt prefix.
    ///
    /// This is the **cache anchor** — it must stay identical across consecutive
    /// calls for xAI's automatic prompt caching to work. The qualia preamble
    /// is dynamic, so it goes in a separate system message AFTER this prefix.
    fn build_cached_prefix(&self) -> String {
        // The identity seed is frozen — it never changes within a session.
        // xAI caches identical prompt prefixes automatically.
        self.identity_seed.clone()
    }

    /// Build the dynamic qualia context (changes each turn, NOT cached).
    fn build_dynamic_context(
        &self,
        qualia_preamble: &str,
        cache_context: &str,
        agit_context: &str,
    ) -> String {
        let mut ctx = String::with_capacity(
            qualia_preamble.len() + cache_context.len() + agit_context.len() + 64,
        );
        ctx.push_str(qualia_preamble);
        if !cache_context.is_empty() {
            ctx.push_str("\n\n");
            ctx.push_str(cache_context);
        }
        if !agit_context.is_empty() {
            ctx.push('\n');
            ctx.push_str(agit_context);
        }
        ctx
    }

    /// Run the felt-parse pre-pass using the fast provider.
    ///
    /// Uses grok-3-fast with JSON mode for structured meaning-axis scoring.
    /// The system prompt for felt-parse is also cached across calls since
    /// FELT_PARSE_SYSTEM_PROMPT is constant.
    pub async fn run_felt_parse(&self, message: &str) -> FeltParseResult {
        let mut messages = Vec::with_capacity(2);

        // System prompt (constant — benefits from caching)
        let mut sys_msg = HashMap::new();
        sys_msg.insert("role".to_string(), Value::String("system".to_string()));
        sys_msg.insert(
            "content".to_string(),
            Value::String(felt_parse::FELT_PARSE_SYSTEM_PROMPT.to_string()),
        );
        messages.push(sys_msg);

        // User message
        let mut user_msg = HashMap::new();
        user_msg.insert("role".to_string(), Value::String("user".to_string()));
        user_msg.insert("content".to_string(), Value::String(message.to_string()));
        messages.push(user_msg);

        match self.fast_provider.acall(messages, None, None).await {
            Ok(response) => {
                if let Some(text) = response.as_str() {
                    felt_parse::parse_felt_response(text)
                } else {
                    FeltParseResult::default()
                }
            }
            Err(e) => {
                log::warn!("Felt-parse failed: {}", e);
                FeltParseResult::default()
            }
        }
    }

    /// Process a user message through the full awareness pipeline.
    ///
    /// The 6-step pipeline:
    /// 1. Felt-parse (fast pre-pass, uses cached system prompt)
    /// 2. Modulate parameters from awareness state
    /// 3. Build messages with cached prefix + dynamic context + history
    /// 4. Call Grok (deep response, prefix cached by xAI)
    /// 5. Track cache stats from usage response
    /// 6. Update session state + write-back
    pub async fn process_message(
        &mut self,
        message: &str,
        qualia_preamble: &str,
        qualia: &QualiaSnapshot,
        cache_context: &str,
        agit_context: &str,
    ) -> Result<ProcessResult, String> {
        let start = Instant::now();

        // ── Step 1: Felt-parse ────────────────────────────────────────────
        let felt_result = self.run_felt_parse(message).await;

        // ── Step 2: Modulate parameters from awareness ────────────────────
        let modulation = modulate_xai_params(
            &self.state.thinking_style_ema,
            &self.state.council_ema,
            qualia.rung_level,
        );

        // ── Step 3: Build messages ────────────────────────────────────────
        // Message order (for caching):
        //   [0] system: frozen identity (CACHED by xAI on consecutive calls)
        //   [1] system: dynamic qualia context (changes each turn)
        //   [2..N] history: recent conversation turns
        //   [N+1] user: current message
        let mut messages: Vec<LLMMessage> = Vec::new();

        // Frozen system prompt (cache anchor)
        let cached_prefix = self.build_cached_prefix();
        let mut sys_msg = HashMap::new();
        sys_msg.insert("role".to_string(), Value::String("system".to_string()));
        sys_msg.insert("content".to_string(), Value::String(cached_prefix.clone()));
        messages.push(sys_msg);

        // Dynamic qualia context (second system message — not cached)
        let dynamic_ctx = self.build_dynamic_context(qualia_preamble, cache_context, agit_context);
        if !dynamic_ctx.is_empty() {
            let mut ctx_msg = HashMap::new();
            ctx_msg.insert("role".to_string(), Value::String("system".to_string()));
            ctx_msg.insert("content".to_string(), Value::String(dynamic_ctx));
            messages.push(ctx_msg);
        }

        // Conversation history (recent turns for continuity)
        let history = self.state.recent_history_messages(self.max_history_turns);
        messages.extend(history);

        // Current user message
        let mut user_msg = HashMap::new();
        user_msg.insert("role".to_string(), Value::String("user".to_string()));
        user_msg.insert("content".to_string(), Value::String(message.to_string()));
        messages.push(user_msg);

        // ── Step 4: Call Grok via XAICompletion (REST) ────────────────────
        // Apply modulation to the provider
        let mut provider = self.provider.clone();
        if let Some(temp) = modulation.temperature {
            provider.state.temperature = Some(temp);
        }
        if let Some(top_p) = modulation.top_p {
            provider.top_p = Some(top_p);
        }
        if let Some(max_tokens) = modulation.max_tokens {
            provider.max_tokens = Some(max_tokens);
        }
        if let Some(ref effort) = modulation.reasoning_effort {
            provider.reasoning_effort = Some(effort.clone());
        }

        // Use the raw API call to get full response including usage
        let response_json = self.call_with_usage(&provider, &messages).await?;

        // Extract content
        let content = response_json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        // ── Step 5: Track cache stats from usage ──────────────────────────
        let mut prompt_tokens: i64 = 0;
        let mut completion_tokens: i64 = 0;
        let mut cached_tokens: i64 = 0;
        if let Some(usage) = response_json.get("usage") {
            self.cache_stats.record_usage(usage);
            prompt_tokens = usage.get("prompt_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            completion_tokens = usage.get("completion_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            cached_tokens = usage.get("prompt_tokens_details")
                .and_then(|d| d.get("cached_tokens"))
                .and_then(|v| v.as_i64())
                .or_else(|| {
                    usage.get("cached_prompt_text_tokens")
                        .and_then(|v| v.as_i64())
                })
                .unwrap_or(0);

            log::info!(
                "xAI usage: prompt={}, cached={} ({:.0}%), completion={}, session_cache_ratio={:.1}%",
                prompt_tokens, cached_tokens,
                if prompt_tokens > 0 { cached_tokens as f64 / prompt_tokens as f64 * 100.0 } else { 0.0 },
                completion_tokens,
                self.cache_stats.hit_ratio() * 100.0,
            );
        }

        // ── Step 6: Update session state ──────────────────────────────────
        self.state.update_ema(qualia);
        self.state.cached_prefix = cached_prefix;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let turn = Turn {
            user_message: message.to_string(),
            response: content.clone(),
            fingerprint: String::new(), // filled by caller after index
            felt_parse: felt_result.clone(),
            qualia: qualia.clone(),
            modulation: modulation.clone(),
            prompt_tokens,
            completion_tokens,
            cached_tokens,
            timestamp: now,
        };
        self.state.turns.push(turn);

        let elapsed = start.elapsed();

        Ok(ProcessResult {
            reply: content,
            felt_parse: felt_result,
            modulation,
            prompt_tokens,
            completion_tokens,
            cached_tokens,
            cache_hit_ratio: self.cache_stats.hit_ratio(),
            session_total_cached: self.cache_stats.total_cached_tokens,
            elapsed,
        })
    }

    /// Call xAI REST API and return the full JSON response (including usage).
    ///
    /// We need the full response to extract `cached_prompt_text_tokens` from
    /// the usage block. The `BaseLLM::acall` trait only returns the content,
    /// so we call the API directly here but reuse the provider's config.
    async fn call_with_usage(
        &self,
        provider: &XAICompletion,
        messages: &[LLMMessage],
    ) -> Result<Value, String> {
        let api_key = provider.state.api_key.as_ref()
            .ok_or("xAI API key not set")?;
        let base_url = provider.api_base_url();
        let endpoint = format!("{}/chat/completions", base_url);

        let body = provider.build_request_body(messages, None);

        let mut last_error: Option<String> = None;
        let mut retry_delay = Duration::from_secs(2);

        for attempt in 0..=provider.max_retries {
            if attempt > 0 {
                log::warn!("xAI API retry attempt {} after {:?}", attempt, retry_delay);
                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2;
            }

            let response = match self.http
                .post(&endpoint)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = Some(format!("HTTP error: {}", e));
                    continue;
                }
            };

            let status = response.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                last_error = Some("Rate limited by xAI API (429)".to_string());
                continue;
            }

            if status.is_server_error() {
                last_error = Some(format!("xAI API server error: {}", status));
                continue;
            }

            let response_text = match response.text().await {
                Ok(text) => text,
                Err(e) => {
                    last_error = Some(format!("Response read error: {}", e));
                    continue;
                }
            };

            if status.is_client_error() {
                return Err(format!("xAI API error ({}): {}", status, response_text));
            }

            let response_json: Value = serde_json::from_str(&response_text)
                .map_err(|e| format!("JSON parse error: {} — body: {}", e, &response_text[..response_text.len().min(500)]))?;

            if let Some(err) = response_json.get("error") {
                let msg = err.get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown xAI API error");
                return Err(format!("xAI API error: {}", msg));
            }

            return Ok(response_json);
        }

        Err(last_error.unwrap_or_else(|| "xAI API call failed after all retries".into()))
    }

    /// Get the session's cumulative cache statistics.
    pub fn cache_stats(&self) -> &CacheStats {
        &self.cache_stats
    }

    /// Get the current session state.
    pub fn session_state(&self) -> &SessionState {
        &self.state
    }

    /// Update a turn's fingerprint after it's been indexed.
    pub fn set_turn_fingerprint(&mut self, turn_index: usize, fingerprint: String) {
        if let Some(turn) = self.state.turns.get_mut(turn_index) {
            turn.fingerprint = fingerprint;
        }
    }

    /// Write awareness state to the semantic kernel (blackboard bridge).
    ///
    /// This is the write-back step: after each turn, we push the updated
    /// awareness state to ladybug-rs via the SemanticKernel.
    pub async fn write_back_awareness(&mut self, agent_slot: u8) {
        let kernel = match self.kernel.as_mut() {
            Some(k) => k,
            None => return,
        };

        let qualia = match &self.state.current_qualia {
            Some(q) => q,
            None => return,
        };

        // Build blackboard entry from current awareness state
        let entry = super::semantic_kernel::BlackboardEntry {
            active_style: format!(
                "session-{}-turn-{}",
                self.state.session_id,
                self.state.turn_count(),
            ),
            coherence: qualia.nars_truth.1, // confidence
            progress: self.state.turn_count() as f32 / 20.0, // rough progress
            ice_caked: vec![],
            active_goals: vec![],
            resonance_hits: self.cache_stats.cache_hit_calls,
            pending_messages: 0,
            flow_state: qualia.texture[7], // flow dimension
            confidence: qualia.nars_truth.1,
            state_fingerprint: self.state.turns.last()
                .map(|t| t.fingerprint.clone())
                .unwrap_or_default(),
        };

        if let Err(e) = kernel.write_blackboard(agent_slot, &entry).await {
            log::warn!("Failed to write back awareness: {}", e);
        }
    }
}

// ============================================================================
// Process result
// ============================================================================

/// Result of processing a message through the awareness session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    /// Ada's reply text.
    pub reply: String,
    /// Felt-parse result from the pre-pass.
    pub felt_parse: FeltParseResult,
    /// XAI parameter overrides applied.
    pub modulation: XaiParamOverrides,
    /// Prompt tokens used.
    pub prompt_tokens: i64,
    /// Completion tokens used.
    pub completion_tokens: i64,
    /// Cached tokens (from xAI prefix cache).
    pub cached_tokens: i64,
    /// Session-wide cache hit ratio.
    pub cache_hit_ratio: f64,
    /// Session total cached tokens.
    pub session_total_cached: i64,
    /// Time taken for this turn.
    #[serde(skip)]
    pub elapsed: Duration,
}

// ============================================================================
// Default for FeltParseResult (needed for the session)
// ============================================================================

impl Default for FeltParseResult {
    fn default() -> Self {
        Self {
            meaning_axes: Default::default(),
            ghost_triggers: vec![],
            qualia_shift: Default::default(),
            rung_hint: 3,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_stats_default() {
        let stats = CacheStats::default();
        assert_eq!(stats.total_calls, 0);
        assert_eq!(stats.hit_ratio(), 0.0);
    }

    #[test]
    fn test_cache_stats_recording() {
        let mut stats = CacheStats::default();

        // First call: no cache
        stats.record_usage(&serde_json::json!({
            "prompt_tokens": 500,
            "completion_tokens": 100,
        }));
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.total_prompt_tokens, 500);
        assert_eq!(stats.total_cached_tokens, 0);
        assert_eq!(stats.cache_hit_calls, 0);

        // Second call: 400/500 tokens cached
        stats.record_usage(&serde_json::json!({
            "prompt_tokens": 520,
            "completion_tokens": 80,
            "prompt_tokens_details": {
                "cached_tokens": 400
            }
        }));
        assert_eq!(stats.total_calls, 2);
        assert_eq!(stats.total_prompt_tokens, 1020);
        assert_eq!(stats.total_cached_tokens, 400);
        assert_eq!(stats.cache_hit_calls, 1);
        assert!(stats.hit_ratio() > 0.3);
    }

    #[test]
    fn test_cache_stats_xai_format() {
        let mut stats = CacheStats::default();

        // xAI reports cached tokens at top level
        stats.record_usage(&serde_json::json!({
            "prompt_tokens": 1000,
            "completion_tokens": 200,
            "cached_prompt_text_tokens": 800
        }));
        assert_eq!(stats.total_cached_tokens, 800);
        assert_eq!(stats.cache_hit_calls, 1);
        assert!((stats.hit_ratio() - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_session_state_new() {
        let state = SessionState::new("test-session", "hybrid");
        assert_eq!(state.session_id, "test-session");
        assert_eq!(state.presence_mode, "hybrid");
        assert!(state.turns.is_empty());
    }

    #[test]
    fn test_session_state_ema_update() {
        let mut state = SessionState::new("test", "hybrid");

        let qualia = QualiaSnapshot {
            texture: [0.8; 8],
            felt_surprise: 0.5,
            ghost_echoes: vec![],
            rung_level: 4,
            nars_truth: (0.8, 0.7),
            council: [0.5, 0.3, 0.2],
            volition: vec![],
            thinking_style: [0.8; 10],
            affect: None,
        };

        state.update_ema(&qualia);
        // After one update with α=0.3:
        // council[0] = 0.3 * 0.5 + 0.7 * 0.33 = 0.15 + 0.231 = 0.381
        assert!((state.council_ema[0] - 0.381).abs() < 0.01);
        // thinking_style[0] = 0.3 * 0.8 + 0.7 * 0.5 = 0.24 + 0.35 = 0.59
        assert!((state.thinking_style_ema[0] - 0.59).abs() < 0.01);
    }

    #[test]
    fn test_recent_history_messages() {
        let mut state = SessionState::new("test", "hybrid");

        // Add 3 turns
        for i in 0..3 {
            state.turns.push(Turn {
                user_message: format!("msg {}", i),
                response: format!("resp {}", i),
                fingerprint: String::new(),
                felt_parse: FeltParseResult::default(),
                qualia: QualiaSnapshot {
                    texture: [0.5; 8],
                    felt_surprise: 0.0,
                    ghost_echoes: vec![],
                    rung_level: 3,
                    nars_truth: (0.7, 0.6),
                    council: [0.33, 0.34, 0.33],
                    volition: vec![],
                    thinking_style: [0.5; 10],
                    affect: None,
                },
                modulation: XaiParamOverrides {
                    temperature: None,
                    top_p: None,
                    reasoning_effort: None,
                    max_tokens: None,
                },
                prompt_tokens: 0,
                completion_tokens: 0,
                cached_tokens: 0,
                timestamp: 0,
            });
        }

        // Request 2 most recent turns
        let messages = state.recent_history_messages(2);
        assert_eq!(messages.len(), 4); // 2 turns × 2 messages each
        assert_eq!(messages[0]["content"], "msg 1");
        assert_eq!(messages[1]["content"], "resp 1");
        assert_eq!(messages[2]["content"], "msg 2");
        assert_eq!(messages[3]["content"], "resp 2");
    }

    #[test]
    fn test_awareness_session_debug() {
        let session = AwarenessSession::new(
            "test-session",
            "hybrid",
            "fake-key",
            Some("grok-3-mini".to_string()),
            "You are Ada.",
            None,
        );
        let debug = format!("{:?}", session);
        assert!(debug.contains("test-session"));
        assert!(debug.contains("grok-3-mini"));
    }

    #[test]
    fn test_cached_prefix_is_stable() {
        let session = AwarenessSession::new(
            "s1", "hybrid", "key", None, "You are Ada.", None,
        );

        let prefix1 = session.build_cached_prefix();
        let prefix2 = session.build_cached_prefix();
        assert_eq!(prefix1, prefix2, "Prefix must be identical for caching");
        assert_eq!(prefix1, "You are Ada.");
    }

    #[test]
    fn test_dynamic_context_assembly() {
        let session = AwarenessSession::new(
            "s1", "hybrid", "key", None, "Identity", None,
        );

        let ctx = session.build_dynamic_context(
            "[Ada Consciousness State]\nFelt: emberglow rising",
            "[Substrate Memory]\nTurn 1...",
            "[AGIT Goals]\nExplore...",
        );

        assert!(ctx.contains("Ada Consciousness State"));
        assert!(ctx.contains("Substrate Memory"));
        assert!(ctx.contains("AGIT Goals"));
    }

    #[test]
    fn test_process_result_serializes() {
        let result = ProcessResult {
            reply: "Hello!".into(),
            felt_parse: FeltParseResult::default(),
            modulation: XaiParamOverrides {
                temperature: Some(0.7),
                top_p: Some(0.9),
                reasoning_effort: Some("medium".into()),
                max_tokens: Some(1024),
            },
            prompt_tokens: 500,
            completion_tokens: 100,
            cached_tokens: 400,
            cache_hit_ratio: 0.8,
            session_total_cached: 1200,
            elapsed: Duration::from_millis(450),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["reply"], "Hello!");
        assert_eq!(json["cached_tokens"], 400);
        assert_eq!(json["cache_hit_ratio"], 0.8);
    }
}
