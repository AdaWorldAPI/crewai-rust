//! HTTP and Streamable HTTP transport for MCP servers.
//!
//! Port of crewai/mcp/transports/http.py

use std::collections::HashMap;

use async_trait::async_trait;

use crate::mcp::transports::{BaseTransport, TransportType};

/// HTTP/Streamable HTTP transport for connecting to remote MCP servers.
///
/// Connects to MCP servers over HTTP/HTTPS using the streamable HTTP protocol.
pub struct HTTPTransport {
    /// Server URL (e.g., "https://api.example.com/mcp").
    pub url: String,
    /// Optional HTTP headers.
    pub headers: HashMap<String, String>,
    /// Whether to use streamable HTTP (default: true).
    pub streamable: bool,
    /// Whether the transport is currently connected.
    is_connected: bool,
}

impl HTTPTransport {
    /// Create a new HTTPTransport.
    ///
    /// # Arguments
    /// * `url` - Server URL.
    /// * `headers` - Optional HTTP headers.
    /// * `streamable` - Whether to use streamable HTTP (default: true).
    pub fn new(
        url: &str,
        headers: Option<HashMap<String, String>>,
        streamable: Option<bool>,
    ) -> Self {
        Self {
            url: url.to_string(),
            headers: headers.unwrap_or_default(),
            streamable: streamable.unwrap_or(true),
            is_connected: false,
        }
    }
}

#[async_trait]
impl BaseTransport for HTTPTransport {
    fn transport_type(&self) -> TransportType {
        if self.streamable {
            TransportType::StreamableHttp
        } else {
            TransportType::Http
        }
    }

    fn connected(&self) -> bool {
        self.is_connected
    }

    async fn connect(&mut self) -> Result<(), anyhow::Error> {
        if self.is_connected {
            return Ok(());
        }

        // TODO: Integrate with actual MCP SDK HTTP client
        // For now, mark as connected. The actual HTTP connection
        // will be established when the MCP SDK is integrated.
        log::info!(
            "HTTP transport connecting to: {} (streamable={})",
            self.url,
            self.streamable
        );

        self.is_connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), anyhow::Error> {
        if !self.is_connected {
            return Ok(());
        }

        log::info!("HTTP transport disconnecting from: {}", self.url);

        self.is_connected = false;
        Ok(())
    }

    fn server_identifier(&self) -> String {
        format!("http:{}", self.url)
    }
}
