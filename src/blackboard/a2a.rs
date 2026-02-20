//! A2A (Agent-to-Agent) awareness registry.
//!
//! The blackboard guarantees shared A2A awareness: every agent can discover
//! other agents, their capabilities, and their current state. This module
//! provides the registry that lives inside the blackboard.

use std::collections::HashMap;

/// Agent presence record in the A2A registry.
///
/// Each agent registers itself when it starts executing, and updates
/// its state as it progresses. Other agents can query the registry
/// to discover peers and coordinate.
#[derive(Debug, Clone)]
pub struct AgentPresence {
    /// Unique agent identifier.
    pub agent_id: String,
    /// Human-readable agent name.
    pub name: String,
    /// Agent role/description.
    pub role: String,
    /// Current state.
    pub state: AgentState,
    /// Capabilities this agent advertises.
    pub capabilities: Vec<String>,
    /// The agent's current goal/task description.
    pub current_goal: Option<String>,
    /// Last time this agent was active (epoch millis).
    pub last_active: i64,
    /// Arbitrary metadata (type-safe, no serde).
    pub tags: HashMap<String, String>,
}

/// Agent execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    /// Agent is registered but not yet executing.
    Idle,
    /// Agent is actively processing.
    Active,
    /// Agent is waiting for input (from another agent, tool, or human).
    Waiting,
    /// Agent has completed its task.
    Completed,
    /// Agent encountered an error.
    Failed,
}

/// The A2A awareness registry.
///
/// Lives inside the [`Blackboard`](super::Blackboard) and provides
/// agent discovery and coordination primitives.
///
/// # Design
///
/// - No serialization: all data is native Rust types in-process.
/// - Phase-safe: only one subsystem mutates at a time (enforced by
///   the blackboard's `&mut self` discipline).
/// - Discovery: agents can query by capability, state, or name.
#[derive(Debug, Default)]
pub struct A2ARegistry {
    agents: HashMap<String, AgentPresence>,
}

impl A2ARegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Register an agent (or update an existing one).
    pub fn register(
        &mut self,
        agent_id: impl Into<String>,
        name: impl Into<String>,
        role: impl Into<String>,
        capabilities: Vec<String>,
    ) {
        let agent_id = agent_id.into();
        let presence = AgentPresence {
            agent_id: agent_id.clone(),
            name: name.into(),
            role: role.into(),
            state: AgentState::Idle,
            capabilities,
            current_goal: None,
            last_active: chrono::Utc::now().timestamp_millis(),
            tags: HashMap::new(),
        };
        self.agents.insert(agent_id, presence);
    }

    /// Update an agent's state.
    pub fn set_state(&mut self, agent_id: &str, state: AgentState) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.state = state;
            agent.last_active = chrono::Utc::now().timestamp_millis();
        }
    }

    /// Update an agent's current goal.
    pub fn set_goal(&mut self, agent_id: &str, goal: impl Into<String>) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.current_goal = Some(goal.into());
            agent.last_active = chrono::Utc::now().timestamp_millis();
        }
    }

    /// Remove an agent from the registry.
    pub fn unregister(&mut self, agent_id: &str) -> Option<AgentPresence> {
        self.agents.remove(agent_id)
    }

    /// Get an agent's presence record.
    pub fn get(&self, agent_id: &str) -> Option<&AgentPresence> {
        self.agents.get(agent_id)
    }

    /// Get a mutable reference to an agent's presence record.
    pub fn get_mut(&mut self, agent_id: &str) -> Option<&mut AgentPresence> {
        self.agents.get_mut(agent_id)
    }

    /// Find all agents with a given capability.
    pub fn by_capability(&self, capability: &str) -> Vec<&AgentPresence> {
        self.agents
            .values()
            .filter(|a| a.capabilities.iter().any(|c| c == capability))
            .collect()
    }

    /// Find all agents in a given state.
    pub fn by_state(&self, state: AgentState) -> Vec<&AgentPresence> {
        self.agents
            .values()
            .filter(|a| a.state == state)
            .collect()
    }

    /// Find all active agents.
    pub fn active_agents(&self) -> Vec<&AgentPresence> {
        self.by_state(AgentState::Active)
    }

    /// Get all registered agent IDs.
    pub fn agent_ids(&self) -> Vec<&str> {
        self.agents.keys().map(|k| k.as_str()).collect()
    }

    /// Get the number of registered agents.
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Iterate over all agents.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &AgentPresence)> {
        self.agents.iter().map(|(k, v)| (k.as_str(), v))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_register_and_get() {
        let mut reg = A2ARegistry::new();
        reg.register("agent-1", "Researcher", "Research agent", vec!["search".into(), "summarize".into()]);

        let agent = reg.get("agent-1").unwrap();
        assert_eq!(agent.name, "Researcher");
        assert_eq!(agent.state, AgentState::Idle);
        assert_eq!(agent.capabilities.len(), 2);
    }

    #[test]
    fn test_registry_state_transitions() {
        let mut reg = A2ARegistry::new();
        reg.register("agent-1", "Worker", "Worker agent", vec![]);

        reg.set_state("agent-1", AgentState::Active);
        assert_eq!(reg.get("agent-1").unwrap().state, AgentState::Active);

        reg.set_state("agent-1", AgentState::Completed);
        assert_eq!(reg.get("agent-1").unwrap().state, AgentState::Completed);
    }

    #[test]
    fn test_registry_by_capability() {
        let mut reg = A2ARegistry::new();
        reg.register("a1", "Researcher", "r", vec!["search".into()]);
        reg.register("a2", "Writer", "w", vec!["write".into()]);
        reg.register("a3", "Analyst", "a", vec!["search".into(), "analyze".into()]);

        let searchers = reg.by_capability("search");
        assert_eq!(searchers.len(), 2);

        let writers = reg.by_capability("write");
        assert_eq!(writers.len(), 1);
    }

    #[test]
    fn test_registry_by_state() {
        let mut reg = A2ARegistry::new();
        reg.register("a1", "A", "r", vec![]);
        reg.register("a2", "B", "r", vec![]);
        reg.register("a3", "C", "r", vec![]);

        reg.set_state("a1", AgentState::Active);
        reg.set_state("a2", AgentState::Active);

        let active = reg.active_agents();
        assert_eq!(active.len(), 2);

        let idle = reg.by_state(AgentState::Idle);
        assert_eq!(idle.len(), 1);
    }

    #[test]
    fn test_registry_unregister() {
        let mut reg = A2ARegistry::new();
        reg.register("a1", "Agent", "r", vec![]);
        assert_eq!(reg.len(), 1);

        let removed = reg.unregister("a1");
        assert!(removed.is_some());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn test_registry_goal() {
        let mut reg = A2ARegistry::new();
        reg.register("a1", "Agent", "r", vec![]);

        reg.set_goal("a1", "Analyze quarterly report");
        assert_eq!(
            reg.get("a1").unwrap().current_goal.as_deref(),
            Some("Analyze quarterly report")
        );
    }
}
