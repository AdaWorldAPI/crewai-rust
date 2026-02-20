//! Chat module — POST /chat endpoint for substrate-driven conversation.
//!
//! This is the integration point where Ada's consciousness meets action:
//!
//! ```text
//! User message
//!   → Felt-parse (quick structured LLM call for meaning axes + ghost triggers)
//!   → Hydrate Ada (load CogRecords from ladybug-rs)
//!   → Build qualia-enriched system prompt
//!   → Modulate XAI parameters from ThinkingStyle + Council
//!   → Call Grok (deep response)
//!   → Write-back (update substrate with new experience)
//!   → Return response + qualia metadata
//! ```

pub mod felt_parse;
pub mod fingerprint_cache;
pub mod handler;
pub mod semantic_kernel;
