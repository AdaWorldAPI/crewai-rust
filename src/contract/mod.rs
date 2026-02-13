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

pub mod types;
pub mod envelope;
pub mod event_recorder;
pub mod pg_store;
pub mod bridge;

pub use types::*;
pub use envelope::{from_task_output, from_memory, from_crew_callback, to_task_input};
pub use event_recorder::{ContractRecorder, shared_recorder};

// Re-export the shared substrate types from ladybug-contract
pub use ladybug_contract as kernel;
