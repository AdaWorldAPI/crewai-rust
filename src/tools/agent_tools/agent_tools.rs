//! Agent tools manager.
//!
//! Corresponds to `crewai/tools/agent_tools/agent_tools.py`.
//!
//! Provides the `AgentTools` struct which creates delegation and question
//! tools for a set of agents.

use super::ask_question_tool::AskQuestionTool;
use super::delegate_work_tool::DelegateWorkTool;

/// Manager class for agent-related tools.
///
/// Creates delegation and question tools that enable inter-agent communication
/// within a crew.
#[derive(Debug, Clone)]
pub struct AgentTools {
    /// List of agent roles available for delegation and questions.
    /// Each entry is the agent's role string.
    pub agent_roles: Vec<String>,
}

impl AgentTools {
    /// Create a new `AgentTools` with the given agent roles.
    pub fn new(agent_roles: Vec<String>) -> Self {
        Self { agent_roles }
    }

    /// Get all available agent tools (delegation + question tools).
    ///
    /// Returns a tuple of `(DelegateWorkTool, AskQuestionTool)` configured
    /// with the available coworkers.
    pub fn tools(&self) -> (DelegateWorkTool, AskQuestionTool) {
        let coworkers = self.agent_roles.join(", ");

        let delegate_tool = DelegateWorkTool {
            name: "Delegate work to coworker".to_string(),
            description: format!(
                "Delegate a specific task to one of the following coworkers: {coworkers}\n\
                 The input to this tool should be the coworker, the task you want them to do, \
                 and ALL necessary context to execute the task."
            ),
            coworker_names: self.agent_roles.clone(),
        };

        let ask_tool = AskQuestionTool {
            name: "Ask question to coworker".to_string(),
            description: format!(
                "Ask a specific question to one of the following coworkers: {coworkers}\n\
                 The input to this tool should be the coworker, the question you have for them, \
                 and ALL necessary context to answer the question."
            ),
            coworker_names: self.agent_roles.clone(),
        };

        (delegate_tool, ask_tool)
    }
}
