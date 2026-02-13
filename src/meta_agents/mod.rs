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
//! - **DTO Registry** (`dto_meta`): Centralized type registry with typed envelopes,
//!   schema validation, and cross-agent type compatibility checking.
//! - **Card Builder** (`card_builder`): Generates and updates A2A agent cards
//!   from blueprints and runtime state.
//! - **Savants** (`savants`): Pre-built domain expert blueprints for all 9 domains
//!   (research, engineering, data analysis, content, planning, QA, security,
//!   DevOps, design).
//! - **Savant Meta** (`savant_meta`): Higher-order `SavantCoordinator` that manages
//!   domain savants with skill-aware routing, cross-domain delegation, and
//!   dynamic A2A card synchronization.
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
//!
//! # Savant Coordinator
//!
//! ```rust
//! use crewai::meta_agents::savant_meta::SavantCoordinator;
//! use crewai::meta_agents::types::{OrchestratedTask, SavantDomain};
//!
//! let mut coord = SavantCoordinator::new("openai/gpt-4o");
//! coord.spawn_all_domains(); // 9 domain savants
//!
//! let mut task = OrchestratedTask::new("Audit code for security")
//!     .with_domain(SavantDomain::Security);
//! let result = coord.execute_task(&mut task);
//! ```
//!
//! # DTO Registry
//!
//! ```rust
//! use crewai::meta_agents::dto_meta::{DtoRegistry, DtoEnvelope};
//! use crewai::meta_agents::types::OrchestratedTask;
//!
//! let mut registry = DtoRegistry::new(); // Pre-loaded with built-in schemas
//! let task = OrchestratedTask::new("My task");
//! let envelope = registry.wrap_task(&task); // Typed, validated envelope
//! ```

pub mod card_builder;
pub mod delegation;
pub mod dto_meta;
pub mod orchestrator;
pub mod savant_meta;
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
pub use dto_meta::{DtoContentType, DtoEnvelope, DtoRegistry, DtoSchema, SchemaVersion, ValidationResult};
pub use orchestrator::{MetaOrchestrator, OrchestratorConfig, OrchestrationResult, PoolStats};
pub use savant_meta::{CrossDomainDelegation, RoutingDecision, SavantCoordinator, SavantEntry};
pub use skill_engine::{SkillEngine, SkillEngineConfig};
pub use spawner::{DecomposedTask, DecompositionPlan, SpawnerAgent};
pub use types::{
    AgentBlueprint, OrchestratedTask, OrchestratedTaskStatus, SavantDomain,
    SkillDescriptor, SpawnedAgentState, TaskPriority,
};
