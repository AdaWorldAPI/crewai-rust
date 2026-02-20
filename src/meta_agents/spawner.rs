//! Auto-attended spawner meta-agent.
//!
//! The `SpawnerAgent` is a meta-agent that analyzes high-level objectives,
//! decomposes them into tasks, selects appropriate domains, spawns worker
//! agents, and handles delegation requests from running agents.
//!
//! Unlike the basic keyword-based `decompose_objective()` in the orchestrator,
//! the spawner uses structured analysis to produce better task decompositions
//! and agent assignments.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::delegation::{
    DelegationDispatch, DelegationRequest, DelegationResponse, DelegationResult,
    OrchestrationEvent,
};
use super::savants;
use super::types::{
    AgentBlueprint, OrchestratedTask, OrchestratedTaskStatus, SavantDomain,
    SkillDescriptor, SpawnedAgentState, TaskPriority,
};

// ---------------------------------------------------------------------------
// Task decomposition
// ---------------------------------------------------------------------------

/// A decomposed sub-task produced by the spawner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposedTask {
    /// Task description.
    pub description: String,
    /// Inferred domain.
    pub domain: SavantDomain,
    /// Required skills.
    pub required_skills: Vec<String>,
    /// Priority.
    pub priority: TaskPriority,
    /// Indices of tasks this depends on (within the decomposition).
    pub depends_on: Vec<usize>,
    /// Suggested tools.
    pub suggested_tools: Vec<String>,
}

/// Result of objective decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionPlan {
    /// Original objective.
    pub objective: String,
    /// Decomposed sub-tasks in execution order.
    pub tasks: Vec<DecomposedTask>,
    /// Domains involved.
    pub domains: Vec<SavantDomain>,
    /// Whether a synthesis step was added.
    pub has_synthesis: bool,
}

// ---------------------------------------------------------------------------
// SpawnerAgent
// ---------------------------------------------------------------------------

/// Auto-attended spawner meta-agent.
///
/// Analyzes objectives, creates task decompositions, and handles
/// delegation requests by selecting or spawning appropriate agents.
pub struct SpawnerAgent {
    /// Default LLM for spawned agents.
    pub default_llm: String,
    /// Available blueprints (registered + savants).
    pub blueprints: Vec<AgentBlueprint>,
    /// Domain keyword mappings for analysis.
    domain_keywords: HashMap<SavantDomain, Vec<&'static str>>,
    /// Events generated during spawning operations.
    events: Vec<OrchestrationEvent>,
}

impl SpawnerAgent {
    /// Create a new spawner with default savant blueprints.
    pub fn new(default_llm: impl Into<String>) -> Self {
        let llm = default_llm.into();
        let blueprints = savants::all_savants(&llm);

        let mut domain_keywords = HashMap::new();
        domain_keywords.insert(SavantDomain::Research, vec![
            "research", "find", "search", "investigate", "discover", "explore",
            "analyze", "study", "survey", "review literature", "look up", "information",
        ]);
        domain_keywords.insert(SavantDomain::Engineering, vec![
            "code", "implement", "build", "develop", "program", "software",
            "debug", "fix", "refactor", "architect", "design system", "deploy",
            "api", "database", "backend", "frontend", "function", "class",
        ]);
        domain_keywords.insert(SavantDomain::DataAnalysis, vec![
            "data", "analyze", "statistics", "metrics", "visualization", "chart",
            "graph", "trend", "pattern", "correlation", "regression", "dashboard",
            "csv", "dataset", "aggregate",
        ]);
        domain_keywords.insert(SavantDomain::ContentCreation, vec![
            "write", "content", "document", "article", "blog", "essay", "report",
            "copy", "draft", "edit", "proofread", "summarize", "narrative",
        ]);
        domain_keywords.insert(SavantDomain::Planning, vec![
            "plan", "strategy", "organize", "roadmap", "timeline", "milestone",
            "decompose", "prioritize", "schedule", "coordinate", "allocate",
        ]);
        domain_keywords.insert(SavantDomain::QualityAssurance, vec![
            "test", "quality", "qa", "verify", "validate", "check", "review",
            "regression", "edge case", "integration test", "unit test",
        ]);
        domain_keywords.insert(SavantDomain::Security, vec![
            "security", "vulnerability", "audit", "penetration", "threat",
            "authentication", "authorization", "encryption", "owasp", "secure",
            "credential", "injection", "xss",
        ]);
        domain_keywords.insert(SavantDomain::DevOps, vec![
            "deploy", "ci/cd", "docker", "kubernetes", "infrastructure",
            "monitoring", "logging", "pipeline", "container", "cloud",
        ]);

        Self {
            default_llm: llm,
            blueprints,
            domain_keywords,
            events: Vec::new(),
        }
    }

    /// Register an additional blueprint.
    pub fn register_blueprint(&mut self, bp: AgentBlueprint) {
        self.blueprints.push(bp);
    }

    // -----------------------------------------------------------------------
    // Objective decomposition
    // -----------------------------------------------------------------------

    /// Decompose a high-level objective into a structured execution plan.
    ///
    /// Uses multi-pass analysis:
    /// 1. Domain detection (keyword scoring)
    /// 2. Task extraction (sentence/clause analysis)
    /// 3. Dependency inference
    /// 4. Synthesis step addition
    pub fn decompose(&self, objective: &str) -> DecompositionPlan {
        // Pass 1: Detect domains with scoring
        let domain_scores = self.score_domains(objective);
        let active_domains: Vec<SavantDomain> = domain_scores.iter()
            .filter(|(_, score)| **score > 0.0)
            .map(|(domain, _)| *domain)
            .collect();

        // Pass 2: Extract sub-tasks
        let mut tasks = self.extract_tasks(objective, &active_domains);

        // Pass 3: Add planning step if multiple domains
        if active_domains.len() > 1 {
            let planning = DecomposedTask {
                description: format!("Analyze and plan approach for: {}", objective),
                domain: SavantDomain::Planning,
                required_skills: vec!["task_decomposition".into()],
                priority: TaskPriority::High,
                depends_on: vec![],
                suggested_tools: vec![],
            };
            // Insert planning at the beginning, adjust dependency indices
            for task in &mut tasks {
                task.depends_on = task.depends_on.iter().map(|i| i + 1).collect();
                task.depends_on.push(0); // All depend on planning
            }
            tasks.insert(0, planning);
        }

        // Pass 4: Add synthesis step if multiple tasks
        let has_synthesis = tasks.len() > 1;
        if has_synthesis {
            let dep_indices: Vec<usize> = (0..tasks.len()).collect();
            tasks.push(DecomposedTask {
                description: format!("Synthesize all results into final deliverable for: {}", objective),
                domain: if active_domains.contains(&SavantDomain::ContentCreation) {
                    SavantDomain::ContentCreation
                } else {
                    SavantDomain::Planning
                },
                required_skills: vec![],
                priority: TaskPriority::High,
                depends_on: dep_indices,
                suggested_tools: vec![],
            });
        }

        DecompositionPlan {
            objective: objective.to_string(),
            tasks,
            domains: active_domains,
            has_synthesis,
        }
    }

    /// Score each domain against the objective (0.0 = no match, higher = better).
    fn score_domains(&self, objective: &str) -> HashMap<SavantDomain, f64> {
        let lower = objective.to_lowercase();
        let mut scores = HashMap::new();

        for (domain, keywords) in &self.domain_keywords {
            let mut score = 0.0;
            for keyword in keywords {
                if lower.contains(keyword) {
                    score += 1.0;
                    // Bonus for exact word match
                    if lower.split_whitespace().any(|w| w == *keyword) {
                        score += 0.5;
                    }
                }
            }
            scores.insert(*domain, score);
        }

        scores
    }

    /// Extract tasks from the objective, one per active domain.
    fn extract_tasks(
        &self,
        objective: &str,
        domains: &[SavantDomain],
    ) -> Vec<DecomposedTask> {
        if domains.is_empty() {
            // Single general task
            return vec![DecomposedTask {
                description: objective.to_string(),
                domain: SavantDomain::General,
                required_skills: vec![],
                priority: TaskPriority::Medium,
                depends_on: vec![],
                suggested_tools: vec![],
            }];
        }

        // Try to extract distinct sub-objectives per domain from clauses
        let clauses = self.split_into_clauses(objective);
        let mut tasks = Vec::new();

        for domain in domains {
            // Find the best matching clause for this domain
            let domain_keywords: &[&str] = self.domain_keywords
                .get(domain)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            let best_clause = clauses.iter()
                .max_by_key(|clause| {
                    let lower = clause.to_lowercase();
                    domain_keywords.iter()
                        .filter(|kw| lower.contains(*kw))
                        .count()
                })
                .map(|s| s.as_str())
                .unwrap_or(objective);

            // Get suggested tools from the domain's savant blueprint
            let tools = self.blueprints.iter()
                .find(|bp| bp.domain == *domain)
                .map(|bp| bp.tools.clone())
                .unwrap_or_default();

            // Get required skills from the domain's savant
            let skills: Vec<String> = self.blueprints.iter()
                .find(|bp| bp.domain == *domain)
                .map(|bp| bp.skills.iter().map(|s| s.id.clone()).collect())
                .unwrap_or_default();

            tasks.push(DecomposedTask {
                description: format!("{} — focus on {} aspects", best_clause.trim(), domain),
                domain: *domain,
                required_skills: skills,
                priority: TaskPriority::Medium,
                depends_on: vec![],
                suggested_tools: tools,
            });
        }

        tasks
    }

    /// Split an objective into clauses (by "and", commas, semicolons).
    fn split_into_clauses(&self, text: &str) -> Vec<String> {
        let mut clauses: Vec<String> = text
            .split(|c: char| c == ',' || c == ';')
            .flat_map(|segment| segment.split(" and "))
            .map(|s| s.trim().to_string())
            .filter(|s| s.len() > 5) // Filter out tiny fragments
            .collect();

        // If we didn't get meaningful splits, return the whole thing
        if clauses.is_empty() {
            clauses.push(text.to_string());
        }

        clauses
    }

    /// Convert a decomposition plan into `OrchestratedTask` instances.
    pub fn plan_to_orchestrated_tasks(&self, plan: &DecompositionPlan) -> Vec<OrchestratedTask> {
        let mut tasks: Vec<OrchestratedTask> = Vec::new();
        let mut id_map: HashMap<usize, String> = HashMap::new(); // plan index → task ID

        for (i, decomposed) in plan.tasks.iter().enumerate() {
            let mut task = OrchestratedTask::new(&decomposed.description)
                .with_domain(decomposed.domain)
                .with_priority(decomposed.priority)
                .with_required_skills(decomposed.required_skills.clone());

            // Map dependency indices to task IDs
            let deps: Vec<String> = decomposed.depends_on.iter()
                .filter_map(|idx| id_map.get(idx).cloned())
                .collect();
            if !deps.is_empty() {
                task = task.with_dependencies(deps);
            }

            id_map.insert(i, task.id.clone());
            tasks.push(task);
        }

        tasks
    }

    // -----------------------------------------------------------------------
    // Delegation handling
    // -----------------------------------------------------------------------

    /// Handle a delegation request by selecting the best blueprint.
    ///
    /// Returns a dispatch indicating which agent should handle the delegation.
    pub fn handle_delegation(
        &mut self,
        request: &DelegationRequest,
        available_agents: &HashMap<String, SpawnedAgentState>,
    ) -> DelegationDispatch {
        // First try to find an existing idle agent
        let mut best_match: Option<(String, f64)> = None;

        for (agent_id, state) in available_agents {
            if state.busy {
                continue;
            }

            let mut score = state.best_skill_match(&request.task_description);

            // Domain bonus
            if let Some(ref domain) = request.target_domain {
                if state.domain == *domain {
                    score += 3.0;
                }
            }

            // Required skills check
            if !request.required_skills.is_empty() {
                let agent_skills: Vec<&str> = state.skills.iter().map(|s| s.id.as_str()).collect();
                let has_all = request.required_skills.iter().all(|rs| agent_skills.contains(&rs.as_str()));
                if has_all {
                    score += 2.0;
                }
            }

            // Performance weight
            score *= state.performance_score;

            if best_match.as_ref().map_or(true, |(_, best)| score > *best) {
                best_match = Some((agent_id.clone(), score));
            }
        }

        if let Some((agent_id, score)) = best_match {
            if score > 0.5 {
                self.events.push(OrchestrationEvent::DelegationDispatched {
                    request_id: request.id.clone(),
                    to_agent: agent_id.clone(),
                    match_score: score,
                });

                return DelegationDispatch {
                    request: request.clone(),
                    assigned_agent: agent_id,
                    match_score: score,
                    auto_spawned: false,
                };
            }
        }

        // No suitable agent found — select the best blueprint to spawn
        let domain = request.target_domain.unwrap_or(SavantDomain::General);
        let bp = savants::savant_for_domain(domain, &self.default_llm);
        let agent_id = format!("delegate-{}", Uuid::new_v4().to_string().split('-').next().unwrap_or("x"));

        self.events.push(OrchestrationEvent::AgentSpawned {
            agent_id: agent_id.clone(),
            domain: bp.domain,
            blueprint_id: bp.id.clone(),
            skills: bp.skills.iter().map(|s| s.id.clone()).collect(),
        });
        self.events.push(OrchestrationEvent::DelegationDispatched {
            request_id: request.id.clone(),
            to_agent: agent_id.clone(),
            match_score: 0.0,
        });

        DelegationDispatch {
            request: request.clone(),
            assigned_agent: agent_id,
            match_score: 0.0,
            auto_spawned: true,
        }
    }

    /// Get the best blueprint for a given domain.
    pub fn blueprint_for_domain(&self, domain: SavantDomain) -> AgentBlueprint {
        self.blueprints.iter()
            .find(|bp| bp.domain == domain)
            .cloned()
            .unwrap_or_else(|| savants::savant_for_domain(domain, &self.default_llm))
    }

    /// Drain all generated events.
    pub fn drain_events(&mut self) -> Vec<OrchestrationEvent> {
        std::mem::take(&mut self.events)
    }
}

impl std::fmt::Debug for SpawnerAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpawnerAgent")
            .field("default_llm", &self.default_llm)
            .field("blueprints", &self.blueprints.len())
            .field("pending_events", &self.events.len())
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
    fn test_spawner_creation() {
        let spawner = SpawnerAgent::new("openai/gpt-4o-mini");
        #[cfg(feature = "chess")]
        assert_eq!(spawner.blueprints.len(), 10); // 10 domain savants (all domains + chess)
        #[cfg(not(feature = "chess"))]
        assert_eq!(spawner.blueprints.len(), 9); // 9 domain savants (chess gated)
        assert!(!spawner.domain_keywords.is_empty());
    }

    #[test]
    fn test_domain_scoring() {
        let spawner = SpawnerAgent::new("openai/gpt-4o-mini");
        let scores = spawner.score_domains("research and implement a secure web application with tests");

        assert!(scores[&SavantDomain::Research] > 0.0);
        assert!(scores[&SavantDomain::Engineering] > 0.0);
        assert!(scores[&SavantDomain::Security] > 0.0);
        assert!(scores[&SavantDomain::QualityAssurance] > 0.0);
    }

    #[test]
    fn test_decompose_simple_objective() {
        let spawner = SpawnerAgent::new("openai/gpt-4o-mini");
        let plan = spawner.decompose("search the web for Rust async patterns");

        assert!(!plan.tasks.is_empty());
        assert!(plan.domains.contains(&SavantDomain::Research));
    }

    #[test]
    fn test_decompose_multi_domain() {
        let spawner = SpawnerAgent::new("openai/gpt-4o-mini");
        let plan = spawner.decompose("research Rust patterns, implement a web scraper, and write documentation");

        // Should have: planning + research + engineering + content + synthesis
        assert!(plan.tasks.len() >= 3);
        assert!(plan.has_synthesis);
        assert!(plan.domains.len() >= 2);
    }

    #[test]
    fn test_plan_to_orchestrated_tasks() {
        let spawner = SpawnerAgent::new("openai/gpt-4o-mini");
        let plan = spawner.decompose("research and implement a feature");
        let tasks = spawner.plan_to_orchestrated_tasks(&plan);

        assert_eq!(tasks.len(), plan.tasks.len());
        // Last task (synthesis) should have dependencies
        if tasks.len() > 1 {
            let last = tasks.last().unwrap();
            assert!(!last.dependencies.is_empty());
        }
    }

    #[test]
    fn test_handle_delegation_with_existing_agent() {
        let mut spawner = SpawnerAgent::new("openai/gpt-4o-mini");
        let bp = savants::research_savant("openai/gpt-4o-mini");
        let state = SpawnedAgentState::new("agent-research", &bp);

        let mut agents = HashMap::new();
        agents.insert("agent-research".to_string(), state);

        let request = DelegationRequest::new("agent-eng", "research Rust patterns")
            .with_domain(SavantDomain::Research);

        let dispatch = spawner.handle_delegation(&request, &agents);

        assert_eq!(dispatch.assigned_agent, "agent-research");
        assert!(!dispatch.auto_spawned);
        assert!(dispatch.match_score > 0.0);
    }

    #[test]
    fn test_handle_delegation_auto_spawn() {
        let mut spawner = SpawnerAgent::new("openai/gpt-4o-mini");

        let request = DelegationRequest::new("agent-eng", "audit for security vulnerabilities")
            .with_domain(SavantDomain::Security);

        let dispatch = spawner.handle_delegation(&request, &HashMap::new());

        assert!(dispatch.auto_spawned);
        assert!(dispatch.assigned_agent.starts_with("delegate-"));
    }

    #[test]
    fn test_split_into_clauses() {
        let spawner = SpawnerAgent::new("openai/gpt-4o-mini");
        let clauses = spawner.split_into_clauses("research patterns, implement code, and write docs");
        assert!(clauses.len() >= 2);
    }

    #[test]
    fn test_events_from_delegation() {
        let mut spawner = SpawnerAgent::new("openai/gpt-4o-mini");

        let request = DelegationRequest::new("agent-1", "do something")
            .with_domain(SavantDomain::General);
        spawner.handle_delegation(&request, &HashMap::new());

        let events = spawner.drain_events();
        assert!(!events.is_empty());

        // Second drain should be empty
        assert!(spawner.drain_events().is_empty());
    }

    #[test]
    fn test_blueprint_for_domain() {
        let spawner = SpawnerAgent::new("anthropic/claude-opus-4-5-20251101");
        let bp = spawner.blueprint_for_domain(SavantDomain::Security);
        assert_eq!(bp.domain, SavantDomain::Security);
        assert_eq!(bp.llm, "anthropic/claude-opus-4-5-20251101");
    }
}
