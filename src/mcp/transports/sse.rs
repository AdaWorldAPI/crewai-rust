//! Server-Sent Events (SSE) transport for MCP servers.
//!
//! Port of crewai/mcp/transports/sse.py

use std::collections::HashMap;

use async_trait::async_trait;

use crate::mcp::transports::{BaseTransport, TransportType};

/// SSE transport for connecting to remote MCP servers.
///
/// Connects to MCP servers using Server-Sent Events for
/// real-time streaming communication.
pub struct SSETransport {
    /// Server URL (e.g., "https://api.example.com/mcp/sse").
    pub url: String,
    /// Optional HTTP headers.
    pub headers: HashMap<String, String>,
    /// Whether the transport is currently connected.
    is_connected: bool,
}

impl SSETransport {
    /// Create a new SSETransport.
    ///
    /// # Arguments
    /// * `url` - Server URL.
    /// * `headers` - Optional HTTP headers.
    pub fn new(url: &str, headers: Option<HashMap<String, String>>) -> Self {
        Self {
            url: url.to_string(),
            headers: headers.unwrap_or_default(),
            is_connected: false,
        }
    }
}

#[async_trait]
impl BaseTransport for SSETransport {
    fn transport_type(&self) -> TransportType {
        TransportType::Sse
    }

    fn connected(&self) -> bool {
        self.is_connected
    }

    async fn connect(&mut self) -> Result<(), anyhow::Error> {
        if self.is_connected {
            return Ok(());
        }

        // TODO: Integrate with actual MCP SDK SSE client
        // For now, mark as connected. The actual SSE connection
        // will be established when the MCP SDK is integrated.
        log::info!("SSE transport connecting to: {}", self.url);

        self.is_connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), anyhow::Error> {
        if !self.is_connected {
            return Ok(());
        }

        log::info!("SSE transport disconnecting from: {}", self.url);

        self.is_connected = false;
        Ok(())
    }

    fn server_identifier(&self) -> String {
        format!("sse:{}", self.url)
    }
}
