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
use super::delegation::{
    AgentFeedback, CapabilityUpdate, CapabilityUpdateTrigger, DelegationDispatch,
    DelegationRequest, DelegationResponse, DelegationResult, OrchestrationEvent,
    SkillAdjustment, SkillAdjustmentType, TaskOutcome,
};
use super::savants;
use super::skill_engine::{SkillEngine, SkillEngineConfig};
use super::spawner::SpawnerAgent;
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
    /// Spawner meta-agent for objective decomposition and delegation routing.
    pub spawner: SpawnerAgent,
    /// Skill adjustment engine with feedback loops.
    pub skill_engine: SkillEngine,
    /// Event log for orchestration lifecycle tracking.
    pub event_log: Vec<OrchestrationEvent>,
    /// Pending delegation requests.
    delegation_queue: Vec<DelegationRequest>,
}

impl MetaOrchestrator {
    /// Create a new `MetaOrchestrator` with the given configuration.
    pub fn new(config: OrchestratorConfig) -> Self {
        let spawner = SpawnerAgent::new(&config.default_llm);
        let skill_engine = SkillEngine::default_engine();
        Self {
            config,
            blueprints: Vec::new(),
            agent_pool: HashMap::new(),
            agents: HashMap::new(),
            agent_cards: HashMap::new(),
            tasks: Vec::new(),
            task_retries: HashMap::new(),
            completed_task_ids: Vec::new(),
            spawner,
            skill_engine,
            event_log: Vec::new(),
            delegation_queue: Vec::new(),
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
    /// it to the pool. Emits an `AgentSpawned` event.
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

        // Emit spawn event
        self.event_log.push(OrchestrationEvent::AgentSpawned {
            agent_id: agent_id.clone(),
            domain: blueprint.domain,
            blueprint_id: blueprint.id.clone(),
            skills: blueprint.skills.iter().map(|s| s.id.clone()).collect(),
        });

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
        self.event_log.push(OrchestrationEvent::TaskQueued {
            task_id: task.id.clone(),
            description: task.description.clone(),
            priority: task.priority,
        });
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
    /// Uses the spawner's multi-pass decomposition engine to analyze the
    /// objective, detect domains, extract sub-tasks, infer dependencies,
    /// and add planning/synthesis steps. The resulting tasks are added
    /// to the orchestration queue.
    ///
    /// # Returns
    ///
    /// A list of task IDs in execution order.
    pub fn decompose_objective(&mut self, objective: &str) -> Vec<String> {
        // Use the spawner's structured decomposition
        let plan = self.spawner.decompose(objective);
        let orchestrated = self.spawner.plan_to_orchestrated_tasks(&plan);

        log::info!(
            "Decomposed '{}' into {} tasks across {} domains (synthesis: {})",
            objective,
            orchestrated.len(),
            plan.domains.len(),
            plan.has_synthesis,
        );

        let task_ids: Vec<String> = orchestrated.iter().map(|t| t.id.clone()).collect();
        self.add_tasks(orchestrated);

        // Collect spawner events into the orchestrator log
        let spawner_events = self.spawner.drain_events();
        self.event_log.extend(spawner_events);

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

                // Emit assignment event
                self.event_log.push(OrchestrationEvent::TaskAssigned {
                    task_id: task_clone.id.clone(),
                    agent_id: agent_id.clone(),
                    match_score: score,
                });

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

                    // Emit assignment event
                    self.event_log.push(OrchestrationEvent::TaskAssigned {
                        task_id: task_clone.id.clone(),
                        agent_id: new_agent_id.clone(),
                        match_score: 0.0,
                    });

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
    /// Runs each assigned task through its agent, collects results,
    /// applies skill feedback via the skill engine, and emits lifecycle events.
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

            let task_id = self.tasks[idx].id.clone();
            self.tasks[idx].start();

            // Emit task started event
            self.event_log.push(OrchestrationEvent::TaskStarted {
                task_id: task_id.clone(),
                agent_id: agent_id.clone(),
            });

            // Build context from completed dependency outputs
            let context = self.build_task_context(idx);

            // Extract task description and required skills before mutable borrow on agents
            let task_description = self.tasks[idx].description.clone();
            let task_required_skills = self.tasks[idx].required_skills.clone();

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
                    let output_preview = if output.len() > 200 {
                        format!("{}...", &output[..200])
                    } else {
                        output.clone()
                    };

                    self.tasks[idx].complete(output);
                    self.completed_task_ids.push(task_id.clone());

                    // Emit task completed event
                    self.event_log.push(OrchestrationEvent::TaskCompleted {
                        task_id: task_id.clone(),
                        agent_id: agent_id.clone(),
                        output_preview,
                    });

                    // Apply skill feedback via the skill engine
                    if self.config.adaptive_skills {
                        // Identify which skills were relevant based on task description
                        let relevant_skills: Vec<String> = if let Some(state) = self.agent_pool.get(&agent_id) {
                            state.skills.iter()
                                .filter(|s| s.match_score(&task_description) > 0.0)
                                .map(|s| s.id.clone())
                                .collect()
                        } else {
                            Vec::new()
                        };

                        let feedback = AgentFeedback::success(&agent_id, &task_id)
                            .with_relevant_skills(relevant_skills);

                        if let (Some(state), Some(card)) = (
                            self.agent_pool.get_mut(&agent_id),
                            self.agent_cards.get_mut(&agent_id),
                        ) {
                            state.complete_task(true);
                            let (_update, events) = self.skill_engine.apply_feedback(&feedback, state, card);
                            self.event_log.extend(events);
                        }
                    } else if let Some(state) = self.agent_pool.get_mut(&agent_id) {
                        state.complete_task(true);
                    }

                    log::info!("Task '{}' completed by agent '{}'", task_id, agent_id);
                }
                Err(error) => {
                    let retries = self.task_retries.entry(task_id.clone()).or_insert(0);
                    *retries += 1;
                    let retry_count = *retries;

                    if retry_count >= self.config.max_task_retries {
                        self.tasks[idx].fail(error.clone());

                        // Emit task failed event
                        self.event_log.push(OrchestrationEvent::TaskFailed {
                            task_id: task_id.clone(),
                            agent_id: agent_id.clone(),
                            error: error.clone(),
                            retry_count,
                        });

                        log::warn!("Task '{}' failed after {} retries: {}", task_id, retry_count, error);
                    } else {
                        // Reset to pending for retry
                        self.tasks[idx].status = OrchestratedTaskStatus::Pending;
                        self.tasks[idx].assigned_agent = None;

                        // Emit task failed event (with retry)
                        self.event_log.push(OrchestrationEvent::TaskFailed {
                            task_id: task_id.clone(),
                            agent_id: agent_id.clone(),
                            error: error.clone(),
                            retry_count,
                        });

                        log::info!("Task '{}' retry {}/{}: {}", task_id, retry_count, self.config.max_task_retries, error);
                    }

                    // Apply failure feedback via the skill engine
                    if self.config.adaptive_skills {
                        let relevant_skills: Vec<String> = if let Some(state) = self.agent_pool.get(&agent_id) {
                            state.skills.iter()
                                .filter(|s| s.match_score(&task_description) > 0.0)
                                .map(|s| s.id.clone())
                                .collect()
                        } else {
                            Vec::new()
                        };

                        let feedback = AgentFeedback::failure(&agent_id, &task_id)
                            .with_relevant_skills(relevant_skills)
                            .with_missing_skills(task_required_skills);

                        if let (Some(state), Some(card)) = (
                            self.agent_pool.get_mut(&agent_id),
                            self.agent_cards.get_mut(&agent_id),
                        ) {
                            state.complete_task(false);
                            let (_update, events) = self.skill_engine.apply_feedback(&feedback, state, card);
                            self.event_log.extend(events);
                        }
                    } else if let Some(state) = self.agent_pool.get_mut(&agent_id) {
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
    /// Repeatedly processes delegation requests, distributes tasks,
    /// executes them, and adjusts skills until no more tasks can be processed.
    /// Emits lifecycle events throughout.
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

            // 1. Process pending delegation requests
            self.process_delegation_queue();

            // 2. Distribute pending tasks
            let distributed = self.distribute_tasks();

            // 3. Execute assigned tasks
            let executed = self.execute_assigned_tasks();

            // 4. Drain events from skill engine and spawner into our log
            let engine_events = self.skill_engine.drain_events();
            self.event_log.extend(engine_events);
            let spawner_events = self.spawner.drain_events();
            self.event_log.extend(spawner_events);

            // 5. Check if we're done
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

        // Emit orchestration finished event
        let completed_count = self.tasks.iter()
            .filter(|t| t.status == OrchestratedTaskStatus::Completed).count();
        let failed_count = self.tasks.iter()
            .filter(|t| t.status == OrchestratedTaskStatus::Failed).count();
        self.event_log.push(OrchestrationEvent::OrchestrationFinished {
            total_tasks: self.tasks.len(),
            completed: completed_count,
            failed: failed_count,
            agents_used: self.agent_pool.len(),
        });

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
            event_log: self.event_log.clone(),
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
    // Delegation
    // -----------------------------------------------------------------------

    /// Submit a delegation request for processing.
    ///
    /// The request is queued and will be processed on the next orchestration
    /// cycle. Returns the delegation request ID.
    pub fn submit_delegation(&mut self, request: DelegationRequest) -> String {
        let request_id = request.id.clone();

        self.event_log.push(OrchestrationEvent::DelegationRequested {
            request_id: request_id.clone(),
            from_agent: request.from_agent.clone(),
            target_domain: request.target_domain,
        });

        self.delegation_queue.push(request);
        log::debug!("Delegation request '{}' queued", request_id);
        request_id
    }

    /// Delegate a task directly from one agent to another.
    ///
    /// Uses the spawner to find the best agent or auto-spawn one, then
    /// creates and assigns a task for the delegation.
    ///
    /// # Returns
    ///
    /// A `DelegationResult` with the outcome.
    pub fn delegate_task(
        &mut self,
        request: DelegationRequest,
    ) -> DelegationResult {
        let request_id = request.id.clone();
        let from_agent = request.from_agent.clone();

        self.event_log.push(OrchestrationEvent::DelegationRequested {
            request_id: request_id.clone(),
            from_agent: from_agent.clone(),
            target_domain: request.target_domain,
        });

        // Use spawner to select or spawn an agent
        let dispatch = self.spawner.handle_delegation(&request, &self.agent_pool);
        let assigned_agent = dispatch.assigned_agent.clone();
        let auto_spawned = dispatch.auto_spawned;

        // If auto-spawned, create the actual agent
        if auto_spawned {
            let domain = request.target_domain.unwrap_or(SavantDomain::General);
            let bp = self.spawner.blueprint_for_domain(domain);
            let agent_id = dispatch.assigned_agent.clone();

            let mut agent = Agent::new(
                bp.role.clone(),
                bp.goal.clone(),
                bp.backstory.clone(),
            );
            agent.llm = Some(bp.llm.clone());
            agent.tools = bp.tools.clone();
            agent.max_iter = bp.max_iter;
            agent.allow_delegation = bp.allow_delegation;
            agent.verbose = false;

            let state = SpawnedAgentState::new(&agent_id, &bp);
            let card = build_card_from_blueprint(&bp, &self.config.base_url);

            self.agents.insert(agent_id.clone(), agent);
            self.agent_pool.insert(agent_id.clone(), state);
            self.agent_cards.insert(agent_id.clone(), card);

            self.event_log.push(OrchestrationEvent::AgentSpawned {
                agent_id: agent_id.clone(),
                domain: bp.domain,
                blueprint_id: bp.id.clone(),
                skills: bp.skills.iter().map(|s| s.id.clone()).collect(),
            });
        }

        self.event_log.push(OrchestrationEvent::DelegationDispatched {
            request_id: request_id.clone(),
            to_agent: assigned_agent.clone(),
            match_score: dispatch.match_score,
        });

        // Create a task for the delegation and execute it
        let mut task = OrchestratedTask::new(&request.task_description)
            .with_priority(request.priority);
        if let Some(ctx) = &request.context {
            task = task.with_context(ctx.clone());
        }
        if let Some(domain) = request.target_domain {
            task = task.with_domain(domain);
        }
        if !request.required_skills.is_empty() {
            task = task.with_required_skills(request.required_skills.clone());
        }

        let task_id = task.id.clone();
        task.assign(&assigned_agent);
        task.start();

        // Execute
        let context = task.context.clone();
        let result = if let Some(agent) = self.agents.get_mut(&assigned_agent) {
            let context_ref = context.as_deref();
            let tool_refs: Vec<String> = agent.tools.clone();
            agent.execute_task(&request.task_description, context_ref, Some(&tool_refs))
        } else {
            Err(format!("Delegated agent '{}' not found", assigned_agent))
        };

        let (success, result_text, error_text) = match result {
            Ok(output) => {
                task.complete(output.clone());
                self.completed_task_ids.push(task_id.clone());

                self.event_log.push(OrchestrationEvent::DelegationCompleted {
                    request_id: request_id.clone(),
                    from_agent: assigned_agent.clone(),
                    success: true,
                });

                // Apply success feedback
                if self.config.adaptive_skills {
                    if let (Some(state), Some(card)) = (
                        self.agent_pool.get_mut(&assigned_agent),
                        self.agent_cards.get_mut(&assigned_agent),
                    ) {
                        state.complete_task(true);
                        let feedback = AgentFeedback::success(&assigned_agent, &task_id);
                        self.skill_engine.apply_feedback(&feedback, state, card);
                    }
                }

                (true, Some(output), None)
            }
            Err(err) => {
                task.fail(err.clone());

                self.event_log.push(OrchestrationEvent::DelegationCompleted {
                    request_id: request_id.clone(),
                    from_agent: assigned_agent.clone(),
                    success: false,
                });

                // Apply failure feedback
                if self.config.adaptive_skills {
                    if let (Some(state), Some(card)) = (
                        self.agent_pool.get_mut(&assigned_agent),
                        self.agent_cards.get_mut(&assigned_agent),
                    ) {
                        state.complete_task(false);
                        let feedback = AgentFeedback::failure(&assigned_agent, &task_id);
                        self.skill_engine.apply_feedback(&feedback, state, card);
                    }
                }

                (false, None, Some(err))
            }
        };

        self.tasks.push(task);

        // Collect events from spawner
        let spawner_events = self.spawner.drain_events();
        self.event_log.extend(spawner_events);

        DelegationResult {
            request_id,
            success,
            result: result_text,
            error: error_text,
            handled_by: assigned_agent,
        }
    }

    /// Process all pending delegation requests.
    ///
    /// Converts each delegation request into a task and assigns it to the
    /// best available agent. Auto-spawns agents if needed.
    pub fn process_delegation_queue(&mut self) {
        if self.delegation_queue.is_empty() {
            return;
        }

        let requests: Vec<DelegationRequest> = std::mem::take(&mut self.delegation_queue);
        log::info!("Processing {} delegation requests", requests.len());

        for request in requests {
            // Use spawner to find best agent
            let dispatch = self.spawner.handle_delegation(&request, &self.agent_pool);

            // If auto-spawned, create the actual agent
            if dispatch.auto_spawned && self.agent_pool.len() < self.config.max_agents {
                let domain = request.target_domain.unwrap_or(SavantDomain::General);
                let bp = self.spawner.blueprint_for_domain(domain);

                let mut agent = Agent::new(
                    bp.role.clone(),
                    bp.goal.clone(),
                    bp.backstory.clone(),
                );
                agent.llm = Some(bp.llm.clone());
                agent.tools = bp.tools.clone();
                agent.max_iter = bp.max_iter;
                agent.allow_delegation = bp.allow_delegation;
                agent.verbose = false;

                let state = SpawnedAgentState::new(&dispatch.assigned_agent, &bp);
                let card = build_card_from_blueprint(&bp, &self.config.base_url);

                self.agents.insert(dispatch.assigned_agent.clone(), agent);
                self.agent_pool.insert(dispatch.assigned_agent.clone(), state);
                self.agent_cards.insert(dispatch.assigned_agent.clone(), card);

                self.event_log.push(OrchestrationEvent::AgentSpawned {
                    agent_id: dispatch.assigned_agent.clone(),
                    domain: bp.domain,
                    blueprint_id: bp.id.clone(),
                    skills: bp.skills.iter().map(|s| s.id.clone()).collect(),
                });
            }

            self.event_log.push(OrchestrationEvent::DelegationDispatched {
                request_id: request.id.clone(),
                to_agent: dispatch.assigned_agent.clone(),
                match_score: dispatch.match_score,
            });

            // Create a task for the delegation
            let mut task = OrchestratedTask::new(&request.task_description)
                .with_priority(request.priority);
            if let Some(ctx) = &request.context {
                task = task.with_context(ctx.clone());
            }
            if let Some(domain) = request.target_domain {
                task = task.with_domain(domain);
            }
            if !request.required_skills.is_empty() {
                task = task.with_required_skills(request.required_skills.clone());
            }

            // Assign to the dispatched agent
            task.assign(&dispatch.assigned_agent);
            if let Some(state) = self.agent_pool.get_mut(&dispatch.assigned_agent) {
                state.assign_task(&task.id);
            }

            self.tasks.push(task);
        }

        // Drain spawner events
        let spawner_events = self.spawner.drain_events();
        self.event_log.extend(spawner_events);
    }

    /// Transfer skills from one agent to another via the skill engine.
    ///
    /// Useful when an agent is being retired or when cross-pollinating
    /// expertise between team members.
    pub fn transfer_skills(
        &mut self,
        from_agent: &str,
        to_agent: &str,
        penalty: f64,
    ) -> Vec<SkillAdjustment> {
        let source = match self.agent_pool.get(from_agent) {
            Some(s) => s.clone(),
            None => return Vec::new(),
        };

        if let Some(target) = self.agent_pool.get_mut(to_agent) {
            let adjustments = self.skill_engine.transfer_skills(&source, target, penalty);

            // Update the target's A2A card
            if let Some(card) = self.agent_cards.get_mut(to_agent) {
                update_card_skills(card, target);
            }

            if !adjustments.is_empty() {
                self.event_log.push(OrchestrationEvent::SkillsAdjusted {
                    agent_id: to_agent.to_string(),
                    adjustments: adjustments.clone(),
                });
                self.event_log.push(OrchestrationEvent::CardUpdated {
                    agent_id: to_agent.to_string(),
                    skill_count: self.agent_pool.get(to_agent).map(|s| s.skills.len()).unwrap_or(0),
                    performance: self.agent_pool.get(to_agent).map(|s| s.performance_score).unwrap_or(0.0),
                });
            }

            adjustments
        } else {
            Vec::new()
        }
    }

    /// Terminate an agent and remove it from the pool.
    ///
    /// Emits an `AgentTerminated` event.
    pub fn terminate_agent(&mut self, agent_id: &str, reason: &str) {
        self.agents.remove(agent_id);
        self.agent_pool.remove(agent_id);
        self.agent_cards.remove(agent_id);

        self.event_log.push(OrchestrationEvent::AgentTerminated {
            agent_id: agent_id.to_string(),
            reason: reason.to_string(),
        });

        log::info!("Terminated agent '{}': {}", agent_id, reason);
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

    /// Get the full event log.
    pub fn get_event_log(&self) -> &[OrchestrationEvent] {
        &self.event_log
    }

    /// Get the delegation queue length.
    pub fn delegation_queue_len(&self) -> usize {
        self.delegation_queue.len()
    }

    /// Get a capability update snapshot for a specific agent.
    pub fn get_capability_update(&self, agent_id: &str) -> Option<CapabilityUpdate> {
        self.agent_pool.get(agent_id).map(|state| CapabilityUpdate {
            agent_id: state.id.clone(),
            skills: state.skills.clone(),
            performance_score: state.performance_score,
            domain: state.domain,
            trigger: CapabilityUpdateTrigger::ManualAdjustment,
        })
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
    /// Full event log from the orchestration.
    pub event_log: Vec<OrchestrationEvent>,
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
        #[cfg(feature = "chess")]
        assert_eq!(orch.blueprints.len(), 10); // 10 domain savants (all domains + chess)
        #[cfg(not(feature = "chess"))]
        assert_eq!(orch.blueprints.len(), 9); // 9 domain savants (chess gated)
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
        // Spawner decomposition: planning + research + engineering + QA + synthesis
        assert!(task_ids.len() >= 3, "Expected at least 3 tasks, got {}", task_ids.len());
        // Should have emitted events
        assert!(!orch.event_log.is_empty(), "Expected orchestration events");
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

    #[test]
    fn test_submit_delegation() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);

        let request = DelegationRequest::new("agent-a", "Research Rust patterns")
            .with_domain(SavantDomain::Research)
            .with_priority(TaskPriority::High);

        let req_id = orch.submit_delegation(request);
        assert!(!req_id.is_empty());
        assert_eq!(orch.delegation_queue_len(), 1);

        // Should have emitted DelegationRequested event
        assert!(orch.event_log.iter().any(|e| matches!(e, OrchestrationEvent::DelegationRequested { .. })));
    }

    #[test]
    fn test_process_delegation_queue() {
        let mut config = OrchestratorConfig::default();
        config.min_match_score = 0.0;
        let mut orch = MetaOrchestrator::with_default_savants(config);

        // Spawn a research agent
        let bp = savants::research_savant("openai/gpt-4o-mini");
        let agent_id = orch.spawn_agent(&bp);

        // Submit delegation
        let request = DelegationRequest::new("agent-x", "Search web for patterns")
            .with_domain(SavantDomain::Research);
        orch.submit_delegation(request);

        // Process the queue
        orch.process_delegation_queue();

        // Queue should be empty now
        assert_eq!(orch.delegation_queue_len(), 0);

        // Should have created a task
        let delegation_tasks: Vec<_> = orch.tasks.iter()
            .filter(|t| t.description.contains("Search web"))
            .collect();
        assert!(!delegation_tasks.is_empty(), "Delegation should create a task");

        // Should have emitted DelegationDispatched event
        assert!(orch.event_log.iter().any(|e| matches!(e, OrchestrationEvent::DelegationDispatched { .. })));
    }

    #[test]
    fn test_process_delegation_auto_spawns() {
        let mut config = OrchestratorConfig::default();
        config.auto_spawn = true;
        let mut orch = MetaOrchestrator::with_default_savants(config);

        // No agents in pool, delegation should auto-spawn
        let request = DelegationRequest::new("agent-x", "Audit code for vulnerabilities")
            .with_domain(SavantDomain::Security);
        orch.submit_delegation(request);
        orch.process_delegation_queue();

        // An agent should have been auto-spawned
        assert!(!orch.agent_pool.is_empty());

        // Should have AgentSpawned event
        let spawn_events: Vec<_> = orch.event_log.iter()
            .filter(|e| matches!(e, OrchestrationEvent::AgentSpawned { .. }))
            .collect();
        // At least 2: one from initial spawn_agent, one from delegation
        assert!(!spawn_events.is_empty());
    }

    #[test]
    fn test_transfer_skills_between_agents() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);

        let research_id = orch.spawn_domain_agent(SavantDomain::Research);
        let eng_id = orch.spawn_domain_agent(SavantDomain::Engineering);

        let research_skills = orch.agent_pool.get(&research_id).unwrap().skills.len();
        let eng_skills_before = orch.agent_pool.get(&eng_id).unwrap().skills.len();

        let adjustments = orch.transfer_skills(&research_id, &eng_id, 0.3);

        let eng_skills_after = orch.agent_pool.get(&eng_id).unwrap().skills.len();
        assert!(eng_skills_after > eng_skills_before, "Engineering agent should have gained research skills");
        assert!(!adjustments.is_empty());

        // Check events
        assert!(orch.event_log.iter().any(|e| matches!(e, OrchestrationEvent::SkillsAdjusted { .. })));
    }

    #[test]
    fn test_terminate_agent() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);

        let agent_id = orch.spawn_domain_agent(SavantDomain::Research);
        assert!(orch.agent_pool.contains_key(&agent_id));

        orch.terminate_agent(&agent_id, "no longer needed");

        assert!(!orch.agent_pool.contains_key(&agent_id));
        assert!(!orch.agents.contains_key(&agent_id));
        assert!(!orch.agent_cards.contains_key(&agent_id));

        assert!(orch.event_log.iter().any(|e| matches!(e, OrchestrationEvent::AgentTerminated { .. })));
    }

    #[test]
    fn test_get_capability_update() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);

        let agent_id = orch.spawn_domain_agent(SavantDomain::Research);
        let update = orch.get_capability_update(&agent_id);

        assert!(update.is_some());
        let update = update.unwrap();
        assert_eq!(update.agent_id, agent_id);
        assert_eq!(update.domain, SavantDomain::Research);
        assert!(!update.skills.is_empty());
    }

    #[test]
    fn test_event_log_from_spawn() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);

        orch.spawn_domain_agent(SavantDomain::Engineering);

        let spawn_events: Vec<_> = orch.event_log.iter()
            .filter(|e| matches!(e, OrchestrationEvent::AgentSpawned { .. }))
            .collect();
        assert_eq!(spawn_events.len(), 1);
    }

    #[test]
    fn test_event_log_from_task_queuing() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);

        let task = OrchestratedTask::new("Test task")
            .with_priority(TaskPriority::High);
        orch.add_task(task);

        let queue_events: Vec<_> = orch.event_log.iter()
            .filter(|e| matches!(e, OrchestrationEvent::TaskQueued { .. }))
            .collect();
        assert_eq!(queue_events.len(), 1);
    }

    #[test]
    fn test_event_log_from_distribution() {
        let mut config = OrchestratorConfig::default();
        config.min_match_score = 0.0;
        let mut orch = MetaOrchestrator::new(config);

        let bp = savants::research_savant("openai/gpt-4o-mini");
        orch.spawn_agent(&bp);

        let task = OrchestratedTask::new("Search for Rust patterns")
            .with_domain(SavantDomain::Research);
        orch.add_task(task);
        orch.distribute_tasks();

        let assign_events: Vec<_> = orch.event_log.iter()
            .filter(|e| matches!(e, OrchestrationEvent::TaskAssigned { .. }))
            .collect();
        assert_eq!(assign_events.len(), 1);
    }

    #[test]
    fn test_decompose_objective_uses_spawner() {
        let config = OrchestratorConfig::default();
        let mut orch = MetaOrchestrator::new(config);

        let task_ids = orch.decompose_objective("research Rust patterns, implement a web scraper, and write documentation");

        // The spawner should produce: planning + research + engineering + content + synthesis
        assert!(task_ids.len() >= 4, "Expected at least 4 tasks for multi-domain objective, got {}", task_ids.len());

        // All tasks should be in the queue
        assert_eq!(orch.tasks.len(), task_ids.len());

        // Last task should have dependencies (synthesis)
        let last_task = orch.tasks.last().unwrap();
        assert!(!last_task.dependencies.is_empty(), "Synthesis task should have dependencies");
    }

    #[test]
    fn test_full_orchestration_event_lifecycle() {
        let mut config = OrchestratorConfig::default();
        config.min_match_score = 0.0;
        let mut orch = MetaOrchestrator::with_default_savants(config);

        // Add a simple task
        let task = OrchestratedTask::new("A simple task");
        orch.add_task(task);

        // Run orchestration (will auto-spawn + assign + execute)
        let result = orch.run();

        // Should have orchestration finished event
        assert!(result.event_log.iter().any(|e|
            matches!(e, OrchestrationEvent::OrchestrationFinished { .. })
        ));

        // Should have at least: TaskQueued + AgentSpawned + TaskAssigned
        assert!(result.event_log.len() >= 3,
            "Expected at least 3 events, got {} events: {:?}", result.event_log.len(), result.event_log);
    }
}
