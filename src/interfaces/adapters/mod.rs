//! Built-in interface adapters.
//!
//! Each adapter implements `InterfaceAdapter` for a specific protocol.
//! New protocols can be added by implementing the trait and registering
//! the factory with the `InterfaceGateway`.

pub mod graph_api;
pub mod mcp_bridge;
pub mod rcon;
pub mod rest_api;
