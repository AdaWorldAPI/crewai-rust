//! Meta-agent DTOs and types for savant orchestration.
//!
//! Provides the type system for meta-agents — higher-order agents that spawn,
//! orchestrate, and adjust the skills of subordinate agents. These types enable
//! automatic task distribution across agents with dynamically-adjusted A2A
//! agent cards.
//!
//! # Architecture
//!
//! ```text
//! MetaOrchestrator
//!   ├── SavantAgent (domain expert, generates sub-task plans)
//!   │     ├── AgentBlueprint (defines how to spawn a worker)
//!   │     └── SkillDescriptor (declares what a worker can do)
//!   ├── AgentPool (manages spawned agents)
//!   │     └── SpawnedAgent (agent + its live A2A card)
//!   └── TaskDistributor (routes tasks to best-fit agents)
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Skill system
// ---------------------------------------------------------------------------

/// Describes a specific skill that an agent possesses.
///
/// Skills are the atomic units of capability that agents advertise in their
/// A2A cards. The orchestrator uses skill descriptors to match tasks to
/// agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDescriptor {
    /// Unique identifier for this skill.
    pub id: String,
    /// Human-readable skill name (e.g., "web_research", "code_review").
    pub name: String,
    /// Detailed description of what this skill does.
    pub description: String,
    /// Domain tags for categorization (e.g., ["research", "analysis"]).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Input modalities this skill accepts (e.g., ["text", "image", "json"]).
    #[serde(default)]
    pub input_modes: Vec<String>,
    /// Output modalities this skill produces.
    #[serde(default)]
    pub output_modes: Vec<String>,
    /// Proficiency level from 0.0 (novice) to 1.0 (expert).
    #[serde(default = "default_proficiency")]
    pub proficiency: f64,
    /// Whether this skill requires specific tools.
    #[serde(default)]
    pub required_tools: Vec<String>,
    /// Maximum concurrent tasks this skill can handle.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: u32,
}

fn default_proficiency() -> f64 { 1.0 }
fn default_max_concurrent() -> u32 { 1 }

impl SkillDescriptor {
    /// Create a new skill descriptor with required fields.
    pub fn new(id: impl Into<String>, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            tags: Vec::new(),
            input_modes: vec!["text".to_string()],
            output_modes: vec!["text".to_string()],
            proficiency: 1.0,
            required_tools: Vec::new(),
            max_concurrent: 1,
        }
    }

    /// Builder: add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Builder: set proficiency.
    pub fn with_proficiency(mut self, p: f64) -> Self {
        self.proficiency = p.clamp(0.0, 1.0);
        self
    }

    /// Builder: add required tools.
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.required_tools = tools;
        self
    }

    /// Compute a match score against a task description.
    ///
    /// Uses keyword overlap between the task and skill tags/description.
    pub fn match_score(&self, task_description: &str) -> f64 {
        let task_lower = task_description.to_lowercase();
        let task_tokens: Vec<&str> = task_lower.split_whitespace().collect();

        let mut score = 0.0;
        let total_keywords = self.tags.len() + 1; // +1 for name

        // Check name match
        if task_lower.contains(&self.name.to_lowercase()) {
            score += 2.0;
        }

        // Check tag matches
        for tag in &self.tags {
            if task_lower.contains(&tag.to_lowercase()) {
                score += 1.0;
            }
        }

        // Check description word overlap
        let desc_lower = self.description.to_lowercase();
        let desc_tokens: Vec<&str> = desc_lower.split_whitespace().collect();
        let overlap = task_tokens.iter()
            .filter(|t| desc_tokens.contains(t))
            .count();
        if !desc_tokens.is_empty() {
            score += (overlap as f64 / desc_tokens.len() as f64) * 2.0;
        }

        // Weight by proficiency
        score * self.proficiency
    }
}

// ---------------------------------------------------------------------------
// Agent blueprint
// ---------------------------------------------------------------------------

/// Domain expertise category for savant agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavantDomain {
    /// Research and information gathering.
    Research,
    /// Software engineering and code generation.
    Engineering,
    /// Data analysis and visualization.
    DataAnalysis,
    /// Content writing and editing.
    ContentCreation,
    /// Strategic planning and project management.
    Planning,
    /// Quality assurance and testing.
    QualityAssurance,
    /// Security analysis and auditing.
    Security,
    /// DevOps, deployment, and infrastructure.
    DevOps,
    /// Design and UX.
    Design,
    /// Chess analysis and game strategy.
    Chess,
    /// General-purpose (no specific domain).
    General,
}

impl std::fmt::Display for SavantDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Research => write!(f, "research"),
            Self::Engineering => write!(f, "engineering"),
            Self::DataAnalysis => write!(f, "data_analysis"),
            Self::ContentCreation => write!(f, "content_creation"),
            Self::Planning => write!(f, "planning"),
            Self::QualityAssurance => write!(f, "quality_assurance"),
            Self::Security => write!(f, "security"),
            Self::DevOps => write!(f, "devops"),
            Self::Design => write!(f, "design"),
            Self::Chess => write!(f, "chess"),
            Self::General => write!(f, "general"),
        }
    }
}

/// Blueprint for spawning an agent with specific capabilities.
///
/// Used by the orchestrator to dynamically create agents configured
/// for particular tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBlueprint {
    /// Unique identifier for this blueprint.
    pub id: String,
    /// The role the spawned agent will assume.
    pub role: String,
    /// The goal for the spawned agent.
    pub goal: String,
    /// Backstory providing context for the agent's expertise.
    pub backstory: String,
    /// LLM identifier (e.g., "openai/gpt-4o", "anthropic/claude-opus-4-5-20251101").
    pub llm: String,
    /// Skills this agent blueprint provides.
    pub skills: Vec<SkillDescriptor>,
    /// Tools the agent should have access to.
    #[serde(default)]
    pub tools: Vec<String>,
    /// Domain expertise.
    pub domain: SavantDomain,
    /// Maximum iterations for the agent.
    #[serde(default = "default_max_iter")]
    pub max_iter: i32,
    /// Whether to allow delegation to other agents.
    #[serde(default)]
    pub allow_delegation: bool,
    /// Extra configuration overrides.
    #[serde(default)]
    pub config: HashMap<String, Value>,
}

fn default_max_iter() -> i32 { 25 }

impl AgentBlueprint {
    /// Create a new agent blueprint.
    pub fn new(
        role: impl Into<String>,
        goal: impl Into<String>,
        backstory: impl Into<String>,
        llm: impl Into<String>,
        domain: SavantDomain,
    ) -> Self {
        let role = role.into();
        Self {
            id: Uuid::new_v4().to_string(),
            role: role.clone(),
            goal: goal.into(),
            backstory: backstory.into(),
            llm: llm.into(),
            skills: Vec::new(),
            tools: Vec::new(),
            domain,
            max_iter: 25,
            allow_delegation: false,
            config: HashMap::new(),
        }
    }

    /// Builder: add a skill.
    pub fn with_skill(mut self, skill: SkillDescriptor) -> Self {
        self.skills.push(skill);
        self
    }

    /// Builder: add tools.
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    /// Builder: allow delegation.
    pub fn with_delegation(mut self) -> Self {
        self.allow_delegation = true;
        self
    }
}

// ---------------------------------------------------------------------------
// Task distribution
// ---------------------------------------------------------------------------

/// Priority level for orchestrated tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Medium
    }
}

/// Current status of an orchestrated task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrchestratedTaskStatus {
    /// Task is queued and waiting for assignment.
    Pending,
    /// Task has been assigned to an agent.
    Assigned,
    /// Agent is actively executing this task.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task failed during execution.
    Failed,
    /// Task was cancelled.
    Cancelled,
}

impl Default for OrchestratedTaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// A task managed by the meta-orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratedTask {
    /// Unique task identifier.
    pub id: String,
    /// Human-readable task description.
    pub description: String,
    /// Task context (from parent task or crew).
    pub context: Option<String>,
    /// Current status.
    pub status: OrchestratedTaskStatus,
    /// Priority level.
    pub priority: TaskPriority,
    /// IDs of tasks this depends on (must complete first).
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Required skill tags for matching.
    #[serde(default)]
    pub required_skills: Vec<String>,
    /// Preferred domain for the executing agent.
    pub preferred_domain: Option<SavantDomain>,
    /// Agent ID assigned to execute this task.
    pub assigned_agent: Option<String>,
    /// Task output (populated on completion).
    pub output: Option<String>,
    /// Error message (populated on failure).
    pub error: Option<String>,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl OrchestratedTask {
    /// Create a new orchestrated task.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.into(),
            context: None,
            status: OrchestratedTaskStatus::Pending,
            priority: TaskPriority::Medium,
            dependencies: Vec::new(),
            required_skills: Vec::new(),
            preferred_domain: None,
            assigned_agent: None,
            output: None,
            error: None,
            metadata: HashMap::new(),
        }
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

    /// Builder: add dependencies.
    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    /// Builder: set required skills.
    pub fn with_required_skills(mut self, skills: Vec<String>) -> Self {
        self.required_skills = skills;
        self
    }

    /// Builder: set preferred domain.
    pub fn with_domain(mut self, domain: SavantDomain) -> Self {
        self.preferred_domain = Some(domain);
        self
    }

    /// Check if all dependencies are satisfied.
    pub fn dependencies_satisfied(&self, completed_ids: &[String]) -> bool {
        self.dependencies.iter().all(|dep| completed_ids.contains(dep))
    }

    /// Mark the task as assigned.
    pub fn assign(&mut self, agent_id: &str) {
        self.status = OrchestratedTaskStatus::Assigned;
        self.assigned_agent = Some(agent_id.to_string());
    }

    /// Mark the task as running.
    pub fn start(&mut self) {
        self.status = OrchestratedTaskStatus::Running;
    }

    /// Mark the task as completed with output.
    pub fn complete(&mut self, output: String) {
        self.status = OrchestratedTaskStatus::Completed;
        self.output = Some(output);
    }

    /// Mark the task as failed with an error.
    pub fn fail(&mut self, error: String) {
        self.status = OrchestratedTaskStatus::Failed;
        self.error = Some(error);
    }
}

// ---------------------------------------------------------------------------
// Spawned agent state
// ---------------------------------------------------------------------------

/// Tracks a spawned agent and its current state in the pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedAgentState {
    /// Agent identifier.
    pub id: String,
    /// Blueprint ID this agent was spawned from.
    pub blueprint_id: String,
    /// Current skills (may be adjusted by orchestrator).
    pub skills: Vec<SkillDescriptor>,
    /// Current domain assignment.
    pub domain: SavantDomain,
    /// Whether the agent is currently busy.
    pub busy: bool,
    /// Number of tasks completed.
    pub tasks_completed: u32,
    /// Number of tasks failed.
    pub tasks_failed: u32,
    /// Running performance score (0.0 - 1.0).
    pub performance_score: f64,
    /// Current task ID (if busy).
    pub current_task: Option<String>,
}

impl SpawnedAgentState {
    /// Create state for a newly spawned agent.
    pub fn new(id: impl Into<String>, blueprint: &AgentBlueprint) -> Self {
        Self {
            id: id.into(),
            blueprint_id: blueprint.id.clone(),
            skills: blueprint.skills.clone(),
            domain: blueprint.domain,
            busy: false,
            tasks_completed: 0,
            tasks_failed: 0,
            performance_score: 1.0,
            current_task: None,
        }
    }

    /// Mark agent as busy with a task.
    pub fn assign_task(&mut self, task_id: &str) {
        self.busy = true;
        self.current_task = Some(task_id.to_string());
    }

    /// Mark agent as available after completing a task.
    pub fn complete_task(&mut self, success: bool) {
        self.busy = false;
        self.current_task = None;
        if success {
            self.tasks_completed += 1;
            // Adjust performance score upward (exponential moving average)
            self.performance_score = self.performance_score * 0.9 + 0.1;
        } else {
            self.tasks_failed += 1;
            // Adjust performance score downward
            self.performance_score = self.performance_score * 0.9;
        }
    }

    /// Adjust skills based on orchestrator feedback.
    pub fn adjust_skills(&mut self, new_skills: Vec<SkillDescriptor>) {
        self.skills = new_skills;
    }

    /// Add a skill to this agent.
    pub fn add_skill(&mut self, skill: SkillDescriptor) {
        // Don't add duplicate skill IDs
        if !self.skills.iter().any(|s| s.id == skill.id) {
            self.skills.push(skill);
        }
    }

    /// Remove a skill by ID.
    pub fn remove_skill(&mut self, skill_id: &str) {
        self.skills.retain(|s| s.id != skill_id);
    }

    /// Best match score across all skills for a task.
    pub fn best_skill_match(&self, task_description: &str) -> f64 {
        self.skills.iter()
            .map(|s| s.match_score(task_description))
            .fold(0.0f64, f64::max)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_descriptor_creation() {
        let skill = SkillDescriptor::new("research_web", "Web Research", "Search the web for information")
            .with_tags(vec!["research".to_string(), "web".to_string()])
            .with_proficiency(0.9);
        assert_eq!(skill.id, "research_web");
        assert_eq!(skill.proficiency, 0.9);
        assert_eq!(skill.tags.len(), 2);
    }

    #[test]
    fn test_skill_match_score() {
        let skill = SkillDescriptor::new("web_research", "Web Research", "Search the internet for information and facts")
            .with_tags(vec!["research".to_string(), "web".to_string(), "search".to_string()]);

        let score = skill.match_score("research the web for information about Rust");
        assert!(score > 0.0, "Expected positive score, got {}", score);

        let score_unrelated = skill.match_score("cook pasta for dinner");
        assert!(score > score_unrelated, "Relevant task should score higher");
    }

    #[test]
    fn test_agent_blueprint_creation() {
        let bp = AgentBlueprint::new(
            "Senior Researcher",
            "Find accurate information",
            "Expert at web research",
            "openai/gpt-4o",
            SavantDomain::Research,
        )
        .with_skill(SkillDescriptor::new("s1", "Web Search", "Search the web"))
        .with_tools(vec!["SerperDevTool".to_string()])
        .with_delegation();

        assert_eq!(bp.role, "Senior Researcher");
        assert!(bp.allow_delegation);
        assert_eq!(bp.skills.len(), 1);
        assert_eq!(bp.tools.len(), 1);
        assert_eq!(bp.domain, SavantDomain::Research);
    }

    #[test]
    fn test_orchestrated_task_lifecycle() {
        let mut task = OrchestratedTask::new("Research Rust async patterns")
            .with_priority(TaskPriority::High)
            .with_domain(SavantDomain::Research);

        assert_eq!(task.status, OrchestratedTaskStatus::Pending);

        task.assign("agent-001");
        assert_eq!(task.status, OrchestratedTaskStatus::Assigned);

        task.start();
        assert_eq!(task.status, OrchestratedTaskStatus::Running);

        task.complete("Found 10 patterns".to_string());
        assert_eq!(task.status, OrchestratedTaskStatus::Completed);
        assert!(task.output.is_some());
    }

    #[test]
    fn test_task_dependencies() {
        let task = OrchestratedTask::new("Summarize results")
            .with_dependencies(vec!["task-1".to_string(), "task-2".to_string()]);

        let completed = vec!["task-1".to_string()];
        assert!(!task.dependencies_satisfied(&completed));

        let completed = vec!["task-1".to_string(), "task-2".to_string()];
        assert!(task.dependencies_satisfied(&completed));
    }

    #[test]
    fn test_spawned_agent_state() {
        let bp = AgentBlueprint::new("Tester", "Test things", "QA expert", "openai/gpt-4o-mini", SavantDomain::QualityAssurance);
        let mut state = SpawnedAgentState::new("agent-001", &bp);

        assert!(!state.busy);
        assert_eq!(state.performance_score, 1.0);

        state.assign_task("task-1");
        assert!(state.busy);
        assert_eq!(state.current_task, Some("task-1".to_string()));

        state.complete_task(true);
        assert!(!state.busy);
        assert_eq!(state.tasks_completed, 1);
        assert!(state.performance_score > 0.9);
    }

    #[test]
    fn test_spawned_agent_skill_adjustment() {
        let bp = AgentBlueprint::new("Worker", "Work", "Backstory", "openai/gpt-4o", SavantDomain::General);
        let mut state = SpawnedAgentState::new("agent-001", &bp);

        state.add_skill(SkillDescriptor::new("s1", "Skill 1", "First skill"));
        state.add_skill(SkillDescriptor::new("s2", "Skill 2", "Second skill"));
        assert_eq!(state.skills.len(), 2);

        // Adding duplicate should be no-op
        state.add_skill(SkillDescriptor::new("s1", "Skill 1", "First skill"));
        assert_eq!(state.skills.len(), 2);

        state.remove_skill("s1");
        assert_eq!(state.skills.len(), 1);
        assert_eq!(state.skills[0].id, "s2");
    }

    #[test]
    fn test_savant_domain_display() {
        assert_eq!(SavantDomain::Research.to_string(), "research");
        assert_eq!(SavantDomain::Engineering.to_string(), "engineering");
        assert_eq!(SavantDomain::DataAnalysis.to_string(), "data_analysis");
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Medium);
        assert!(TaskPriority::Medium > TaskPriority::Low);
    }
}
