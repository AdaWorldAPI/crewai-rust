//! Model Context Protocol (MCP) integration for crewAI.
//!
//! Corresponds to `crewai/mcp/`.
//!
//! This module provides the MCP client, server configuration types,
//! transport layers (Stdio, HTTP, SSE), and tool filtering for connecting
//! agents to MCP-compatible tool servers.
//!
//! MCP allows agents to discover and invoke tools exposed by external
//! servers using a standardized protocol with different transport mechanisms.

pub mod client;
pub mod config;
pub mod filters;
pub mod transports;

// Re-export main types.
pub use client::MCPClient;
pub use config::{MCPServerConfig, MCPServerHTTP, MCPServerSSE, MCPServerStdio};
pub use filters::{StaticToolFilter, ToolFilterContext};
pub use transports::{BaseTransport, TransportType};
