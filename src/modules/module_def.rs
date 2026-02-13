//! Module definition types — the YAML schema for deployable agent packages.
//!
//! A `ModuleDef` is pure data: it describes *what* an agent is and *how* it
//! connects to external systems.  The [`super::loader::ModuleLoader`] resolves
//! a definition into a [`ModuleInstance`], and the
//! [`super::runtime::ModuleRuntime`] activates it.
//!
//! # Example YAML
//!
//! ```yaml
//! module:
//!   id: "soc:incident_response"
//!   version: "1.0.0"
//!   description: "SOC Level 2 analyst"
//!   thinking_style: [0.9, 0.2, 0.8, 0.5, 0.7, 0.95, 0.6]
//!   domain: security
//!   agent:
//!     role: "SOC Analyst"
//!     goal: "Triage incidents"
//!     backstory: "Expert analyst."
//!     llm: "anthropic/claude-sonnet-4-20250514"
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::capabilities::{CapabilityPolicy, CapabilityTool, InterfaceProtocol};
use crate::meta_agents::{SavantDomain, SkillDescriptor};

// ============================================================================
// Top-level wrapper
// ============================================================================

/// A complete module definition loaded from YAML.
///
/// The top-level `module:` key wraps the inner definition so YAML files are
/// self-describing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDef {
    pub module: ModuleInner,
}

impl ModuleDef {
    /// Parse a `ModuleDef` from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Parse a `ModuleDef` from a YAML file on disk.
    pub fn from_yaml_file(path: &str) -> Result<Self, super::error::ModuleError> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_yaml(&content)?)
    }
}

// ============================================================================
// ModuleInner — the real payload
// ============================================================================

/// The inner module definition containing all configuration sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInner {
    // --- Identity ---
    /// Namespaced identifier (e.g. `"soc:incident_response"`).
    pub id: String,
    /// Semantic version.
    pub version: String,
    /// Human-readable description.
    pub description: String,

    // --- Cognitive Profile ---
    /// 7-axis thinking style vector:
    /// `[analytical, creative, systematic, intuitive, collaborative, critical, adaptive]`
    ///
    /// Each axis ranges from 0.0 (low) to 1.0 (high).
    pub thinking_style: [f32; 7],

    /// Savant domain this module's agent belongs to.
    pub domain: SavantDomain,

    /// Cognitive gating configuration — when the agent should HOLD or BLOCK.
    #[serde(default)]
    pub collapse_gate: Option<CollapseGateConfig>,

    // --- Agent Config ---
    /// Agent identity and behavior configuration.
    pub agent: ModuleAgentConfig,

    // --- Interfaces ---
    /// External system bindings.  Each becomes a `Capability` + adapter.
    #[serde(default)]
    pub interfaces: Vec<ModuleInterface>,

    // --- Knowledge Sources ---
    /// Knowledge backing for the agent (RAG, BindSpace, MCP).
    #[serde(default)]
    pub knowledge: Vec<KnowledgeSource>,

    // --- Policy / RBAC ---
    /// Access control, auditing, and data-classification rules.
    #[serde(default)]
    pub policy: ModulePolicy,

    // --- Skills ---
    /// Skills the agent starts with (maps to `SkillDescriptor`).
    #[serde(default)]
    pub skills: Vec<SkillDescriptor>,
}

// ============================================================================
// Agent configuration
// ============================================================================

/// Configuration for the agent spawned by a module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleAgentConfig {
    /// Agent role (persona).
    pub role: String,
    /// Agent goal (high-level objective).
    pub goal: String,
    /// Agent backstory (expertise and personality).
    pub backstory: String,
    /// LLM model identifier (e.g. `"anthropic/claude-sonnet-4-20250514"`).
    pub llm: String,
    /// Maximum LLM iterations per task.
    #[serde(default = "default_max_iter")]
    pub max_iter: i32,
    /// Whether this agent can delegate to other agents.
    #[serde(default)]
    pub allow_delegation: bool,
    /// Cross-module delegation targets (module IDs).
    #[serde(default)]
    pub delegation_targets: Vec<String>,
}

fn default_max_iter() -> i32 {
    25
}

// ============================================================================
// Cognitive gating
// ============================================================================

/// Configuration for the cognitive collapse gate.
///
/// The gate runs *before* each tool call and decides:
/// - **Flow** — confidence is high enough, proceed.
/// - **Hold** — confidence is below threshold, escalate.
/// - **Block** — tool name matches a block pattern, deny.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollapseGateConfig {
    /// Minimum confidence (0.0–1.0) to allow a tool call.
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,
    /// Glob patterns that always block (e.g. `"delete_*"`).
    #[serde(default)]
    pub block_patterns: Vec<String>,
    /// Agent or role to escalate HOLD decisions to.
    #[serde(default)]
    pub escalate_to: Option<String>,
}

fn default_min_confidence() -> f64 {
    0.7
}

// ============================================================================
// Interfaces
// ============================================================================

/// An external interface binding declared in a module.
///
/// Each interface is converted to a [`Capability`] by the loader and bound to
/// the agent via the [`InterfaceGateway`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInterface {
    /// Interface identifier (e.g. `"jira:incidents"`).
    pub id: String,
    /// Protocol — reuses the existing `InterfaceProtocol` enum.
    pub protocol: InterfaceProtocol,
    /// Path to an OpenAPI spec for auto-generating tools.
    #[serde(default)]
    pub spec: Option<String>,
    /// Environment variable holding the endpoint URL.
    #[serde(default)]
    pub endpoint_env: Option<String>,
    /// Authentication configuration.
    #[serde(default)]
    pub auth: Option<InterfaceAuth>,
    /// Manually declared tools (when no spec is provided).
    #[serde(default)]
    pub tools: Vec<CapabilityTool>,
    /// Override properties on auto-generated tools.
    #[serde(default)]
    pub tools_override: HashMap<String, ToolOverride>,
    /// Interface-level policy constraints.
    #[serde(default)]
    pub policy: Option<CapabilityPolicy>,
    /// BindSpace address prefix (for ladybug interfaces).
    #[serde(default)]
    pub bindspace_prefix: Option<u8>,
}

/// Authentication configuration for an interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceAuth {
    /// Auth scheme: `"oauth2"`, `"api_key"`, `"iam_role"`, `"password"`.
    pub scheme: String,
    /// Environment variable holding the token / API key.
    #[serde(default)]
    pub token_env: Option<String>,
    /// Environment variable holding the IAM role ARN (for AWS).
    #[serde(default)]
    pub role_env: Option<String>,
    /// OAuth2 scopes.
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Per-tool property overrides applied on top of auto-generated tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOverride {
    /// Require human approval before execution.
    #[serde(default)]
    pub requires_approval: Option<bool>,
    /// Apply collapse gate (HOLD if confidence too low).
    #[serde(default)]
    pub collapse_gate: Option<bool>,
    /// Audit logging level override.
    #[serde(default)]
    pub audit_level: Option<String>,
    /// Check BindSpace resonance before executing.
    #[serde(default)]
    pub requires_resonance: Option<bool>,
    /// Additional RBAC roles required for this tool.
    #[serde(default)]
    pub requires_roles: Option<Vec<String>>,
}

// ============================================================================
// Knowledge sources
// ============================================================================

/// A knowledge source backing an agent's reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum KnowledgeSource {
    /// RAG-indexed document corpus.
    #[serde(rename = "rag")]
    Rag {
        /// Path or URI to the source documents.
        source: String,
        /// Index name in the vector store.
        index: String,
        /// Embedding model override.
        #[serde(default)]
        embedding_model: Option<String>,
    },
    /// BindSpace resonance search (ladybug integration).
    #[serde(rename = "bindspace")]
    BindSpace {
        /// BindSpace address prefix.
        prefix: u8,
        /// Minimum resonance score to include results.
        #[serde(default = "default_resonance_threshold")]
        resonance_threshold: f32,
        /// Maximum number of resonance hits.
        #[serde(default = "default_max_results")]
        max_results: usize,
    },
    /// Knowledge from an MCP interface.
    #[serde(rename = "mcp_knowledge")]
    McpKnowledge {
        /// Reference to a declared interface ID.
        interface_ref: String,
    },
}

fn default_resonance_threshold() -> f32 {
    0.65
}
fn default_max_results() -> usize {
    10
}

// ============================================================================
// Policy / RBAC
// ============================================================================

/// Module-level policy and RBAC configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModulePolicy {
    /// Minimum roles required to activate this module.
    #[serde(default)]
    pub requires_roles: Vec<String>,
    /// Elevated roles that bypass the collapse gate.
    #[serde(default)]
    pub elevated_roles: Vec<String>,
    /// Maximum concurrent agents for this module.
    #[serde(default)]
    pub max_concurrent_agents: Option<usize>,
    /// Data classification level (e.g. `"confidential"`).
    #[serde(default)]
    pub data_classification: Option<String>,
    /// Audit logging level (e.g. `"full"`, `"summary"`, `"none"`).
    #[serde(default)]
    pub audit_level: Option<String>,
    /// Geographic restrictions (e.g. `["eu", "us"]`).
    #[serde(default)]
    pub geo_restrictions: Vec<String>,
    /// Per-tool RBAC overrides.
    #[serde(default)]
    pub tool_policies: HashMap<String, ToolPolicy>,
}

/// Per-tool policy override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPolicy {
    /// Roles required to use this specific tool.
    #[serde(default)]
    pub requires_roles: Vec<String>,
    /// Minimum confidence for this tool.
    #[serde(default)]
    pub min_confidence: Option<f64>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_yaml() -> &'static str {
        r#"
module:
  id: "test:minimal"
  version: "0.1.0"
  description: "Minimal test module"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: general
  agent:
    role: "Test Agent"
    goal: "Run tests"
    backstory: "A test agent."
    llm: "test/model"
"#
    }

    #[test]
    fn test_parse_minimal_module() {
        let def = ModuleDef::from_yaml(minimal_yaml()).unwrap();
        assert_eq!(def.module.id, "test:minimal");
        assert_eq!(def.module.version, "0.1.0");
        assert_eq!(def.module.domain, SavantDomain::General);
        assert_eq!(def.module.thinking_style.len(), 7);
        assert_eq!(def.module.agent.role, "Test Agent");
        assert_eq!(def.module.agent.max_iter, 25); // default
        assert!(def.module.interfaces.is_empty());
        assert!(def.module.skills.is_empty());
        assert!(def.module.collapse_gate.is_none());
    }

    #[test]
    fn test_roundtrip_yaml() {
        let def = ModuleDef::from_yaml(minimal_yaml()).unwrap();
        let yaml_out = serde_yaml::to_string(&def).unwrap();
        let def2 = ModuleDef::from_yaml(&yaml_out).unwrap();
        assert_eq!(def.module.id, def2.module.id);
        assert_eq!(def.module.version, def2.module.version);
        assert_eq!(def.module.thinking_style, def2.module.thinking_style);
        assert_eq!(def.module.domain, def2.module.domain);
    }

    #[test]
    fn test_thinking_style_seven_elements() {
        let def = ModuleDef::from_yaml(minimal_yaml()).unwrap();
        assert_eq!(def.module.thinking_style.len(), 7);
        for &val in &def.module.thinking_style {
            assert!(val >= 0.0 && val <= 1.0);
        }
    }

    #[test]
    fn test_thinking_style_wrong_count_fails() {
        let yaml = r#"
module:
  id: "test:bad"
  version: "0.1.0"
  description: "Bad thinking style"
  thinking_style: [0.5, 0.5, 0.5]
  domain: general
  agent:
    role: "Agent"
    goal: "Goal"
    backstory: "Backstory"
    llm: "test/model"
"#;
        assert!(ModuleDef::from_yaml(yaml).is_err());
    }

    #[test]
    fn test_savant_domain_from_string() {
        let yaml = r#"
module:
  id: "test:security"
  version: "0.1.0"
  description: "Security module"
  thinking_style: [0.9, 0.2, 0.8, 0.5, 0.7, 0.95, 0.6]
  domain: security
  agent:
    role: "Analyst"
    goal: "Analyze"
    backstory: "Expert."
    llm: "test/model"
"#;
        let def = ModuleDef::from_yaml(yaml).unwrap();
        assert_eq!(def.module.domain, SavantDomain::Security);
    }

    #[test]
    fn test_all_savant_domains() {
        for domain_str in [
            "research", "engineering", "data_analysis", "content_creation",
            "planning", "quality_assurance", "security", "dev_ops", "design", "general",
        ] {
            let yaml = format!(
                r#"
module:
  id: "test:{}"
  version: "0.1.0"
  description: "Test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: {}
  agent:
    role: "R"
    goal: "G"
    backstory: "B"
    llm: "m"
"#,
                domain_str.to_lowercase(),
                domain_str
            );
            let def = ModuleDef::from_yaml(&yaml).unwrap();
            assert_eq!(def.module.id, format!("test:{}", domain_str));
        }
    }

    #[test]
    fn test_collapse_gate_defaults() {
        let yaml = r#"
module:
  id: "test:gate"
  version: "0.1.0"
  description: "Gate test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: general
  collapse_gate:
    block_patterns:
      - "delete_*"
  agent:
    role: "R"
    goal: "G"
    backstory: "B"
    llm: "m"
"#;
        let def = ModuleDef::from_yaml(yaml).unwrap();
        let gate = def.module.collapse_gate.unwrap();
        assert_eq!(gate.min_confidence, 0.7); // default
        assert_eq!(gate.block_patterns, vec!["delete_*"]);
        assert!(gate.escalate_to.is_none());
    }

    #[test]
    fn test_interface_parsing() {
        let yaml = r#"
module:
  id: "test:iface"
  version: "0.1.0"
  description: "Interface test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: dev_ops
  agent:
    role: "Admin"
    goal: "Manage"
    backstory: "Expert"
    llm: "m"
  interfaces:
    - id: "graph:users"
      protocol: ms_graph
      auth:
        scheme: "oauth2"
        token_env: "MS_TOKEN"
        scopes: ["User.Read"]
      tools:
        - name: "list_users"
          description: "List users"
          args_schema:
            query:
              type: "string"
              required: true
"#;
        let def = ModuleDef::from_yaml(yaml).unwrap();
        assert_eq!(def.module.interfaces.len(), 1);
        let iface = &def.module.interfaces[0];
        assert_eq!(iface.id, "graph:users");
        assert_eq!(iface.protocol, InterfaceProtocol::MsGraph);
        assert!(iface.auth.is_some());
        let auth = iface.auth.as_ref().unwrap();
        assert_eq!(auth.scheme, "oauth2");
        assert_eq!(auth.scopes, vec!["User.Read"]);
        assert_eq!(iface.tools.len(), 1);
        assert_eq!(iface.tools[0].name, "list_users");
    }

    #[test]
    fn test_knowledge_sources() {
        let yaml = r#"
module:
  id: "test:knowledge"
  version: "0.1.0"
  description: "Knowledge test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: research
  agent:
    role: "R"
    goal: "G"
    backstory: "B"
    llm: "m"
  knowledge:
    - type: rag
      source: "s3://docs/"
      index: "docs_index"
      embedding_model: "jina-v3"
    - type: bindspace
      prefix: 14
      resonance_threshold: 0.7
      max_results: 5
    - type: mcp_knowledge
      interface_ref: "knowledge:graph"
"#;
        let def = ModuleDef::from_yaml(yaml).unwrap();
        assert_eq!(def.module.knowledge.len(), 3);

        match &def.module.knowledge[0] {
            KnowledgeSource::Rag { source, index, embedding_model } => {
                assert_eq!(source, "s3://docs/");
                assert_eq!(index, "docs_index");
                assert_eq!(embedding_model.as_deref(), Some("jina-v3"));
            }
            _ => panic!("Expected Rag"),
        }

        match &def.module.knowledge[1] {
            KnowledgeSource::BindSpace { prefix, resonance_threshold, max_results } => {
                assert_eq!(*prefix, 14);
                assert_eq!(*resonance_threshold, 0.7);
                assert_eq!(*max_results, 5);
            }
            _ => panic!("Expected BindSpace"),
        }

        match &def.module.knowledge[2] {
            KnowledgeSource::McpKnowledge { interface_ref } => {
                assert_eq!(interface_ref, "knowledge:graph");
            }
            _ => panic!("Expected McpKnowledge"),
        }
    }

    #[test]
    fn test_module_policy() {
        let yaml = r#"
module:
  id: "test:policy"
  version: "0.1.0"
  description: "Policy test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: security
  agent:
    role: "R"
    goal: "G"
    backstory: "B"
    llm: "m"
  policy:
    requires_roles: ["analyst"]
    elevated_roles: ["lead", "ciso"]
    max_concurrent_agents: 3
    data_classification: "confidential"
    audit_level: "full"
    geo_restrictions: ["eu", "us"]
    tool_policies:
      "jira/close_incident":
        requires_roles: ["analyst"]
        min_confidence: 0.85
"#;
        let def = ModuleDef::from_yaml(yaml).unwrap();
        let policy = &def.module.policy;
        assert_eq!(policy.requires_roles, vec!["analyst"]);
        assert_eq!(policy.elevated_roles, vec!["lead", "ciso"]);
        assert_eq!(policy.max_concurrent_agents, Some(3));
        assert_eq!(policy.data_classification.as_deref(), Some("confidential"));
        assert_eq!(policy.audit_level.as_deref(), Some("full"));
        assert_eq!(policy.geo_restrictions, vec!["eu", "us"]);
        let tp = &policy.tool_policies["jira/close_incident"];
        assert_eq!(tp.requires_roles, vec!["analyst"]);
        assert_eq!(tp.min_confidence, Some(0.85));
    }

    #[test]
    fn test_skills_parsing() {
        let yaml = r#"
module:
  id: "test:skills"
  version: "0.1.0"
  description: "Skills test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: engineering
  agent:
    role: "Dev"
    goal: "Code"
    backstory: "Expert"
    llm: "m"
  skills:
    - id: "rust_dev"
      name: "Rust Development"
      description: "Write Rust code"
      tags: ["rust", "systems"]
      proficiency: 0.9
    - id: "code_review"
      name: "Code Review"
      description: "Review pull requests"
      tags: ["review"]
      proficiency: 0.85
"#;
        let def = ModuleDef::from_yaml(yaml).unwrap();
        assert_eq!(def.module.skills.len(), 2);
        assert_eq!(def.module.skills[0].id, "rust_dev");
        assert_eq!(def.module.skills[0].proficiency, 0.9);
        assert_eq!(def.module.skills[1].id, "code_review");
    }

    #[test]
    fn test_delegation_targets() {
        let yaml = r#"
module:
  id: "test:deleg"
  version: "0.1.0"
  description: "Delegation test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: general
  agent:
    role: "R"
    goal: "G"
    backstory: "B"
    llm: "m"
    allow_delegation: true
    delegation_targets:
      - "soc:threat_intel"
      - "coding:agent"
"#;
        let def = ModuleDef::from_yaml(yaml).unwrap();
        assert!(def.module.agent.allow_delegation);
        assert_eq!(def.module.agent.delegation_targets, vec!["soc:threat_intel", "coding:agent"]);
    }

    #[test]
    fn test_tool_override_parsing() {
        let yaml = r#"
module:
  id: "test:override"
  version: "0.1.0"
  description: "Override test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: general
  agent:
    role: "R"
    goal: "G"
    backstory: "B"
    llm: "m"
  interfaces:
    - id: "api:test"
      protocol: rest_api
      tools:
        - name: "dangerous_action"
          description: "Does something dangerous"
      tools_override:
        dangerous_action:
          requires_approval: true
          collapse_gate: true
          audit_level: "full"
          requires_roles: ["admin"]
"#;
        let def = ModuleDef::from_yaml(yaml).unwrap();
        let ovr = &def.module.interfaces[0].tools_override["dangerous_action"];
        assert_eq!(ovr.requires_approval, Some(true));
        assert_eq!(ovr.collapse_gate, Some(true));
        assert_eq!(ovr.audit_level.as_deref(), Some("full"));
        assert_eq!(ovr.requires_roles.as_ref().unwrap(), &vec!["admin".to_string()]);
    }

    #[test]
    fn test_full_module_yaml() {
        let yaml = r#"
module:
  id: "soc:incident_response"
  version: "1.0.0"
  description: "SOC Level 2 incident response and threat correlation"
  thinking_style: [0.9, 0.2, 0.8, 0.5, 0.7, 0.95, 0.6]
  domain: security
  collapse_gate:
    min_confidence: 0.7
    block_patterns:
      - "delete_*"
      - "terminate_*"
    escalate_to: "soc_lead"
  agent:
    role: "SOC Level 2 Analyst"
    goal: "Triage, correlate, and respond to security incidents"
    backstory: "You are a senior SOC analyst."
    llm: "anthropic/claude-sonnet-4-20250514"
    max_iter: 50
    allow_delegation: true
    delegation_targets:
      - "soc:threat_intel"
  interfaces:
    - id: "jira:incidents"
      protocol: rest_api
      auth:
        scheme: "oauth2"
        token_env: "JIRA_OAUTH_TOKEN"
        scopes: ["read:jira-work", "write:jira-work"]
      tools:
        - name: "create_incident"
          description: "Create a JIRA incident"
          requires_approval: true
        - name: "close_incident"
          description: "Close a JIRA incident"
      tools_override:
        create_incident:
          audit_level: "full"
        close_incident:
          collapse_gate: true
  knowledge:
    - type: rag
      source: "s3://soc-runbooks/"
      index: "soc_playbooks"
    - type: bindspace
      prefix: 14
      resonance_threshold: 0.65
      max_results: 10
  policy:
    requires_roles: ["soc_analyst"]
    elevated_roles: ["soc_lead", "ciso"]
    data_classification: "confidential"
    audit_level: "full"
  skills:
    - id: "threat_triage"
      name: "Threat Triage"
      description: "Classify and prioritize security incidents"
      tags: ["security", "triage"]
      proficiency: 0.8
"#;
        let def = ModuleDef::from_yaml(yaml).unwrap();
        assert_eq!(def.module.id, "soc:incident_response");
        assert_eq!(def.module.domain, SavantDomain::Security);
        assert!(def.module.collapse_gate.is_some());
        assert_eq!(def.module.interfaces.len(), 1);
        assert_eq!(def.module.knowledge.len(), 2);
        assert_eq!(def.module.skills.len(), 1);
        assert_eq!(def.module.agent.delegation_targets, vec!["soc:threat_intel"]);
    }
}
