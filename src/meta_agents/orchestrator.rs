//! Auto-attended agent spawner and orchestrator.
//!
//! The `MetaOrchestrator` is the top-level controller that:
//!
//! 1. **Analyzes** incoming high-level objectives and decomposes them into tasks
//! 2. **Spawns** agents from blueprints based on task requirements
//! 3. **Distributes** tasks to the best-matching agents using skill scoring
//! 4. **Adjusts** agent skills and A2A cards based on performance feedback
//! 5. **Orchestrates** the full lifecycle: pending → assigned → running → completed
//!
//! The orchestrator operates in an auto-attended mode — it requires no human
//! intervention once started, automatically handling task assignment, agent
//! pooling, retries, and performance-based skill adjustment.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::a2a::client::AgentCard;
use crate::agent::Agent;

use super::card_builder::{build_card_from_blueprint, build_card_from_state, update_card_skills};
use super::savants;
use super::types::{
    AgentBlueprint, OrchestratedTask, OrchestratedTaskStatus, SavantDomain,
    SkillDescriptor, SpawnedAgentState, TaskPriority,
};

// ---------------------------------------------------------------------------
// Orchestrator configuration
// ---------------------------------------------------------------------------

/// Configuration for the meta-orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Default LLM for spawned agents (e.g., "openai/gpt-4o").
    pub default_llm: String,
    /// Base URL for agent card generation.
    #[serde(default = "default_base_url")]
    pub base_url: String,
    /// Maximum number of agents in the pool.
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,
    /// Maximum retries per task before marking as failed.
    #[serde(default = "default_max_retries")]
    pub max_task_retries: u32,
    /// Whether to auto-spawn agents when no match is found.
    #[serde(default = "default_true")]
    pub auto_spawn: bool,
    /// Whether to adjust skills based on performance.
    #[serde(default = "default_true")]
    pub adaptive_skills: bool,
    /// Minimum skill match score to assign a task (0.0 - 10.0).
    #[serde(default = "default_min_score")]
    pub min_match_score: f64,
}

fn default_base_url() -> String { "http://localhost:8080".to_string() }
fn default_max_agents() -> usize { 20 }
fn default_max_retries() -> u32 { 3 }
fn default_true() -> bool { true }
fn default_min_score() -> f64 { 0.5 }

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            default_llm: "openai/gpt-4o-mini".to_string(),
            base_url: default_base_url(),
            max_agents: default_max_agents(),
            max_task_retries: default_max_retries(),
            auto_spawn: true,
            adaptive_skills: true,
            min_match_score: default_min_score(),
        }
    }
}

// ---------------------------------------------------------------------------
// MetaOrchestrator
// ---------------------------------------------------------------------------

/// The auto-attended meta-orchestrator.
///
/// Manages a pool of agents, distributes tasks based on skill matching,
/// and dynamically adjusts agent capabilities based on performance.
pub struct MetaOrchestrator {
    /// Orchestrator configuration.
    pub config: OrchestratorConfig,
    /// Registered agent blueprints (templates for spawning).
    pub blueprints: Vec<AgentBlueprint>,
    /// Active agent pool with state tracking.
    pub agent_pool: HashMap<String, SpawnedAgentState>,
    /// Live agents (the actual Agent instances).
    pub agents: HashMap<String, Agent>,
    /// Generated A2A cards for each agent.
    pub agent_cards: HashMap<String, AgentCard>,
    /// Task queue (all tasks regardless of status).
    pub tasks: Vec<OrchestratedTask>,
    /// Task retry counters.
    task_retries: HashMap<String, u32>,
    /// Completed task IDs (for dependency checking).
    completed_task_ids: Vec<String>,
}

impl MetaOrchestrator {
    /// Create a new `MetaOrchestrator` with the given configuration.
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            config,
            blueprints: Vec::new(),
            agent_pool: HashMap::new(),
            agents: HashMap::new(),
            agent_cards: HashMap::new(),
            tasks: Vec::new(),
            task_retries: HashMap::new(),
            completed_task_ids: Vec::new(),
        }
    }

    /// Create an orchestrator with default savant blueprints pre-loaded.
    pub fn with_default_savants(config: OrchestratorConfig) -> Self {
        let mut orch = Self::new(config.clone());
        let all = savants::all_savants(&config.default_llm);
        for bp in all {
            orch.register_blueprint(bp);
        }
        orch
    }

    // -----------------------------------------------------------------------
    // Blueprint management
    // -----------------------------------------------------------------------

    /// Register an agent blueprint.
    pub fn register_blueprint(&mut self, blueprint: AgentBlueprint) {
        log::debug!("Registered blueprint: {} (domain: {})", blueprint.role, blueprint.domain);
        self.blueprints.push(blueprint);
    }

    /// Register blueprints for specific domains.
    pub fn register_domain_savants(&mut self, domains: &[SavantDomain]) {
        for domain in domains {
            let bp = savants::savant_for_domain(*domain, &self.config.default_llm);
            self.register_blueprint(bp);
        }
    }

    // -----------------------------------------------------------------------
    // Agent spawning
    // -----------------------------------------------------------------------

    /// Spawn an agent from a blueprint.
    ///
    /// Creates a live `Agent` instance, generates its A2A card, and adds
    /// it to the pool.
    ///
    /// # Returns
    ///
    /// The ID of the spawned agent.
    pub fn spawn_agent(&mut self, blueprint: &AgentBlueprint) -> String {
        let agent_id = format!("agent-{}", Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));

        // Create the live Agent
        let mut agent = Agent::new(
            blueprint.role.clone(),
            blueprint.goal.clone(),
            blueprint.backstory.clone(),
        );
        agent.llm = Some(blueprint.llm.clone());
        agent.tools = blueprint.tools.clone();
        agent.max_iter = blueprint.max_iter;
        agent.allow_delegation = blueprint.allow_delegation;
        agent.verbose = false;

        // Create the agent state
        let state = SpawnedAgentState::new(&agent_id, blueprint);

        // Generate A2A card
        let card = build_card_from_blueprint(blueprint, &self.config.base_url);

        log::info!(
            "Spawned agent '{}' ({}) in domain '{}' with {} skills",
            agent_id, blueprint.role, blueprint.domain, blueprint.skills.len(),
        );

        // Store everything
        self.agents.insert(agent_id.clone(), agent);
        self.agent_pool.insert(agent_id.clone(), state);
        self.agent_cards.insert(agent_id.clone(), card);

        agent_id
    }

    /// Spawn an agent for a specific domain using its savant blueprint.
    pub fn spawn_domain_agent(&mut self, domain: SavantDomain) -> String {
        let bp = savants::savant_for_domain(domain, &self.config.default_llm);
        self.spawn_agent(&bp)
    }

    // -----------------------------------------------------------------------
    // Task management
    // -----------------------------------------------------------------------

    /// Add a task to the orchestration queue.
    pub fn add_task(&mut self, task: OrchestratedTask) {
        log::debug!("Added task: {} (priority: {:?})", task.id, task.priority);
        self.tasks.push(task);
    }

    /// Add multiple tasks to the queue.
    pub fn add_tasks(&mut self, tasks: Vec<OrchestratedTask>) {
        for task in tasks {
            self.add_task(task);
        }
    }

    /// Decompose a high-level objective into orchestrated tasks.
    ///
    /// Uses keyword analysis to infer required domains and creates
    /// tasks with appropriate skill requirements and dependencies.
    pub fn decompose_objective(&mut self, objective: &str) -> Vec<String> {
        let mut task_ids = Vec::new();

        // Infer domains from objective keywords
        let domains = infer_domains(objective);

        if domains.is_empty() {
            // Single general task
            let task = OrchestratedTask::new(objective)
                .with_priority(TaskPriority::High);
            task_ids.push(task.id.clone());
            self.add_task(task);
        } else {
            // Create a research/planning task first
            let planning_task = OrchestratedTask::new(format!("Plan and decompose: {}", objective))
                .with_priority(TaskPriority::High)
                .with_domain(SavantDomain::Planning)
                .with_required_skills(vec!["task_decomposition".into()]);
            let planning_id = planning_task.id.clone();
            task_ids.push(planning_id.clone());
            self.add_task(planning_task);

            // Create domain-specific execution tasks that depend on planning
            for domain in &domains {
                let task = OrchestratedTask::new(format!(
                    "{} work for: {}",
                    domain, objective,
                ))
                .with_priority(TaskPriority::Medium)
                .with_domain(*domain)
                .with_dependencies(vec![planning_id.clone()]);
                task_ids.push(task.id.clone());
                self.add_task(task);
            }

            // Create a synthesis task that depends on all domain tasks
            let dep_ids = task_ids.clone();
            let synthesis_task = OrchestratedTask::new(format!("Synthesize results for: {}", objective))
                .with_priority(TaskPriority::High)
                .with_dependencies(dep_ids);
            task_ids.push(synthesis_task.id.clone());
            self.add_task(synthesis_task);
        }

        task_ids
    }

    // -----------------------------------------------------------------------
    // Task distribution
    // -----------------------------------------------------------------------

    /// Find the best agent for a given task.
    ///
    /// Scores all available (non-busy) agents against the task based on:
    /// - Skill match score
    /// - Domain match bonus
    /// - Performance score weight
    ///
    /// Returns the agent ID and match score, or None if no suitable agent.
    pub fn find_best_agent(&self, task: &OrchestratedTask) -> Option<(String, f64)> {
        let mut best: Option<(String, f64)> = None;

        for (agent_id, state) in &self.agent_pool {
            if state.busy {
                continue;
            }

            // Base score from skill matching
            let mut score = state.best_skill_match(&task.description);

            // Domain bonus
            if let Some(preferred_domain) = &task.preferred_domain {
                if state.domain == *preferred_domain {
                    score += 3.0;
                }
            }

            // Required skills check
            if !task.required_skills.is_empty() {
                let agent_skill_ids: Vec<&str> = state.skills.iter().map(|s| s.id.as_str()).collect();
                let has_all = task.required_skills.iter().all(|req| agent_skill_ids.contains(&req.as_str()));
                if has_all {
                    score += 2.0;
                } else {
                    score *= 0.5; // Penalty for missing required skills
                }
            }

            // Performance weight
            score *= state.performance_score;

            if score > self.config.min_match_score {
                if best.as_ref().map_or(true, |(_, best_score)| score > *best_score) {
                    best = Some((agent_id.clone(), score));
                }
            }
        }

        best
    }

    /// Assign pending tasks to available agents.
    ///
    /// Returns the number of tasks assigned.
    pub fn distribute_tasks(&mut self) -> usize {
        let mut assigned = 0;

        // Get indices of tasks that are pending and have dependencies satisfied
        let assignable_indices: Vec<usize> = self.tasks.iter()
            .enumerate()
            .filter(|(_, t)| {
                t.status == OrchestratedTaskStatus::Pending
                    && t.dependencies_satisfied(&self.completed_task_ids)
            })
            .map(|(i, _)| i)
            .collect();

        // Sort by priority (critical first)
        let mut indices_with_priority: Vec<(usize, TaskPriority)> = assignable_indices.iter()
            .map(|&i| (i, self.tasks[i].priority))
            .collect();
        indices_with_priority.sort_by(|a, b| b.1.cmp(&a.1));

        for (idx, _) in indices_with_priority {
            // Clone the task for matching (avoid borrow issues)
            let task_clone = self.tasks[idx].clone();

            if let Some((agent_id, score)) = self.find_best_agent(&task_clone) {
                log::info!(
                    "Assigning task '{}' to agent '{}' (score: {:.2})",
                    task_clone.id, agent_id, score,
                );

                // Update task
                self.tasks[idx].assign(&agent_id);

                // Update agent state
                if let Some(state) = self.agent_pool.get_mut(&agent_id) {
                    state.assign_task(&task_clone.id);
                }

                assigned += 1;
            } else if self.config.auto_spawn {
                // Try to auto-spawn an agent for this task
                if self.agent_pool.len() < self.config.max_agents {
                    let domain = task_clone.preferred_domain.unwrap_or(SavantDomain::General);
                    let new_agent_id = self.spawn_domain_agent(domain);
                    log::info!(
                        "Auto-spawned agent '{}' for task '{}'",
                        new_agent_id, task_clone.id,
                    );

                    // Assign the task to the newly spawned agent
                    self.tasks[idx].assign(&new_agent_id);
                    if let Some(state) = self.agent_pool.get_mut(&new_agent_id) {
                        state.assign_task(&task_clone.id);
                    }
                    assigned += 1;
                }
            }
        }

        assigned
    }

    // -----------------------------------------------------------------------
    // Task execution
    // -----------------------------------------------------------------------

    /// Execute all assigned tasks.
    ///
    /// Runs each assigned task through its agent and collects results.
    /// Returns the number of tasks that completed (successfully or not).
    pub fn execute_assigned_tasks(&mut self) -> usize {
        let mut executed = 0;

        // Find assigned tasks
        let assigned_indices: Vec<usize> = self.tasks.iter()
            .enumerate()
            .filter(|(_, t)| t.status == OrchestratedTaskStatus::Assigned)
            .map(|(i, _)| i)
            .collect();

        for idx in assigned_indices {
            let agent_id = match &self.tasks[idx].assigned_agent {
                Some(id) => id.clone(),
                None => continue,
            };

            self.tasks[idx].start();

            // Build context from completed dependency outputs
            let context = self.build_task_context(idx);

            // Extract task description before mutable borrow on agents
            let task_description = self.tasks[idx].description.clone();

            // Execute through the agent
            let result = if let Some(agent) = self.agents.get_mut(&agent_id) {
                let context_ref = context.as_deref();
                let tool_refs: Vec<String> = agent.tools.clone();
                agent.execute_task(&task_description, context_ref, Some(&tool_refs))
            } else {
                Err(format!("Agent '{}' not found in pool", agent_id))
            };

            match result {
                Ok(output) => {
                    self.tasks[idx].complete(output);
                    self.completed_task_ids.push(self.tasks[idx].id.clone());

                    if let Some(state) = self.agent_pool.get_mut(&agent_id) {
                        state.complete_task(true);
                        // Update A2A card if adaptive
                        if self.config.adaptive_skills {
                            if let Some(card) = self.agent_cards.get_mut(&agent_id) {
                                update_card_skills(card, state);
                            }
                        }
                    }
                    log::info!("Task '{}' completed by agent '{}'", self.tasks[idx].id, agent_id);
                }
                Err(error) => {
                    let task_id = self.tasks[idx].id.clone();
                    let retries = self.task_retries.entry(task_id.clone()).or_insert(0);
                    *retries += 1;

                    if *retries >= self.config.max_task_retries {
                        self.tasks[idx].fail(error.clone());
                        log::warn!("Task '{}' failed after {} retries: {}", task_id, retries, error);
                    } else {
                        // Reset to pending for retry
                        self.tasks[idx].status = OrchestratedTaskStatus::Pending;
                        self.tasks[idx].assigned_agent = None;
                        log::info!("Task '{}' retry {}/{}: {}", task_id, retries, self.config.max_task_retries, error);
                    }

                    if let Some(state) = self.agent_pool.get_mut(&agent_id) {
                        state.complete_task(false);
                    }
                }
            }

            executed += 1;
        }

        executed
    }

    /// Build context string from completed dependency outputs.
    fn build_task_context(&self, task_idx: usize) -> Option<String> {
        let deps = &self.tasks[task_idx].dependencies;
        if deps.is_empty() {
            return self.tasks[task_idx].context.clone();
        }

        let dep_outputs: Vec<String> = self.tasks.iter()
            .filter(|t| deps.contains(&t.id) && t.output.is_some())
            .map(|t| format!("Result from '{}': {}", t.description, t.output.as_ref().unwrap()))
            .collect();

        if dep_outputs.is_empty() {
            return self.tasks[task_idx].context.clone();
        }

        let mut context = dep_outputs.join("\n\n");
        if let Some(ref base_ctx) = self.tasks[task_idx].context {
            context = format!("{}\n\n{}", base_ctx, context);
        }
        Some(context)
    }

    // -----------------------------------------------------------------------
    // Run loop
    // -----------------------------------------------------------------------

    /// Run the full orchestration loop until all tasks are done.
    ///
    /// Repeatedly distributes tasks, executes them, and adjusts skills
    /// until no more tasks can be processed.
    ///
    /// Returns a summary of all task outcomes.
    pub fn run(&mut self) -> OrchestrationResult {
        log::info!(
            "Starting orchestration with {} tasks, {} blueprints, {} pooled agents",
            self.tasks.len(), self.blueprints.len(), self.agent_pool.len(),
        );

        let mut iterations = 0;
        let max_iterations = self.tasks.len() * 5 + 10; // Safety limit

        loop {
            iterations += 1;
            if iterations > max_iterations {
                log::warn!("Orchestration hit max iterations ({}), stopping", max_iterations);
                break;
            }

            // 1. Distribute pending tasks
            let distributed = self.distribute_tasks();

            // 2. Execute assigned tasks
            let executed = self.execute_assigned_tasks();

            // 3. Check if we're done
            let pending = self.tasks.iter().filter(|t| t.status == OrchestratedTaskStatus::Pending).count();
            let assigned = self.tasks.iter().filter(|t| t.status == OrchestratedTaskStatus::Assigned).count();
            let running = self.tasks.iter().filter(|t| t.status == OrchestratedTaskStatus::Running).count();

            if pending == 0 && assigned == 0 && running == 0 {
                break;
            }

            // If nothing happened this iteration and there's no progress, break
            if distributed == 0 && executed == 0 && pending > 0 {
                log::warn!("Orchestration stalled: {} pending tasks with no progress", pending);
                break;
            }
        }

        self.build_result()
    }

    /// Build the orchestration result summary.
    fn build_result(&self) -> OrchestrationResult {
        let completed: Vec<_> = self.tasks.iter()
            .filter(|t| t.status == OrchestratedTaskStatus::Completed)
            .cloned()
            .collect();
        let failed: Vec<_> = self.tasks.iter()
            .filter(|t| t.status == OrchestratedTaskStatus::Failed)
            .cloned()
            .collect();
        let pending: Vec<_> = self.tasks.iter()
            .filter(|t| t.status == OrchestratedTaskStatus::Pending)
            .cloned()
            .collect();

        let agent_cards: Vec<AgentCard> = self.agent_cards.values().cloned().collect();

        OrchestrationResult {
            total_tasks: self.tasks.len(),
            completed_tasks: completed.len(),
            failed_tasks: failed.len(),
            pending_tasks: pending.len(),
            agents_spawned: self.agent_pool.len(),
            completed,
            failed,
            pending,
            agent_cards,
        }
    }

    // -----------------------------------------------------------------------
    // Skill adjustment
    // -----------------------------------------------------------------------

    /// Adjust agent skills based on task performance.
    ///
    /// Agents that successfully complete tasks get their matching skills
    /// boosted. Agents that fail get penalized.
    pub fn adjust_agent_skills(&mut self, agent_id: &str, task: &OrchestratedTask, success: bool) {
        if !self.config.adaptive_skills {
            return;
        }

        if let Some(state) = self.agent_pool.get_mut(agent_id) {
            if success {
                // Boost proficiency for skills that matched the task
                for skill in &mut state.skills {
                    if skill.match_score(&task.description) > 0.0 {
                        skill.proficiency = (skill.proficiency * 1.05).min(1.0);
                    }
                }

                // If the task required skills the agent didn't have, add them
                for req_skill in &task.required_skills {
                    if !state.skills.iter().any(|s| s.id == *req_skill) {
                        state.add_skill(SkillDescriptor::new(
                            req_skill,
                            req_skill,
                            format!("Learned from task: {}", task.description),
                        ));
                    }
                }
            } else {
                // Reduce proficiency for skills that didn't help
                for skill in &mut state.skills {
                    if skill.match_score(&task.description) > 0.0 {
                        skill.proficiency = (skill.proficiency * 0.9).max(0.1);
                    }
                }
            }

            // Update the agent's A2A card
            if let Some(card) = self.agent_cards.get_mut(agent_id) {
                update_card_skills(card, state);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Introspection
    // -----------------------------------------------------------------------

    /// Get all agent cards in the current pool.
    pub fn get_agent_cards(&self) -> Vec<&AgentCard> {
        self.agent_cards.values().collect()
    }

    /// Get the card for a specific agent.
    pub fn get_agent_card(&self, agent_id: &str) -> Option<&AgentCard> {
        self.agent_cards.get(agent_id)
    }

    /// Get pool statistics.
    pub fn pool_stats(&self) -> PoolStats {
        let total = self.agent_pool.len();
        let busy = self.agent_pool.values().filter(|s| s.busy).count();
        let idle = total - busy;
        let avg_performance = if total > 0 {
            self.agent_pool.values().map(|s| s.performance_score).sum::<f64>() / total as f64
        } else {
            0.0
        };

        PoolStats {
            total_agents: total,
            busy_agents: busy,
            idle_agents: idle,
            average_performance: avg_performance,
        }
    }
}

impl std::fmt::Debug for MetaOrchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetaOrchestrator")
            .field("blueprints", &self.blueprints.len())
            .field("agents", &self.agent_pool.len())
            .field("tasks", &self.tasks.len())
            .field("config", &self.config)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Result of an orchestration run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResult {
    /// Total number of tasks.
    pub total_tasks: usize,
    /// Number of completed tasks.
    pub completed_tasks: usize,
    /// Number of failed tasks.
    pub failed_tasks: usize,
    /// Number of tasks still pending.
    pub pending_tasks: usize,
    /// Total agents spawned during orchestration.
    pub agents_spawned: usize,
    /// Completed task details.
    pub completed: Vec<OrchestratedTask>,
    /// Failed task details.
    pub failed: Vec<OrchestratedTask>,
    /// Still-pending task details.
    pub pending: Vec<OrchestratedTask>,
    /// Final agent cards for all spawned agents.
    pub agent_cards: Vec<AgentCard>,
}

/// Agent pool statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    /// Total agents in pool.
    pub total_agents: usize,
    /// Currently busy agents.
    pub busy_agents: usize,
    /// Idle agents ready for work.
    pub idle_agents: usize,
    /// Average performance score across all agents.
    pub average_performance: f64,
}

// ---------------------------------------------------------------------------
// Domain inference
// ---------------------------------------------------------------------------

/// Infer relevant domains from an objective string.
fn infer_domains(objective: &str) -> Vec<SavantDomain> {
    let lower = objective.to_lowercase();
    let mut domains = Vec::new();

    if lower.contains("research") || lower.contains("find") || lower.contains("search") || lower.contains("investigate") {
        domains.push(SavantDomain::Research);
    }
    if lower.contains("code") || lower.contains("implement") || lower.contains("build") || lower.contains("develop") || lower.contains("program") {
        domains.push(SavantDomain::Engineering);
    }
    if lower.contains("data") || lower.contains("analy") || lower.contains("statistic") || lower.contains("metric") {
        domains.push(SavantDomain::DataAnalysis);
    }
    if lower.contains("write") || lower.contains("content") || lower.contains("document") || lower.contains("article") || lower.contains("blog") {
        domains.push(SavantDomain::ContentCreation);
    }
    if lower.contains("plan") || lower.contains("strateg") || lower.contains("organiz") || lower.contains("roadmap") {
        domains.push(SavantDomain::Planning);
    }
    if lower.contains("test") || lower.contains("quality") || lower.contains("qa") || lower.contains("verify") {
        domains.push(SavantDomain::QualityAssurance);
    }
    if lower.contains("secur") || lower.contains("vulnerab") || lower.contains("audit") || lower.contains("penetration") {
        domains.push(SavantDomain::Security);
    }

    domains
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let config = OrchestratorConfig::default();
        let orch = MetaOrchestrator::new(config);
        assert!(orch.blueprints.is_empty());
        assert!(orch.agent_pool.is_empty());
        assert!(orch.tasks.is_empty());
    }

    #[test]
    fn test_orchestrator_with_default_savants() {
        let config = OrchestratorConfig::default();
        let orch = MetaOrchestrator::with_default_savants(config);
        assert_eq!(orch.blueprints.len(), 7); // 7 domain savants
    }

    #[test]
    fn test_spawn_agent() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);
        let bp = savants::research_savant("openai/gpt-4o-mini");
        let agent_id = orch.spawn_agent(&bp);

        assert!(orch.agent_pool.contains_key(&agent_id));
        assert!(orch.agents.contains_key(&agent_id));
        assert!(orch.agent_cards.contains_key(&agent_id));
    }

    #[test]
    fn test_spawn_domain_agent() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);
        let agent_id = orch.spawn_domain_agent(SavantDomain::Engineering);

        let state = orch.agent_pool.get(&agent_id).unwrap();
        assert_eq!(state.domain, SavantDomain::Engineering);
    }

    #[test]
    fn test_add_and_distribute_tasks() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);

        // Spawn a research agent
        let bp = savants::research_savant("openai/gpt-4o-mini");
        orch.spawn_agent(&bp);

        // Add a research task
        let task = OrchestratedTask::new("Research Rust async patterns")
            .with_domain(SavantDomain::Research)
            .with_priority(TaskPriority::High);
        orch.add_task(task);

        // Distribute
        let assigned = orch.distribute_tasks();
        assert_eq!(assigned, 1);

        // Verify task was assigned
        assert_eq!(orch.tasks[0].status, OrchestratedTaskStatus::Assigned);
        assert!(orch.tasks[0].assigned_agent.is_some());
    }

    #[test]
    fn test_auto_spawn_on_distribution() {
        let mut config = OrchestratorConfig::default();
        config.auto_spawn = true;
        config.min_match_score = 0.0; // Accept any match
        let mut orch = MetaOrchestrator::with_default_savants(config);

        // Add a task with no agents spawned
        let task = OrchestratedTask::new("Review code for security vulnerabilities")
            .with_domain(SavantDomain::Security);
        orch.add_task(task);

        // Distribution should auto-spawn
        let assigned = orch.distribute_tasks();
        assert_eq!(assigned, 1);
        assert!(!orch.agent_pool.is_empty(), "Agent should have been auto-spawned");
    }

    #[test]
    fn test_decompose_objective() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);

        let task_ids = orch.decompose_objective("Research and implement a web scraper with tests");
        // Should have: planning + research + engineering + QA + synthesis
        assert!(task_ids.len() >= 3, "Expected at least 3 tasks, got {}", task_ids.len());
    }

    #[test]
    fn test_find_best_agent_domain_preference() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);

        // Spawn two agents in different domains
        let research_id = orch.spawn_domain_agent(SavantDomain::Research);
        let _eng_id = orch.spawn_domain_agent(SavantDomain::Engineering);

        // Task preferring research domain
        let task = OrchestratedTask::new("Find information about Rust")
            .with_domain(SavantDomain::Research);

        let best = orch.find_best_agent(&task);
        assert!(best.is_some());
        assert_eq!(best.unwrap().0, research_id, "Research agent should be preferred");
    }

    #[test]
    fn test_pool_stats() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);

        orch.spawn_domain_agent(SavantDomain::Research);
        orch.spawn_domain_agent(SavantDomain::Engineering);

        let stats = orch.pool_stats();
        assert_eq!(stats.total_agents, 2);
        assert_eq!(stats.idle_agents, 2);
        assert_eq!(stats.busy_agents, 0);
        assert!((stats.average_performance - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_infer_domains() {
        let domains = infer_domains("Research and implement a secure web application");
        assert!(domains.contains(&SavantDomain::Research));
        assert!(domains.contains(&SavantDomain::Engineering));
        assert!(domains.contains(&SavantDomain::Security));
    }

    #[test]
    fn test_infer_domains_empty() {
        let domains = infer_domains("hello world");
        assert!(domains.is_empty());
    }

    #[test]
    fn test_task_dependencies_block_distribution() {
        let config = OrchestratorConfig {
            min_match_score: 0.0,
            ..OrchestratorConfig::default()
        };
        let mut orch = MetaOrchestrator::new(config);
        orch.spawn_domain_agent(SavantDomain::General);

        // Task 2 depends on task 1
        let task1 = OrchestratedTask::new("Task one");
        let task1_id = task1.id.clone();
        let task2 = OrchestratedTask::new("Task two")
            .with_dependencies(vec![task1_id.clone()]);

        orch.add_task(task1);
        orch.add_task(task2);

        // Only task 1 should be assigned (task 2 blocked by dependency)
        let assigned = orch.distribute_tasks();
        assert_eq!(assigned, 1);
        assert_eq!(orch.tasks[0].status, OrchestratedTaskStatus::Assigned);
        assert_eq!(orch.tasks[1].status, OrchestratedTaskStatus::Pending);
    }

    #[test]
    fn test_adjust_agent_skills_success() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);
        let agent_id = orch.spawn_domain_agent(SavantDomain::Research);

        let task = OrchestratedTask::new("Search the web for Rust patterns")
            .with_required_skills(vec!["new_skill_xyz".into()]);

        // Adjust for success
        orch.adjust_agent_skills(&agent_id, &task, true);

        let state = orch.agent_pool.get(&agent_id).unwrap();
        // Should have learned the new required skill
        assert!(state.skills.iter().any(|s| s.id == "new_skill_xyz"));
    }

    #[test]
    fn test_orchestrator_debug() {
        let config = OrchestratorConfig::default();
        let orch = MetaOrchestrator::with_default_savants(config);
        let debug = format!("{:?}", orch);
        assert!(debug.contains("MetaOrchestrator"));
        assert!(debug.contains("blueprints"));
    }
}
