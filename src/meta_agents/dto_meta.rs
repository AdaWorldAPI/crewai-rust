//! DTO/Types meta-agent — manages type definitions and data transfer schemas.
//!
//! The `DtoRegistry` provides a centralized type system for the meta-agent
//! orchestration pipeline. It manages:
//!
//! - **Schema registration**: register and look up DTO schemas by name
//! - **Type-safe envelopes**: wrap task inputs/outputs in typed envelopes
//! - **Schema validation**: validate data against registered schemas
//! - **Cross-agent type compatibility**: ensure agents agree on data formats
//! - **Schema evolution**: version tracking for evolving DTOs
//!
//! # Architecture
//!
//! ```text
//! DtoRegistry
//!   ├── SchemaStore          (name → DtoSchema mapping)
//!   ├── EnvelopeFactory      (creates typed envelopes for agent I/O)
//!   ├── TypeValidator         (validates data against schemas)
//!   └── CompatibilityChecker (ensures cross-agent type agreement)
//! ```
//!
//! # Built-in Schemas
//!
//! The registry comes pre-loaded with schemas for all meta-agent DTOs:
//!
//! - `delegation_request` — DelegationRequest envelope
//! - `delegation_response` — DelegationResponse envelope
//! - `orchestrated_task` — OrchestratedTask envelope
//! - `agent_feedback` — AgentFeedback envelope
//! - `skill_descriptor` — SkillDescriptor envelope
//! - `capability_update` — CapabilityUpdate envelope
//! - `orchestration_event` — OrchestrationEvent envelope

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::delegation::{
    AgentFeedback, CapabilityUpdate, DelegationRequest, DelegationResponse, OrchestrationEvent,
};
use super::types::{
    AgentBlueprint, OrchestratedTask, SavantDomain, SkillDescriptor, SpawnedAgentState,
};

// ---------------------------------------------------------------------------
// Schema definitions
// ---------------------------------------------------------------------------

/// Content type for DTO envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DtoContentType {
    /// Plain text content.
    Text,
    /// JSON structured content.
    Json,
    /// Binary content (base64 encoded).
    Binary,
    /// Task result content.
    TaskResult,
    /// Agent capability advertisement.
    Capability,
    /// Delegation protocol message.
    Delegation,
    /// Skill descriptor.
    Skill,
    /// Orchestration event.
    Event,
}

impl std::fmt::Display for DtoContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text/plain"),
            Self::Json => write!(f, "application/json"),
            Self::Binary => write!(f, "application/octet-stream"),
            Self::TaskResult => write!(f, "application/x-task-result"),
            Self::Capability => write!(f, "application/x-capability"),
            Self::Delegation => write!(f, "application/x-delegation"),
            Self::Skill => write!(f, "application/x-skill"),
            Self::Event => write!(f, "application/x-event"),
        }
    }
}

/// Version information for schema evolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Major version (breaking changes).
    pub major: u32,
    /// Minor version (backward-compatible additions).
    pub minor: u32,
    /// Patch version (bug fixes).
    pub patch: u32,
}

impl SchemaVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Check compatibility: same major version is compatible.
    pub fn is_compatible_with(&self, other: &SchemaVersion) -> bool {
        self.major == other.major
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A registered DTO schema definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DtoSchema {
    /// Unique schema name (e.g., "delegation_request").
    pub name: String,
    /// Schema version.
    pub version: SchemaVersion,
    /// Content type this schema describes.
    pub content_type: DtoContentType,
    /// Description of the schema.
    pub description: String,
    /// Required field names.
    pub required_fields: Vec<String>,
    /// Optional field names.
    pub optional_fields: Vec<String>,
    /// Domain this schema is primarily associated with.
    pub domain: Option<SavantDomain>,
    /// Example JSON value conforming to this schema.
    pub example: Option<Value>,
}

// ---------------------------------------------------------------------------
// Typed envelopes
// ---------------------------------------------------------------------------

/// A typed envelope wrapping data with schema metadata.
///
/// Envelopes are the standard wrapper for all data flowing between
/// agents in the meta-agent system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DtoEnvelope {
    /// Unique envelope ID.
    pub id: String,
    /// Schema name this envelope conforms to.
    pub schema: String,
    /// Schema version.
    pub version: SchemaVersion,
    /// Content type.
    pub content_type: DtoContentType,
    /// The actual payload.
    pub payload: Value,
    /// Source agent ID (who created this envelope).
    pub source_agent: Option<String>,
    /// Target agent ID (who should consume this).
    pub target_agent: Option<String>,
    /// Source domain.
    pub source_domain: Option<SavantDomain>,
    /// Timestamp (ISO 8601).
    pub timestamp: String,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl DtoEnvelope {
    /// Create a new envelope for a given schema.
    pub fn new(schema: &str, content_type: DtoContentType, payload: Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            schema: schema.to_string(),
            version: SchemaVersion::new(1, 0, 0),
            content_type,
            payload,
            source_agent: None,
            target_agent: None,
            source_domain: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        }
    }

    /// Builder: set source agent.
    pub fn from_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.source_agent = Some(agent_id.into());
        self
    }

    /// Builder: set target agent.
    pub fn to_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.target_agent = Some(agent_id.into());
        self
    }

    /// Builder: set source domain.
    pub fn with_domain(mut self, domain: SavantDomain) -> Self {
        self.source_domain = Some(domain);
        self
    }

    /// Builder: add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Create an envelope from a DelegationRequest.
    pub fn from_delegation_request(request: &DelegationRequest) -> Self {
        let payload = serde_json::to_value(request).unwrap_or_default();
        Self::new("delegation_request", DtoContentType::Delegation, payload)
            .from_agent(&request.from_agent)
    }

    /// Create an envelope from a DelegationResponse.
    pub fn from_delegation_response(response: &DelegationResponse) -> Self {
        let payload = serde_json::to_value(response).unwrap_or_default();
        Self::new("delegation_response", DtoContentType::Delegation, payload)
            .from_agent(&response.from_agent)
    }

    /// Create an envelope from an OrchestratedTask.
    pub fn from_task(task: &OrchestratedTask) -> Self {
        let payload = serde_json::to_value(task).unwrap_or_default();
        let mut env = Self::new("orchestrated_task", DtoContentType::TaskResult, payload);
        if let Some(ref domain) = task.preferred_domain {
            env = env.with_domain(*domain);
        }
        if let Some(ref agent_id) = task.assigned_agent {
            env = env.from_agent(agent_id.as_str());
        }
        env
    }

    /// Create an envelope from an AgentFeedback.
    pub fn from_feedback(feedback: &AgentFeedback) -> Self {
        let payload = serde_json::to_value(feedback).unwrap_or_default();
        Self::new("agent_feedback", DtoContentType::Json, payload)
            .from_agent(&feedback.agent_id)
    }

    /// Create an envelope from a SkillDescriptor.
    pub fn from_skill(skill: &SkillDescriptor) -> Self {
        let payload = serde_json::to_value(skill).unwrap_or_default();
        Self::new("skill_descriptor", DtoContentType::Skill, payload)
    }

    /// Create an envelope from a CapabilityUpdate.
    pub fn from_capability_update(update: &CapabilityUpdate) -> Self {
        let payload = serde_json::to_value(update).unwrap_or_default();
        Self::new("capability_update", DtoContentType::Capability, payload)
            .from_agent(&update.agent_id)
            .with_domain(update.domain)
    }

    /// Create an envelope from an OrchestrationEvent.
    pub fn from_event(event: &OrchestrationEvent) -> Self {
        let payload = serde_json::to_value(event).unwrap_or_default();
        Self::new("orchestration_event", DtoContentType::Event, payload)
    }

    /// Create a text envelope.
    pub fn text(schema: &str, text: impl Into<String>) -> Self {
        Self::new(schema, DtoContentType::Text, Value::String(text.into()))
    }

    /// Create a JSON envelope.
    pub fn json(schema: &str, data: Value) -> Self {
        Self::new(schema, DtoContentType::Json, data)
    }

    /// Extract the payload as a typed value.
    pub fn extract<T: serde::de::DeserializeOwned>(&self) -> Result<T, String> {
        serde_json::from_value(self.payload.clone())
            .map_err(|e| format!("Failed to extract {}: {}", self.schema, e))
    }
}

// ---------------------------------------------------------------------------
// Validation result
// ---------------------------------------------------------------------------

/// Result of validating data against a schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the data is valid.
    pub valid: bool,
    /// Schema that was validated against.
    pub schema: String,
    /// List of validation errors.
    pub errors: Vec<String>,
    /// List of warnings (valid but potentially problematic).
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// DtoRegistry
// ---------------------------------------------------------------------------

/// Central registry for DTO schemas and type management.
///
/// Manages schema definitions, creates typed envelopes, validates data,
/// and checks cross-agent type compatibility.
pub struct DtoRegistry {
    /// Registered schemas (name → schema).
    schemas: HashMap<String, DtoSchema>,
    /// Envelope history for audit trail.
    envelope_log: Vec<DtoEnvelope>,
    /// Compatibility cache: (schema_a, schema_b) → compatible.
    compatibility_cache: HashMap<(String, String), bool>,
}

impl DtoRegistry {
    /// Create a new registry with built-in meta-agent schemas.
    pub fn new() -> Self {
        let mut registry = Self {
            schemas: HashMap::new(),
            envelope_log: Vec::new(),
            compatibility_cache: HashMap::new(),
        };
        registry.register_builtin_schemas();
        registry
    }

    /// Register built-in schemas for all meta-agent DTOs.
    fn register_builtin_schemas(&mut self) {
        self.register(DtoSchema {
            name: "delegation_request".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            content_type: DtoContentType::Delegation,
            description: "Request from one agent to delegate a sub-task to another".to_string(),
            required_fields: vec![
                "id".into(), "from_agent".into(), "task_description".into(), "priority".into(),
            ],
            optional_fields: vec![
                "to_agent".into(), "target_domain".into(), "required_skills".into(),
                "context".into(), "max_turns".into(), "metadata".into(),
            ],
            domain: None,
            example: None,
        });

        self.register(DtoSchema {
            name: "delegation_response".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            content_type: DtoContentType::Delegation,
            description: "Response from a delegate agent back to the orchestrator".to_string(),
            required_fields: vec![
                "request_id".into(), "from_agent".into(), "success".into(),
            ],
            optional_fields: vec![
                "result".into(), "error".into(), "skills_used".into(),
                "iterations_used".into(), "metadata".into(),
            ],
            domain: None,
            example: None,
        });

        self.register(DtoSchema {
            name: "orchestrated_task".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            content_type: DtoContentType::TaskResult,
            description: "A task managed by the meta-orchestrator".to_string(),
            required_fields: vec![
                "id".into(), "description".into(), "status".into(), "priority".into(),
            ],
            optional_fields: vec![
                "context".into(), "dependencies".into(), "required_skills".into(),
                "preferred_domain".into(), "assigned_agent".into(), "output".into(),
                "error".into(), "metadata".into(),
            ],
            domain: None,
            example: None,
        });

        self.register(DtoSchema {
            name: "agent_feedback".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            content_type: DtoContentType::Json,
            description: "Performance feedback for an agent's task execution".to_string(),
            required_fields: vec![
                "id".into(), "agent_id".into(), "task_id".into(), "outcome".into(),
            ],
            optional_fields: vec![
                "relevant_skills".into(), "missing_skills".into(), "suggested_skills".into(),
                "proficiency_deltas".into(), "notes".into(),
            ],
            domain: None,
            example: None,
        });

        self.register(DtoSchema {
            name: "skill_descriptor".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            content_type: DtoContentType::Skill,
            description: "Describes a specific skill that an agent possesses".to_string(),
            required_fields: vec!["id".into(), "name".into(), "description".into()],
            optional_fields: vec![
                "tags".into(), "input_modes".into(), "output_modes".into(),
                "proficiency".into(), "required_tools".into(), "max_concurrent".into(),
            ],
            domain: None,
            example: None,
        });

        self.register(DtoSchema {
            name: "capability_update".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            content_type: DtoContentType::Capability,
            description: "Notification that an agent's capabilities have changed".to_string(),
            required_fields: vec![
                "agent_id".into(), "skills".into(), "performance_score".into(),
                "domain".into(), "trigger".into(),
            ],
            optional_fields: vec![],
            domain: None,
            example: None,
        });

        self.register(DtoSchema {
            name: "orchestration_event".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            content_type: DtoContentType::Event,
            description: "Lifecycle event emitted during orchestration".to_string(),
            required_fields: vec!["event_type".into(), "data".into()],
            optional_fields: vec![],
            domain: None,
            example: None,
        });

        self.register(DtoSchema {
            name: "agent_blueprint".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            content_type: DtoContentType::Json,
            description: "Template for spawning an agent with specific capabilities".to_string(),
            required_fields: vec![
                "id".into(), "role".into(), "goal".into(), "backstory".into(),
                "llm".into(), "domain".into(),
            ],
            optional_fields: vec![
                "skills".into(), "tools".into(), "max_iter".into(),
                "allow_delegation".into(), "config".into(),
            ],
            domain: None,
            example: None,
        });

        self.register(DtoSchema {
            name: "savant_entry".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            content_type: DtoContentType::Json,
            description: "A registered savant instance with live state".to_string(),
            required_fields: vec![
                "id".into(), "domain".into(), "skills".into(), "blueprint_id".into(),
                "busy".into(), "performance_score".into(),
            ],
            optional_fields: vec![
                "current_task".into(), "delegation_targets".into(), "auto_spawned".into(),
            ],
            domain: None,
            example: None,
        });

        self.register(DtoSchema {
            name: "cross_domain_delegation".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            content_type: DtoContentType::Delegation,
            description: "Record of a cross-domain delegation between savants".to_string(),
            required_fields: vec![
                "id".into(), "from_savant".into(), "from_domain".into(),
                "to_savant".into(), "to_domain".into(), "task_description".into(),
            ],
            optional_fields: vec!["success".into(), "result".into()],
            domain: None,
            example: None,
        });

        self.register(DtoSchema {
            name: "routing_decision".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            content_type: DtoContentType::Json,
            description: "Result of skill-based routing to a savant".to_string(),
            required_fields: vec![
                "savant_id".into(), "match_score".into(), "domain".into(),
            ],
            optional_fields: vec!["matched_skills".into(), "auto_spawned".into()],
            domain: None,
            example: None,
        });
    }

    // -----------------------------------------------------------------------
    // Schema management
    // -----------------------------------------------------------------------

    /// Register a new schema.
    pub fn register(&mut self, schema: DtoSchema) {
        self.schemas.insert(schema.name.clone(), schema);
    }

    /// Get a schema by name.
    pub fn get_schema(&self, name: &str) -> Option<&DtoSchema> {
        self.schemas.get(name)
    }

    /// Get all registered schema names.
    pub fn schema_names(&self) -> Vec<&str> {
        self.schemas.keys().map(|s| s.as_str()).collect()
    }

    /// Total number of registered schemas.
    pub fn schema_count(&self) -> usize {
        self.schemas.len()
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    /// Validate a JSON value against a named schema.
    pub fn validate(&self, schema_name: &str, data: &Value) -> ValidationResult {
        let schema = match self.schemas.get(schema_name) {
            Some(s) => s,
            None => return ValidationResult {
                valid: false,
                schema: schema_name.to_string(),
                errors: vec![format!("Schema '{}' not registered", schema_name)],
                warnings: Vec::new(),
            },
        };

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check required fields
        if let Some(obj) = data.as_object() {
            for field in &schema.required_fields {
                if !obj.contains_key(field) {
                    errors.push(format!("Missing required field: '{}'", field));
                }
            }

            // Warn about unknown fields
            let all_known: Vec<&str> = schema.required_fields.iter()
                .chain(schema.optional_fields.iter())
                .map(|s| s.as_str())
                .collect();
            for key in obj.keys() {
                if !all_known.contains(&key.as_str()) {
                    warnings.push(format!("Unknown field: '{}'", key));
                }
            }
        } else {
            errors.push("Data must be a JSON object".to_string());
        }

        ValidationResult {
            valid: errors.is_empty(),
            schema: schema_name.to_string(),
            errors,
            warnings,
        }
    }

    /// Validate an envelope's payload against its declared schema.
    pub fn validate_envelope(&self, envelope: &DtoEnvelope) -> ValidationResult {
        self.validate(&envelope.schema, &envelope.payload)
    }

    // -----------------------------------------------------------------------
    // Compatibility checking
    // -----------------------------------------------------------------------

    /// Check if two schemas are compatible (same major version).
    pub fn schemas_compatible(&mut self, schema_a: &str, schema_b: &str) -> bool {
        let key = (schema_a.to_string(), schema_b.to_string());
        if let Some(&cached) = self.compatibility_cache.get(&key) {
            return cached;
        }

        let compatible = match (self.schemas.get(schema_a), self.schemas.get(schema_b)) {
            (Some(a), Some(b)) => {
                // Compatible if same name and version-compatible, or
                // same content type (structural compatibility)
                (a.name == b.name && a.version.is_compatible_with(&b.version))
                    || a.content_type == b.content_type
            }
            _ => false,
        };

        self.compatibility_cache.insert(key, compatible);
        compatible
    }

    /// Check if an agent's output schema is compatible with another agent's input.
    pub fn agents_compatible(
        &mut self,
        output_schema: &str,
        input_schema: &str,
    ) -> bool {
        self.schemas_compatible(output_schema, input_schema)
    }

    // -----------------------------------------------------------------------
    // Envelope factory
    // -----------------------------------------------------------------------

    /// Create and log an envelope.
    pub fn create_envelope(
        &mut self,
        schema: &str,
        content_type: DtoContentType,
        payload: Value,
    ) -> DtoEnvelope {
        let envelope = DtoEnvelope::new(schema, content_type, payload);
        self.envelope_log.push(envelope.clone());
        envelope
    }

    /// Wrap a task in a typed envelope.
    pub fn wrap_task(&mut self, task: &OrchestratedTask) -> DtoEnvelope {
        let envelope = DtoEnvelope::from_task(task);
        self.envelope_log.push(envelope.clone());
        envelope
    }

    /// Wrap a delegation request in a typed envelope.
    pub fn wrap_delegation_request(&mut self, request: &DelegationRequest) -> DtoEnvelope {
        let envelope = DtoEnvelope::from_delegation_request(request);
        self.envelope_log.push(envelope.clone());
        envelope
    }

    /// Wrap feedback in a typed envelope.
    pub fn wrap_feedback(&mut self, feedback: &AgentFeedback) -> DtoEnvelope {
        let envelope = DtoEnvelope::from_feedback(feedback);
        self.envelope_log.push(envelope.clone());
        envelope
    }

    /// Wrap a capability update in a typed envelope.
    pub fn wrap_capability_update(&mut self, update: &CapabilityUpdate) -> DtoEnvelope {
        let envelope = DtoEnvelope::from_capability_update(update);
        self.envelope_log.push(envelope.clone());
        envelope
    }

    /// Wrap an orchestration event in a typed envelope.
    pub fn wrap_event(&mut self, event: &OrchestrationEvent) -> DtoEnvelope {
        let envelope = DtoEnvelope::from_event(event);
        self.envelope_log.push(envelope.clone());
        envelope
    }

    // -----------------------------------------------------------------------
    // Introspection
    // -----------------------------------------------------------------------

    /// Get the envelope audit log.
    pub fn envelope_log(&self) -> &[DtoEnvelope] {
        &self.envelope_log
    }

    /// Clear the envelope log.
    pub fn clear_log(&mut self) {
        self.envelope_log.clear();
    }

    /// Get schemas for a specific content type.
    pub fn schemas_for_type(&self, content_type: DtoContentType) -> Vec<&DtoSchema> {
        self.schemas.values()
            .filter(|s| s.content_type == content_type)
            .collect()
    }

    /// Get schemas for a specific domain.
    pub fn schemas_for_domain(&self, domain: SavantDomain) -> Vec<&DtoSchema> {
        self.schemas.values()
            .filter(|s| s.domain == Some(domain))
            .collect()
    }
}

impl Default for DtoRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DtoRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DtoRegistry")
            .field("schemas", &self.schemas.len())
            .field("envelope_log", &self.envelope_log.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta_agents::delegation::{
        CapabilityUpdateTrigger, TaskOutcome,
    };
    use crate::meta_agents::types::TaskPriority;

    #[test]
    fn test_registry_creation() {
        let registry = DtoRegistry::new();
        assert!(registry.schema_count() >= 10);
        assert!(registry.get_schema("delegation_request").is_some());
        assert!(registry.get_schema("orchestrated_task").is_some());
        assert!(registry.get_schema("agent_feedback").is_some());
        assert!(registry.get_schema("skill_descriptor").is_some());
        assert!(registry.get_schema("capability_update").is_some());
        assert!(registry.get_schema("orchestration_event").is_some());
    }

    #[test]
    fn test_schema_names() {
        let registry = DtoRegistry::new();
        let names = registry.schema_names();
        assert!(names.contains(&"delegation_request"));
        assert!(names.contains(&"agent_blueprint"));
        assert!(names.contains(&"routing_decision"));
    }

    #[test]
    fn test_envelope_from_delegation_request() {
        let request = DelegationRequest::new("agent-1", "Research Rust patterns")
            .with_domain(SavantDomain::Research)
            .with_priority(TaskPriority::High);

        let envelope = DtoEnvelope::from_delegation_request(&request);
        assert_eq!(envelope.schema, "delegation_request");
        assert_eq!(envelope.content_type, DtoContentType::Delegation);
        assert_eq!(envelope.source_agent, Some("agent-1".to_string()));

        // Should be extractable back
        let extracted: DelegationRequest = envelope.extract().unwrap();
        assert_eq!(extracted.from_agent, "agent-1");
        assert_eq!(extracted.target_domain, Some(SavantDomain::Research));
    }

    #[test]
    fn test_envelope_from_task() {
        let task = OrchestratedTask::new("Test task")
            .with_domain(SavantDomain::Engineering)
            .with_priority(TaskPriority::Critical);

        let envelope = DtoEnvelope::from_task(&task);
        assert_eq!(envelope.schema, "orchestrated_task");
        assert_eq!(envelope.source_domain, Some(SavantDomain::Engineering));

        let extracted: OrchestratedTask = envelope.extract().unwrap();
        assert_eq!(extracted.description, "Test task");
    }

    #[test]
    fn test_envelope_from_feedback() {
        let feedback = AgentFeedback::success("agent-1", "task-1")
            .with_relevant_skills(vec!["web_research".into()]);

        let envelope = DtoEnvelope::from_feedback(&feedback);
        assert_eq!(envelope.schema, "agent_feedback");
        assert_eq!(envelope.source_agent, Some("agent-1".to_string()));

        let extracted: AgentFeedback = envelope.extract().unwrap();
        assert_eq!(extracted.outcome, TaskOutcome::Success);
    }

    #[test]
    fn test_envelope_from_skill() {
        let skill = SkillDescriptor::new("web_research", "Web Research", "Search the web");
        let envelope = DtoEnvelope::from_skill(&skill);
        assert_eq!(envelope.schema, "skill_descriptor");
        assert_eq!(envelope.content_type, DtoContentType::Skill);
    }

    #[test]
    fn test_envelope_from_capability_update() {
        let update = CapabilityUpdate {
            agent_id: "agent-1".to_string(),
            skills: vec![SkillDescriptor::new("s1", "Skill", "Desc")],
            performance_score: 0.95,
            domain: SavantDomain::Research,
            trigger: CapabilityUpdateTrigger::TaskOutcome,
        };

        let envelope = DtoEnvelope::from_capability_update(&update);
        assert_eq!(envelope.source_agent, Some("agent-1".to_string()));
        assert_eq!(envelope.source_domain, Some(SavantDomain::Research));
    }

    #[test]
    fn test_envelope_from_event() {
        let event = OrchestrationEvent::AgentSpawned {
            agent_id: "a-1".into(),
            domain: SavantDomain::Security,
            blueprint_id: "bp-1".into(),
            skills: vec!["threat_modeling".into()],
        };

        let envelope = DtoEnvelope::from_event(&event);
        assert_eq!(envelope.schema, "orchestration_event");
        assert_eq!(envelope.content_type, DtoContentType::Event);
    }

    #[test]
    fn test_text_envelope() {
        let env = DtoEnvelope::text("custom_text", "Hello world");
        assert_eq!(env.content_type, DtoContentType::Text);
        assert_eq!(env.payload, Value::String("Hello world".to_string()));
    }

    #[test]
    fn test_json_envelope() {
        let data = serde_json::json!({"key": "value", "count": 42});
        let env = DtoEnvelope::json("custom_json", data.clone());
        assert_eq!(env.content_type, DtoContentType::Json);
        assert_eq!(env.payload, data);
    }

    #[test]
    fn test_validate_valid_data() {
        let registry = DtoRegistry::new();
        let data = serde_json::json!({
            "id": "req-1",
            "from_agent": "agent-1",
            "task_description": "Do something",
            "priority": "high"
        });
        let result = registry.validate("delegation_request", &data);
        assert!(result.valid, "Expected valid, got errors: {:?}", result.errors);
    }

    #[test]
    fn test_validate_missing_required() {
        let registry = DtoRegistry::new();
        let data = serde_json::json!({
            "id": "req-1",
            // missing from_agent, task_description, priority
        });
        let result = registry.validate("delegation_request", &data);
        assert!(!result.valid);
        assert!(result.errors.len() >= 2);
    }

    #[test]
    fn test_validate_unknown_schema() {
        let registry = DtoRegistry::new();
        let data = serde_json::json!({});
        let result = registry.validate("nonexistent_schema", &data);
        assert!(!result.valid);
        assert!(result.errors[0].contains("not registered"));
    }

    #[test]
    fn test_validate_envelope() {
        let registry = DtoRegistry::new();
        let task = OrchestratedTask::new("Test task")
            .with_priority(TaskPriority::High);
        let envelope = DtoEnvelope::from_task(&task);
        let result = registry.validate_envelope(&envelope);
        assert!(result.valid, "Expected valid, got errors: {:?}", result.errors);
    }

    #[test]
    fn test_schema_compatibility() {
        let mut registry = DtoRegistry::new();
        // Same schema is compatible
        assert!(registry.schemas_compatible("delegation_request", "delegation_request"));
        // Same content type is compatible
        assert!(registry.schemas_compatible("delegation_request", "delegation_response"));
    }

    #[test]
    fn test_wrap_and_log() {
        let mut registry = DtoRegistry::new();
        let task = OrchestratedTask::new("Tracked task");
        registry.wrap_task(&task);

        assert_eq!(registry.envelope_log().len(), 1);
        assert_eq!(registry.envelope_log()[0].schema, "orchestrated_task");
    }

    #[test]
    fn test_clear_log() {
        let mut registry = DtoRegistry::new();
        registry.wrap_task(&OrchestratedTask::new("Task 1"));
        registry.wrap_task(&OrchestratedTask::new("Task 2"));
        assert_eq!(registry.envelope_log().len(), 2);

        registry.clear_log();
        assert!(registry.envelope_log().is_empty());
    }

    #[test]
    fn test_schemas_for_type() {
        let registry = DtoRegistry::new();
        let delegation_schemas = registry.schemas_for_type(DtoContentType::Delegation);
        assert!(delegation_schemas.len() >= 2); // request + response + cross_domain
    }

    #[test]
    fn test_content_type_display() {
        assert_eq!(DtoContentType::Json.to_string(), "application/json");
        assert_eq!(DtoContentType::Text.to_string(), "text/plain");
        assert_eq!(DtoContentType::Delegation.to_string(), "application/x-delegation");
        assert_eq!(DtoContentType::Event.to_string(), "application/x-event");
    }

    #[test]
    fn test_schema_version_compatibility() {
        let v1 = SchemaVersion::new(1, 0, 0);
        let v1_1 = SchemaVersion::new(1, 1, 0);
        let v2 = SchemaVersion::new(2, 0, 0);

        assert!(v1.is_compatible_with(&v1_1));
        assert!(!v1.is_compatible_with(&v2));
    }

    #[test]
    fn test_schema_version_display() {
        let v = SchemaVersion::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn test_envelope_builder_chain() {
        let env = DtoEnvelope::json("test", serde_json::json!({}))
            .from_agent("agent-1")
            .to_agent("agent-2")
            .with_domain(SavantDomain::Engineering)
            .with_metadata("key", Value::String("value".to_string()));

        assert_eq!(env.source_agent, Some("agent-1".to_string()));
        assert_eq!(env.target_agent, Some("agent-2".to_string()));
        assert_eq!(env.source_domain, Some(SavantDomain::Engineering));
        assert_eq!(env.metadata.get("key"), Some(&Value::String("value".to_string())));
    }

    #[test]
    fn test_registry_debug() {
        let registry = DtoRegistry::new();
        let debug = format!("{:?}", registry);
        assert!(debug.contains("DtoRegistry"));
        assert!(debug.contains("schemas"));
    }

    #[test]
    fn test_register_custom_schema() {
        let mut registry = DtoRegistry::new();
        let initial_count = registry.schema_count();

        registry.register(DtoSchema {
            name: "custom_type".to_string(),
            version: SchemaVersion::new(1, 0, 0),
            content_type: DtoContentType::Json,
            description: "A custom domain-specific DTO".to_string(),
            required_fields: vec!["value".into()],
            optional_fields: vec!["metadata".into()],
            domain: Some(SavantDomain::Engineering),
            example: Some(serde_json::json!({"value": 42})),
        });

        assert_eq!(registry.schema_count(), initial_count + 1);
        assert!(registry.get_schema("custom_type").is_some());
    }

    #[test]
    fn test_delegation_roundtrip_via_envelope() {
        let request = DelegationRequest::new("agent-1", "Analyze security")
            .with_domain(SavantDomain::Security)
            .with_skills(vec!["threat_modeling".into()])
            .with_context("Review this codebase")
            .with_priority(TaskPriority::Critical);

        let envelope = DtoEnvelope::from_delegation_request(&request);
        let roundtripped: DelegationRequest = envelope.extract().unwrap();

        assert_eq!(roundtripped.from_agent, request.from_agent);
        assert_eq!(roundtripped.task_description, request.task_description);
        assert_eq!(roundtripped.target_domain, request.target_domain);
        assert_eq!(roundtripped.required_skills, request.required_skills);
        assert_eq!(roundtripped.priority, request.priority);
    }
}
