//! Skill adjustment engine with feedback loops.
//!
//! Processes `AgentFeedback` to update agent skills, proficiencies, and
//! A2A cards. Implements exponential moving average for proficiency scores,
//! skill discovery from task outcomes, and cross-agent skill transfer.

use std::collections::HashMap;

use crate::a2a::client::AgentCard;

use super::card_builder::update_card_skills;
use super::delegation::{
    AgentFeedback, CapabilityUpdate, CapabilityUpdateTrigger, OrchestrationEvent,
    SkillAdjustment, SkillAdjustmentType, TaskOutcome,
};
use super::types::{SkillDescriptor, SpawnedAgentState};

/// Configuration for the skill adjustment engine.
#[derive(Debug, Clone)]
pub struct SkillEngineConfig {
    /// EMA alpha for proficiency boost on success (0.0-1.0).
    pub success_alpha: f64,
    /// EMA alpha for proficiency penalty on failure (0.0-1.0).
    pub failure_alpha: f64,
    /// Minimum proficiency floor (skills never go below this).
    pub min_proficiency: f64,
    /// Maximum proficiency ceiling.
    pub max_proficiency: f64,
    /// Whether to auto-add skills discovered from task feedback.
    pub auto_discover_skills: bool,
    /// Proficiency threshold below which skills are removed.
    pub removal_threshold: f64,
    /// Initial proficiency for newly discovered skills.
    pub discovery_initial_proficiency: f64,
}

impl Default for SkillEngineConfig {
    fn default() -> Self {
        Self {
            success_alpha: 0.05,
            failure_alpha: 0.08,
            min_proficiency: 0.1,
            max_proficiency: 1.0,
            auto_discover_skills: true,
            removal_threshold: 0.05,
            discovery_initial_proficiency: 0.5,
        }
    }
}

/// The skill adjustment engine.
///
/// Processes feedback and applies adjustments to agent states and A2A cards.
pub struct SkillEngine {
    /// Engine configuration.
    pub config: SkillEngineConfig,
    /// Event log from adjustments.
    events: Vec<OrchestrationEvent>,
}

impl SkillEngine {
    /// Create a new skill engine with the given configuration.
    pub fn new(config: SkillEngineConfig) -> Self {
        Self {
            config,
            events: Vec::new(),
        }
    }

    /// Create a skill engine with default configuration.
    pub fn default_engine() -> Self {
        Self::new(SkillEngineConfig::default())
    }

    /// Process feedback and apply adjustments to the agent state and card.
    ///
    /// Returns a `CapabilityUpdate` describing what changed, plus any
    /// `OrchestrationEvent`s generated.
    pub fn apply_feedback(
        &mut self,
        feedback: &AgentFeedback,
        state: &mut SpawnedAgentState,
        card: &mut AgentCard,
    ) -> (CapabilityUpdate, Vec<OrchestrationEvent>) {
        let mut adjustments: Vec<SkillAdjustment> = Vec::new();

        match feedback.outcome {
            TaskOutcome::ExcellentSuccess | TaskOutcome::Success => {
                self.apply_success_adjustments(feedback, state, &mut adjustments);
            }
            TaskOutcome::PartialSuccess => {
                // Mild boost for relevant skills, mild penalty for non-relevant
                self.apply_partial_success_adjustments(feedback, state, &mut adjustments);
            }
            TaskOutcome::Failure | TaskOutcome::Timeout => {
                self.apply_failure_adjustments(feedback, state, &mut adjustments);
            }
        }

        // Apply explicit proficiency deltas from feedback
        for (skill_id, delta) in &feedback.proficiency_deltas {
            if let Some(skill) = state.skills.iter_mut().find(|s| s.id == *skill_id) {
                let old = skill.proficiency;
                skill.proficiency = (skill.proficiency + delta)
                    .clamp(self.config.min_proficiency, self.config.max_proficiency);
                let adj_type = if *delta >= 0.0 {
                    SkillAdjustmentType::ProficiencyBoosted
                } else {
                    SkillAdjustmentType::ProficiencyReduced
                };
                adjustments.push(SkillAdjustment {
                    skill_id: skill_id.clone(),
                    adjustment_type: adj_type,
                    old_proficiency: Some(old),
                    new_proficiency: Some(skill.proficiency),
                });
            }
        }

        // Auto-discover new skills from suggested_skills
        if self.config.auto_discover_skills {
            for suggested in &feedback.suggested_skills {
                if !state.skills.iter().any(|s| s.id == suggested.id) {
                    let mut new_skill = suggested.clone();
                    new_skill.proficiency = self.config.discovery_initial_proficiency;
                    state.add_skill(new_skill);
                    adjustments.push(SkillAdjustment {
                        skill_id: suggested.id.clone(),
                        adjustment_type: SkillAdjustmentType::SkillAdded,
                        old_proficiency: None,
                        new_proficiency: Some(self.config.discovery_initial_proficiency),
                    });
                }
            }
        }

        // Remove skills below threshold
        let to_remove: Vec<String> = state.skills.iter()
            .filter(|s| s.proficiency < self.config.removal_threshold)
            .map(|s| s.id.clone())
            .collect();
        for skill_id in &to_remove {
            state.remove_skill(skill_id);
            adjustments.push(SkillAdjustment {
                skill_id: skill_id.clone(),
                adjustment_type: SkillAdjustmentType::SkillRemoved,
                old_proficiency: None,
                new_proficiency: None,
            });
        }

        // Update the A2A card
        update_card_skills(card, state);

        // Build events
        let mut events = Vec::new();
        if !adjustments.is_empty() {
            events.push(OrchestrationEvent::SkillsAdjusted {
                agent_id: state.id.clone(),
                adjustments: adjustments.clone(),
            });
        }
        events.push(OrchestrationEvent::CardUpdated {
            agent_id: state.id.clone(),
            skill_count: state.skills.len(),
            performance: state.performance_score,
        });

        self.events.extend(events.clone());

        let update = CapabilityUpdate {
            agent_id: state.id.clone(),
            skills: state.skills.clone(),
            performance_score: state.performance_score,
            domain: state.domain,
            trigger: CapabilityUpdateTrigger::TaskOutcome,
        };

        (update, events)
    }

    /// Apply adjustments for a successful outcome.
    fn apply_success_adjustments(
        &self,
        feedback: &AgentFeedback,
        state: &mut SpawnedAgentState,
        adjustments: &mut Vec<SkillAdjustment>,
    ) {
        // Boost proficiency for skills that match the task
        for skill in &mut state.skills {
            if feedback.relevant_skills.contains(&skill.id) {
                let old = skill.proficiency;
                // EMA: new = old + alpha * (1.0 - old)
                skill.proficiency = (skill.proficiency + self.config.success_alpha * (self.config.max_proficiency - skill.proficiency))
                    .min(self.config.max_proficiency);
                adjustments.push(SkillAdjustment {
                    skill_id: skill.id.clone(),
                    adjustment_type: SkillAdjustmentType::ProficiencyBoosted,
                    old_proficiency: Some(old),
                    new_proficiency: Some(skill.proficiency),
                });
            }
        }

        // Add missing skills with low initial proficiency
        if self.config.auto_discover_skills {
            for missing in &feedback.missing_skills {
                if !state.skills.iter().any(|s| s.id == *missing) {
                    let new_skill = SkillDescriptor::new(
                        missing,
                        missing,
                        format!("Discovered as needed during task {}", feedback.task_id),
                    ).with_proficiency(self.config.discovery_initial_proficiency);
                    state.add_skill(new_skill);
                    adjustments.push(SkillAdjustment {
                        skill_id: missing.clone(),
                        adjustment_type: SkillAdjustmentType::SkillAdded,
                        old_proficiency: None,
                        new_proficiency: Some(self.config.discovery_initial_proficiency),
                    });
                }
            }
        }

        // Update performance score
        state.performance_score = (state.performance_score * 0.9 + 0.1).min(1.0);
    }

    /// Apply adjustments for a partial success.
    fn apply_partial_success_adjustments(
        &self,
        feedback: &AgentFeedback,
        state: &mut SpawnedAgentState,
        adjustments: &mut Vec<SkillAdjustment>,
    ) {
        // Mild boost for relevant skills (half the success alpha)
        let mild_alpha = self.config.success_alpha * 0.5;
        for skill in &mut state.skills {
            if feedback.relevant_skills.contains(&skill.id) {
                let old = skill.proficiency;
                skill.proficiency = (skill.proficiency + mild_alpha * (self.config.max_proficiency - skill.proficiency))
                    .min(self.config.max_proficiency);
                adjustments.push(SkillAdjustment {
                    skill_id: skill.id.clone(),
                    adjustment_type: SkillAdjustmentType::ProficiencyBoosted,
                    old_proficiency: Some(old),
                    new_proficiency: Some(skill.proficiency),
                });
            }
        }
    }

    /// Apply adjustments for a failure outcome.
    fn apply_failure_adjustments(
        &self,
        feedback: &AgentFeedback,
        state: &mut SpawnedAgentState,
        adjustments: &mut Vec<SkillAdjustment>,
    ) {
        // Reduce proficiency for relevant skills
        for skill in &mut state.skills {
            if feedback.relevant_skills.contains(&skill.id) {
                let old = skill.proficiency;
                // EMA down: new = old - alpha * old
                skill.proficiency = (skill.proficiency * (1.0 - self.config.failure_alpha))
                    .max(self.config.min_proficiency);
                adjustments.push(SkillAdjustment {
                    skill_id: skill.id.clone(),
                    adjustment_type: SkillAdjustmentType::ProficiencyReduced,
                    old_proficiency: Some(old),
                    new_proficiency: Some(skill.proficiency),
                });
            }
        }

        // Penalize performance score
        state.performance_score = (state.performance_score * 0.85).max(0.1);
    }

    /// Transfer skills from one agent to another.
    ///
    /// Copies skills that the source has but the target doesn't,
    /// with reduced proficiency (transfer penalty).
    pub fn transfer_skills(
        &self,
        source: &SpawnedAgentState,
        target: &mut SpawnedAgentState,
        transfer_penalty: f64,
    ) -> Vec<SkillAdjustment> {
        let mut adjustments = Vec::new();
        let penalty = transfer_penalty.clamp(0.0, 1.0);

        for src_skill in &source.skills {
            if !target.skills.iter().any(|s| s.id == src_skill.id) {
                let mut new_skill = src_skill.clone();
                new_skill.proficiency = (src_skill.proficiency * (1.0 - penalty))
                    .max(self.config.min_proficiency);
                target.add_skill(new_skill);
                adjustments.push(SkillAdjustment {
                    skill_id: src_skill.id.clone(),
                    adjustment_type: SkillAdjustmentType::SkillAdded,
                    old_proficiency: None,
                    new_proficiency: Some(src_skill.proficiency * (1.0 - penalty)),
                });
            }
        }

        adjustments
    }

    /// Get all events generated by the engine.
    pub fn drain_events(&mut self) -> Vec<OrchestrationEvent> {
        std::mem::take(&mut self.events)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta_agents::card_builder::build_card_from_state;
    use crate::meta_agents::types::{AgentBlueprint, SavantDomain};

    fn make_agent() -> (SpawnedAgentState, AgentCard) {
        let bp = AgentBlueprint::new("Test", "Goal", "Back", "openai/gpt-4o-mini", SavantDomain::Research)
            .with_skill(SkillDescriptor::new("web_research", "Web Research", "Search web").with_proficiency(0.8))
            .with_skill(SkillDescriptor::new("synthesis", "Synthesis", "Combine info").with_proficiency(0.7));
        let state = SpawnedAgentState::new("agent-test", &bp);
        let card = build_card_from_state(&state, "http://localhost");
        (state, card)
    }

    #[test]
    fn test_success_boosts_proficiency() {
        let mut engine = SkillEngine::default_engine();
        let (mut state, mut card) = make_agent();

        let old_prof = state.skills[0].proficiency;
        let feedback = AgentFeedback::success("agent-test", "task-1")
            .with_relevant_skills(vec!["web_research".into()]);

        let (update, events) = engine.apply_feedback(&feedback, &mut state, &mut card);

        assert!(state.skills[0].proficiency > old_prof);
        assert_eq!(update.agent_id, "agent-test");
        assert!(!events.is_empty());
    }

    #[test]
    fn test_failure_reduces_proficiency() {
        let mut engine = SkillEngine::default_engine();
        let (mut state, mut card) = make_agent();

        let old_prof = state.skills[0].proficiency;
        let feedback = AgentFeedback::failure("agent-test", "task-1")
            .with_relevant_skills(vec!["web_research".into()]);

        engine.apply_feedback(&feedback, &mut state, &mut card);

        assert!(state.skills[0].proficiency < old_prof);
    }

    #[test]
    fn test_missing_skills_auto_discovered() {
        let mut engine = SkillEngine::default_engine();
        let (mut state, mut card) = make_agent();
        assert_eq!(state.skills.len(), 2);

        let feedback = AgentFeedback::success("agent-test", "task-1")
            .with_missing_skills(vec!["data_analysis".into()]);

        engine.apply_feedback(&feedback, &mut state, &mut card);

        assert_eq!(state.skills.len(), 3);
        let new_skill = state.skills.iter().find(|s| s.id == "data_analysis").unwrap();
        assert!((new_skill.proficiency - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_explicit_proficiency_deltas() {
        let mut engine = SkillEngine::default_engine();
        let (mut state, mut card) = make_agent();

        let feedback = AgentFeedback::success("agent-test", "task-1")
            .with_proficiency_delta("web_research", 0.1);

        let old = state.skills[0].proficiency;
        engine.apply_feedback(&feedback, &mut state, &mut card);
        assert!((state.skills[0].proficiency - (old + 0.1)).abs() < 0.01 || state.skills[0].proficiency > old);
    }

    #[test]
    fn test_skill_transfer() {
        let engine = SkillEngine::default_engine();
        let (source, _) = make_agent();

        let bp = AgentBlueprint::new("Target", "Goal", "Back", "openai/gpt-4o-mini", SavantDomain::Engineering);
        let mut target = SpawnedAgentState::new("agent-target", &bp);
        assert!(target.skills.is_empty());

        let adjustments = engine.transfer_skills(&source, &mut target, 0.3);

        assert_eq!(target.skills.len(), 2);
        assert_eq!(adjustments.len(), 2);
        // Transfer penalty should reduce proficiency
        assert!(target.skills[0].proficiency < source.skills[0].proficiency);
    }

    #[test]
    fn test_skills_below_threshold_removed() {
        let mut engine = SkillEngine::new(SkillEngineConfig {
            removal_threshold: 0.15,
            failure_alpha: 0.95, // Aggressive penalty for testing
            ..SkillEngineConfig::default()
        });
        let (mut state, mut card) = make_agent();

        // Set one skill very low
        state.skills[1].proficiency = 0.12;

        let feedback = AgentFeedback::success("agent-test", "task-1");
        engine.apply_feedback(&feedback, &mut state, &mut card);

        // The low-proficiency skill should have been removed
        assert_eq!(state.skills.len(), 1);
        assert_eq!(state.skills[0].id, "web_research");
    }

    #[test]
    fn test_drain_events() {
        let mut engine = SkillEngine::default_engine();
        let (mut state, mut card) = make_agent();

        let feedback = AgentFeedback::success("agent-test", "task-1");
        engine.apply_feedback(&feedback, &mut state, &mut card);

        let events = engine.drain_events();
        assert!(!events.is_empty());

        // Second drain should be empty
        let events2 = engine.drain_events();
        assert!(events2.is_empty());
    }
}
