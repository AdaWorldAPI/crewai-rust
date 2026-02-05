//! Stdio transport for MCP servers running as local processes.
//!
//! Port of crewai/mcp/transports/stdio.py

use std::collections::HashMap;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::{Child, Command};

use crate::mcp::transports::{BaseTransport, TransportType};

/// Stdio transport for connecting to local MCP servers.
///
/// Connects to MCP servers running as local processes, communicating
/// via standard input/output streams. Supports Python, Node.js, and
/// other command-line servers.
pub struct StdioTransport {
    /// Command to execute (e.g., "python", "node", "npx").
    pub command: String,
    /// Command arguments (e.g., vec!["server.py"] or vec!["-y", "@mcp/server"]).
    pub args: Vec<String>,
    /// Environment variables to pass to the process.
    pub env: HashMap<String, String>,
    /// Whether the transport is currently connected.
    is_connected: bool,
    /// The child process handle.
    process: Option<Child>,
}

impl StdioTransport {
    /// Create a new StdioTransport.
    ///
    /// # Arguments
    /// * `command` - Command to execute.
    /// * `args` - Command arguments.
    /// * `env` - Environment variables.
    pub fn new(
        command: &str,
        args: Option<Vec<String>>,
        env: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            command: command.to_string(),
            args: args.unwrap_or_default(),
            env: env.unwrap_or_default(),
            is_connected: false,
            process: None,
        }
    }
}

#[async_trait]
impl BaseTransport for StdioTransport {
    fn transport_type(&self) -> TransportType {
        TransportType::Stdio
    }

    fn connected(&self) -> bool {
        self.is_connected
    }

    async fn connect(&mut self) -> Result<(), anyhow::Error> {
        if self.is_connected {
            return Ok(());
        }

        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Merge environment variables
        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        let child = cmd.spawn().map_err(|e| {
            anyhow::anyhow!(
                "Failed to start MCP server process '{}': {}",
                self.command,
                e
            )
        })?;

        self.process = Some(child);
        self.is_connected = true;

        log::info!(
            "Stdio transport connected: {} {}",
            self.command,
            self.args.join(" ")
        );

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), anyhow::Error> {
        if !self.is_connected {
            return Ok(());
        }

        if let Some(ref mut process) = self.process {
            // Try graceful termination first
            let _ = process.kill().await;
        }

        self.process = None;
        self.is_connected = false;

        log::info!(
            "Stdio transport disconnected: {} {}",
            self.command,
            self.args.join(" ")
        );

        Ok(())
    }

    fn server_identifier(&self) -> String {
        format!("stdio:{}:{}", self.command, self.args.join(":"))
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        if let Some(ref mut process) = self.process {
            // Best-effort kill on drop
            let _ = process.start_kill();
        }
    }
}
