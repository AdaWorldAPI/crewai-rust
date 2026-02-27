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

pub mod envelope;
pub mod event_recorder;
pub mod pg_store;
pub mod pipeline;
pub mod router;
pub mod subsystem;
pub mod types;

// Ladybug-rs integration modules — only available with the `ladybug` feature.
#[cfg(feature = "ladybug")]
pub mod bridge;
#[cfg(feature = "ladybug")]
pub mod wire_bridge;

pub use envelope::{from_crew_callback, from_memory, from_task_output, to_task_input};
pub use event_recorder::{shared_recorder, ContractRecorder};
pub use pipeline::Pipeline;
pub use router::{StepDomain, StepHandler, StepResult, StepRouter};
pub use subsystem::{Subsystem, SubsystemRegistry};
pub use types::*;

// Re-export the shared substrate types from ladybug-contract (only with feature)
#[cfg(feature = "ladybug")]
pub use ladybug_contract as kernel;

// Unified CogRecord schema constants — the canonical 2×8192 layout.
// Available via `crate::contract::schema` when compiled with ladybug.
#[cfg(feature = "ladybug")]
pub use ladybug_contract::schema;
