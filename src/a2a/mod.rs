//! A2A (Agent-to-Agent) protocol module.
//!
//! Corresponds to `crewai/a2a/`.
//!
//! Provides configuration, type definitions, error codes, wrapper logic,
//! authentication schemes, extensions, update mechanisms, and utilities
//! for the A2A protocol integration.

pub mod auth;
pub mod client;
pub mod config;
pub mod errors;
pub mod extensions;
pub mod types;
pub mod updates;
pub mod utils;
pub mod wrapper;

// Re-exports used by submodules
pub use types::{PartsDict as Part, ProtocolVersion, TransportType};
