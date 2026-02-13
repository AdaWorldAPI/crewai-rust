//! Delegation protocol DTOs for agent-to-agent communication.
//!
//! Defines the message types, request/response envelopes, and feedback
//! structures that agents use to delegate work to each other and report
//! results back through the orchestrator.
//!
//! # Protocol Flow
//!
//! ```text
//! Agent A                Orchestrator               Agent B
//!   │                        │                         │
//!   ├─ DelegationRequest ──►│                         │
//!   │                        ├─ DelegationDispatch ──►│
//!   │                        │                         ├── (executes)
//!   │                        │◄─ DelegationResponse ──┤
//!   │◄─ DelegationResult ───┤                         │
//!   │                        │                         │
//!   │   AgentFeedback ─────►│ (skill adjustment)      │
//!   │                        │                         │
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::types::{SavantDomain, SkillDescriptor, TaskPriority};

// ---------------------------------------------------------------------------
// Delegation request / response
// ---------------------------------------------------------------------------

/// A request from one agent to delegate a sub-task to another agent.
///
/// Created when an agent determines it needs help from a specialist
/// (e.g., a researcher needing a security expert to audit findings).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRequest {
    /// Unique request ID.
    pub id: String,
    /// ID of the requesting agent.
    pub from_agent: String,
    /// Preferred target agent ID (if known), or None for auto-routing.
    pub to_agent: Option<String>,
    /// Preferred target domain (if auto-routing).
    pub target_domain: Option<SavantDomain>,
    /// Required skills on the target agent.
    #[serde(default)]
    pub required_skills: Vec<String>,
    /// The task description to delegate.
    pub task_description: String,
    /// Context from the delegating agent's current work.
    pub context: Option<String>,
    /// Priority for the delegated task.
    #[serde(default)]
    pub priority: TaskPriority,
    /// Maximum turns the delegate should use.
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    /// Arbitrary metadata for extensions.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

fn default_max_turns() -> u32 { 10 }

impl DelegationRequest {
    /// Create a new delegation request.
    pub fn new(
        from_agent: impl Into<String>,
        task_description: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from_agent: from_agent.into(),
            to_agent: None,
            target_domain: None,
            required_skills: Vec::new(),
            task_description: task_description.into(),
            context: None,
            priority: TaskPriority::Medium,
            max_turns: 10,
            metadata: HashMap::new(),
        }
    }

    /// Builder: set target agent.
    pub fn to(mut self, agent_id: impl Into<String>) -> Self {
        self.to_agent = Some(agent_id.into());
        self
    }

    /// Builder: set target domain.
    pub fn with_domain(mut self, domain: SavantDomain) -> Self {
        self.target_domain = Some(domain);
        self
    }

    /// Builder: set required skills.
    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.required_skills = skills;
        self
    }

    /// Builder: set context.
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context = Some(ctx.into());
        self
    }

    /// Builder: set priority.
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }
}

/// The orchestrator's internal dispatch when routing a delegation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationDispatch {
    /// The original request.
    pub request: DelegationRequest,
    /// Agent ID selected to handle the delegation.
    pub assigned_agent: String,
    /// Match score that justified the assignment.
    pub match_score: f64,
    /// Whether the agent was auto-spawned for this delegation.
    pub auto_spawned: bool,
}

/// Response from the delegate agent back to the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationResponse {
    /// Request ID being responded to.
    pub request_id: String,
    /// Agent that executed the delegation.
    pub from_agent: String,
    /// Whether the delegation succeeded.
    pub success: bool,
    /// Result text (on success).
    pub result: Option<String>,
    /// Error message (on failure).
    pub error: Option<String>,
    /// Skills actually used during execution.
    #[serde(default)]
    pub skills_used: Vec<String>,
    /// Number of iterations used.
    pub iterations_used: u32,
    /// Metadata produced during execution.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl DelegationResponse {
    /// Create a successful response.
    pub fn success(
        request_id: impl Into<String>,
        from_agent: impl Into<String>,
        result: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            from_agent: from_agent.into(),
            success: true,
            result: Some(result.into()),
            error: None,
            skills_used: Vec::new(),
            iterations_used: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create a failure response.
    pub fn failure(
        request_id: impl Into<String>,
        from_agent: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            from_agent: from_agent.into(),
            success: false,
            result: None,
            error: Some(error.into()),
            skills_used: Vec::new(),
            iterations_used: 0,
            metadata: HashMap::new(),
        }
    }
}

/// Result delivered back to the requesting agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationResult {
    /// Original request ID.
    pub request_id: String,
    /// Whether the delegation succeeded.
    pub success: bool,
    /// Result text (on success).
    pub result: Option<String>,
    /// Error message (on failure).
    pub error: Option<String>,
    /// Agent that handled the delegation.
    pub handled_by: String,
}

// ---------------------------------------------------------------------------
// Agent feedback (for skill learning)
// ---------------------------------------------------------------------------

/// Outcome assessment for a completed task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskOutcome {
    /// Task completed successfully with good quality.
    ExcellentSuccess,
    /// Task completed successfully.
    Success,
    /// Task completed but with mediocre quality.
    PartialSuccess,
    /// Task failed.
    Failure,
    /// Task timed out.
    Timeout,
}

/// Feedback about an agent's performance on a specific task.
///
/// Used by the orchestrator to adjust skills and A2A cards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFeedback {
    /// ID of the feedback entry.
    pub id: String,
    /// Agent that was evaluated.
    pub agent_id: String,
    /// Task that was executed.
    pub task_id: String,
    /// Overall outcome.
    pub outcome: TaskOutcome,
    /// Skills that were relevant to this task.
    #[serde(default)]
    pub relevant_skills: Vec<String>,
    /// Skills that were discovered as needed but missing.
    #[serde(default)]
    pub missing_skills: Vec<String>,
    /// Suggested new skills to add to the agent.
    #[serde(default)]
    pub suggested_skills: Vec<SkillDescriptor>,
    /// Proficiency adjustments: skill_id → delta (-1.0 to +1.0).
    #[serde(default)]
    pub proficiency_deltas: HashMap<String, f64>,
    /// Free-form notes about the execution.
    pub notes: Option<String>,
}

impl AgentFeedback {
    /// Create feedback for a successful task.
    pub fn success(agent_id: impl Into<String>, task_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.into(),
            task_id: task_id.into(),
            outcome: TaskOutcome::Success,
            relevant_skills: Vec::new(),
            missing_skills: Vec::new(),
            suggested_skills: Vec::new(),
            proficiency_deltas: HashMap::new(),
            notes: None,
        }
    }

    /// Create feedback for a failed task.
    pub fn failure(agent_id: impl Into<String>, task_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.into(),
            task_id: task_id.into(),
            outcome: TaskOutcome::Failure,
            relevant_skills: Vec::new(),
            missing_skills: Vec::new(),
            suggested_skills: Vec::new(),
            proficiency_deltas: HashMap::new(),
            notes: None,
        }
    }

    /// Builder: set relevant skills.
    pub fn with_relevant_skills(mut self, skills: Vec<String>) -> Self {
        self.relevant_skills = skills;
        self
    }

    /// Builder: set missing skills.
    pub fn with_missing_skills(mut self, skills: Vec<String>) -> Self {
        self.missing_skills = skills;
        self
    }

    /// Builder: add a proficiency delta.
    pub fn with_proficiency_delta(mut self, skill_id: impl Into<String>, delta: f64) -> Self {
        self.proficiency_deltas.insert(skill_id.into(), delta.clamp(-1.0, 1.0));
        self
    }

    /// Builder: add suggested new skills.
    pub fn with_suggested_skills(mut self, skills: Vec<SkillDescriptor>) -> Self {
        self.suggested_skills = skills;
        self
    }
}

// ---------------------------------------------------------------------------
// Orchestration lifecycle events
// ---------------------------------------------------------------------------

/// Types of events emitted during orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum OrchestrationEvent {
    /// An agent was spawned into the pool.
    AgentSpawned {
        agent_id: String,
        domain: SavantDomain,
        blueprint_id: String,
        skills: Vec<String>,
    },
    /// An agent was terminated from the pool.
    AgentTerminated {
        agent_id: String,
        reason: String,
    },
    /// A task was added to the queue.
    TaskQueued {
        task_id: String,
        description: String,
        priority: TaskPriority,
    },
    /// A task was assigned to an agent.
    TaskAssigned {
        task_id: String,
        agent_id: String,
        match_score: f64,
    },
    /// A task started executing.
    TaskStarted {
        task_id: String,
        agent_id: String,
    },
    /// A task completed successfully.
    TaskCompleted {
        task_id: String,
        agent_id: String,
        output_preview: String,
    },
    /// A task failed.
    TaskFailed {
        task_id: String,
        agent_id: String,
        error: String,
        retry_count: u32,
    },
    /// A delegation request was created.
    DelegationRequested {
        request_id: String,
        from_agent: String,
        target_domain: Option<SavantDomain>,
    },
    /// A delegation was dispatched to an agent.
    DelegationDispatched {
        request_id: String,
        to_agent: String,
        match_score: f64,
    },
    /// A delegation completed.
    DelegationCompleted {
        request_id: String,
        from_agent: String,
        success: bool,
    },
    /// Agent skills were adjusted.
    SkillsAdjusted {
        agent_id: String,
        adjustments: Vec<SkillAdjustment>,
    },
    /// An A2A card was updated.
    CardUpdated {
        agent_id: String,
        skill_count: usize,
        performance: f64,
    },
    /// Orchestration completed.
    OrchestrationFinished {
        total_tasks: usize,
        completed: usize,
        failed: usize,
        agents_used: usize,
    },
}

/// A single skill adjustment record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAdjustment {
    /// Skill ID.
    pub skill_id: String,
    /// Type of adjustment.
    pub adjustment_type: SkillAdjustmentType,
    /// Old proficiency (if applicable).
    pub old_proficiency: Option<f64>,
    /// New proficiency (if applicable).
    pub new_proficiency: Option<f64>,
}

/// Type of skill adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillAdjustmentType {
    /// Proficiency was increased.
    ProficiencyBoosted,
    /// Proficiency was decreased.
    ProficiencyReduced,
    /// Skill was added.
    SkillAdded,
    /// Skill was removed.
    SkillRemoved,
}

// ---------------------------------------------------------------------------
// Agent capability update notification
// ---------------------------------------------------------------------------

/// Notification that an agent's capabilities have changed.
///
/// Sent to the orchestrator and to peer agents when an agent's
/// A2A card is updated (e.g., after skill adjustment).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityUpdate {
    /// Agent whose capabilities changed.
    pub agent_id: String,
    /// Updated skill list.
    pub skills: Vec<SkillDescriptor>,
    /// Updated performance score.
    pub performance_score: f64,
    /// Domain of the agent.
    pub domain: SavantDomain,
    /// What triggered the update.
    pub trigger: CapabilityUpdateTrigger,
}

/// What triggered a capability update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityUpdateTrigger {
    /// Initial spawn.
    Spawn,
    /// Task completion (success or failure).
    TaskOutcome,
    /// Explicit skill adjustment by orchestrator.
    ManualAdjustment,
    /// Delegation feedback.
    DelegationFeedback,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delegation_request_builder() {
        let req = DelegationRequest::new("agent-1", "Research Rust async patterns")
            .with_domain(SavantDomain::Research)
            .with_skills(vec!["web_research".into()])
            .with_context("Building a web scraper")
            .with_priority(TaskPriority::High);

        assert_eq!(req.from_agent, "agent-1");
        assert_eq!(req.target_domain, Some(SavantDomain::Research));
        assert_eq!(req.required_skills, vec!["web_research"]);
        assert_eq!(req.priority, TaskPriority::High);
        assert!(req.context.is_some());
    }

    #[test]
    fn test_delegation_response_success() {
        let resp = DelegationResponse::success("req-1", "agent-2", "Found 10 patterns");
        assert!(resp.success);
        assert_eq!(resp.result, Some("Found 10 patterns".to_string()));
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_delegation_response_failure() {
        let resp = DelegationResponse::failure("req-1", "agent-2", "API key missing");
        assert!(!resp.success);
        assert!(resp.result.is_none());
        assert_eq!(resp.error, Some("API key missing".to_string()));
    }

    #[test]
    fn test_delegation_result() {
        let result = DelegationResult {
            request_id: "req-1".into(),
            success: true,
            result: Some("done".into()),
            error: None,
            handled_by: "agent-2".into(),
        };
        assert!(result.success);
    }

    #[test]
    fn test_agent_feedback_builder() {
        let fb = AgentFeedback::success("agent-1", "task-1")
            .with_relevant_skills(vec!["web_research".into()])
            .with_proficiency_delta("web_research", 0.05)
            .with_missing_skills(vec!["data_analysis".into()]);

        assert_eq!(fb.outcome, TaskOutcome::Success);
        assert_eq!(fb.relevant_skills, vec!["web_research"]);
        assert_eq!(fb.missing_skills, vec!["data_analysis"]);
        assert_eq!(fb.proficiency_deltas.get("web_research"), Some(&0.05));
    }

    #[test]
    fn test_orchestration_event_serialization() {
        let event = OrchestrationEvent::AgentSpawned {
            agent_id: "a-1".into(),
            domain: SavantDomain::Research,
            blueprint_id: "bp-1".into(),
            skills: vec!["web_research".into()],
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("AgentSpawned"));
        assert!(json.contains("research"));
    }

    #[test]
    fn test_capability_update() {
        let update = CapabilityUpdate {
            agent_id: "a-1".into(),
            skills: vec![SkillDescriptor::new("s1", "Skill", "Desc")],
            performance_score: 0.95,
            domain: SavantDomain::Engineering,
            trigger: CapabilityUpdateTrigger::TaskOutcome,
        };
        assert_eq!(update.trigger, CapabilityUpdateTrigger::TaskOutcome);
        assert_eq!(update.skills.len(), 1);
    }

    #[test]
    fn test_skill_adjustment() {
        let adj = SkillAdjustment {
            skill_id: "web_research".into(),
            adjustment_type: SkillAdjustmentType::ProficiencyBoosted,
            old_proficiency: Some(0.8),
            new_proficiency: Some(0.84),
        };
        assert_eq!(adj.adjustment_type, SkillAdjustmentType::ProficiencyBoosted);
    }
}
