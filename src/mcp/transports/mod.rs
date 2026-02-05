//! Transport layer implementations for MCP connections.
//!
//! Corresponds to `crewai/mcp/transports/`.
//!
//! This module provides the transport abstraction and concrete implementations
//! for communicating with MCP servers over different protocols:
//!
//! - **Stdio** (`StdioTransport`): Connects to local MCP servers running as
//!   child processes, communicating via stdin/stdout.
//! - **HTTP** (`HTTPTransport`): Connects to remote MCP servers over HTTP/HTTPS,
//!   optionally using streamable HTTP transport.
//! - **SSE** (`SSETransport`): Connects to remote MCP servers using Server-Sent
//!   Events for real-time streaming communication.
//!
//! All transports implement the `BaseTransport` trait, which defines the common
//! interface for connection management. The `TransportType` enum identifies
//! the type of transport being used.

pub mod http;
pub mod sse;
pub mod stdio;

use async_trait::async_trait;

pub use http::HTTPTransport;
pub use sse::SSETransport;
pub use stdio::StdioTransport;

// ---------------------------------------------------------------------------
// TransportType
// ---------------------------------------------------------------------------

/// MCP transport types.
///
/// Identifies the protocol used by a transport implementation.
///
/// Corresponds to `crewai.mcp.transports.base.TransportType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportType {
    /// Standard I/O transport (local child process).
    Stdio,
    /// HTTP transport (non-streaming).
    Http,
    /// Streamable HTTP transport.
    StreamableHttp,
    /// Server-Sent Events transport.
    Sse,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportType::Stdio => write!(f, "stdio"),
            TransportType::Http => write!(f, "http"),
            TransportType::StreamableHttp => write!(f, "streamable-http"),
            TransportType::Sse => write!(f, "sse"),
        }
    }
}

impl TransportType {
    /// Get the string value of the transport type.
    ///
    /// Returns the same string as `Display`, matching the Python
    /// `TransportType(str, Enum)` `.value` attribute.
    pub fn value(&self) -> &str {
        match self {
            TransportType::Stdio => "stdio",
            TransportType::Http => "http",
            TransportType::StreamableHttp => "streamable-http",
            TransportType::Sse => "sse",
        }
    }

    /// Parse a transport type from a string.
    ///
    /// # Arguments
    ///
    /// * `s` - String to parse (case-insensitive).
    ///
    /// # Returns
    ///
    /// The matching `TransportType`, or `None` if the string is not recognized.
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "stdio" => Some(TransportType::Stdio),
            "http" => Some(TransportType::Http),
            "streamable-http" | "streamable_http" => Some(TransportType::StreamableHttp),
            "sse" => Some(TransportType::Sse),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// BaseTransport
// ---------------------------------------------------------------------------

/// Base trait for MCP transport implementations.
///
/// Defines the interface that all transport implementations must follow.
/// Transports handle the low-level communication with MCP servers,
/// including connection establishment, disconnection, and stream management.
///
/// In the Python implementation, `BaseTransport` also manages read/write
/// streams and acts as an async context manager. In Rust, the stream
/// management is handled internally by each transport implementation.
///
/// Corresponds to `crewai.mcp.transports.base.BaseTransport`.
#[async_trait]
pub trait BaseTransport: Send + Sync {
    /// Return the transport type.
    ///
    /// Corresponds to `BaseTransport.transport_type` property in Python.
    fn transport_type(&self) -> TransportType;

    /// Check if transport is currently connected.
    ///
    /// Returns `true` if the transport has an active connection to the
    /// MCP server.
    ///
    /// Corresponds to `BaseTransport.connected` property in Python.
    fn connected(&self) -> bool;

    /// Establish connection to the MCP server.
    ///
    /// Sets up the underlying communication channel (process, HTTP connection,
    /// or SSE stream) and prepares the transport for sending/receiving messages.
    ///
    /// If the transport is already connected, this should be a no-op.
    ///
    /// # Errors
    ///
    /// * Connection failures (server unreachable, authentication errors, etc.).
    /// * MCP SDK not available (dependency not installed).
    ///
    /// Corresponds to `BaseTransport.connect()` in Python.
    async fn connect(&mut self) -> Result<(), anyhow::Error>;

    /// Close connection to the MCP server.
    ///
    /// Cleans up the underlying communication channel and releases resources.
    /// If the transport is not connected, this should be a no-op.
    ///
    /// # Errors
    ///
    /// * Clean-up failures (process termination errors, etc.).
    ///
    /// Corresponds to `BaseTransport.disconnect()` in Python.
    async fn disconnect(&mut self) -> Result<(), anyhow::Error>;

    /// Return a string identifier for this server.
    ///
    /// Used for caching, logging, and event emission. The format
    /// depends on the transport type:
    /// - Stdio: `"stdio:{command}:{arg1}:{arg2}:..."`
    /// - HTTP: `"http:{url}"`
    /// - SSE: `"sse:{url}"`
    fn server_identifier(&self) -> String;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_type_display() {
        assert_eq!(TransportType::Stdio.to_string(), "stdio");
        assert_eq!(TransportType::Http.to_string(), "http");
        assert_eq!(TransportType::StreamableHttp.to_string(), "streamable-http");
        assert_eq!(TransportType::Sse.to_string(), "sse");
    }

    #[test]
    fn test_transport_type_value() {
        assert_eq!(TransportType::Stdio.value(), "stdio");
        assert_eq!(TransportType::Http.value(), "http");
        assert_eq!(TransportType::StreamableHttp.value(), "streamable-http");
        assert_eq!(TransportType::Sse.value(), "sse");
    }

    #[test]
    fn test_transport_type_from_str() {
        assert_eq!(TransportType::from_str_opt("stdio"), Some(TransportType::Stdio));
        assert_eq!(TransportType::from_str_opt("http"), Some(TransportType::Http));
        assert_eq!(TransportType::from_str_opt("streamable-http"), Some(TransportType::StreamableHttp));
        assert_eq!(TransportType::from_str_opt("streamable_http"), Some(TransportType::StreamableHttp));
        assert_eq!(TransportType::from_str_opt("sse"), Some(TransportType::Sse));
        assert_eq!(TransportType::from_str_opt("unknown"), None);
    }

    #[test]
    fn test_transport_type_from_str_case_insensitive() {
        assert_eq!(TransportType::from_str_opt("STDIO"), Some(TransportType::Stdio));
        assert_eq!(TransportType::from_str_opt("Http"), Some(TransportType::Http));
        assert_eq!(TransportType::from_str_opt("SSE"), Some(TransportType::Sse));
    }

    #[test]
    fn test_transport_type_equality() {
        assert_eq!(TransportType::Stdio, TransportType::Stdio);
        assert_ne!(TransportType::Stdio, TransportType::Http);
    }

    #[test]
    fn test_transport_type_clone() {
        let t = TransportType::StreamableHttp;
        let t2 = t;
        assert_eq!(t, t2);
    }

    #[test]
    fn test_transport_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(TransportType::Stdio);
        set.insert(TransportType::Http);
        set.insert(TransportType::Sse);
        set.insert(TransportType::StreamableHttp);
        assert_eq!(set.len(), 4);

        // Inserting a duplicate should not increase the set size.
        set.insert(TransportType::Stdio);
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn test_stdio_transport_basic() {
        let transport = StdioTransport::new("echo", None, None);
        assert_eq!(transport.transport_type(), TransportType::Stdio);
        assert!(!transport.connected());
        assert!(transport.server_identifier().starts_with("stdio:echo"));
    }

    #[test]
    fn test_http_transport_basic() {
        let transport = HTTPTransport::new("https://example.com/mcp", None, None);
        assert!(!transport.connected());
        assert!(transport.server_identifier().starts_with("http:"));
        assert!(transport.server_identifier().contains("example.com"));
    }

    #[test]
    fn test_http_transport_streamable() {
        let transport_streamable = HTTPTransport::new("https://example.com/mcp", None, Some(true));
        assert_eq!(transport_streamable.transport_type(), TransportType::StreamableHttp);

        let transport_plain = HTTPTransport::new("https://example.com/mcp", None, Some(false));
        assert_eq!(transport_plain.transport_type(), TransportType::Http);
    }

    #[test]
    fn test_sse_transport_basic() {
        let transport = SSETransport::new("https://example.com/sse", None);
        assert_eq!(transport.transport_type(), TransportType::Sse);
        assert!(!transport.connected());
        assert!(transport.server_identifier().starts_with("sse:"));
    }
}
