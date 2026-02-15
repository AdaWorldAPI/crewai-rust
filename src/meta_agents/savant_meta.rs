//! Savant meta-agent — higher-order coordinator for domain savants.
//!
//! The `SavantCoordinator` sits above the individual domain savants and provides:
//!
//! - **Auto-attended spawning**: spawns domain savants on demand when tasks require them
//! - **Skill-aware routing**: matches incoming work to the best savant by skill profile
//! - **Cross-domain delegation**: allows savants to delegate to each other via A2A cards
//! - **Dynamic skill adjustment**: adjusts savant skills after every task, updating A2A cards
//! - **Composite orchestration**: decomposes multi-domain objectives and runs them in parallel
//!
//! # Architecture
//!
//! ```text
//! SavantCoordinator
//!   ├── SavantRegistry       (tracks spawned savants & their A2A cards)
//!   ├── SkillRouter          (matches tasks → savants by skill profile)
//!   ├── DelegationBroker     (handles cross-domain delegation flows)
//!   └── CardSynchronizer     (keeps A2A cards in sync with skill changes)
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::a2a::client::AgentCard;
use crate::agent::Agent;

use super::card_builder::{build_card_from_blueprint, build_card_from_state, update_card_skills};
use super::delegation::{
    AgentFeedback, CapabilityUpdate, CapabilityUpdateTrigger, DelegationRequest,
    DelegationResult, OrchestrationEvent, SkillAdjustment, SkillAdjustmentType, TaskOutcome,
};
use super::savants;
use super::skill_engine::{SkillEngine, SkillEngineConfig};
use super::spawner::SpawnerAgent;
use super::types::{
    AgentBlueprint, OrchestratedTask, OrchestratedTaskStatus, SavantDomain,
    SkillDescriptor, SpawnedAgentState, TaskPriority,
};

// ---------------------------------------------------------------------------
// Savant registry entry
// ---------------------------------------------------------------------------

/// A registered savant instance with its live state and A2A card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavantEntry {
    /// Savant identifier.
    pub id: String,
    /// Domain this savant specializes in.
    pub domain: SavantDomain,
    /// Current skills (adjusted over time).
    pub skills: Vec<SkillDescriptor>,
    /// Blueprint this savant was spawned from.
    pub blueprint_id: String,
    /// Whether the savant is currently busy.
    pub busy: bool,
    /// Tasks completed by this savant.
    pub tasks_completed: u32,
    /// Tasks failed by this savant.
    pub tasks_failed: u32,
    /// Current performance score (0.0 - 1.0, EMA).
    pub performance_score: f64,
    /// Current task assignment (if busy).
    pub current_task: Option<String>,
    /// Domains this savant can delegate to.
    pub delegation_targets: Vec<SavantDomain>,
    /// Whether this savant was auto-spawned vs manually registered.
    pub auto_spawned: bool,
}

impl SavantEntry {
    /// Create a new savant entry from a blueprint.
    pub fn from_blueprint(id: &str, blueprint: &AgentBlueprint, auto_spawned: bool) -> Self {
        let delegation_targets = if blueprint.allow_delegation {
            // Compute delegation targets: every domain except this one
            vec![
                SavantDomain::Research,
                SavantDomain::Engineering,
                SavantDomain::DataAnalysis,
                SavantDomain::ContentCreation,
                SavantDomain::Planning,
                SavantDomain::QualityAssurance,
                SavantDomain::Security,
                SavantDomain::DevOps,
                SavantDomain::Design,
            ]
            .into_iter()
            .filter(|d| *d != blueprint.domain)
            .collect()
        } else {
            Vec::new()
        };

        Self {
            id: id.to_string(),
            domain: blueprint.domain,
            skills: blueprint.skills.clone(),
            blueprint_id: blueprint.id.clone(),
            busy: false,
            tasks_completed: 0,
            tasks_failed: 0,
            performance_score: 1.0,
            current_task: None,
            delegation_targets,
            auto_spawned,
        }
    }

    /// Assign a task to this savant.
    pub fn assign(&mut self, task_id: &str) {
        self.busy = true;
        self.current_task = Some(task_id.to_string());
    }

    /// Complete the current task.
    pub fn complete(&mut self, success: bool) {
        self.busy = false;
        self.current_task = None;
        if success {
            self.tasks_completed += 1;
            self.performance_score = (self.performance_score * 0.9 + 0.1).min(1.0);
        } else {
            self.tasks_failed += 1;
            self.performance_score = (self.performance_score * 0.85).max(0.1);
        }
    }

    /// Best skill match score for a task description.
    pub fn skill_match(&self, description: &str) -> f64 {
        self.skills
            .iter()
            .map(|s| s.match_score(description))
            .fold(0.0f64, f64::max)
    }

    /// Whether this savant can delegate to a given domain.
    pub fn can_delegate_to(&self, domain: SavantDomain) -> bool {
        self.delegation_targets.contains(&domain)
    }
}

// ---------------------------------------------------------------------------
// Skill routing result
// ---------------------------------------------------------------------------

/// Result of skill-based routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Selected savant ID.
    pub savant_id: String,
    /// Match score that justified the selection.
    pub match_score: f64,
    /// Domain of the selected savant.
    pub domain: SavantDomain,
    /// Skills that contributed to the match.
    pub matched_skills: Vec<String>,
    /// Whether a new savant was auto-spawned.
    pub auto_spawned: bool,
}

// ---------------------------------------------------------------------------
// Cross-domain delegation record
// ---------------------------------------------------------------------------

/// Record of a cross-domain delegation between savants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDomainDelegation {
    /// Delegation ID.
    pub id: String,
    /// Source savant that initiated the delegation.
    pub from_savant: String,
    /// Source domain.
    pub from_domain: SavantDomain,
    /// Target savant that handled the delegation.
    pub to_savant: String,
    /// Target domain.
    pub to_domain: SavantDomain,
    /// Task description.
    pub task_description: String,
    /// Whether the delegation succeeded.
    pub success: Option<bool>,
    /// Result output (on success).
    pub result: Option<String>,
}

// ---------------------------------------------------------------------------
// SavantCoordinator
// ---------------------------------------------------------------------------

/// Higher-order meta-agent that coordinates domain savants.
///
/// Manages a registry of savant instances, routes tasks by skill profile,
/// handles cross-domain delegation, and keeps A2A cards synchronized with
/// dynamic skill changes.
pub struct SavantCoordinator {
    /// Default LLM for spawning savants.
    pub default_llm: String,
    /// Registered savant entries (savant_id → entry).
    pub registry: HashMap<String, SavantEntry>,
    /// Live Agent instances (savant_id → Agent).
    pub agents: HashMap<String, Agent>,
    /// A2A cards for each savant (savant_id → card).
    pub cards: HashMap<String, AgentCard>,
    /// Available blueprints for auto-spawning.
    pub blueprints: Vec<AgentBlueprint>,
    /// Skill engine for feedback-driven adjustments.
    pub skill_engine: SkillEngine,
    /// Cross-domain delegation history.
    pub delegation_history: Vec<CrossDomainDelegation>,
    /// Event log.
    pub event_log: Vec<OrchestrationEvent>,
    /// Maximum savants in pool.
    pub max_savants: usize,
    /// Auto-spawn when no matching savant exists.
    pub auto_spawn: bool,
    /// Minimum skill match score for routing.
    pub min_match_score: f64,
    /// Base URL for A2A card generation.
    pub base_url: String,
}

impl SavantCoordinator {
    /// Create a new coordinator with all domain savant blueprints.
    pub fn new(default_llm: impl Into<String>) -> Self {
        let llm = default_llm.into();
        let blueprints = savants::all_savants(&llm);

        Self {
            default_llm: llm,
            registry: HashMap::new(),
            agents: HashMap::new(),
            cards: HashMap::new(),
            blueprints,
            skill_engine: SkillEngine::default_engine(),
            delegation_history: Vec::new(),
            event_log: Vec::new(),
            max_savants: 20,
            auto_spawn: true,
            min_match_score: 0.3,
            base_url: "http://localhost:8080".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // Savant lifecycle
    // -----------------------------------------------------------------------

    /// Spawn a savant for a specific domain.
    ///
    /// Creates a live Agent instance, generates an A2A card, and registers
    /// the savant in the coordinator. Returns the savant ID.
    pub fn spawn_savant(&mut self, domain: SavantDomain) -> String {
        let bp = savants::savant_for_domain(domain, &self.default_llm);
        self.spawn_from_blueprint(&bp, true)
    }

    /// Spawn a savant from a specific blueprint.
    pub fn spawn_from_blueprint(&mut self, blueprint: &AgentBlueprint, auto_spawned: bool) -> String {
        let savant_id = format!(
            "savant-{}-{}",
            blueprint.domain,
            Uuid::new_v4().to_string().split('-').next().unwrap_or("x")
        );

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

        // Create entry and card
        let entry = SavantEntry::from_blueprint(&savant_id, blueprint, auto_spawned);
        let card = build_card_from_blueprint(blueprint, &self.base_url);

        // Emit spawn event
        self.event_log.push(OrchestrationEvent::AgentSpawned {
            agent_id: savant_id.clone(),
            domain: blueprint.domain,
            blueprint_id: blueprint.id.clone(),
            skills: blueprint.skills.iter().map(|s| s.id.clone()).collect(),
        });

        self.agents.insert(savant_id.clone(), agent);
        self.registry.insert(savant_id.clone(), entry);
        self.cards.insert(savant_id.clone(), card);

        savant_id
    }

    /// Spawn all domain savants at once.
    pub fn spawn_all_domains(&mut self) -> Vec<String> {
        let domains = vec![
            SavantDomain::Research,
            SavantDomain::Engineering,
            SavantDomain::DataAnalysis,
            SavantDomain::ContentCreation,
            SavantDomain::Planning,
            SavantDomain::QualityAssurance,
            SavantDomain::Security,
            SavantDomain::DevOps,
            SavantDomain::Design,
        ];

        domains.iter().map(|d| self.spawn_savant(*d)).collect()
    }

    /// Terminate a savant and remove it from the registry.
    pub fn terminate_savant(&mut self, savant_id: &str, reason: &str) {
        self.agents.remove(savant_id);
        self.registry.remove(savant_id);
        self.cards.remove(savant_id);

        self.event_log.push(OrchestrationEvent::AgentTerminated {
            agent_id: savant_id.to_string(),
            reason: reason.to_string(),
        });
    }

    // -----------------------------------------------------------------------
    // Skill-aware routing
    // -----------------------------------------------------------------------

    /// Route a task to the best available savant.
    ///
    /// Scoring factors:
    /// 1. Skill match score (keyword overlap × proficiency)
    /// 2. Domain match bonus (+3.0 if task domain matches)
    /// 3. Required skills check (+2.0 if all present, ×0.5 penalty if missing)
    /// 4. Performance weight (score × performance_score)
    /// 5. Delegation capability bonus (+0.5 if savant can delegate)
    ///
    /// Auto-spawns a domain savant if no match exceeds `min_match_score`.
    pub fn route_task(&mut self, task: &OrchestratedTask) -> RoutingDecision {
        let mut best: Option<(String, f64, Vec<String>)> = None;

        for (savant_id, entry) in &self.registry {
            if entry.busy {
                continue;
            }

            let mut score = 0.0f64;
            let mut matched_skills = Vec::new();

            // Skill match scoring
            for skill in &entry.skills {
                let s = skill.match_score(&task.description);
                if s > 0.0 {
                    score += s;
                    matched_skills.push(skill.id.clone());
                }
            }

            // Domain match bonus
            if let Some(preferred) = &task.preferred_domain {
                if entry.domain == *preferred {
                    score += 3.0;
                }
            }

            // Required skills check
            if !task.required_skills.is_empty() {
                let skill_ids: Vec<&str> = entry.skills.iter().map(|s| s.id.as_str()).collect();
                let has_all = task.required_skills.iter().all(|r| skill_ids.contains(&r.as_str()));
                if has_all {
                    score += 2.0;
                } else {
                    score *= 0.5;
                }
            }

            // Delegation capability bonus
            if !entry.delegation_targets.is_empty() {
                score += 0.5;
            }

            // Performance weight
            score *= entry.performance_score;

            if best.as_ref().map_or(true, |(_, bs, _)| score > *bs) {
                best = Some((savant_id.clone(), score, matched_skills));
            }
        }

        // Check if we have a good enough match
        if let Some((id, score, matched_skills)) = best {
            if score >= self.min_match_score {
                let domain = self.registry.get(&id).map(|e| e.domain).unwrap_or(SavantDomain::General);
                return RoutingDecision {
                    savant_id: id,
                    match_score: score,
                    domain,
                    matched_skills,
                    auto_spawned: false,
                };
            }
        }

        // Auto-spawn if enabled
        if self.auto_spawn && self.registry.len() < self.max_savants {
            let domain = task.preferred_domain.unwrap_or(SavantDomain::General);
            let savant_id = self.spawn_savant(domain);
            let entry = self.registry.get(&savant_id).unwrap();
            let matched_skills: Vec<String> = entry.skills.iter().map(|s| s.id.clone()).collect();

            RoutingDecision {
                savant_id,
                match_score: 0.0,
                domain,
                matched_skills,
                auto_spawned: true,
            }
        } else {
            // Fallback: route to the least-busy general savant
            let fallback_id = self.registry.keys().next().cloned().unwrap_or_default();
            let domain = self.registry.get(&fallback_id).map(|e| e.domain).unwrap_or(SavantDomain::General);
            RoutingDecision {
                savant_id: fallback_id,
                match_score: 0.0,
                domain,
                matched_skills: Vec::new(),
                auto_spawned: false,
            }
        }
    }

    // -----------------------------------------------------------------------
    // Task execution
    // -----------------------------------------------------------------------

    /// Execute a task through the best-matched savant.
    ///
    /// Routes the task, assigns it, executes via the Agent, applies feedback,
    /// and updates the A2A card.
    pub fn execute_task(&mut self, task: &mut OrchestratedTask) -> Result<String, String> {
        let decision = self.route_task(task);
        let savant_id = decision.savant_id.clone();

        // Assign
        task.assign(&savant_id);
        if let Some(entry) = self.registry.get_mut(&savant_id) {
            entry.assign(&task.id);
        }

        self.event_log.push(OrchestrationEvent::TaskAssigned {
            task_id: task.id.clone(),
            agent_id: savant_id.clone(),
            match_score: decision.match_score,
        });

        // Execute
        task.start();
        self.event_log.push(OrchestrationEvent::TaskStarted {
            task_id: task.id.clone(),
            agent_id: savant_id.clone(),
        });

        let result = if let Some(agent) = self.agents.get_mut(&savant_id) {
            let tools: Vec<String> = agent.tools.clone();
            agent.execute_task(&task.description, task.context.as_deref(), Some(&tools))
        } else {
            Err(format!("Savant '{}' not found", savant_id))
        };

        match &result {
            Ok(output) => {
                let preview = if output.len() > 200 {
                    format!("{}...", &output[..200])
                } else {
                    output.clone()
                };
                task.complete(output.clone());

                self.event_log.push(OrchestrationEvent::TaskCompleted {
                    task_id: task.id.clone(),
                    agent_id: savant_id.clone(),
                    output_preview: preview,
                });

                // Apply success feedback
                self.apply_savant_feedback(&savant_id, &task.id, &task.description, true);
            }
            Err(error) => {
                task.fail(error.clone());

                self.event_log.push(OrchestrationEvent::TaskFailed {
                    task_id: task.id.clone(),
                    agent_id: savant_id.clone(),
                    error: error.clone(),
                    retry_count: 0,
                });

                self.apply_savant_feedback(&savant_id, &task.id, &task.description, false);
            }
        }

        result
    }

    // -----------------------------------------------------------------------
    // Cross-domain delegation
    // -----------------------------------------------------------------------

    /// Delegate a task from one savant to another in a different domain.
    pub fn cross_domain_delegate(
        &mut self,
        from_savant_id: &str,
        target_domain: SavantDomain,
        task_description: &str,
        context: Option<&str>,
    ) -> Result<CrossDomainDelegation, String> {
        // Verify the source savant can delegate to the target domain
        let can_delegate = self.registry.get(from_savant_id)
            .map(|e| e.can_delegate_to(target_domain))
            .unwrap_or(false);
        let from_domain = self.registry.get(from_savant_id)
            .map(|e| e.domain)
            .unwrap_or(SavantDomain::General);

        if !can_delegate {
            return Err(format!(
                "Savant '{}' cannot delegate to domain '{}'",
                from_savant_id, target_domain
            ));
        }

        // Find or spawn a target savant
        let target_id = self.find_savant_for_domain(target_domain)
            .unwrap_or_else(|| self.spawn_savant(target_domain));

        // Create task
        let mut task = OrchestratedTask::new(task_description)
            .with_domain(target_domain)
            .with_priority(TaskPriority::High);
        if let Some(ctx) = context {
            task = task.with_context(ctx);
        }

        let delegation_id = Uuid::new_v4().to_string();

        self.event_log.push(OrchestrationEvent::DelegationRequested {
            request_id: delegation_id.clone(),
            from_agent: from_savant_id.to_string(),
            target_domain: Some(target_domain),
        });

        // Execute
        let result = self.execute_task(&mut task);

        let (success, result_text) = match result {
            Ok(output) => (Some(true), Some(output)),
            Err(_) => (Some(false), None),
        };

        self.event_log.push(OrchestrationEvent::DelegationCompleted {
            request_id: delegation_id.clone(),
            from_agent: target_id.clone(),
            success: success.unwrap_or(false),
        });

        let record = CrossDomainDelegation {
            id: delegation_id,
            from_savant: from_savant_id.to_string(),
            from_domain,
            to_savant: target_id,
            to_domain: target_domain,
            task_description: task_description.to_string(),
            success,
            result: result_text,
        };

        self.delegation_history.push(record.clone());
        Ok(record)
    }

    /// Find an idle savant for a specific domain.
    fn find_savant_for_domain(&self, domain: SavantDomain) -> Option<String> {
        self.registry.iter()
            .filter(|(_, e)| e.domain == domain && !e.busy)
            .max_by(|(_, a), (_, b)| {
                a.performance_score.partial_cmp(&b.performance_score).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(id, _)| id.clone())
    }

    // -----------------------------------------------------------------------
    // Feedback & skill adjustment
    // -----------------------------------------------------------------------

    /// Apply feedback to a savant and update its A2A card.
    fn apply_savant_feedback(
        &mut self,
        savant_id: &str,
        task_id: &str,
        task_description: &str,
        success: bool,
    ) {
        // Build feedback
        let relevant_skills: Vec<String> = self.registry.get(savant_id)
            .map(|e| {
                e.skills.iter()
                    .filter(|s| s.match_score(task_description) > 0.0)
                    .map(|s| s.id.clone())
                    .collect()
            })
            .unwrap_or_default();

        let feedback = if success {
            AgentFeedback::success(savant_id, task_id)
                .with_relevant_skills(relevant_skills)
        } else {
            AgentFeedback::failure(savant_id, task_id)
                .with_relevant_skills(relevant_skills)
        };

        // Apply to agent state and card
        // First build SpawnedAgentState from SavantEntry for the skill engine
        if let Some(entry) = self.registry.get(savant_id) {
            let mut state = SpawnedAgentState {
                id: entry.id.clone(),
                blueprint_id: entry.blueprint_id.clone(),
                skills: entry.skills.clone(),
                domain: entry.domain,
                busy: entry.busy,
                tasks_completed: entry.tasks_completed,
                tasks_failed: entry.tasks_failed,
                performance_score: entry.performance_score,
                current_task: entry.current_task.clone(),
            };

            if let Some(card) = self.cards.get_mut(savant_id) {
                state.complete_task(success);
                let (_update, events) = self.skill_engine.apply_feedback(&feedback, &mut state, card);
                self.event_log.extend(events);

                // Sync state back to entry
                if let Some(entry) = self.registry.get_mut(savant_id) {
                    entry.skills = state.skills;
                    entry.performance_score = state.performance_score;
                    entry.tasks_completed = state.tasks_completed;
                    entry.tasks_failed = state.tasks_failed;
                    entry.busy = state.busy;
                    entry.current_task = state.current_task;
                }
            }
        }
    }

    /// Transfer skills from one savant to another.
    pub fn transfer_skills(
        &mut self,
        from_savant: &str,
        to_savant: &str,
        penalty: f64,
    ) -> Vec<SkillAdjustment> {
        let source_skills: Vec<SkillDescriptor> = self.registry.get(from_savant)
            .map(|e| e.skills.clone())
            .unwrap_or_default();

        if source_skills.is_empty() {
            return Vec::new();
        }

        let mut adjustments = Vec::new();
        let penalty = penalty.clamp(0.0, 1.0);

        if let Some(target_entry) = self.registry.get_mut(to_savant) {
            for src_skill in &source_skills {
                if !target_entry.skills.iter().any(|s| s.id == src_skill.id) {
                    let mut new_skill = src_skill.clone();
                    new_skill.proficiency = (src_skill.proficiency * (1.0 - penalty)).max(0.1);
                    target_entry.skills.push(new_skill);
                    adjustments.push(SkillAdjustment {
                        skill_id: src_skill.id.clone(),
                        adjustment_type: SkillAdjustmentType::SkillAdded,
                        old_proficiency: None,
                        new_proficiency: Some(src_skill.proficiency * (1.0 - penalty)),
                    });
                }
            }

            // Update card
            if let Some(card) = self.cards.get_mut(to_savant) {
                let state = SpawnedAgentState {
                    id: target_entry.id.clone(),
                    blueprint_id: target_entry.blueprint_id.clone(),
                    skills: target_entry.skills.clone(),
                    domain: target_entry.domain,
                    busy: target_entry.busy,
                    tasks_completed: target_entry.tasks_completed,
                    tasks_failed: target_entry.tasks_failed,
                    performance_score: target_entry.performance_score,
                    current_task: target_entry.current_task.clone(),
                };
                update_card_skills(card, &state);
            }

            if !adjustments.is_empty() {
                self.event_log.push(OrchestrationEvent::SkillsAdjusted {
                    agent_id: to_savant.to_string(),
                    adjustments: adjustments.clone(),
                });
            }
        }

        adjustments
    }

    // -----------------------------------------------------------------------
    // Introspection
    // -----------------------------------------------------------------------

    /// Get all A2A cards.
    pub fn get_cards(&self) -> Vec<&AgentCard> {
        self.cards.values().collect()
    }

    /// Get a savant entry by ID.
    pub fn get_savant(&self, savant_id: &str) -> Option<&SavantEntry> {
        self.registry.get(savant_id)
    }

    /// List all savants in a domain.
    pub fn savants_in_domain(&self, domain: SavantDomain) -> Vec<&SavantEntry> {
        self.registry.values()
            .filter(|e| e.domain == domain)
            .collect()
    }

    /// Get all spawned domain coverage.
    pub fn domain_coverage(&self) -> HashMap<SavantDomain, usize> {
        let mut coverage = HashMap::new();
        for entry in self.registry.values() {
            *coverage.entry(entry.domain).or_insert(0) += 1;
        }
        coverage
    }

    /// Get the event log.
    pub fn get_event_log(&self) -> &[OrchestrationEvent] {
        &self.event_log
    }

    /// Total skills across all savants.
    pub fn total_skills(&self) -> usize {
        self.registry.values().map(|e| e.skills.len()).sum()
    }

    /// Average performance across all savants.
    pub fn average_performance(&self) -> f64 {
        if self.registry.is_empty() {
            return 0.0;
        }
        let total: f64 = self.registry.values().map(|e| e.performance_score).sum();
        total / self.registry.len() as f64
    }
}

impl std::fmt::Debug for SavantCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SavantCoordinator")
            .field("savants", &self.registry.len())
            .field("domains_covered", &self.domain_coverage().len())
            .field("total_skills", &self.total_skills())
            .field("avg_performance", &self.average_performance())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_creation() {
        let coord = SavantCoordinator::new("openai/gpt-4o-mini");
        assert!(coord.registry.is_empty());
        #[cfg(feature = "chess")]
        assert_eq!(coord.blueprints.len(), 10); // All domains + chess
        #[cfg(not(feature = "chess"))]
        assert_eq!(coord.blueprints.len(), 9); // No chess
    }

    #[test]
    fn test_spawn_savant() {
        let mut coord = SavantCoordinator::new("openai/gpt-4o-mini");
        let id = coord.spawn_savant(SavantDomain::Research);

        assert!(coord.registry.contains_key(&id));
        assert!(coord.agents.contains_key(&id));
        assert!(coord.cards.contains_key(&id));

        let entry = coord.registry.get(&id).unwrap();
        assert_eq!(entry.domain, SavantDomain::Research);
        assert!(!entry.skills.is_empty());
        assert!(entry.auto_spawned);
    }

    #[test]
    fn test_spawn_all_domains() {
        let mut coord = SavantCoordinator::new("openai/gpt-4o-mini");
        let ids = coord.spawn_all_domains();

        assert_eq!(ids.len(), 9);
        assert_eq!(coord.domain_coverage().len(), 9);
    }

    #[test]
    fn test_route_task_domain_preference() {
        let mut coord = SavantCoordinator::new("openai/gpt-4o-mini");
        coord.spawn_all_domains();

        let task = OrchestratedTask::new("Research Rust async patterns")
            .with_domain(SavantDomain::Research);

        let decision = coord.route_task(&task);
        assert_eq!(decision.domain, SavantDomain::Research);
        assert!(decision.match_score > 0.0);
        assert!(!decision.auto_spawned);
    }

    #[test]
    fn test_route_task_auto_spawn() {
        let mut coord = SavantCoordinator::new("openai/gpt-4o-mini");
        // Don't spawn any savants

        let task = OrchestratedTask::new("Deploy to Kubernetes")
            .with_domain(SavantDomain::DevOps);

        let decision = coord.route_task(&task);
        assert_eq!(decision.domain, SavantDomain::DevOps);
        assert!(decision.auto_spawned);
        assert!(coord.registry.contains_key(&decision.savant_id));
    }

    #[test]
    fn test_savant_entry_delegation_targets() {
        let bp = savants::research_savant("openai/gpt-4o-mini");
        let entry = SavantEntry::from_blueprint("test-savant", &bp, false);

        // Research savant has delegation enabled
        assert!(entry.can_delegate_to(SavantDomain::Engineering));
        assert!(entry.can_delegate_to(SavantDomain::Security));
        // Should not be able to delegate to itself
        assert!(!entry.can_delegate_to(SavantDomain::Research));
    }

    #[test]
    fn test_savant_entry_lifecycle() {
        let bp = savants::engineering_savant("openai/gpt-4o-mini");
        let mut entry = SavantEntry::from_blueprint("eng-1", &bp, true);

        assert!(!entry.busy);
        assert_eq!(entry.performance_score, 1.0);

        entry.assign("task-1");
        assert!(entry.busy);

        entry.complete(true);
        assert!(!entry.busy);
        assert_eq!(entry.tasks_completed, 1);

        entry.assign("task-2");
        entry.complete(false);
        assert_eq!(entry.tasks_failed, 1);
        assert!(entry.performance_score < 1.0);
    }

    #[test]
    fn test_terminate_savant() {
        let mut coord = SavantCoordinator::new("openai/gpt-4o-mini");
        let id = coord.spawn_savant(SavantDomain::Security);
        assert!(coord.registry.contains_key(&id));

        coord.terminate_savant(&id, "no longer needed");
        assert!(!coord.registry.contains_key(&id));
        assert!(!coord.agents.contains_key(&id));
        assert!(!coord.cards.contains_key(&id));
    }

    #[test]
    fn test_transfer_skills_between_savants() {
        let mut coord = SavantCoordinator::new("openai/gpt-4o-mini");
        let research_id = coord.spawn_savant(SavantDomain::Research);
        let eng_id = coord.spawn_savant(SavantDomain::Engineering);

        let eng_skills_before = coord.registry.get(&eng_id).unwrap().skills.len();
        let adjustments = coord.transfer_skills(&research_id, &eng_id, 0.3);

        let eng_skills_after = coord.registry.get(&eng_id).unwrap().skills.len();
        assert!(eng_skills_after > eng_skills_before);
        assert!(!adjustments.is_empty());
    }

    #[test]
    fn test_domain_coverage() {
        let mut coord = SavantCoordinator::new("openai/gpt-4o-mini");
        coord.spawn_savant(SavantDomain::Research);
        coord.spawn_savant(SavantDomain::Research); // two research savants
        coord.spawn_savant(SavantDomain::Engineering);

        let coverage = coord.domain_coverage();
        assert_eq!(coverage.get(&SavantDomain::Research), Some(&2));
        assert_eq!(coverage.get(&SavantDomain::Engineering), Some(&1));
        assert_eq!(coverage.get(&SavantDomain::Security), None);
    }

    #[test]
    fn test_average_performance() {
        let mut coord = SavantCoordinator::new("openai/gpt-4o-mini");
        coord.spawn_all_domains();
        assert!((coord.average_performance() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_coordinator_debug() {
        let mut coord = SavantCoordinator::new("openai/gpt-4o-mini");
        coord.spawn_all_domains();
        let debug = format!("{:?}", coord);
        assert!(debug.contains("SavantCoordinator"));
        assert!(debug.contains("savants"));
    }

    #[test]
    fn test_a2a_cards_generated() {
        let mut coord = SavantCoordinator::new("openai/gpt-4o-mini");
        coord.spawn_all_domains();

        let cards = coord.get_cards();
        assert_eq!(cards.len(), 9);
        for card in cards {
            assert!(!card.skills.is_empty(), "Card '{}' should have skills", card.name);
            assert!(card.description.is_some());
        }
    }

    #[test]
    fn test_event_log_from_spawn() {
        let mut coord = SavantCoordinator::new("openai/gpt-4o-mini");
        coord.spawn_savant(SavantDomain::Engineering);

        let events = coord.get_event_log();
        let spawn_events: Vec<_> = events.iter()
            .filter(|e| matches!(e, OrchestrationEvent::AgentSpawned { .. }))
            .collect();
        assert_eq!(spawn_events.len(), 1);
    }

    #[test]
    fn test_find_savant_for_domain() {
        let mut coord = SavantCoordinator::new("openai/gpt-4o-mini");
        coord.spawn_savant(SavantDomain::Security);
        coord.spawn_savant(SavantDomain::Research);

        assert!(coord.find_savant_for_domain(SavantDomain::Security).is_some());
        assert!(coord.find_savant_for_domain(SavantDomain::Research).is_some());
        assert!(coord.find_savant_for_domain(SavantDomain::Design).is_none());
    }

    #[test]
    fn test_savants_in_domain() {
        let mut coord = SavantCoordinator::new("openai/gpt-4o-mini");
        coord.spawn_savant(SavantDomain::Engineering);
        coord.spawn_savant(SavantDomain::Engineering);
        coord.spawn_savant(SavantDomain::Research);

        assert_eq!(coord.savants_in_domain(SavantDomain::Engineering).len(), 2);
        assert_eq!(coord.savants_in_domain(SavantDomain::Research).len(), 1);
        assert_eq!(coord.savants_in_domain(SavantDomain::Security).len(), 0);
    }
}
