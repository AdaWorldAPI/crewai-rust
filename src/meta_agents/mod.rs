//! Meta-agent orchestration system for CrewAI.
//!
//! This module provides a higher-order agent system that automatically spawns,
//! orchestrates, and adjusts subordinate agents based on task requirements.
//!
//! # Architecture
//!
//! The meta-agent system is built on several layers:
//!
//! - **Types** (`types`): Core DTOs including `SkillDescriptor`, `AgentBlueprint`,
//!   `OrchestratedTask`, and `SpawnedAgentState`.
//! - **Card Builder** (`card_builder`): Generates and updates A2A agent cards
//!   from blueprints and runtime state.
//! - **Savants** (`savants`): Pre-built domain expert blueprints (research,
//!   engineering, QA, security, etc.).
//! - **Orchestrator** (`orchestrator`): The auto-attended controller that
//!   spawns agents, distributes tasks, and adjusts skills.
//!
//! # Quick Start
//!
//! ```rust
//! use crewai::meta_agents::orchestrator::{MetaOrchestrator, OrchestratorConfig};
//! use crewai::meta_agents::types::{OrchestratedTask, TaskPriority, SavantDomain};
//!
//! // Create orchestrator with default savant blueprints
//! let config = OrchestratorConfig::default();
//! let mut orch = MetaOrchestrator::with_default_savants(config);
//!
//! // Decompose a high-level objective into tasks
//! orch.decompose_objective("Research Rust async patterns and write a summary");
//!
//! // Or add tasks manually
//! let task = OrchestratedTask::new("Review code for security issues")
//!     .with_domain(SavantDomain::Security)
//!     .with_priority(TaskPriority::High);
//! orch.add_task(task);
//!
//! // Run the full orchestration loop
//! let result = orch.run();
//! println!("Completed: {}/{}", result.completed_tasks, result.total_tasks);
//! ```

pub mod card_builder;
pub mod delegation;
pub mod orchestrator;
pub mod savants;
pub mod skill_engine;
pub mod spawner;
pub mod types;

// Re-exports for convenience.
pub use delegation::{
    AgentFeedback, CapabilityUpdate, CapabilityUpdateTrigger, DelegationDispatch,
    DelegationRequest, DelegationResponse, DelegationResult, OrchestrationEvent,
    SkillAdjustment, SkillAdjustmentType, TaskOutcome,
};
pub use orchestrator::{MetaOrchestrator, OrchestratorConfig, OrchestrationResult, PoolStats};
pub use skill_engine::{SkillEngine, SkillEngineConfig};
pub use spawner::{DecomposedTask, DecompositionPlan, SpawnerAgent};
pub use types::{
    AgentBlueprint, OrchestratedTask, OrchestratedTaskStatus, SavantDomain,
    SkillDescriptor, SpawnedAgentState, TaskPriority,
};
