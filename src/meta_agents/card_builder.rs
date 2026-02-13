//! A2A agent card builder for meta-agents.
//!
//! Generates and dynamically updates A2A agent cards based on agent blueprints
//! and runtime skill adjustments. Cards are the primary mechanism through which
//! agents advertise their capabilities to the orchestrator and to each other.

use crate::a2a::client::{AgentCapabilities, AgentCard, AgentProvider, AgentSkill};

use super::types::{AgentBlueprint, SkillDescriptor, SpawnedAgentState};

/// Builds an A2A `AgentCard` from an `AgentBlueprint`.
///
/// The generated card includes:
/// - Agent name, description, and URL
/// - Skills derived from the blueprint's `SkillDescriptor` list
/// - Capabilities (streaming, multi-turn, push notifications)
/// - Provider organization info
///
/// # Arguments
///
/// * `blueprint` - The agent blueprint to convert.
/// * `base_url` - Base URL where the agent is reachable.
pub fn build_card_from_blueprint(blueprint: &AgentBlueprint, base_url: &str) -> AgentCard {
    let skills = blueprint.skills.iter()
        .map(|s| skill_descriptor_to_a2a_skill(s))
        .collect();

    AgentCard {
        name: blueprint.role.clone(),
        description: Some(format!(
            "{}. Domain: {}. LLM: {}.",
            blueprint.goal, blueprint.domain, blueprint.llm,
        )),
        url: format!("{}/agents/{}", base_url, blueprint.id),
        version: Some("1.0.0".to_string()),
        capabilities: AgentCapabilities {
            streaming: false,
            push_notifications: false,
            multi_turn: blueprint.allow_delegation,
        },
        skills,
        provider: Some(AgentProvider {
            organization: "CrewAI Meta-Agent System".to_string(),
            url: Some(base_url.to_string()),
        }),
        default_input_modes: vec!["text/plain".to_string(), "application/json".to_string()],
        default_output_modes: vec!["text/plain".to_string(), "application/json".to_string()],
        security_schemes: Vec::new(),
        extensions: Vec::new(),
    }
}

/// Builds an A2A `AgentCard` from a live `SpawnedAgentState`.
///
/// This version uses the agent's current (potentially adjusted) skills
/// rather than the original blueprint skills, reflecting runtime changes
/// made by the orchestrator.
///
/// # Arguments
///
/// * `state` - The current spawned agent state.
/// * `base_url` - Base URL where the agent is reachable.
pub fn build_card_from_state(state: &SpawnedAgentState, base_url: &str) -> AgentCard {
    let skills = state.skills.iter()
        .map(|s| skill_descriptor_to_a2a_skill(s))
        .collect();

    AgentCard {
        name: state.id.clone(),
        description: Some(format!(
            "Agent in {} domain. Performance: {:.0}%. Tasks completed: {}.",
            state.domain, state.performance_score * 100.0, state.tasks_completed,
        )),
        url: format!("{}/agents/{}", base_url, state.id),
        version: Some("1.0.0".to_string()),
        capabilities: AgentCapabilities {
            streaming: false,
            push_notifications: false,
            multi_turn: true,
        },
        skills,
        provider: Some(AgentProvider {
            organization: "CrewAI Meta-Agent Pool".to_string(),
            url: Some(base_url.to_string()),
        }),
        default_input_modes: vec!["text/plain".to_string(), "application/json".to_string()],
        default_output_modes: vec!["text/plain".to_string(), "application/json".to_string()],
        security_schemes: Vec::new(),
        extensions: Vec::new(),
    }
}

/// Convert an internal `SkillDescriptor` to an A2A protocol `AgentSkill`.
fn skill_descriptor_to_a2a_skill(skill: &SkillDescriptor) -> AgentSkill {
    AgentSkill {
        id: skill.id.clone(),
        name: skill.name.clone(),
        description: Some(skill.description.clone()),
        input_modes: skill.input_modes.clone(),
        output_modes: skill.output_modes.clone(),
        tags: skill.tags.clone(),
    }
}

/// Update an existing agent card with new skills from adjusted state.
///
/// This is used when the orchestrator dynamically adjusts an agent's
/// skills (adding or removing capabilities based on task performance).
pub fn update_card_skills(card: &mut AgentCard, state: &SpawnedAgentState) {
    card.skills = state.skills.iter()
        .map(|s| skill_descriptor_to_a2a_skill(s))
        .collect();

    // Update description to reflect performance changes
    card.description = Some(format!(
        "Agent in {} domain. Performance: {:.0}%. Tasks completed: {}.",
        state.domain, state.performance_score * 100.0, state.tasks_completed,
    ));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta_agents::types::{SavantDomain};

    #[test]
    fn test_build_card_from_blueprint() {
        let bp = AgentBlueprint::new(
            "Code Reviewer",
            "Review code for bugs and best practices",
            "Senior engineer with 10 years of experience",
            "openai/gpt-4o",
            SavantDomain::Engineering,
        )
        .with_skill(SkillDescriptor::new("code_review", "Code Review", "Review source code"))
        .with_skill(SkillDescriptor::new("security_audit", "Security Audit", "Check for vulnerabilities"));

        let card = build_card_from_blueprint(&bp, "https://crew.local");

        assert_eq!(card.name, "Code Reviewer");
        assert_eq!(card.skills.len(), 2);
        assert_eq!(card.skills[0].id, "code_review");
        assert_eq!(card.skills[1].id, "security_audit");
        assert!(card.url.starts_with("https://crew.local"));
        assert!(card.description.as_ref().unwrap().contains("engineering"));
    }

    #[test]
    fn test_build_card_from_state() {
        let bp = AgentBlueprint::new("Worker", "Do work", "Backstory", "openai/gpt-4o-mini", SavantDomain::General);
        let mut state = SpawnedAgentState::new("agent-42", &bp);
        state.add_skill(SkillDescriptor::new("s1", "Skill One", "First skill"));
        state.tasks_completed = 5;
        state.performance_score = 0.85;

        let card = build_card_from_state(&state, "https://pool.local");

        assert_eq!(card.name, "agent-42");
        assert_eq!(card.skills.len(), 1);
        assert!(card.description.as_ref().unwrap().contains("85%"));
        assert!(card.description.as_ref().unwrap().contains("5"));
    }

    #[test]
    fn test_update_card_skills() {
        let bp = AgentBlueprint::new("Worker", "Do work", "Backstory", "openai/gpt-4o-mini", SavantDomain::General);
        let mut state = SpawnedAgentState::new("agent-1", &bp);

        let mut card = build_card_from_state(&state, "https://local");
        assert_eq!(card.skills.len(), 0);

        // Add skills and update
        state.add_skill(SkillDescriptor::new("new_skill", "New Skill", "Added at runtime"));
        update_card_skills(&mut card, &state);

        assert_eq!(card.skills.len(), 1);
        assert_eq!(card.skills[0].id, "new_skill");
    }
}
