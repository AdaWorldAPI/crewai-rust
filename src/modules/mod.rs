//! Module system — YAML-declared deployable agent packages.
//!
//! A **module** bundles a cognitive profile, external interfaces, knowledge
//! sources, RBAC policy, and skills into a single YAML file.  Drop a YAML in
//! `modules/` and get a fully configured, cognitively gated, RBAC-enforced
//! agent with external tool bindings.
//!
//! # Architecture
//!
//! ```text
//! YAML file
//!   ↓  ModuleLoader::load_file()
//! ModuleInstance (resolved: blueprint + capabilities + gate)
//!   ↓  ModuleRuntime::activate_module()
//! Live agent (SavantCoordinator) + bound interfaces (InterfaceGateway)
//!   + cognitive gate (pre-tool-call) + thinking style (enrichment)
//! ```
//!
//! # Example
//!
//! ```rust
//! use crewai::modules::{ModuleLoader, ModuleRuntime};
//!
//! let mut loader = ModuleLoader::new();
//! // let instance = loader.load_file("modules/soc_incident_response.yaml").unwrap();
//! // let mut runtime = ModuleRuntime::new("anthropic/claude-sonnet-4-20250514");
//! // let agent_id = runtime.activate_module(instance).unwrap();
//! ```

pub mod error;
pub mod loader;
pub mod module_def;
pub mod openapi_parser;
pub mod runtime;

// Re-exports
pub use error::ModuleError;
pub use loader::{ModuleInstance, ModuleLoader};
pub use module_def::{
    CollapseGateConfig, InterfaceAuth, KnowledgeSource, ModuleAgentConfig, ModuleDef,
    ModuleInner, ModuleInterface, ModulePolicy, PersonaProfile, SelfModifyBounds, ToolOverride,
    ToolPolicy,
};
// PersonaProfile is re-exported from the persona module directly
pub use openapi_parser::{parse_openapi_file, parse_openapi_spec};
pub use runtime::{
    AgentState, CognitiveGate, GateDecision, InnerThoughtHook, ModuleRuntime, ResonanceConfig,
};
