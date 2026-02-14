//! Module loader — resolves YAML definitions into ready-to-spawn instances.
//!
//! The loader converts a [`ModuleDef`] into a [`ModuleInstance`] by:
//! 1. Parsing and validating the YAML
//! 2. Converting each interface to a [`Capability`]
//! 3. Building an [`AgentBlueprint`] from the agent config + skills
//! 4. Applying RBAC rules to the [`RbacManager`]
//! 5. Constructing the cognitive gate (if configured)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::capabilities::capability::{
    Capability, CapabilityInterface, CapabilityPolicy, CapabilityTool,
};
use crate::capabilities::registry::CapabilityRegistry;
use crate::meta_agents::types::AgentBlueprint;
use crate::policy::rbac::RbacManager;

use super::error::ModuleError;
use super::module_def::{ModuleDef, ModuleInner, ModuleInterface};
use super::openapi_parser;
use super::runtime::CognitiveGate;

// ============================================================================
// ModuleInstance — resolved, ready-to-spawn
// ============================================================================

/// A fully resolved module, ready for activation by [`ModuleRuntime`].
#[derive(Debug, Clone)]
pub struct ModuleInstance {
    /// The original definition.
    pub def: ModuleDef,
    /// Agent blueprint ready for `SavantCoordinator::spawn_from_blueprint`.
    pub blueprint: AgentBlueprint,
    /// Resolved capabilities (one per interface).
    pub capabilities: Vec<Capability>,
    /// Cognitive gate (if configured).
    pub gate: Option<CognitiveGate>,
    /// Thinking style vector (copied for quick access).
    pub thinking_style: [f32; 10],
}

// ============================================================================
// ModuleLoader
// ============================================================================

/// Loads YAML module definitions and resolves them into `ModuleInstance`s.
pub struct ModuleLoader {
    /// Capability registry for registering resolved capabilities.
    registry: CapabilityRegistry,
    /// RBAC manager for registering module role grants.
    rbac: RbacManager,
    /// Directories to search for module YAML files.
    search_paths: Vec<PathBuf>,
}

impl ModuleLoader {
    /// Create a new loader with default search path (`modules/`).
    pub fn new() -> Self {
        Self {
            registry: CapabilityRegistry::new(),
            rbac: RbacManager::new(),
            search_paths: vec![PathBuf::from("modules/")],
        }
    }

    /// Create a new loader with a custom search path.
    pub fn with_search_path(path: impl Into<PathBuf>) -> Self {
        Self {
            registry: CapabilityRegistry::new(),
            rbac: RbacManager::new(),
            search_paths: vec![path.into()],
        }
    }

    /// Add an additional search path.
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        self.search_paths.push(path.into());
    }

    /// Get the capability registry.
    pub fn registry(&self) -> &CapabilityRegistry {
        &self.registry
    }

    /// Get the RBAC manager.
    pub fn rbac(&self) -> &RbacManager {
        &self.rbac
    }

    /// Take ownership of the registry.
    pub fn into_registry(self) -> CapabilityRegistry {
        self.registry
    }

    /// Take ownership of the RBAC manager.
    pub fn into_rbac(self) -> RbacManager {
        self.rbac
    }

    // -----------------------------------------------------------------------
    // Loading
    // -----------------------------------------------------------------------

    /// Load a single module from a YAML file path.
    pub fn load_file(&mut self, path: &str) -> Result<ModuleInstance, ModuleError> {
        let def = ModuleDef::from_yaml_file(path)?;
        self.resolve(def)
    }

    /// Load a single module from a YAML string.
    pub fn load_yaml(&mut self, yaml: &str) -> Result<ModuleInstance, ModuleError> {
        let def = ModuleDef::from_yaml(yaml)?;
        self.resolve(def)
    }

    /// Load all modules from all search paths.
    ///
    /// Returns the successfully loaded modules.  Files that fail to parse are
    /// logged and skipped.
    pub fn load_all(&mut self) -> Result<Vec<ModuleInstance>, ModuleError> {
        let mut instances = Vec::new();
        let paths: Vec<PathBuf> = self.search_paths.clone();

        for search_path in &paths {
            if !search_path.exists() {
                continue;
            }
            let entries = std::fs::read_dir(search_path)?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                    match self.load_file(path.to_str().unwrap_or_default()) {
                        Ok(instance) => instances.push(instance),
                        Err(e) => {
                            log::warn!("Skipping module {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        Ok(instances)
    }

    // -----------------------------------------------------------------------
    // Resolution
    // -----------------------------------------------------------------------

    /// Resolve a `ModuleDef` into a `ModuleInstance`.
    fn resolve(&mut self, def: ModuleDef) -> Result<ModuleInstance, ModuleError> {
        // Validate thinking style range
        for (i, &val) in def.module.thinking_style.iter().enumerate() {
            if !(0.0..=1.0).contains(&val) {
                return Err(ModuleError::Validation(format!(
                    "thinking_style[{}] = {} is outside 0.0..1.0",
                    i, val
                )));
            }
        }

        // 1. Convert interfaces → Capabilities
        let mut capabilities = Vec::new();
        for iface in &def.module.interfaces {
            let cap = self.interface_to_capability(&def.module, iface)?;
            self.registry.register(cap.clone());
            capabilities.push(cap);
        }

        // 2. Build AgentBlueprint
        let mut blueprint = AgentBlueprint::new(
            &def.module.agent.role,
            &def.module.agent.goal,
            &def.module.agent.backstory,
            &def.module.agent.llm,
            def.module.domain,
        )
        .with_tools(
            capabilities
                .iter()
                .flat_map(|c| c.tool_names().into_iter().map(String::from))
                .collect(),
        );

        blueprint.id = def.module.id.clone();
        blueprint.skills = def.module.skills.clone();
        blueprint.max_iter = def.module.agent.max_iter;
        blueprint.allow_delegation = def.module.agent.allow_delegation;

        // 3. Register RBAC grants
        for role in &def.module.policy.requires_roles {
            self.rbac.grant_capability_to_role(role, &def.module.id);
        }
        for role in &def.module.policy.elevated_roles {
            self.rbac
                .grant_capability_to_role(role, &format!("{}:elevated", def.module.id));
        }

        // 4. Build cognitive gate
        let gate = def.module.collapse_gate.as_ref().map(|cg| CognitiveGate {
            min_confidence: cg.min_confidence,
            block_patterns: cg.block_patterns.clone(),
            escalate_to: cg.escalate_to.clone(),
        });

        let thinking_style = def.module.thinking_style;

        Ok(ModuleInstance {
            def,
            blueprint,
            capabilities,
            gate,
            thinking_style,
        })
    }

    /// Convert a `ModuleInterface` into a `Capability`.
    fn interface_to_capability(
        &self,
        module: &ModuleInner,
        iface: &ModuleInterface,
    ) -> Result<Capability, ModuleError> {
        let mut tools: Vec<CapabilityTool> = iface.tools.clone();

        // If an OpenAPI spec is provided, parse it and merge tools
        if let Some(spec_path) = &iface.spec {
            match self.parse_openapi_tools(spec_path) {
                Ok(spec_tools) => tools.extend(spec_tools),
                Err(e) => {
                    log::warn!(
                        "Failed to parse OpenAPI spec '{}' for interface '{}': {}",
                        spec_path,
                        iface.id,
                        e
                    );
                    // Non-fatal: continue with manually declared tools only
                }
            }
        }

        // Apply tool overrides
        for tool in &mut tools {
            if let Some(ovr) = iface.tools_override.get(&tool.name) {
                if let Some(v) = ovr.requires_approval {
                    tool.requires_approval = v;
                }
                if let Some(roles) = &ovr.requires_roles {
                    tool.requires_roles = roles.clone();
                }
                // collapse_gate and requires_resonance are stored as tool-level
                // metadata consumed by the runtime, not by the capability itself.
            }
        }

        // Build connection config
        let mut config = HashMap::new();
        if let Some(env_var) = &iface.endpoint_env {
            config.insert(
                "endpoint_env".to_string(),
                serde_json::Value::String(env_var.clone()),
            );
        }
        if let Some(auth) = &iface.auth {
            config.insert(
                "auth_scheme".to_string(),
                serde_json::Value::String(auth.scheme.clone()),
            );
            if let Some(token_env) = &auth.token_env {
                config.insert(
                    "token_env".to_string(),
                    serde_json::Value::String(token_env.clone()),
                );
            }
            if let Some(role_env) = &auth.role_env {
                config.insert(
                    "role_env".to_string(),
                    serde_json::Value::String(role_env.clone()),
                );
            }
            if !auth.scopes.is_empty() {
                config.insert(
                    "scopes".to_string(),
                    serde_json::json!(auth.scopes),
                );
            }
        }
        if let Some(prefix) = iface.bindspace_prefix {
            config.insert(
                "bindspace_prefix".to_string(),
                serde_json::Value::Number(prefix.into()),
            );
        }

        Ok(Capability {
            id: iface.id.clone(),
            version: module.version.clone(),
            description: format!("{} interface for module {}", iface.id, module.id),
            tags: vec![module.domain.to_string(), module.id.clone()],
            metadata: Default::default(),
            interface: CapabilityInterface {
                protocol: iface.protocol.clone(),
                config,
                endpoint_template: iface
                    .endpoint_env
                    .as_ref()
                    .map(|e| format!("${{{}}}", e)),
                auth_scheme: iface.auth.as_ref().map(|a| a.scheme.clone()),
            },
            tools,
            policy: iface.policy.clone().unwrap_or_default(),
            depends_on: Vec::new(),
            cam_opcode_range: None,
        })
    }

    /// Parse an OpenAPI spec file into capability tools.
    fn parse_openapi_tools(&self, spec_path: &str) -> Result<Vec<CapabilityTool>, ModuleError> {
        openapi_parser::parse_openapi_file(spec_path)
    }
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
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
  id: "test:loader"
  version: "1.0.0"
  description: "Loader test module"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: general
  agent:
    role: "Test Agent"
    goal: "Test things"
    backstory: "A test agent"
    llm: "test/model"
"#
    }

    #[test]
    fn test_load_yaml_minimal() {
        let mut loader = ModuleLoader::new();
        let instance = loader.load_yaml(minimal_yaml()).unwrap();
        assert_eq!(instance.def.module.id, "test:loader");
        assert_eq!(instance.blueprint.role, "Test Agent");
        assert_eq!(instance.blueprint.domain, crate::meta_agents::SavantDomain::General);
        assert!(instance.capabilities.is_empty());
        assert!(instance.gate.is_none());
    }

    #[test]
    fn test_load_yaml_with_interface() {
        let yaml = r#"
module:
  id: "test:iface"
  version: "1.0.0"
  description: "Interface test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: dev_ops
  agent:
    role: "Admin"
    goal: "Manage"
    backstory: "Expert"
    llm: "test/model"
  interfaces:
    - id: "test:api"
      protocol: rest_api
      auth:
        scheme: "api_key"
        token_env: "TEST_KEY"
      tools:
        - name: "list_items"
          description: "List items"
        - name: "create_item"
          description: "Create item"
          requires_approval: true
"#;
        let mut loader = ModuleLoader::new();
        let instance = loader.load_yaml(yaml).unwrap();

        assert_eq!(instance.capabilities.len(), 1);
        let cap = &instance.capabilities[0];
        assert_eq!(cap.id, "test:api");
        assert_eq!(cap.tools.len(), 2);
        assert_eq!(cap.interface.auth_scheme.as_deref(), Some("api_key"));

        // Blueprint should include tool names
        assert!(instance.blueprint.tools.contains(&"list_items".to_string()));
        assert!(instance.blueprint.tools.contains(&"create_item".to_string()));
    }

    #[test]
    fn test_load_yaml_with_gate() {
        let yaml = r#"
module:
  id: "test:gate"
  version: "1.0.0"
  description: "Gate test"
  thinking_style: [0.9, 0.1, 0.8, 0.3, 0.5, 0.9, 0.4, 0.85, 0.9, 0.7]
  domain: security
  collapse_gate:
    min_confidence: 0.8
    block_patterns: ["delete_*", "drop_*"]
    escalate_to: "lead"
  agent:
    role: "Analyst"
    goal: "Analyze"
    backstory: "Expert"
    llm: "test/model"
"#;
        let mut loader = ModuleLoader::new();
        let instance = loader.load_yaml(yaml).unwrap();

        let gate = instance.gate.as_ref().unwrap();
        assert_eq!(gate.min_confidence, 0.8);
        assert_eq!(gate.block_patterns, vec!["delete_*", "drop_*"]);
        assert_eq!(gate.escalate_to.as_deref(), Some("lead"));
    }

    #[test]
    fn test_load_yaml_rbac_registration() {
        let yaml = r#"
module:
  id: "test:rbac"
  version: "1.0.0"
  description: "RBAC test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: general
  agent:
    role: "R"
    goal: "G"
    backstory: "B"
    llm: "m"
  policy:
    requires_roles: ["analyst", "operator"]
    elevated_roles: ["lead"]
"#;
        let mut loader = ModuleLoader::new();
        let _instance = loader.load_yaml(yaml).unwrap();

        // Verify RBAC grants were registered
        let roles = loader.rbac().all_roles();
        assert!(roles.contains(&"analyst"));
        assert!(roles.contains(&"operator"));
        assert!(roles.contains(&"lead"));
    }

    #[test]
    fn test_load_yaml_tool_overrides() {
        let yaml = r#"
module:
  id: "test:ovr"
  version: "1.0.0"
  description: "Override test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: general
  agent:
    role: "R"
    goal: "G"
    backstory: "B"
    llm: "m"
  interfaces:
    - id: "test:api"
      protocol: rest_api
      tools:
        - name: "dangerous_op"
          description: "A dangerous operation"
        - name: "safe_op"
          description: "A safe operation"
      tools_override:
        dangerous_op:
          requires_approval: true
          requires_roles: ["admin"]
"#;
        let mut loader = ModuleLoader::new();
        let instance = loader.load_yaml(yaml).unwrap();

        let cap = &instance.capabilities[0];
        let dangerous = cap.tools.iter().find(|t| t.name == "dangerous_op").unwrap();
        assert!(dangerous.requires_approval);
        assert_eq!(dangerous.requires_roles, vec!["admin".to_string()]);

        let safe = cap.tools.iter().find(|t| t.name == "safe_op").unwrap();
        assert!(!safe.requires_approval);
    }

    #[test]
    fn test_load_invalid_yaml() {
        let mut loader = ModuleLoader::new();
        let result = loader.load_yaml("not: valid: yaml: [[[");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_missing_file() {
        let mut loader = ModuleLoader::new();
        let result = loader.load_file("/nonexistent/path.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_thinking_style_range() {
        let yaml = r#"
module:
  id: "test:bad_ts"
  version: "1.0.0"
  description: "Bad thinking style"
  thinking_style: [0.5, 1.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: general
  agent:
    role: "R"
    goal: "G"
    backstory: "B"
    llm: "m"
"#;
        let mut loader = ModuleLoader::new();
        let result = loader.load_yaml(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("thinking_style"));
    }

    #[test]
    fn test_load_from_directory() {
        // Create a temp directory with module files
        let dir = tempfile::tempdir().unwrap();
        let yaml = r#"
module:
  id: "test:dir_mod"
  version: "1.0.0"
  description: "Directory module"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: general
  agent:
    role: "R"
    goal: "G"
    backstory: "B"
    llm: "m"
"#;
        std::fs::write(dir.path().join("test_module.yaml"), yaml).unwrap();
        // Also write a non-yaml file that should be skipped
        std::fs::write(dir.path().join("readme.txt"), "not a module").unwrap();

        let mut loader = ModuleLoader::with_search_path(dir.path());
        let instances = loader.load_all().unwrap();
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].def.module.id, "test:dir_mod");
    }

    #[test]
    fn test_load_all_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let mut loader = ModuleLoader::with_search_path(dir.path());
        let instances = loader.load_all().unwrap();
        assert!(instances.is_empty());
    }

    #[test]
    fn test_load_all_nonexistent_directory() {
        let mut loader = ModuleLoader::with_search_path("/nonexistent/path");
        let instances = loader.load_all().unwrap();
        assert!(instances.is_empty());
    }

    #[test]
    fn test_capability_registration() {
        let yaml = r#"
module:
  id: "test:cap_reg"
  version: "1.0.0"
  description: "Cap registration test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: engineering
  agent:
    role: "Dev"
    goal: "Code"
    backstory: "Expert"
    llm: "m"
  interfaces:
    - id: "github:api"
      protocol: rest_api
      tools:
        - name: "list_repos"
          description: "List repos"
    - id: "ci:runner"
      protocol: rest_api
      tools:
        - name: "trigger_build"
          description: "Trigger build"
"#;
        let mut loader = ModuleLoader::new();
        let _instance = loader.load_yaml(yaml).unwrap();

        // Capabilities should be registered in the registry
        assert_eq!(loader.registry().len(), 2);
    }

    #[test]
    fn test_blueprint_id_matches_module_id() {
        let mut loader = ModuleLoader::new();
        let instance = loader.load_yaml(minimal_yaml()).unwrap();
        assert_eq!(instance.blueprint.id, "test:loader");
    }

    #[test]
    fn test_blueprint_skills_from_module() {
        let yaml = r#"
module:
  id: "test:skills"
  version: "1.0.0"
  description: "Skills test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: engineering
  agent:
    role: "Dev"
    goal: "Code"
    backstory: "Expert"
    llm: "m"
  skills:
    - id: "rust_dev"
      name: "Rust"
      description: "Rust development"
      proficiency: 0.9
    - id: "python_dev"
      name: "Python"
      description: "Python development"
      proficiency: 0.8
"#;
        let mut loader = ModuleLoader::new();
        let instance = loader.load_yaml(yaml).unwrap();
        assert_eq!(instance.blueprint.skills.len(), 2);
        assert_eq!(instance.blueprint.skills[0].id, "rust_dev");
        assert_eq!(instance.blueprint.skills[0].proficiency, 0.9);
    }
}
