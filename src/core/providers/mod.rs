//! Provider interfaces for extensible crewAI components.
//!
//! Corresponds to `crewai/core/providers/`.
//!
//! Provides the trait-based provider pattern used throughout crewAI for
//! extensibility. Each provider defines a protocol that can be swapped
//! out for custom implementations.

pub mod content_processor;
pub mod crew_provider;
pub mod hitl_provider;
pub mod human_input;

pub use content_processor::{ContentProcessorProvider, NoOpContentProcessor};
pub use crew_provider::CrewProvider;
pub use hitl_provider::{ConsoleHITLProvider, HITLProvider};
pub use human_input::{HumanInputProvider, SyncHumanInputProvider};
