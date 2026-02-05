//! A2A configuration types.
//!
//! Corresponds to `crewai/a2a/config.py`.

use serde::{Deserialize, Serialize};

use crate::a2a::types::{ProtocolVersion, TransportType};

// ---------------------------------------------------------------------------
// Signing
// ---------------------------------------------------------------------------

/// Signing algorithm for AgentCard JWS signing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SigningAlgorithm {
    RS256,
    RS384,
    RS512,
    ES256,
    ES384,
    ES512,
    PS256,
    PS384,
    PS512,
}

impl Default for SigningAlgorithm {
    fn default() -> Self {
        Self::RS256
    }
}

/// Configuration for AgentCard JWS signing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCardSigningConfig {
    /// Path to a PEM-encoded private key file.
    pub private_key_path: Option<String>,
    /// PEM-encoded private key as a string.
    pub private_key_pem: Option<String>,
    /// Optional key identifier for the JWS header (kid claim).
    pub key_id: Option<String>,
    /// Signing algorithm.
    #[serde(default)]
    pub algorithm: SigningAlgorithm,
}

// ---------------------------------------------------------------------------
// Transport configs
// ---------------------------------------------------------------------------

/// gRPC server transport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GRPCServerConfig {
    #[serde(default = "default_grpc_host")]
    pub host: String,
    #[serde(default = "default_grpc_port")]
    pub port: u16,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    #[serde(default = "default_max_workers")]
    pub max_workers: usize,
    #[serde(default)]
    pub reflection_enabled: bool,
}

fn default_grpc_host() -> String { "localhost".to_string() }
fn default_grpc_port() -> u16 { 50051 }
fn default_max_workers() -> usize { 10 }

impl Default for GRPCServerConfig {
    fn default() -> Self {
        Self {
            host: default_grpc_host(),
            port: default_grpc_port(),
            tls_cert_path: None,
            tls_key_path: None,
            max_workers: default_max_workers(),
            reflection_enabled: false,
        }
    }
}

/// gRPC client transport configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GRPCClientConfig {
    pub max_send_message_length: Option<usize>,
    pub max_receive_message_length: Option<usize>,
    pub keepalive_time_ms: Option<u64>,
    pub keepalive_timeout_ms: Option<u64>,
}

/// JSON-RPC server transport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONRPCServerConfig {
    #[serde(default = "default_rpc_path")]
    pub rpc_path: String,
    #[serde(default = "default_agent_card_path")]
    pub agent_card_path: String,
}

fn default_rpc_path() -> String { "/a2a".to_string() }
fn default_agent_card_path() -> String { "/.well-known/agent-card.json".to_string() }

impl Default for JSONRPCServerConfig {
    fn default() -> Self {
        Self {
            rpc_path: default_rpc_path(),
            agent_card_path: default_agent_card_path(),
        }
    }
}

/// JSON-RPC client transport configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JSONRPCClientConfig {
    pub max_request_size: Option<usize>,
}

/// HTTP+JSON transport configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HTTPJSONConfig {}

/// Configuration for outgoing webhook push notifications.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerPushNotificationConfig {
    pub signature_secret: Option<String>,
}

/// Transport configuration for A2A server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerTransportConfig {
    #[serde(default)]
    pub preferred: TransportType,
    #[serde(default)]
    pub jsonrpc: JSONRPCServerConfig,
    pub grpc: Option<GRPCServerConfig>,
    pub http_json: Option<HTTPJSONConfig>,
}

impl Default for ServerTransportConfig {
    fn default() -> Self {
        Self {
            preferred: TransportType::JSONRPC,
            jsonrpc: JSONRPCServerConfig::default(),
            grpc: None,
            http_json: None,
        }
    }
}

/// Transport configuration for A2A client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientTransportConfig {
    pub preferred: Option<TransportType>,
    #[serde(default = "default_supported_transports")]
    pub supported: Vec<TransportType>,
    #[serde(default)]
    pub jsonrpc: JSONRPCClientConfig,
    #[serde(default)]
    pub grpc: GRPCClientConfig,
}

fn default_supported_transports() -> Vec<TransportType> {
    vec![TransportType::JSONRPC]
}

impl Default for ClientTransportConfig {
    fn default() -> Self {
        Self {
            preferred: None,
            supported: default_supported_transports(),
            jsonrpc: JSONRPCClientConfig::default(),
            grpc: GRPCClientConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// A2A config types
// ---------------------------------------------------------------------------

/// Configuration for connecting to remote A2A agents (client side).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AClientConfig {
    /// A2A agent endpoint URL.
    pub endpoint: String,
    /// Authentication scheme name (resolved at runtime).
    pub auth: Option<String>,
    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// Maximum conversation turns with A2A agent.
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    /// If `true`, raise error when agent unreachable; if `false`, skip.
    #[serde(default = "default_true")]
    pub fail_fast: bool,
    /// If `true`, return A2A result directly when completed.
    #[serde(default)]
    pub trust_remote_completion_status: bool,
    /// Media types the client can accept in responses.
    #[serde(default = "default_accepted_output_modes")]
    pub accepted_output_modes: Vec<String>,
    /// Extension URIs the client supports.
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Transport configuration.
    #[serde(default)]
    pub transport: ClientTransportConfig,
}

fn default_timeout() -> u64 { 120 }
fn default_max_turns() -> u32 { 10 }
fn default_true() -> bool { true }
fn default_accepted_output_modes() -> Vec<String> { vec!["application/json".to_string()] }

/// Configuration for exposing a Crew or Agent as an A2A server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AServerConfig {
    /// Human-readable name for the agent.
    pub name: Option<String>,
    /// Human-readable description of the agent.
    pub description: Option<String>,
    /// Version string for the agent card.
    #[serde(default = "default_version")]
    pub version: String,
    /// Default supported input MIME types.
    #[serde(default = "default_mime_types")]
    pub default_input_modes: Vec<String>,
    /// Default supported output MIME types.
    #[serde(default = "default_mime_types")]
    pub default_output_modes: Vec<String>,
    /// A2A protocol version this agent supports.
    #[serde(default)]
    pub protocol_version: ProtocolVersion,
    /// URL to the agent's documentation.
    pub documentation_url: Option<String>,
    /// URL to an icon for the agent.
    pub icon_url: Option<String>,
    /// Preferred endpoint URL for the agent.
    pub url: Option<String>,
    /// Configuration for signing the AgentCard with JWS.
    pub signing_config: Option<AgentCardSigningConfig>,
    /// Configuration for outgoing push notifications.
    pub push_notifications: Option<ServerPushNotificationConfig>,
    /// Transport configuration.
    #[serde(default)]
    pub transport: ServerTransportConfig,
    /// Authentication scheme for A2A endpoints.
    pub auth: Option<String>,
}

fn default_version() -> String { "1.0.0".to_string() }
fn default_mime_types() -> Vec<String> {
    vec!["text/plain".to_string(), "application/json".to_string()]
}

/// Legacy A2A config (deprecated, use `A2AClientConfig` instead).
#[deprecated(note = "Use A2AClientConfig or A2AServerConfig instead")]
pub type A2AConfig = A2AClientConfig;
