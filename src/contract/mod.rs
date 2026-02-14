//! Unified execution contract for crewAI-rust.
//!
//! Implements the shared contract between crewai-rust, n8n-rs, and ladybug-rs.
//! Types are copied from n8n-rs (source of truth) and serialize byte-identically.
//!
//! # Modules
//!
//! - [`types`] — `StepStatus`, `UnifiedStep`, `UnifiedExecution`, `DataEnvelope`,
//!   `StepDelegationRequest`, `StepDelegationResponse`
//! - [`envelope`] — crewAI-specific conversions (task output, memory, callbacks)
//! - [`event_recorder`] — Event bus integration for recording crew/task lifecycle
//! - [`pg_store`] — (feature `postgres`) PostgreSQL persistence
//!
//! # Standalone vs Full Mode
//!
//! Without the `ladybug` feature, crewai-rust works as a standalone Rust port of
//! the Python crewAI framework — agents, tasks, crews, LLM integration all work.
//!
//! With `ladybug` enabled (compiled into the unified ladybug-rs Docker), the
//! binary wire protocol (CogPackets), V1 type bridges, and cognitive substrate
//! integration become available.

pub mod types;
pub mod envelope;
pub mod event_recorder;
pub mod pg_store;

// Ladybug-rs integration modules — only available with the `ladybug` feature.
#[cfg(feature = "ladybug")]
pub mod bridge;
#[cfg(feature = "ladybug")]
pub mod wire_bridge;

pub use types::*;
pub use envelope::{from_task_output, from_memory, from_crew_callback, to_task_input};
pub use event_recorder::{ContractRecorder, shared_recorder};

// Re-export the shared substrate types from ladybug-contract (only with feature)
#[cfg(feature = "ladybug")]
pub use ladybug_contract as kernel;
