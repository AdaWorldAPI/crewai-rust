//! Lightweight agent for standalone LLM interactions.
//!
//! Corresponds to `crewai/lite_agent.py` and `crewai/lite_agent_output.py`.
//!
//! `LiteAgent` provides a simpler agent interface than the full `Agent`,
//! suitable for direct LLM conversations without the overhead of crew
//! orchestration, task management, or complex tool handling.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::utilities::types::LLMMessage;

// ---------------------------------------------------------------------------
// LiteAgentOutput
// ---------------------------------------------------------------------------

/// Represents the result of a `LiteAgent` execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteAgentOutput {
    /// Raw output string from the agent.
    #[serde(default)]
    pub raw: String,
    /// Structured pydantic-like output (as JSON value).
    pub pydantic: Option<Value>,
    /// Role of the agent that produced this output.
    #[serde(default)]
    pub agent_role: String,
    /// Token usage metrics for this execution.
    pub usage_metrics: Option<HashMap<String, Value>>,
    /// Messages exchanged during execution.
    #[serde(default)]
    pub messages: Vec<LLMMessage>,
}

impl Default for LiteAgentOutput {
    fn default() -> Self {
        Self {
            raw: String::new(),
            pydantic: None,
            agent_role: String::new(),
            usage_metrics: None,
            messages: Vec::new(),
        }
    }
}

impl LiteAgentOutput {
    /// Create a new `LiteAgentOutput`.
    pub fn new(raw: impl Into<String>, agent_role: impl Into<String>) -> Self {
        Self {
            raw: raw.into(),
            agent_role: agent_role.into(),
            ..Default::default()
        }
    }

    /// Convert pydantic output to a dictionary (JSON map).
    pub fn to_dict(&self) -> Value {
        self.pydantic
            .clone()
            .unwrap_or(Value::Object(Default::default()))
    }
}

impl fmt::Display for LiteAgentOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

// ---------------------------------------------------------------------------
// LiteAgent
// ---------------------------------------------------------------------------

/// A lightweight agent for standalone LLM interactions.
///
/// Unlike the full `Agent`, `LiteAgent` does not require a crew, task
/// definitions, or complex delegation. It directly manages LLM conversations
/// with optional tool calling, guardrails, and hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteAgent {
    /// Role name for the agent.
    #[serde(default = "default_role")]
    pub role: String,
    /// Goal description for the agent.
    #[serde(default)]
    pub goal: String,
    /// Backstory/context for the agent.
    #[serde(default)]
    pub backstory: String,
    /// LLM model identifier.
    pub llm: String,
    /// Whether to enable verbose logging.
    #[serde(default)]
    pub verbose: bool,
    /// Maximum number of iterations for tool-calling loops.
    #[serde(default = "default_max_iter")]
    pub max_iter: usize,
    /// Maximum requests per minute rate limit.
    pub max_rpm: Option<u32>,
    /// Whether to allow delegation to other agents.
    #[serde(default)]
    pub allow_delegation: bool,
    /// System message to prepend to conversations.
    pub system_message: Option<String>,
    /// Messages accumulated during the current execution.
    #[serde(skip)]
    pub messages: Vec<LLMMessage>,
    /// Current iteration count.
    #[serde(skip)]
    pub iterations: usize,
}

fn default_role() -> String {
    "Helpful Assistant".to_string()
}

fn default_max_iter() -> usize {
    25
}

impl Default for LiteAgent {
    fn default() -> Self {
        Self {
            role: default_role(),
            goal: String::new(),
            backstory: String::new(),
            llm: "gpt-4o".to_string(),
            verbose: false,
            max_iter: default_max_iter(),
            max_rpm: None,
            allow_delegation: false,
            system_message: None,
            messages: Vec::new(),
            iterations: 0,
        }
    }
}

impl LiteAgent {
    /// Create a new `LiteAgent` with the specified LLM model.
    pub fn new(llm: impl Into<String>) -> Self {
        Self {
            llm: llm.into(),
            ..Default::default()
        }
    }

    /// Create a builder for configuring a `LiteAgent`.
    pub fn builder(llm: impl Into<String>) -> LiteAgentBuilder {
        LiteAgentBuilder {
            agent: Self::new(llm),
        }
    }

    /// Execute the agent with the given messages (placeholder).
    ///
    /// In the full implementation, this drives the LLM conversation
    /// loop with tool calling, guardrails, and hooks.
    pub fn kickoff(
        &mut self,
        messages: Vec<LLMMessage>,
    ) -> Result<LiteAgentOutput, String> {
        self.messages = messages;
        self.iterations = 0;

        // Stub: the full implementation calls the LLM, handles tools, etc.
        Err("LiteAgent.kickoff() not yet implemented".to_string())
    }

    /// Reset the agent's execution state.
    pub fn reset(&mut self) {
        self.messages.clear();
        self.iterations = 0;
    }
}

/// Builder for configuring a `LiteAgent`.
pub struct LiteAgentBuilder {
    agent: LiteAgent,
}

impl LiteAgentBuilder {
    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.agent.role = role.into();
        self
    }

    pub fn goal(mut self, goal: impl Into<String>) -> Self {
        self.agent.goal = goal.into();
        self
    }

    pub fn backstory(mut self, backstory: impl Into<String>) -> Self {
        self.agent.backstory = backstory.into();
        self
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.agent.verbose = verbose;
        self
    }

    pub fn max_iter(mut self, max_iter: usize) -> Self {
        self.agent.max_iter = max_iter;
        self
    }

    pub fn max_rpm(mut self, max_rpm: u32) -> Self {
        self.agent.max_rpm = Some(max_rpm);
        self
    }

    pub fn system_message(mut self, msg: impl Into<String>) -> Self {
        self.agent.system_message = Some(msg.into());
        self
    }

    pub fn build(self) -> LiteAgent {
        self.agent
    }
}
