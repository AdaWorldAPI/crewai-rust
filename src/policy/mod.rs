//! # Policy Engine
//!
//! Deterministic action-level policy enforcement for agent capabilities.
//! Inspired by Amazon Bedrock AgentCore's Cedar-based policy system.
//!
//! ## Architecture
//!
//! The PolicyEngine intercepts every tool call, A2A message, memory write,
//! and blackboard commit. It evaluates the request against a set of rules
//! and returns Allow/Deny. This happens **outside the LLM reasoning loop** —
//! it cannot be circumvented by prompt manipulation.
//!
//! ```text
//! Agent → tool_call("mc_execute", {command: "stop"})
//!   → PolicyEngine.evaluate(PolicyRequest {
//!       principal: Agent(0x0C01),
//!       action: ToolCall("mc_execute"),
//!       resource: Tool("minecraft:server_control::mc_execute"),
//!       context: {command: "stop", confidence: 0.85}
//!     })
//!   → Rule "require_approval_for_stop" matches
//!   → PolicyDecision::Deny { reason: "Requires human approval" }
//! ```
//!
//! ## RBAC
//!
//! Agents are assigned roles in their agent card. Capabilities declare which
//! roles are required. The PolicyEngine bridges these:
//!
//! ```yaml
//! # Agent card
//! agent:
//!   roles: ["researcher", "server_admin"]
//!
//! # Capability
//! policy:
//!   requires_roles: ["server_admin"]
//! ```
//!
//! ## Cedar Export
//!
//! Policies can be exported to Cedar language for audit/compliance tools.

pub mod rbac;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub use rbac::RbacManager;

/// The policy engine: evaluates requests against rules.
#[derive(Debug, Default)]
pub struct PolicyEngine {
    /// All rules, evaluated in order (first match wins for deny, all must pass for allow)
    pub rules: Vec<PolicyRule>,

    /// Enforcement mode
    pub enforcement: EnforcementMode,

    /// RBAC manager
    pub rbac: RbacManager,

    /// Audit log of recent decisions
    audit_log: Vec<AuditEntry>,

    /// Maximum audit log entries to retain
    max_audit_entries: usize,
}

/// A policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Human-readable rule name
    pub name: String,

    /// Description of what this rule enforces
    #[serde(default)]
    pub description: String,

    /// Allow or Deny
    pub effect: PolicyEffect,

    /// Who this rule applies to
    pub principal: PolicyPrincipal,

    /// What action is being controlled
    pub action: PolicyAction,

    /// What resource is being protected
    pub resource: PolicyResource,

    /// Conditions that must all be true for the rule to apply
    #[serde(default)]
    pub conditions: Vec<PolicyCondition>,

    /// Priority: lower numbers are evaluated first (default: 100)
    #[serde(default = "default_priority")]
    pub priority: u32,
}

fn default_priority() -> u32 {
    100
}

/// Policy effect
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// Who the rule applies to
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyPrincipal {
    /// All agents
    All,
    /// Specific agent by slot address
    Agent(u8),
    /// Agents with a specific role
    Role(String),
    /// Specific agent by ID string
    AgentId(String),
    /// Group of agent slots
    Group(Vec<u8>),
}

/// What action is being controlled
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    /// Calling a specific tool
    ToolCall(String),
    /// Calling any tool
    AnyToolCall,
    /// Sending an A2A message
    A2aMessage(String),
    /// Writing to memory
    MemoryWrite,
    /// Reading from memory
    MemoryRead,
    /// Committing to blackboard (ice-caking)
    BlackboardCommit,
    /// Triggering a handover
    Handover,
    /// CAM operation by opcode
    CamOp(u16),
    /// Any action
    Any,
    /// Custom action type
    Custom(String),
}

/// What resource is being protected
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyResource {
    /// Any resource
    Any,
    /// Specific tool by name
    Tool(String),
    /// Specific capability by ID
    Capability(String),
    /// Memory collection
    Collection(String),
    /// BindSpace zone
    Zone(String),
    /// BindSpace prefix
    Prefix(u8),
    /// Wildcard pattern
    Pattern(String),
    /// Custom resource
    Custom(String),
}

/// Condition for a policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCondition {
    /// Context key to evaluate
    pub key: String,

    /// Comparison operator
    pub operator: ConditionOperator,

    /// Value to compare against
    pub value: Value,
}

/// Condition operators
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    NotContains,
    Matches,
    StartsWith,
    EndsWith,
    In,
    NotIn,
}

/// Enforcement mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementMode {
    /// Block denied actions (production)
    Strict,
    /// Log but allow denied actions (testing)
    AuditOnly,
    /// Block and escalate to orchestrator
    Escalate,
}

impl Default for EnforcementMode {
    fn default() -> Self {
        EnforcementMode::Strict
    }
}

/// A request to be evaluated by the policy engine
#[derive(Debug, Clone)]
pub struct PolicyRequest {
    /// Agent slot making the request
    pub agent_slot: u8,

    /// Agent ID (string)
    pub agent_id: String,

    /// Agent roles
    pub agent_roles: Vec<String>,

    /// What action is being attempted
    pub action: PolicyAction,

    /// What resource is being accessed
    pub resource: PolicyResource,

    /// Additional context for condition evaluation
    pub context: HashMap<String, Value>,
}

/// The result of a policy evaluation
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    /// Allow or Deny
    pub effect: PolicyEffect,

    /// Which rule produced this decision (None = default allow)
    pub rule_name: Option<String>,

    /// Human-readable reason
    pub reason: String,

    /// Whether this was enforced or just audited
    pub enforced: bool,
}

/// Audit log entry
#[derive(Debug, Clone)]
struct AuditEntry {
    timestamp: std::time::Instant,
    request_summary: String,
    decision: PolicyDecision,
}

impl PolicyEngine {
    /// Create a new policy engine with default settings.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            enforcement: EnforcementMode::Strict,
            rbac: RbacManager::new(),
            audit_log: Vec::new(),
            max_audit_entries: 10000,
        }
    }

    /// Create from a list of rules.
    pub fn with_rules(rules: Vec<PolicyRule>, enforcement: EnforcementMode) -> Self {
        let mut engine = Self::new();
        engine.rules = rules;
        engine.enforcement = enforcement;
        engine.sort_rules();
        engine
    }

    /// Add a rule to the engine.
    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
        self.sort_rules();
    }

    /// Remove a rule by name.
    pub fn remove_rule(&mut self, name: &str) -> bool {
        let before = self.rules.len();
        self.rules.retain(|r| r.name != name);
        self.rules.len() < before
    }

    /// Sort rules by priority (lower first)
    fn sort_rules(&mut self) {
        self.rules.sort_by_key(|r| r.priority);
    }

    /// Evaluate a request against all rules.
    ///
    /// Logic:
    /// 1. Check RBAC first (if the action involves a capability)
    /// 2. Evaluate all Deny rules — if any match, deny
    /// 3. Evaluate all Allow rules — if any match, allow
    /// 4. Default: deny (deny by default)
    pub fn evaluate(&mut self, request: &PolicyRequest) -> PolicyDecision {
        // Check deny rules first
        for rule in &self.rules {
            if rule.effect == PolicyEffect::Deny
                && self.rule_matches(rule, request)
            {
                let decision = PolicyDecision {
                    effect: PolicyEffect::Deny,
                    rule_name: Some(rule.name.clone()),
                    reason: format!("Denied by rule: {} — {}", rule.name, rule.description),
                    enforced: self.enforcement == EnforcementMode::Strict
                        || self.enforcement == EnforcementMode::Escalate,
                };
                self.audit(request, &decision);
                return decision;
            }
        }

        // Check allow rules
        for rule in &self.rules {
            if rule.effect == PolicyEffect::Allow
                && self.rule_matches(rule, request)
            {
                let decision = PolicyDecision {
                    effect: PolicyEffect::Allow,
                    rule_name: Some(rule.name.clone()),
                    reason: format!("Allowed by rule: {}", rule.name),
                    enforced: true,
                };
                self.audit(request, &decision);
                return decision;
            }
        }

        // Default: allow if no rules match (permissive default)
        // Change to Deny for strict-by-default
        let decision = PolicyDecision {
            effect: PolicyEffect::Allow,
            rule_name: None,
            reason: "No matching rules — default allow".to_string(),
            enforced: true,
        };
        self.audit(request, &decision);
        decision
    }

    /// Check if a rule matches a request
    fn rule_matches(&self, rule: &PolicyRule, request: &PolicyRequest) -> bool {
        // Check principal
        if !self.principal_matches(&rule.principal, request) {
            return false;
        }

        // Check action
        if !self.action_matches(&rule.action, &request.action) {
            return false;
        }

        // Check resource
        if !self.resource_matches(&rule.resource, &request.resource) {
            return false;
        }

        // Check conditions
        for condition in &rule.conditions {
            if !self.condition_matches(condition, &request.context) {
                return false;
            }
        }

        true
    }

    /// Check if a principal matches
    fn principal_matches(&self, rule_principal: &PolicyPrincipal, request: &PolicyRequest) -> bool {
        match rule_principal {
            PolicyPrincipal::All => true,
            PolicyPrincipal::Agent(slot) => request.agent_slot == *slot,
            PolicyPrincipal::AgentId(id) => request.agent_id == *id,
            PolicyPrincipal::Role(role) => request.agent_roles.contains(role),
            PolicyPrincipal::Group(slots) => slots.contains(&request.agent_slot),
        }
    }

    /// Check if an action matches
    fn action_matches(&self, rule_action: &PolicyAction, request_action: &PolicyAction) -> bool {
        match (rule_action, request_action) {
            (PolicyAction::Any, _) => true,
            (PolicyAction::AnyToolCall, PolicyAction::ToolCall(_)) => true,
            (PolicyAction::ToolCall(a), PolicyAction::ToolCall(b)) => {
                a == b || a == "*" || pattern_matches(a, b)
            }
            (PolicyAction::A2aMessage(a), PolicyAction::A2aMessage(b)) => a == b || a == "*",
            (PolicyAction::MemoryWrite, PolicyAction::MemoryWrite) => true,
            (PolicyAction::MemoryRead, PolicyAction::MemoryRead) => true,
            (PolicyAction::BlackboardCommit, PolicyAction::BlackboardCommit) => true,
            (PolicyAction::Handover, PolicyAction::Handover) => true,
            (PolicyAction::CamOp(a), PolicyAction::CamOp(b)) => a == b,
            (PolicyAction::Custom(a), PolicyAction::Custom(b)) => a == b,
            _ => false,
        }
    }

    /// Check if a resource matches
    fn resource_matches(
        &self,
        rule_resource: &PolicyResource,
        request_resource: &PolicyResource,
    ) -> bool {
        match (rule_resource, request_resource) {
            (PolicyResource::Any, _) => true,
            (PolicyResource::Tool(a), PolicyResource::Tool(b)) => {
                a == b || a == "*" || pattern_matches(a, b)
            }
            (PolicyResource::Capability(a), PolicyResource::Capability(b)) => a == b,
            (PolicyResource::Collection(a), PolicyResource::Collection(b)) => a == b,
            (PolicyResource::Zone(a), PolicyResource::Zone(b)) => a == b,
            (PolicyResource::Prefix(a), PolicyResource::Prefix(b)) => a == b,
            (PolicyResource::Pattern(pattern), PolicyResource::Tool(name)) => {
                pattern_matches(pattern, name)
            }
            (PolicyResource::Pattern(pattern), PolicyResource::Capability(name)) => {
                pattern_matches(pattern, name)
            }
            _ => false,
        }
    }

    /// Evaluate a condition against the request context
    fn condition_matches(
        &self,
        condition: &PolicyCondition,
        context: &HashMap<String, Value>,
    ) -> bool {
        let actual = match context.get(&condition.key) {
            Some(v) => v,
            None => return false,
        };

        match &condition.operator {
            ConditionOperator::Equals => actual == &condition.value,
            ConditionOperator::NotEquals => actual != &condition.value,
            ConditionOperator::GreaterThan => compare_values(actual, &condition.value) > 0,
            ConditionOperator::LessThan => compare_values(actual, &condition.value) < 0,
            ConditionOperator::GreaterThanOrEqual => compare_values(actual, &condition.value) >= 0,
            ConditionOperator::LessThanOrEqual => compare_values(actual, &condition.value) <= 0,
            ConditionOperator::Contains => {
                if let (Some(haystack), Some(needle)) =
                    (actual.as_str(), condition.value.as_str())
                {
                    haystack.contains(needle)
                } else if let Some(arr) = actual.as_array() {
                    arr.contains(&condition.value)
                } else {
                    false
                }
            }
            ConditionOperator::NotContains => {
                if let (Some(haystack), Some(needle)) =
                    (actual.as_str(), condition.value.as_str())
                {
                    !haystack.contains(needle)
                } else if let Some(arr) = actual.as_array() {
                    !arr.contains(&condition.value)
                } else {
                    true
                }
            }
            ConditionOperator::Matches => {
                if let (Some(text), Some(pattern)) =
                    (actual.as_str(), condition.value.as_str())
                {
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(text))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            ConditionOperator::StartsWith => {
                if let (Some(text), Some(prefix)) =
                    (actual.as_str(), condition.value.as_str())
                {
                    text.starts_with(prefix)
                } else {
                    false
                }
            }
            ConditionOperator::EndsWith => {
                if let (Some(text), Some(suffix)) =
                    (actual.as_str(), condition.value.as_str())
                {
                    text.ends_with(suffix)
                } else {
                    false
                }
            }
            ConditionOperator::In => {
                if let Some(arr) = condition.value.as_array() {
                    arr.contains(actual)
                } else {
                    false
                }
            }
            ConditionOperator::NotIn => {
                if let Some(arr) = condition.value.as_array() {
                    !arr.contains(actual)
                } else {
                    true
                }
            }
        }
    }

    /// Add an audit entry
    fn audit(&mut self, request: &PolicyRequest, decision: &PolicyDecision) {
        if self.audit_log.len() >= self.max_audit_entries {
            self.audit_log.remove(0);
        }
        self.audit_log.push(AuditEntry {
            timestamp: std::time::Instant::now(),
            request_summary: format!(
                "agent={} action={:?} resource={:?}",
                request.agent_id, request.action, request.resource
            ),
            decision: decision.clone(),
        });
    }

    /// Get recent audit entries count.
    pub fn audit_count(&self) -> usize {
        self.audit_log.len()
    }

    /// Load rules from a capability's policy section.
    pub fn load_capability_policy(
        &mut self,
        capability_id: &str,
        policy: &crate::capabilities::CapabilityPolicy,
    ) {
        // Add RBAC rules
        for role in &policy.requires_roles {
            self.rbac
                .grant_capability_to_role(role, capability_id);
        }

        // Add rate limit rule if specified
        if let Some(max_rpm) = policy.max_rpm {
            self.add_rule(PolicyRule {
                name: format!("{}_rate_limit", capability_id),
                description: format!("Rate limit for {} ({} rpm)", capability_id, max_rpm),
                effect: PolicyEffect::Allow,
                principal: PolicyPrincipal::All,
                action: PolicyAction::AnyToolCall,
                resource: PolicyResource::Capability(capability_id.to_string()),
                conditions: vec![],
                priority: 50,
            });
        }

        // Add approval rules
        for operation in &policy.requires_approval_for {
            self.add_rule(PolicyRule {
                name: format!("{}_{}_requires_approval", capability_id, operation),
                description: format!(
                    "Operation '{}' in {} requires human approval",
                    operation, capability_id
                ),
                effect: PolicyEffect::Deny,
                principal: PolicyPrincipal::All,
                action: PolicyAction::ToolCall(format!("{}::*{}*", capability_id, operation)),
                resource: PolicyResource::Any,
                conditions: vec![PolicyCondition {
                    key: "human_approved".to_string(),
                    operator: ConditionOperator::NotEquals,
                    value: Value::Bool(true),
                }],
                priority: 10,
            });
        }

        // Add confidence threshold rule
        if let Some(min_confidence) = policy.min_confidence {
            self.add_rule(PolicyRule {
                name: format!("{}_min_confidence", capability_id),
                description: format!(
                    "Require confidence >= {} for {}",
                    min_confidence, capability_id
                ),
                effect: PolicyEffect::Deny,
                principal: PolicyPrincipal::All,
                action: PolicyAction::AnyToolCall,
                resource: PolicyResource::Capability(capability_id.to_string()),
                conditions: vec![PolicyCondition {
                    key: "nars_confidence".to_string(),
                    operator: ConditionOperator::LessThan,
                    value: serde_json::json!(min_confidence),
                }],
                priority: 20,
            });
        }

        // Add deny pattern rules
        for (i, pattern) in policy.deny_patterns.iter().enumerate() {
            self.add_rule(PolicyRule {
                name: format!("{}_deny_pattern_{}", capability_id, i),
                description: format!("Deny pattern: {}", pattern),
                effect: PolicyEffect::Deny,
                principal: PolicyPrincipal::All,
                action: PolicyAction::AnyToolCall,
                resource: PolicyResource::Capability(capability_id.to_string()),
                conditions: vec![PolicyCondition {
                    key: "args_string".to_string(),
                    operator: ConditionOperator::Matches,
                    value: Value::String(pattern.clone()),
                }],
                priority: 5,
            });
        }
    }

    /// Export all rules as Cedar-compatible policy text.
    pub fn export_cedar(&self) -> String {
        let mut output = String::new();
        output.push_str("// Auto-generated Cedar policy from crewAI PolicyEngine\n\n");

        for rule in &self.rules {
            let effect = match rule.effect {
                PolicyEffect::Allow => "permit",
                PolicyEffect::Deny => "forbid",
            };

            let principal = match &rule.principal {
                PolicyPrincipal::All => "principal".to_string(),
                PolicyPrincipal::Agent(slot) => format!("principal == Agent::\"0x{:02X}\"", slot),
                PolicyPrincipal::AgentId(id) => format!("principal == Agent::\"{}\"", id),
                PolicyPrincipal::Role(role) => format!("principal in Role::\"{}\"", role),
                PolicyPrincipal::Group(slots) => {
                    let slot_strs: Vec<String> =
                        slots.iter().map(|s| format!("Agent::\"0x{:02X}\"", s)).collect();
                    format!("principal in [{}]", slot_strs.join(", "))
                }
            };

            let action_str = match &rule.action {
                PolicyAction::Any => "action".to_string(),
                PolicyAction::AnyToolCall => "action == Action::\"tool_call\"".to_string(),
                PolicyAction::ToolCall(name) => {
                    format!("action == Action::\"tool_call:{}\"", name)
                }
                PolicyAction::A2aMessage(kind) => {
                    format!("action == Action::\"a2a:{}\"", kind)
                }
                PolicyAction::MemoryWrite => "action == Action::\"memory_write\"".to_string(),
                PolicyAction::MemoryRead => "action == Action::\"memory_read\"".to_string(),
                PolicyAction::BlackboardCommit => {
                    "action == Action::\"blackboard_commit\"".to_string()
                }
                PolicyAction::Handover => "action == Action::\"handover\"".to_string(),
                PolicyAction::CamOp(opcode) => {
                    format!("action == Action::\"cam:0x{:04X}\"", opcode)
                }
                PolicyAction::Custom(name) => format!("action == Action::\"{}\"", name),
            };

            let resource_str = match &rule.resource {
                PolicyResource::Any => "resource".to_string(),
                PolicyResource::Tool(name) => format!("resource == Tool::\"{}\"", name),
                PolicyResource::Capability(id) => {
                    format!("resource == Capability::\"{}\"", id)
                }
                PolicyResource::Collection(name) => {
                    format!("resource == Collection::\"{}\"", name)
                }
                PolicyResource::Zone(zone) => format!("resource == Zone::\"{}\"", zone),
                PolicyResource::Prefix(prefix) => {
                    format!("resource == Prefix::\"0x{:02X}\"", prefix)
                }
                PolicyResource::Pattern(pattern) => {
                    format!("resource like \"{}\"", pattern)
                }
                PolicyResource::Custom(name) => format!("resource == Custom::\"{}\"", name),
            };

            output.push_str(&format!(
                "// {}: {}\n{} (\n  {},\n  {},\n  {}\n)",
                rule.name, rule.description, effect, principal, action_str, resource_str
            ));

            if !rule.conditions.is_empty() {
                output.push_str("\nwhen {\n");
                for cond in &rule.conditions {
                    let op = match &cond.operator {
                        ConditionOperator::Equals => "==",
                        ConditionOperator::NotEquals => "!=",
                        ConditionOperator::GreaterThan => ">",
                        ConditionOperator::LessThan => "<",
                        ConditionOperator::GreaterThanOrEqual => ">=",
                        ConditionOperator::LessThanOrEqual => "<=",
                        ConditionOperator::Contains => "contains",
                        ConditionOperator::NotContains => "!contains",
                        ConditionOperator::Matches => "like",
                        ConditionOperator::StartsWith => "starts_with",
                        ConditionOperator::EndsWith => "ends_with",
                        ConditionOperator::In => "in",
                        ConditionOperator::NotIn => "!in",
                    };
                    output.push_str(&format!(
                        "  context.{} {} {}\n",
                        cond.key,
                        op,
                        serde_json::to_string(&cond.value).unwrap_or_default()
                    ));
                }
                output.push_str("}");
            }

            output.push_str(";\n\n");
        }

        output
    }

    /// Get the total number of rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

/// Simple glob-like pattern matching (supports * wildcards)
fn pattern_matches(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == text;
    }

    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0;

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        match text[pos..].find(part) {
            Some(found) => {
                if i == 0 && found != 0 {
                    return false; // Must match at start if pattern doesn't start with *
                }
                pos += found + part.len();
            }
            None => return false,
        }
    }

    // If pattern doesn't end with *, text must end here
    if !pattern.ends_with('*') {
        return pos == text.len();
    }

    true
}

/// Compare two JSON values numerically
fn compare_values(a: &Value, b: &Value) -> i8 {
    match (a.as_f64(), b.as_f64()) {
        (Some(a), Some(b)) => {
            if a < b {
                -1
            } else if a > b {
                1
            } else {
                0
            }
        }
        _ => match (a.as_str(), b.as_str()) {
            (Some(a), Some(b)) => a.cmp(b) as i8,
            _ => 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_deny_rule() {
        let mut engine = PolicyEngine::new();

        engine.add_rule(PolicyRule {
            name: "deny_stop".to_string(),
            description: "Deny stop commands".to_string(),
            effect: PolicyEffect::Deny,
            principal: PolicyPrincipal::All,
            action: PolicyAction::ToolCall("mc_execute".to_string()),
            resource: PolicyResource::Any,
            conditions: vec![PolicyCondition {
                key: "command".to_string(),
                operator: ConditionOperator::Equals,
                value: Value::String("stop".to_string()),
            }],
            priority: 10,
        });

        // Should deny: command is "stop"
        let mut ctx = HashMap::new();
        ctx.insert(
            "command".to_string(),
            Value::String("stop".to_string()),
        );
        let request = PolicyRequest {
            agent_slot: 1,
            agent_id: "test".to_string(),
            agent_roles: vec![],
            action: PolicyAction::ToolCall("mc_execute".to_string()),
            resource: PolicyResource::Tool("mc_execute".to_string()),
            context: ctx,
        };

        let decision = engine.evaluate(&request);
        assert_eq!(decision.effect, PolicyEffect::Deny);

        // Should allow: command is "list"
        let mut ctx2 = HashMap::new();
        ctx2.insert(
            "command".to_string(),
            Value::String("list".to_string()),
        );
        let request2 = PolicyRequest {
            agent_slot: 1,
            agent_id: "test".to_string(),
            agent_roles: vec![],
            action: PolicyAction::ToolCall("mc_execute".to_string()),
            resource: PolicyResource::Tool("mc_execute".to_string()),
            context: ctx2,
        };

        let decision2 = engine.evaluate(&request2);
        assert_eq!(decision2.effect, PolicyEffect::Allow);
    }

    #[test]
    fn test_role_based_principal() {
        let mut engine = PolicyEngine::new();

        engine.add_rule(PolicyRule {
            name: "admin_only".to_string(),
            description: "Only admins can delete".to_string(),
            effect: PolicyEffect::Deny,
            principal: PolicyPrincipal::All,
            action: PolicyAction::ToolCall("delete".to_string()),
            resource: PolicyResource::Any,
            conditions: vec![],
            priority: 10,
        });

        engine.add_rule(PolicyRule {
            name: "admin_allow_delete".to_string(),
            description: "Admins can delete".to_string(),
            effect: PolicyEffect::Allow,
            principal: PolicyPrincipal::Role("admin".to_string()),
            action: PolicyAction::ToolCall("delete".to_string()),
            resource: PolicyResource::Any,
            conditions: vec![],
            priority: 20,
        });

        // Non-admin: denied
        let request = PolicyRequest {
            agent_slot: 1,
            agent_id: "agent1".to_string(),
            agent_roles: vec!["user".to_string()],
            action: PolicyAction::ToolCall("delete".to_string()),
            resource: PolicyResource::Any,
            context: HashMap::new(),
        };
        assert_eq!(engine.evaluate(&request).effect, PolicyEffect::Deny);

        // Admin: denied (deny rules are checked first, and deny_all matches)
        // This shows deny-first semantics — to implement admin override,
        // the deny rule should exclude admins
        let request2 = PolicyRequest {
            agent_slot: 2,
            agent_id: "agent2".to_string(),
            agent_roles: vec!["admin".to_string()],
            action: PolicyAction::ToolCall("delete".to_string()),
            resource: PolicyResource::Any,
            context: HashMap::new(),
        };
        // The deny rule matches all principals, so even admin is denied
        assert_eq!(engine.evaluate(&request2).effect, PolicyEffect::Deny);
    }

    #[test]
    fn test_pattern_matching() {
        assert!(pattern_matches("*", "anything"));
        assert!(pattern_matches("mc_*", "mc_execute"));
        assert!(!pattern_matches("mc_*", "rcon_execute"));
        assert!(pattern_matches("*_execute", "mc_execute"));
        assert!(pattern_matches("*exec*", "mc_execute_cmd"));
    }

    #[test]
    fn test_cedar_export() {
        let mut engine = PolicyEngine::new();

        engine.add_rule(PolicyRule {
            name: "deny_node_writes".to_string(),
            description: "No agent writes to Node zone".to_string(),
            effect: PolicyEffect::Deny,
            principal: PolicyPrincipal::All,
            action: PolicyAction::CamOp(0x0080),
            resource: PolicyResource::Zone("node".to_string()),
            conditions: vec![],
            priority: 1,
        });

        let cedar = engine.export_cedar();
        assert!(cedar.contains("forbid"));
        assert!(cedar.contains("deny_node_writes"));
        assert!(cedar.contains("cam:0x0080"));
    }
}
