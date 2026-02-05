//! RBAC (Role-Based Access Control) manager.
//!
//! Maps agents → roles → capabilities. Provides the bridge between
//! agent card role assignments and capability RBAC requirements.

use std::collections::{HashMap, HashSet};

/// RBAC manager: tracks role assignments and capability grants.
#[derive(Debug, Default)]
pub struct RbacManager {
    /// Agent → roles mapping
    agent_roles: HashMap<String, HashSet<String>>,

    /// Role → capabilities mapping
    role_capabilities: HashMap<String, HashSet<String>>,
}

impl RbacManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Assign a role to an agent.
    pub fn assign_role(&mut self, agent_id: &str, role: &str) {
        self.agent_roles
            .entry(agent_id.to_string())
            .or_default()
            .insert(role.to_string());
    }

    /// Remove a role from an agent.
    pub fn revoke_role(&mut self, agent_id: &str, role: &str) -> bool {
        if let Some(roles) = self.agent_roles.get_mut(agent_id) {
            roles.remove(role)
        } else {
            false
        }
    }

    /// Grant a capability to a role.
    pub fn grant_capability_to_role(&mut self, role: &str, capability_id: &str) {
        self.role_capabilities
            .entry(role.to_string())
            .or_default()
            .insert(capability_id.to_string());
    }

    /// Revoke a capability from a role.
    pub fn revoke_capability_from_role(&mut self, role: &str, capability_id: &str) -> bool {
        if let Some(caps) = self.role_capabilities.get_mut(role) {
            caps.remove(capability_id)
        } else {
            false
        }
    }

    /// Get all roles assigned to an agent.
    pub fn get_agent_roles(&self, agent_id: &str) -> Vec<&str> {
        self.agent_roles
            .get(agent_id)
            .map(|roles| roles.iter().map(|r| r.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get all capabilities accessible to an agent (through its roles).
    pub fn get_agent_capabilities(&self, agent_id: &str) -> HashSet<&str> {
        let roles = match self.agent_roles.get(agent_id) {
            Some(r) => r,
            None => return HashSet::new(),
        };

        let mut caps = HashSet::new();
        for role in roles {
            if let Some(role_caps) = self.role_capabilities.get(role) {
                for cap in role_caps {
                    caps.insert(cap.as_str());
                }
            }
        }
        caps
    }

    /// Check if an agent can use a specific capability.
    pub fn can_use_capability(&self, agent_id: &str, capability_id: &str) -> bool {
        let caps = self.get_agent_capabilities(agent_id);
        caps.contains(capability_id)
    }

    /// Check if an agent has a specific role.
    pub fn has_role(&self, agent_id: &str, role: &str) -> bool {
        self.agent_roles
            .get(agent_id)
            .map(|roles| roles.contains(role))
            .unwrap_or(false)
    }

    /// List all known roles.
    pub fn all_roles(&self) -> Vec<&str> {
        let mut roles: HashSet<&str> = HashSet::new();
        for agent_roles in self.agent_roles.values() {
            for role in agent_roles {
                roles.insert(role.as_str());
            }
        }
        for role in self.role_capabilities.keys() {
            roles.insert(role.as_str());
        }
        roles.into_iter().collect()
    }

    /// List all agents with a specific role.
    pub fn agents_with_role(&self, role: &str) -> Vec<&str> {
        self.agent_roles
            .iter()
            .filter(|(_, roles)| roles.contains(role))
            .map(|(agent, _)| agent.as_str())
            .collect()
    }

    /// Get a summary of the RBAC state.
    pub fn summary(&self) -> RbacSummary {
        RbacSummary {
            total_agents: self.agent_roles.len(),
            total_roles: self.all_roles().len(),
            total_capabilities: self
                .role_capabilities
                .values()
                .flat_map(|caps| caps.iter())
                .collect::<HashSet<_>>()
                .len(),
        }
    }
}

/// Summary of RBAC state
#[derive(Debug)]
pub struct RbacSummary {
    pub total_agents: usize,
    pub total_roles: usize,
    pub total_capabilities: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_assignment() {
        let mut rbac = RbacManager::new();

        rbac.assign_role("agent-1", "admin");
        rbac.assign_role("agent-1", "researcher");
        rbac.assign_role("agent-2", "viewer");

        assert!(rbac.has_role("agent-1", "admin"));
        assert!(rbac.has_role("agent-1", "researcher"));
        assert!(!rbac.has_role("agent-1", "viewer"));
        assert!(rbac.has_role("agent-2", "viewer"));
    }

    #[test]
    fn test_capability_grants() {
        let mut rbac = RbacManager::new();

        rbac.assign_role("agent-1", "server_admin");
        rbac.grant_capability_to_role("server_admin", "minecraft:server_control");
        rbac.grant_capability_to_role("server_admin", "minecraft:player_management");

        assert!(rbac.can_use_capability("agent-1", "minecraft:server_control"));
        assert!(rbac.can_use_capability("agent-1", "minecraft:player_management"));
        assert!(!rbac.can_use_capability("agent-2", "minecraft:server_control"));
    }

    #[test]
    fn test_revocation() {
        let mut rbac = RbacManager::new();

        rbac.assign_role("agent-1", "admin");
        rbac.grant_capability_to_role("admin", "cap:delete");

        assert!(rbac.can_use_capability("agent-1", "cap:delete"));

        rbac.revoke_role("agent-1", "admin");
        assert!(!rbac.can_use_capability("agent-1", "cap:delete"));
    }

    #[test]
    fn test_agents_with_role() {
        let mut rbac = RbacManager::new();

        rbac.assign_role("alpha", "researcher");
        rbac.assign_role("beta", "researcher");
        rbac.assign_role("gamma", "admin");

        let researchers = rbac.agents_with_role("researcher");
        assert_eq!(researchers.len(), 2);
        assert!(researchers.contains(&"alpha"));
        assert!(researchers.contains(&"beta"));
    }
}
