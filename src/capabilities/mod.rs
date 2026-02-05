//! # Capability Registry
//!
//! Provides a YAML-importable capability system where agents reference named
//! capability bundles. Each capability defines a set of tools, required
//! interfaces, policy constraints, and RBAC roles.
//!
//! ## Architecture
//!
//! Capabilities are the bridge between the agent card (YAML) and the interface
//! gateway (runtime). When an agent card declares:
//!
//! ```yaml
//! capabilities:
//!   - minecraft:server_control
//!   - o365:mail_reader
//! ```
//!
//! The capability registry resolves each identifier to a `Capability` struct
//! that describes the tools, interfaces, and access policies needed.
//! The interface gateway then connects the actual adapters at runtime.
//!
//! ## Capability Resolution Flow
//!
//! 1. Agent YAML declares `capabilities: [minecraft:server_control]`
//! 2. `CapabilityRegistry::resolve("minecraft:server_control")` returns the `Capability`
//! 3. `InterfaceGateway::bind_capability(cap)` connects the adapter and registers tools
//! 4. `PolicyEngine::load_capability_policy(cap)` installs RBAC rules
//! 5. Agent now has access to the tools, gated by policy

pub mod capability;
pub mod registry;

pub use capability::{
    Capability, CapabilityInterface, CapabilityMetadata, CapabilityPolicy, CapabilityTool,
    InterfaceProtocol, ToolArgSchema,
};
pub use registry::CapabilityRegistry;
