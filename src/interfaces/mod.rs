//! # Interface Gateway
//!
//! The interface gateway connects agent capabilities to external systems.
//! It provides a uniform adapter-based architecture where any protocol
//! (REST, GraphQL, RCON, MCP, MS Graph, SSH, etc.) is accessed through
//! a common `InterfaceAdapter` trait.
//!
//! ## Architecture
//!
//! ```text
//! Agent YAML
//!   │ capabilities: [minecraft:server_control, o365:mail]
//!   ▼
//! CapabilityRegistry
//!   │ resolve() → Capability { interface: { protocol: rcon } }
//!   ▼
//! InterfaceGateway
//!   │ bind_capability() → registers adapter + tools
//!   │ invoke() → routes tool call to adapter
//!   ▼
//! InterfaceAdapter (trait)
//!   ├── RestApiAdapter      (OpenAPI endpoints)
//!   ├── GraphqlAdapter      (GraphQL endpoints)
//!   ├── RconAdapter         (Minecraft, Source engine)
//!   ├── MsGraphAdapter      (Microsoft 365: mail, calendar, teams)
//!   ├── McpBridgeAdapter    (MCP servers)
//!   ├── WebSocketAdapter    (WebSocket connections)
//!   ├── SshAdapter          (SSH/SFTP)
//!   ├── DatabaseAdapter     (SQL databases)
//!   └── NativeAdapter       (Local Rust functions)
//! ```
//!
//! ## RBAC Integration
//!
//! Every invocation passes through the PolicyEngine before reaching the adapter:
//!
//! ```text
//! invoke("mc_execute", args)
//!   → PolicyEngine.evaluate(agent, tool, args)
//!     → if Deny: return PolicyViolation
//!     → if Allow: adapter.execute(tool, args)
//! ```
//!
//! ## Extending
//!
//! To add a new protocol:
//! 1. Implement `InterfaceAdapter` for your protocol
//! 2. Register it with `gateway.register_adapter(protocol, adapter)`
//! 3. Create capability YAML files that reference the protocol

pub mod adapter;
pub mod adapters;
pub mod gateway;

pub use adapter::InterfaceAdapter;
pub use gateway::InterfaceGateway;
