//! Type definitions for A2A protocol message parts.
//!
//! Corresponds to `crewai/a2a/types.py`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Transport protocol type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportType {
    JSONRPC,
    GRPC,
    #[serde(rename = "HTTP+JSON")]
    HttpJson,
}

impl Default for TransportType {
    fn default() -> Self {
        Self::JSONRPC
    }
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONRPC => write!(f, "JSONRPC"),
            Self::GRPC => write!(f, "GRPC"),
            Self::HttpJson => write!(f, "HTTP+JSON"),
        }
    }
}

/// A2A protocol version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolVersion {
    #[serde(rename = "0.2.0")]
    V0_2_0,
    #[serde(rename = "0.2.1")]
    V0_2_1,
    #[serde(rename = "0.2.2")]
    V0_2_2,
    #[serde(rename = "0.2.3")]
    V0_2_3,
    #[serde(rename = "0.2.4")]
    V0_2_4,
    #[serde(rename = "0.2.5")]
    V0_2_5,
    #[serde(rename = "0.2.6")]
    V0_2_6,
    #[serde(rename = "0.3.0")]
    V0_3_0,
    #[serde(rename = "0.4.0")]
    V0_4_0,
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::V0_3_0
    }
}

/// Protocol for the dynamically created AgentResponse model.
pub trait AgentResponseProtocol {
    fn a2a_ids(&self) -> &[String];
    fn message(&self) -> &str;
    fn is_a2a(&self) -> bool;
}

/// Metadata for A2A message parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartsMetadata {
    /// MIME type for the part content.
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// JSON schema for the part content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<HashMap<String, Value>>,
}

/// A2A message part containing text and optional metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartsDict {
    /// The text content of the message part.
    pub text: String,
    /// Optional metadata describing the part content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<PartsMetadata>,
}

/// URL type (validated as HTTP URL in the Python version).
pub type Url = String;
