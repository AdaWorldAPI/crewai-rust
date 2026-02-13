//! # CrewAI - Rust Port
//!
//! A 1:1 Rust port of the crewAI Python framework for AI agent orchestration.
//! Version 1.9.3
//!
//! CrewAI is a production-grade AI agent orchestration framework with extensive
//! LLM provider support, memory management, RAG capabilities, and event-driven
//! architecture for both crews (autonomous multi-agent systems) and flows
//! (deterministic event-driven workflows).

pub mod a2a;
pub mod agent;
pub mod agents;
pub mod capabilities;
pub mod cli;
pub mod contract;
pub mod context;
pub mod core;
pub mod crew;
pub mod crews;
pub mod events;
pub mod experimental;
pub mod flow;
pub mod hooks;
pub mod interfaces;
pub mod knowledge;
pub mod lite_agent;
pub mod llm;
pub mod llms;
pub mod mcp;
pub mod memory;
pub mod meta_agents;
pub mod modules;
pub mod policy;
pub mod process;
pub mod project;
pub mod rag;
pub mod security;
pub mod server;
pub mod task;
pub mod tasks;
pub mod telemetry;
pub mod tools;
pub mod translations;
pub mod types;
pub mod utilities;

// Re-exports matching Python's __init__.py __all__
pub use agent::Agent;
pub use crew::Crew;
pub use crews::crew_output::CrewOutput;
pub use flow::Flow;
pub use knowledge::Knowledge;
pub use llm::LLM;
pub use llms::base_llm::BaseLLM;
pub use process::Process;
pub use task::Task;
pub use tasks::llm_guardrail::LLMGuardrail;
pub use tasks::task_output::TaskOutput;

// Unified Execution Contract re-exports
pub use contract::types::{
    DataEnvelope, EnvelopeMetadata, StepDelegationRequest, StepDelegationResponse, StepStatus,
    UnifiedExecution, UnifiedStep,
};

/// Library version matching Python crewai 1.9.3
pub const VERSION: &str = "1.9.3";
