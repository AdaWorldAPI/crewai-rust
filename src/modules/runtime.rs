//! Module runtime — activates modules, enforces cognitive gates, routes tasks.
//!
//! The runtime is the execution layer that wires together:
//! - [`SavantCoordinator`] for agent lifecycle
//! - [`InterfaceGateway`] for capability bindings
//! - [`PolicyEngine`] + [`RbacManager`] for access control
//! - Cognitive gates for pre-tool-call decision enforcement
//! - Thinking styles for ladybug enrichment

use std::collections::HashMap;

use crate::interfaces::InterfaceGateway;
use crate::meta_agents::{OrchestratedTask, RoutingDecision, SavantCoordinator};
use crate::policy::rbac::RbacManager;
use crate::policy::PolicyEngine;

use super::error::ModuleError;
use super::loader::ModuleInstance;

// ============================================================================
// Cognitive gate
// ============================================================================

/// A cognitive gate that runs before each tool call.
///
/// Determines whether the agent should **Flow** (proceed), **Hold** (escalate),
/// or **Block** (deny) based on confidence and pattern matching.
#[derive(Debug, Clone)]
pub struct CognitiveGate {
    /// Minimum confidence (0.0–1.0) to allow a tool call.
    pub min_confidence: f64,
    /// Glob patterns that always block (e.g. `"delete_*"`).
    pub block_patterns: Vec<String>,
    /// Agent or role to escalate HOLD decisions to.
    pub escalate_to: Option<String>,
}

/// Configuration for BindSpace resonance queries.
#[derive(Debug, Clone)]
pub struct ResonanceConfig {
    /// BindSpace address prefix.
    pub prefix: u8,
    /// Minimum resonance threshold.
    pub threshold: f32,
    /// Maximum resonance results.
    pub max_results: usize,
}

/// The decision produced by a cognitive gate check.
#[derive(Debug, Clone, PartialEq)]
pub enum GateDecision {
    /// Confidence is high enough and no patterns match — proceed.
    Flow,
    /// Confidence is below threshold — escalate for review.
    Hold {
        escalate_to: Option<String>,
        confidence: f64,
        required: f64,
    },
    /// Tool name matches a block pattern — deny.
    Block { reason: String },
}

// ============================================================================
// Agent State & Inner Thought Hook
// ============================================================================

/// Runtime snapshot of an agent's cognitive state.
///
/// Passed to `InnerThoughtHook` between steps so the agent can introspect
/// and optionally self-modify its thinking style.
#[derive(Debug, Clone)]
pub struct AgentState {
    /// Current thinking style vector (may have been modified by previous hooks).
    pub current_thinking_style: [f32; 10],
    /// Persona profile (if configured).
    pub persona: Option<super::module_def::PersonaProfile>,
    /// Custom properties from the module definition.
    pub custom_properties: std::collections::HashMap<String, serde_yaml::Value>,
    /// Number of steps completed so far in this session.
    pub step_count: u32,
    /// Current confidence level (0.0–1.0).
    pub confidence: f64,
    /// Whether the last action succeeded.
    pub last_action_succeeded: bool,
}

/// Inner thought hook — self-reflection callback between agent steps.
///
/// The runtime calls this after each step when `enable_inner_loop` is true.
/// The hook receives the current `AgentState` and may return a modified
/// thinking style vector.  If `None` is returned, the style is unchanged.
///
/// # Example (conceptual)
///
/// ```ignore
/// let hook: InnerThoughtHook = Box::new(|state| {
///     if !state.last_action_succeeded && state.confidence < 0.5 {
///         let mut ts = state.current_thinking_style;
///         ts[6] += 0.1; // boost contingency
///         ts[8] += 0.1; // boost validation
///         Some(ts)
///     } else {
///         None
///     }
/// });
/// ```
pub type InnerThoughtHook =
    Box<dyn Fn(&AgentState) -> Option<[f32; 10]> + Send + Sync>;

// ============================================================================
// ModuleRuntime
// ============================================================================

/// The module execution runtime.
///
/// Manages module activation/deactivation, cognitive gate checks, thinking
/// style lookups, and task routing across active modules.
pub struct ModuleRuntime {
    /// Savant coordinator for agent lifecycle.
    coordinator: SavantCoordinator,
    /// Interface gateway for capability bindings.
    gateway: InterfaceGateway,
    /// Policy engine for rule evaluation.
    policy: PolicyEngine,
    /// RBAC manager for role-based access.
    rbac: RbacManager,
    /// Cognitive gates keyed by agent ID.
    gates: HashMap<String, CognitiveGate>,
    /// Thinking style vectors keyed by agent ID.
    thinking_styles: HashMap<String, [f32; 10]>,
    /// Resonance configs keyed by agent ID.
    resonance_configs: HashMap<String, ResonanceConfig>,
    /// Active module instances keyed by module ID.
    active_modules: HashMap<String, ActiveModule>,
}

/// An activated module with its spawned agent ID.
#[derive(Debug, Clone)]
struct ActiveModule {
    instance: ModuleInstance,
    agent_id: String,
}

impl ModuleRuntime {
    /// Create a new runtime with the given default LLM.
    pub fn new(default_llm: impl Into<String>) -> Self {
        Self {
            coordinator: SavantCoordinator::new(default_llm),
            gateway: InterfaceGateway::new(),
            policy: PolicyEngine::new(),
            rbac: RbacManager::new(),
            gates: HashMap::new(),
            thinking_styles: HashMap::new(),
            resonance_configs: HashMap::new(),
            active_modules: HashMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Module lifecycle
    // -----------------------------------------------------------------------

    /// Activate a module: spawn its agent, bind capabilities, register gates.
    ///
    /// Returns the spawned agent ID on success.
    pub fn activate_module(
        &mut self,
        instance: ModuleInstance,
    ) -> Result<String, ModuleError> {
        let module_id = instance.def.module.id.clone();

        if self.active_modules.contains_key(&module_id) {
            return Err(ModuleError::AlreadyActive(module_id));
        }

        // 1. Spawn agent from blueprint via SavantCoordinator
        let agent_id = self
            .coordinator
            .spawn_from_blueprint(&instance.blueprint, false);

        // 2. Capabilities are stored on the module instance and can be bound
        //    to the gateway asynchronously via `bind_capabilities_async`.
        //    InterfaceGateway::bind_capability is async, so we defer binding.
        //    The capabilities metadata is available via get_module().

        // 3. Load RBAC rules
        for role in &instance.def.module.policy.requires_roles {
            self.rbac.grant_capability_to_role(role, &module_id);
        }
        for role in &instance.def.module.policy.elevated_roles {
            self.rbac
                .grant_capability_to_role(role, &format!("{}:elevated", module_id));
        }

        // 4. Load capability policies into PolicyEngine
        for cap in &instance.capabilities {
            self.policy.load_capability_policy(&cap.id, &cap.policy);
        }

        // 5. Store cognitive gate
        if let Some(gate) = &instance.gate {
            self.gates.insert(agent_id.clone(), gate.clone());
        }

        // 6. Store thinking style
        self.thinking_styles
            .insert(agent_id.clone(), instance.thinking_style);

        // 7. Store resonance configs from knowledge sources
        for ks in &instance.def.module.knowledge {
            if let super::module_def::KnowledgeSource::BindSpace {
                prefix,
                resonance_threshold,
                max_results,
            } = ks
            {
                self.resonance_configs.insert(
                    agent_id.clone(),
                    ResonanceConfig {
                        prefix: *prefix,
                        threshold: *resonance_threshold,
                        max_results: *max_results,
                    },
                );
            }
        }

        // 8. Record activation
        self.active_modules.insert(
            module_id,
            ActiveModule {
                instance,
                agent_id: agent_id.clone(),
            },
        );

        Ok(agent_id)
    }

    /// Deactivate a module: cleanup agent, unbind capabilities, remove gates.
    pub fn deactivate_module(&mut self, module_id: &str) -> Result<(), ModuleError> {
        let active = self
            .active_modules
            .remove(module_id)
            .ok_or_else(|| ModuleError::NotFound(module_id.to_string()))?;

        // Unbind capabilities
        for cap in &active.instance.capabilities {
            let _ = self.gateway.unbind_capability(&cap.id);
        }

        // Remove cognitive gate
        self.gates.remove(&active.agent_id);

        // Remove thinking style
        self.thinking_styles.remove(&active.agent_id);

        // Remove resonance config
        self.resonance_configs.remove(&active.agent_id);

        // Terminate savant
        self.coordinator
            .terminate_savant(&active.agent_id, "module deactivated");

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Cognitive gate
    // -----------------------------------------------------------------------

    /// Check the cognitive gate before a tool call.
    ///
    /// Returns [`GateDecision::Flow`] if no gate is configured or the check
    /// passes.
    pub fn check_gate(
        &self,
        agent_id: &str,
        tool_name: &str,
        confidence: f64,
    ) -> GateDecision {
        let gate = match self.gates.get(agent_id) {
            Some(g) => g,
            None => return GateDecision::Flow,
        };

        // Check block patterns first (hard deny)
        for pattern in &gate.block_patterns {
            if glob_match(pattern, tool_name) {
                return GateDecision::Block {
                    reason: format!(
                        "Tool '{}' blocked by pattern '{}'",
                        tool_name, pattern
                    ),
                };
            }
        }

        // Check confidence threshold
        if confidence < gate.min_confidence {
            return GateDecision::Hold {
                escalate_to: gate.escalate_to.clone(),
                confidence,
                required: gate.min_confidence,
            };
        }

        GateDecision::Flow
    }

    // -----------------------------------------------------------------------
    // Thinking style
    // -----------------------------------------------------------------------

    /// Get the thinking style vector for an agent.
    ///
    /// Used by ladybug enrichment to apply cognitive profiles to data
    /// processing.
    pub fn thinking_style(&self, agent_id: &str) -> Option<&[f32; 10]> {
        self.thinking_styles.get(agent_id)
    }

    // -----------------------------------------------------------------------
    // Task routing
    // -----------------------------------------------------------------------

    /// Route a task to the best module's agent.
    ///
    /// Delegates to the underlying `SavantCoordinator::route_task`.
    pub fn route_task(&mut self, task: &OrchestratedTask) -> Option<RoutingDecision> {
        // Use SavantCoordinator's routing which considers domain + skill match
        let decision = self.coordinator.route_task(task);
        if decision.match_score > 0.0 {
            Some(decision)
        } else {
            None
        }
    }

    // -----------------------------------------------------------------------
    // Introspection
    // -----------------------------------------------------------------------

    /// List active module IDs.
    pub fn active_modules(&self) -> Vec<&str> {
        self.active_modules.keys().map(|s| s.as_str()).collect()
    }

    /// Get a module instance by ID.
    pub fn get_module(&self, module_id: &str) -> Option<&ModuleInstance> {
        self.active_modules
            .get(module_id)
            .map(|am| &am.instance)
    }

    /// Get the agent ID for a module.
    pub fn agent_id_for_module(&self, module_id: &str) -> Option<&str> {
        self.active_modules
            .get(module_id)
            .map(|am| am.agent_id.as_str())
    }

    /// Get the resonance config for an agent.
    pub fn resonance_config(&self, agent_id: &str) -> Option<&ResonanceConfig> {
        self.resonance_configs.get(agent_id)
    }

    /// Get the coordinator (for direct access).
    pub fn coordinator(&self) -> &SavantCoordinator {
        &self.coordinator
    }

    /// Get the gateway (for direct access).
    pub fn gateway(&self) -> &InterfaceGateway {
        &self.gateway
    }

    /// Get the RBAC manager.
    pub fn rbac(&self) -> &RbacManager {
        &self.rbac
    }

    /// Check if an agent has a required role for a module.
    pub fn check_rbac(
        &self,
        agent_id: &str,
        module_id: &str,
    ) -> bool {
        let module = match self.active_modules.get(module_id) {
            Some(m) => m,
            None => return false,
        };

        let required_roles = &module.instance.def.module.policy.requires_roles;
        if required_roles.is_empty() {
            return true; // No role requirement
        }

        // Check if agent has any of the required roles
        for role in required_roles {
            if self.rbac.has_role(agent_id, role) {
                return true;
            }
        }

        false
    }
}

impl std::fmt::Debug for ModuleRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleRuntime")
            .field("active_modules", &self.active_modules.keys().collect::<Vec<_>>())
            .field("gates", &self.gates.len())
            .field("thinking_styles", &self.thinking_styles.len())
            .finish()
    }
}

// ============================================================================
// Glob matching (minimal)
// ============================================================================

/// Simple glob matching — supports `*` as wildcard.
fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if let Some(prefix) = pattern.strip_suffix('*') {
        return text.starts_with(prefix);
    }

    if let Some(suffix) = pattern.strip_prefix('*') {
        return text.ends_with(suffix);
    }

    if pattern.contains('*') {
        // Split on first '*' and check prefix + suffix
        let parts: Vec<&str> = pattern.splitn(2, '*').collect();
        if parts.len() == 2 {
            return text.starts_with(parts[0]) && text.ends_with(parts[1]);
        }
    }

    pattern == text
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::loader::ModuleLoader;

    fn load_test_instance(yaml: &str) -> ModuleInstance {
        let mut loader = ModuleLoader::new();
        loader.load_yaml(yaml).unwrap()
    }

    fn minimal_instance() -> ModuleInstance {
        load_test_instance(
            r#"
module:
  id: "test:runtime"
  version: "1.0.0"
  description: "Runtime test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: general
  agent:
    role: "Test Agent"
    goal: "Test things"
    backstory: "A test agent"
    llm: "test/model"
"#,
        )
    }

    fn gated_instance() -> ModuleInstance {
        load_test_instance(
            r#"
module:
  id: "test:gated"
  version: "1.0.0"
  description: "Gated test"
  thinking_style: [0.9, 0.1, 0.8, 0.3, 0.5, 0.95, 0.4, 0.85, 0.9, 0.7]
  domain: security
  collapse_gate:
    min_confidence: 0.8
    block_patterns: ["delete_*", "terminate_*", "drop_database"]
    escalate_to: "lead"
  agent:
    role: "Analyst"
    goal: "Analyze"
    backstory: "Expert"
    llm: "test/model"
"#,
        )
    }

    #[test]
    fn test_activate_module() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance = minimal_instance();
        let agent_id = runtime.activate_module(instance).unwrap();
        assert!(!agent_id.is_empty());
        assert_eq!(runtime.active_modules().len(), 1);
        assert!(runtime.active_modules().contains(&"test:runtime"));
    }

    #[test]
    fn test_activate_duplicate_fails() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance1 = minimal_instance();
        let instance2 = minimal_instance();
        runtime.activate_module(instance1).unwrap();
        let result = runtime.activate_module(instance2);
        assert!(result.is_err());
    }

    #[test]
    fn test_deactivate_module() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance = minimal_instance();
        runtime.activate_module(instance).unwrap();
        assert_eq!(runtime.active_modules().len(), 1);

        runtime.deactivate_module("test:runtime").unwrap();
        assert!(runtime.active_modules().is_empty());
    }

    #[test]
    fn test_deactivate_nonexistent_fails() {
        let mut runtime = ModuleRuntime::new("test/model");
        let result = runtime.deactivate_module("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_gate_flow_when_confident() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance = gated_instance();
        let agent_id = runtime.activate_module(instance).unwrap();

        let decision = runtime.check_gate(&agent_id, "search_logs", 0.95);
        assert_eq!(decision, GateDecision::Flow);
    }

    #[test]
    fn test_gate_hold_when_below_threshold() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance = gated_instance();
        let agent_id = runtime.activate_module(instance).unwrap();

        let decision = runtime.check_gate(&agent_id, "search_logs", 0.5);
        match decision {
            GateDecision::Hold {
                escalate_to,
                confidence,
                required,
            } => {
                assert_eq!(escalate_to.as_deref(), Some("lead"));
                assert_eq!(confidence, 0.5);
                assert_eq!(required, 0.8);
            }
            other => panic!("Expected Hold, got {:?}", other),
        }
    }

    #[test]
    fn test_gate_block_on_pattern() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance = gated_instance();
        let agent_id = runtime.activate_module(instance).unwrap();

        // "delete_*" pattern
        let decision = runtime.check_gate(&agent_id, "delete_user", 0.99);
        match decision {
            GateDecision::Block { reason } => {
                assert!(reason.contains("delete_user"));
                assert!(reason.contains("delete_*"));
            }
            other => panic!("Expected Block, got {:?}", other),
        }

        // "terminate_*" pattern
        let decision = runtime.check_gate(&agent_id, "terminate_instance", 0.99);
        assert!(matches!(decision, GateDecision::Block { .. }));

        // Exact match pattern
        let decision = runtime.check_gate(&agent_id, "drop_database", 0.99);
        assert!(matches!(decision, GateDecision::Block { .. }));
    }

    #[test]
    fn test_gate_flow_without_gate() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance = minimal_instance();
        let agent_id = runtime.activate_module(instance).unwrap();

        // No gate configured — always Flow
        let decision = runtime.check_gate(&agent_id, "any_tool", 0.1);
        assert_eq!(decision, GateDecision::Flow);
    }

    #[test]
    fn test_thinking_style_retrieval() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance = gated_instance();
        let expected_style = instance.thinking_style;
        let agent_id = runtime.activate_module(instance).unwrap();

        let style = runtime.thinking_style(&agent_id).unwrap();
        assert_eq!(*style, expected_style);
    }

    #[test]
    fn test_thinking_style_none_for_unknown() {
        let runtime = ModuleRuntime::new("test/model");
        assert!(runtime.thinking_style("nonexistent").is_none());
    }

    #[test]
    fn test_route_task_to_module() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance = load_test_instance(
            r#"
module:
  id: "test:security"
  version: "1.0.0"
  description: "Security module"
  thinking_style: [0.9, 0.1, 0.8, 0.3, 0.5, 0.95, 0.4, 0.85, 0.9, 0.7]
  domain: security
  agent:
    role: "Security Analyst"
    goal: "Analyze threats"
    backstory: "Expert security analyst"
    llm: "test/model"
  skills:
    - id: "threat_analysis"
      name: "Threat Analysis"
      description: "Analyze security threats"
      tags: ["security", "threats"]
      proficiency: 0.9
"#,
        );
        runtime.activate_module(instance).unwrap();

        let task = OrchestratedTask::new("Analyze the security threat in the logs")
            .with_domain(crate::meta_agents::SavantDomain::Security);

        let decision = runtime.route_task(&task);
        assert!(decision.is_some());
    }

    #[test]
    fn test_get_module_by_id() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance = minimal_instance();
        runtime.activate_module(instance).unwrap();

        let module = runtime.get_module("test:runtime");
        assert!(module.is_some());
        assert_eq!(module.unwrap().def.module.id, "test:runtime");

        assert!(runtime.get_module("nonexistent").is_none());
    }

    #[test]
    fn test_agent_id_for_module() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance = minimal_instance();
        let agent_id = runtime.activate_module(instance).unwrap();

        assert_eq!(
            runtime.agent_id_for_module("test:runtime"),
            Some(agent_id.as_str())
        );
        assert!(runtime.agent_id_for_module("nonexistent").is_none());
    }

    #[test]
    fn test_resonance_config_from_knowledge() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance = load_test_instance(
            r#"
module:
  id: "test:resonance"
  version: "1.0.0"
  description: "Resonance test"
  thinking_style: [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]
  domain: general
  agent:
    role: "R"
    goal: "G"
    backstory: "B"
    llm: "m"
  knowledge:
    - type: bindspace
      prefix: 14
      resonance_threshold: 0.7
      max_results: 5
"#,
        );
        let agent_id = runtime.activate_module(instance).unwrap();

        let config = runtime.resonance_config(&agent_id).unwrap();
        assert_eq!(config.prefix, 14);
        assert_eq!(config.threshold, 0.7);
        assert_eq!(config.max_results, 5);
    }

    #[test]
    fn test_rbac_check() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance = load_test_instance(
            r#"
module:
  id: "test:rbac_mod"
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
    requires_roles: ["analyst"]
"#,
        );
        runtime.activate_module(instance).unwrap();

        // Agent without role → denied
        assert!(!runtime.check_rbac("agent-1", "test:rbac_mod"));

        // Assign role and check again
        runtime.rbac.assign_role("agent-1", "analyst");
        assert!(runtime.check_rbac("agent-1", "test:rbac_mod"));
    }

    #[test]
    fn test_rbac_no_roles_required() {
        let mut runtime = ModuleRuntime::new("test/model");
        let instance = minimal_instance();
        runtime.activate_module(instance).unwrap();

        // No roles required → always allowed
        assert!(runtime.check_rbac("anyone", "test:runtime"));
    }

    #[test]
    fn test_cross_module_delegation() {
        let mut runtime = ModuleRuntime::new("test/model");

        // Activate two modules
        let soc = load_test_instance(
            r#"
module:
  id: "soc:analyst"
  version: "1.0.0"
  description: "SOC analyst"
  thinking_style: [0.9, 0.1, 0.8, 0.5, 0.7, 0.95, 0.6, 0.85, 0.9, 0.75]
  domain: security
  agent:
    role: "SOC Analyst"
    goal: "Triage incidents"
    backstory: "Expert analyst"
    llm: "test/model"
    allow_delegation: true
    delegation_targets: ["coding:agent"]
"#,
        );
        let coding = load_test_instance(
            r#"
module:
  id: "coding:agent"
  version: "1.0.0"
  description: "Coding agent"
  thinking_style: [0.6, 0.8, 0.7, 0.4, 0.3, 0.7, 0.9, 0.65, 0.6, 0.8]
  domain: engineering
  agent:
    role: "Developer"
    goal: "Write code"
    backstory: "Expert developer"
    llm: "test/model"
"#,
        );

        runtime.activate_module(soc).unwrap();
        runtime.activate_module(coding).unwrap();

        assert_eq!(runtime.active_modules().len(), 2);

        // Verify delegation targets are stored
        let soc_mod = runtime.get_module("soc:analyst").unwrap();
        assert!(soc_mod
            .def
            .module
            .agent
            .delegation_targets
            .contains(&"coding:agent".to_string()));
    }

    #[test]
    fn test_full_integration_load_activate_gate() {
        let mut loader = ModuleLoader::new();
        let instance = loader
            .load_yaml(
                r#"
module:
  id: "integration:test"
  version: "1.0.0"
  description: "Full integration test"
  thinking_style: [0.8, 0.3, 0.9, 0.2, 0.6, 0.85, 0.5, 0.8, 0.85, 0.7]
  domain: dev_ops
  collapse_gate:
    min_confidence: 0.75
    block_patterns: ["destroy_*"]
    escalate_to: "ops_lead"
  agent:
    role: "Ops Engineer"
    goal: "Manage infrastructure"
    backstory: "Expert ops engineer"
    llm: "test/model"
    max_iter: 30
  interfaces:
    - id: "infra:api"
      protocol: rest_api
      tools:
        - name: "list_servers"
          description: "List servers"
          read_only: true
        - name: "destroy_server"
          description: "Destroy a server"
          requires_approval: true
  policy:
    requires_roles: ["ops_engineer"]
    elevated_roles: ["ops_lead"]
    audit_level: "full"
  skills:
    - id: "infra_mgmt"
      name: "Infrastructure Management"
      description: "Manage cloud infrastructure"
      proficiency: 0.85
"#,
            )
            .unwrap();

        let mut runtime = ModuleRuntime::new("test/model");
        let agent_id = runtime.activate_module(instance).unwrap();

        // Gate: Flow for safe ops with high confidence
        assert_eq!(
            runtime.check_gate(&agent_id, "list_servers", 0.9),
            GateDecision::Flow
        );

        // Gate: Hold for safe ops with low confidence
        match runtime.check_gate(&agent_id, "list_servers", 0.5) {
            GateDecision::Hold { escalate_to, .. } => {
                assert_eq!(escalate_to.as_deref(), Some("ops_lead"));
            }
            other => panic!("Expected Hold, got {:?}", other),
        }

        // Gate: Block for destroy pattern
        assert!(matches!(
            runtime.check_gate(&agent_id, "destroy_server", 0.99),
            GateDecision::Block { .. }
        ));

        // Thinking style
        let style = runtime.thinking_style(&agent_id).unwrap();
        assert_eq!(style[0], 0.8); // analytical
        assert_eq!(style[2], 0.9); // systematic

        // Module retrieval
        let module = runtime.get_module("integration:test").unwrap();
        assert_eq!(module.capabilities.len(), 1);
        assert_eq!(module.blueprint.skills.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Glob matching tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_glob_match() {
        assert!(glob_match("delete_*", "delete_user"));
        assert!(glob_match("delete_*", "delete_everything"));
        assert!(!glob_match("delete_*", "create_user"));

        assert!(glob_match("*_user", "delete_user"));
        assert!(glob_match("*_user", "create_user"));
        assert!(!glob_match("*_user", "delete_role"));

        assert!(glob_match("*", "anything"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "other"));

        assert!(glob_match("pre*fix", "prefix"));
        assert!(glob_match("pre*fix", "pre_something_fix"));
        assert!(!glob_match("pre*fix", "pre_something_end"));
    }
}
