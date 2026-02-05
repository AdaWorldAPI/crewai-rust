//! Capability definition — the unit of YAML-importable agent functionality.
//!
//! A capability is a self-contained bundle that declares:
//! - What tools it provides
//! - What interface/protocol it needs to connect
//! - What RBAC roles are required to use it
//! - What policy constraints apply
//!
//! Capabilities are loaded from YAML files in the `capabilities/` directory
//! and resolved by the `CapabilityRegistry`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A capability bundle — the unit of YAML-importable functionality.
///
/// Example YAML:
/// ```yaml
/// capability:
///   id: "minecraft:server_control"
///   version: "1.0.0"
///   description: "Control a Minecraft server via RCON protocol"
///   tags: ["gaming", "server", "rcon"]
///   interface:
///     protocol: "rcon"
///     config:
///       default_port: 25575
///       requires_auth: true
///   tools:
///     - name: "mc_execute"
///       description: "Execute a Minecraft server command"
///       args_schema:
///         command: { type: "string", required: true }
///   policy:
///     requires_roles: ["server_admin"]
///     max_rpm: 30
///     requires_approval_for: ["stop", "op"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// Namespaced identifier: "namespace:name" (e.g., "minecraft:server_control")
    pub id: String,

    /// Semantic version
    pub version: String,

    /// Human-readable description of what this capability provides
    pub description: String,

    /// Searchable tags for capability discovery
    #[serde(default)]
    pub tags: Vec<String>,

    /// Capability metadata (author, license, homepage, etc.)
    #[serde(default)]
    pub metadata: CapabilityMetadata,

    /// Interface specification: what protocol/adapter this capability needs
    pub interface: CapabilityInterface,

    /// Tools provided by this capability
    #[serde(default)]
    pub tools: Vec<CapabilityTool>,

    /// Policy constraints that apply when this capability is active
    #[serde(default)]
    pub policy: CapabilityPolicy,

    /// Optional: other capabilities this one depends on
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Optional: CAM opcode range reserved for this capability's tools
    #[serde(default)]
    pub cam_opcode_range: Option<(u16, u16)>,
}

/// Metadata about the capability (for registry/discovery)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilityMetadata {
    /// Author or maintainer
    #[serde(default)]
    pub author: Option<String>,

    /// License identifier (e.g., "Apache-2.0")
    #[serde(default)]
    pub license: Option<String>,

    /// Homepage or documentation URL
    #[serde(default)]
    pub homepage: Option<String>,

    /// Minimum crewAI-rust version required
    #[serde(default)]
    pub min_crewai_version: Option<String>,

    /// Fingerprint hint for semantic discovery
    #[serde(default)]
    pub fingerprint_hint: Option<String>,
}

/// Interface specification — what adapter/protocol this capability needs.
///
/// The interface gateway uses this to select and configure the appropriate adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityInterface {
    /// Protocol identifier
    pub protocol: InterfaceProtocol,

    /// Protocol-specific configuration
    #[serde(default)]
    pub config: HashMap<String, Value>,

    /// Optional: endpoint URL template (can contain `{variable}` placeholders)
    #[serde(default)]
    pub endpoint_template: Option<String>,

    /// Optional: authentication scheme required
    #[serde(default)]
    pub auth_scheme: Option<String>,
}

/// Supported interface protocols
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceProtocol {
    /// REST API (OpenAPI spec)
    RestApi,
    /// GraphQL endpoint
    Graphql,
    /// gRPC service
    Grpc,
    /// Model Context Protocol (stdio, HTTP, or SSE)
    Mcp,
    /// RCON protocol (Minecraft, Source engine, etc.)
    Rcon,
    /// WebSocket
    Websocket,
    /// Arrow Flight (ladybug-rs native)
    ArrowFlight,
    /// Microsoft Graph API
    MsGraph,
    /// AWS SDK (Bedrock, S3, etc.)
    AwsSdk,
    /// SSH/SFTP
    Ssh,
    /// Database connection (SQL)
    Database,
    /// Native Rust function
    Native,
    /// Custom protocol (adapter must be registered manually)
    Custom(String),
}

/// A tool provided by a capability.
///
/// When the capability is bound to an agent, these become `CrewStructuredTool`
/// instances available in the agent's tool set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityTool {
    /// Tool name (must be unique within the capability)
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Input argument schema
    #[serde(default)]
    pub args_schema: HashMap<String, ToolArgSchema>,

    /// Whether this tool's result should be returned as the final answer
    #[serde(default)]
    pub result_as_answer: bool,

    /// Optional: CAM opcode for BindSpace addressing
    #[serde(default)]
    pub cam_opcode: Option<u16>,

    /// Fingerprint hint for semantic matching
    #[serde(default)]
    pub fingerprint_hint: Option<String>,

    /// Optional: specific RBAC roles required for this tool (beyond capability-level)
    #[serde(default)]
    pub requires_roles: Vec<String>,

    /// Optional: requires human approval before execution
    #[serde(default)]
    pub requires_approval: bool,

    /// Optional: idempotent (safe to retry)
    #[serde(default)]
    pub idempotent: bool,

    /// Optional: read-only (doesn't mutate external state)
    #[serde(default)]
    pub read_only: bool,

    /// Optional: rate limit override for this specific tool
    #[serde(default)]
    pub max_rpm: Option<u32>,
}

/// Schema for a tool argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolArgSchema {
    /// Argument type: "string", "integer", "number", "boolean", "array", "object"
    #[serde(rename = "type")]
    pub arg_type: String,

    /// Whether this argument is required
    #[serde(default)]
    pub required: bool,

    /// Default value (if not required)
    #[serde(default)]
    pub default: Option<Value>,

    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// Enum values (if restricted)
    #[serde(rename = "enum", default)]
    pub enum_values: Option<Vec<Value>>,

    /// For array types: item schema
    #[serde(default)]
    pub items: Option<Box<ToolArgSchema>>,

    /// Validation pattern (regex)
    #[serde(default)]
    pub pattern: Option<String>,
}

/// Policy constraints for a capability
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilityPolicy {
    /// RBAC roles required to use any tool in this capability
    #[serde(default)]
    pub requires_roles: Vec<String>,

    /// Rate limit: max requests per minute across all tools
    #[serde(default)]
    pub max_rpm: Option<u32>,

    /// Specific operations that require human approval
    #[serde(default)]
    pub requires_approval_for: Vec<String>,

    /// Data classification level: "public", "internal", "confidential", "restricted"
    #[serde(default)]
    pub data_classification: Option<String>,

    /// Geographic restrictions (e.g., ["us", "eu"])
    #[serde(default)]
    pub geo_restrictions: Vec<String>,

    /// Audit logging level: "none", "summary", "full"
    #[serde(default)]
    pub audit_level: Option<String>,

    /// Maximum NARS confidence required before external calls
    #[serde(default)]
    pub min_confidence: Option<f64>,

    /// Deny patterns: tool args matching these patterns are blocked
    #[serde(default)]
    pub deny_patterns: Vec<String>,

    /// Custom Cedar policy rules for this capability
    #[serde(default)]
    pub cedar_rules: Vec<String>,
}

impl Capability {
    /// Parse a capability from YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        let wrapper: CapabilityWrapper = serde_yaml::from_str(yaml)?;
        Ok(wrapper.capability)
    }

    /// Parse a capability from a YAML file path.
    pub fn from_yaml_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let cap = Self::from_yaml(&content)?;
        Ok(cap)
    }

    /// Get the namespace from the capability ID (e.g., "minecraft" from "minecraft:server_control")
    pub fn namespace(&self) -> &str {
        self.id.split(':').next().unwrap_or(&self.id)
    }

    /// Get the name from the capability ID (e.g., "server_control" from "minecraft:server_control")
    pub fn name(&self) -> &str {
        self.id.split(':').nth(1).unwrap_or(&self.id)
    }

    /// Check if a given role satisfies this capability's RBAC requirements
    pub fn role_satisfies(&self, roles: &[String]) -> bool {
        if self.policy.requires_roles.is_empty() {
            return true;
        }
        self.policy
            .requires_roles
            .iter()
            .all(|required| roles.contains(required))
    }

    /// Check if a specific tool requires approval
    pub fn tool_requires_approval(&self, tool_name: &str) -> bool {
        self.policy
            .requires_approval_for
            .iter()
            .any(|pattern| tool_name.contains(pattern))
    }

    /// Get all tool names provided by this capability
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name.as_str()).collect()
    }
}

/// Wrapper for YAML deserialization (capability is nested under `capability:` key)
#[derive(Debug, Deserialize)]
struct CapabilityWrapper {
    capability: Capability,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_capability_yaml() {
        let yaml = r#"
capability:
  id: "minecraft:server_control"
  version: "1.0.0"
  description: "Control a Minecraft server via RCON protocol"
  tags:
    - "gaming"
    - "server"
    - "rcon"
  interface:
    protocol: rcon
    config:
      default_port: 25575
      requires_auth: true
  tools:
    - name: "mc_execute"
      description: "Execute a Minecraft server command"
      args_schema:
        command:
          type: "string"
          required: true
  policy:
    requires_roles:
      - "server_admin"
    max_rpm: 30
    requires_approval_for:
      - "stop"
      - "op"
"#;

        let cap = Capability::from_yaml(yaml).unwrap();
        assert_eq!(cap.id, "minecraft:server_control");
        assert_eq!(cap.namespace(), "minecraft");
        assert_eq!(cap.name(), "server_control");
        assert_eq!(cap.tools.len(), 1);
        assert_eq!(cap.tools[0].name, "mc_execute");
        assert_eq!(cap.policy.requires_roles, vec!["server_admin"]);
        assert!(cap.tool_requires_approval("stop_server"));
        assert!(!cap.tool_requires_approval("list_players"));
        assert!(!cap.role_satisfies(&[]));
        assert!(cap.role_satisfies(&["server_admin".to_string()]));
    }

    #[test]
    fn test_namespace_and_name() {
        let yaml = r#"
capability:
  id: "o365:mail_reader"
  version: "1.0.0"
  description: "Read emails from Microsoft 365"
  interface:
    protocol: ms_graph
"#;

        let cap = Capability::from_yaml(yaml).unwrap();
        assert_eq!(cap.namespace(), "o365");
        assert_eq!(cap.name(), "mail_reader");
    }
}
