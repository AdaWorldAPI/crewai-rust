//! Prompt generation and management utilities for CrewAI agents.
//!
//! Corresponds to `crewai/utilities/prompts.py`.

use serde::{Deserialize, Serialize};

use crate::utilities::i18n::I18N;

/// Result with only prompt field for standard mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardPromptResult {
    /// The generated prompt string.
    pub prompt: String,
}

/// Result with system, user, and prompt fields for system prompt mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPromptResult {
    /// The system prompt component.
    pub system: String,
    /// The user prompt component.
    pub user: String,
    /// The full generated prompt string.
    pub prompt: String,
}

/// Unified prompt result returned by task_execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromptResult {
    Standard(StandardPromptResult),
    System(SystemPromptResult),
}

/// Component identifiers for prompt building.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptComponent {
    RolePlaying,
    Tools,
    NoTools,
    NativeTools,
    Task,
    NativeTask,
    TaskNoTools,
}

impl PromptComponent {
    fn as_str(&self) -> &'static str {
        match self {
            Self::RolePlaying => "role_playing",
            Self::Tools => "tools",
            Self::NoTools => "no_tools",
            Self::NativeTools => "native_tools",
            Self::Task => "task",
            Self::NativeTask => "native_task",
            Self::TaskNoTools => "task_no_tools",
        }
    }
}

/// Agent info needed for prompt interpolation.
pub struct AgentInfo {
    pub goal: String,
    pub role: String,
    pub backstory: String,
}

/// Manages and generates prompts for a generic agent.
#[derive(Debug, Clone)]
pub struct Prompts {
    /// Internationalization support.
    pub i18n: I18N,
    /// Indicates if the agent has access to tools.
    pub has_tools: bool,
    /// Whether to use native function calling instead of ReAct format.
    pub use_native_tool_calling: bool,
    /// Custom system prompt template.
    pub system_template: Option<String>,
    /// Custom user prompt template.
    pub prompt_template: Option<String>,
    /// Custom response prompt template.
    pub response_template: Option<String>,
    /// Whether to use the system prompt when no custom templates are provided.
    pub use_system_prompt: bool,
}

impl Default for Prompts {
    fn default() -> Self {
        Self {
            i18n: I18N::default(),
            has_tools: false,
            use_native_tool_calling: false,
            system_template: None,
            prompt_template: None,
            response_template: None,
            use_system_prompt: false,
        }
    }
}

impl Prompts {
    /// Generate a prompt result for task execution.
    pub fn task_execution(&self, agent: &AgentInfo) -> PromptResult {
        let mut slices: Vec<PromptComponent> = vec![PromptComponent::RolePlaying];

        if self.has_tools {
            if !self.use_native_tool_calling {
                slices.push(PromptComponent::Tools);
            }
        } else {
            slices.push(PromptComponent::NoTools);
        }

        let system = self.build_prompt(&slices, agent, None, None, None);

        let task_slice = if self.use_native_tool_calling {
            PromptComponent::NativeTask
        } else if self.has_tools {
            PromptComponent::Task
        } else {
            PromptComponent::TaskNoTools
        };

        slices.push(task_slice);

        if self.system_template.is_none()
            && self.prompt_template.is_none()
            && self.use_system_prompt
        {
            let user = self.build_prompt(&[task_slice], agent, None, None, None);
            let prompt = self.build_prompt(&slices, agent, None, None, None);
            PromptResult::System(SystemPromptResult {
                system,
                user,
                prompt,
            })
        } else {
            let prompt = self.build_prompt(
                &slices,
                agent,
                self.system_template.as_deref(),
                self.prompt_template.as_deref(),
                self.response_template.as_deref(),
            );
            PromptResult::Standard(StandardPromptResult { prompt })
        }
    }

    /// Build a prompt string from specified components.
    fn build_prompt(
        &self,
        components: &[PromptComponent],
        agent: &AgentInfo,
        system_template: Option<&str>,
        prompt_template: Option<&str>,
        response_template: Option<&str>,
    ) -> String {
        let prompt = if system_template.is_none() || prompt_template.is_none() {
            components
                .iter()
                .map(|c| self.i18n.slice(c.as_str()))
                .collect::<Vec<_>>()
                .join("")
        } else {
            let sys_tmpl = system_template.unwrap();
            let prm_tmpl = prompt_template.unwrap();

            let template_parts: String = components
                .iter()
                .filter(|c| **c != PromptComponent::Task)
                .map(|c| self.i18n.slice(c.as_str()))
                .collect::<Vec<_>>()
                .join("");

            let system = sys_tmpl.replace("{{ .System }}", &template_parts);
            let task_text = self.i18n.slice("task");
            let user = prm_tmpl.replace("{{ .Prompt }}", &task_text);

            if let Some(resp_tmpl) = response_template {
                let response = resp_tmpl.split("{{ .Response }}").next().unwrap_or("");
                format!("{}\n{}\n{}", system, user, response)
            } else {
                format!("{}\n{}", system, user)
            }
        };

        prompt
            .replace("{goal}", &agent.goal)
            .replace("{role}", &agent.role)
            .replace("{backstory}", &agent.backstory)
    }
}
