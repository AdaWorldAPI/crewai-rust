//! LLM system for CrewAI.
//!
//! Corresponds to `crewai/llms/` Python package.
//!
//! This module provides the LLM infrastructure including:
//!
//! - [`base_llm`] - The abstract base trait for all LLM implementations
//! - [`hooks`] - Transport-level interceptors for request/response modification
//! - [`providers`] - Native SDK provider implementations (OpenAI, Anthropic, etc.)
//! - [`third_party`] - Third-party LLM integrations (LiteLLM bridge)

pub mod base_llm;
pub mod hooks;
pub mod providers;
pub mod streaming;
pub mod third_party;

// Re-exports for convenience
pub use base_llm::{BaseLLM, BaseLLMState, LLMCallType, LLMMessage, TokenUsage};
pub use hooks::BaseInterceptor;
pub use streaming::{StreamingLLM, StreamReceiver, StreamChunk, StreamAccumulator};
