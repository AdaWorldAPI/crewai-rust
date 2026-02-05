//! Agent executor for crew AI agents.
//!
//! Corresponds to `crewai/agents/crew_agent_executor.py`.
//!
//! Handles agent execution flow including LLM interactions, tool execution,
//! and memory management. This is the main execution loop that drives agent
//! behavior, supporting both ReAct text-based and native function calling
//! patterns.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;

use serde_json::Value;

use super::parser::AgentFinish;
use super::tools_handler::ToolsHandler;
use crate::tools::structured_tool::CrewStructuredTool;

// ---------------------------------------------------------------------------
// LLM Message type alias (re-export from base_llm for convenience)
// ---------------------------------------------------------------------------

/// A single message in an LLM conversation.
pub type LLMMessage = HashMap<String, Value>;

// ---------------------------------------------------------------------------
// CrewAgentExecutor
// ---------------------------------------------------------------------------

/// Executor for crew agents.
///
/// Manages the execution lifecycle of an agent including prompt formatting,
/// LLM interactions, tool execution, and feedback handling. Supports both
/// ReAct text-based tool calling and native function calling patterns.
pub struct CrewAgentExecutor {
    /// The language model instance (type-erased).
    pub llm: Box<dyn Any + Send + Sync>,
    /// The task being executed (type-erased).
    pub task: Box<dyn Any + Send + Sync>,
    /// The agent performing the execution (type-erased).
    pub agent: Box<dyn Any + Send + Sync>,
    /// The crew this agent belongs to (type-erased).
    pub crew: Box<dyn Any + Send + Sync>,
    /// Prompt templates (system + user or single prompt).
    pub prompt: HashMap<String, String>,
    /// Available structured tools.
    pub tools: Vec<CrewStructuredTool>,
    /// Comma-separated tool names string.
    pub tools_names: String,
    /// Stop word list for the LLM.
    pub stop: Vec<String>,
    /// Maximum iterations before forcing a final answer.
    pub max_iter: u32,
    /// Optional callbacks list.
    pub callbacks: Vec<Box<dyn Any + Send + Sync>>,
    /// Tool handler for caching and tracking.
    pub tools_handler: ToolsHandler,
    /// Original BaseTool objects (before conversion to structured tools).
    pub original_tools: Vec<Box<dyn Any + Send + Sync>>,
    /// Optional step callback function.
    pub step_callback: Option<Box<dyn Fn(&dyn Any) + Send + Sync>>,
    /// Tool descriptions string.
    pub tools_description: String,
    /// Optional function calling LLM (type-erased).
    pub function_calling_llm: Option<Box<dyn Any + Send + Sync>>,
    /// Whether to respect context window limits.
    pub respect_context_window: bool,
    /// Optional RPM limit check function.
    pub request_within_rpm_limit: Option<Box<dyn Fn() -> bool + Send + Sync>>,
    /// Optional response model for structured outputs.
    pub response_model: Option<Box<dyn Any + Send + Sync>>,
    /// Whether to ask for human input at the end.
    pub ask_for_human_input: bool,
    /// Conversation message history.
    pub messages: Vec<LLMMessage>,
    /// Current iteration count.
    pub iterations: u32,
    /// Number of iterations after which to log errors.
    pub log_error_after: u32,
}

impl fmt::Debug for CrewAgentExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CrewAgentExecutor")
            .field("tools_names", &self.tools_names)
            .field("max_iter", &self.max_iter)
            .field("iterations", &self.iterations)
            .field("messages_count", &self.messages.len())
            .field("tools_count", &self.tools.len())
            .field("respect_context_window", &self.respect_context_window)
            .field("ask_for_human_input", &self.ask_for_human_input)
            .finish()
    }
}

impl CrewAgentExecutor {
    /// Create a new `CrewAgentExecutor`.
    ///
    /// # Arguments
    ///
    /// * `llm` - Language model instance.
    /// * `task` - Task to execute.
    /// * `agent` - Agent performing the execution.
    /// * `crew` - Crew this agent belongs to.
    /// * `prompt` - Prompt templates.
    /// * `max_iter` - Maximum iterations.
    /// * `tools` - Available structured tools.
    /// * `tools_names` - Comma-separated tool names.
    /// * `stop` - Stop words for the LLM.
    /// * `tools_description` - Tool descriptions string.
    /// * `tools_handler` - Tool handler for caching.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        llm: Box<dyn Any + Send + Sync>,
        task: Box<dyn Any + Send + Sync>,
        agent: Box<dyn Any + Send + Sync>,
        crew: Box<dyn Any + Send + Sync>,
        prompt: HashMap<String, String>,
        max_iter: u32,
        tools: Vec<CrewStructuredTool>,
        tools_names: String,
        stop: Vec<String>,
        tools_description: String,
        tools_handler: ToolsHandler,
    ) -> Self {
        Self {
            llm,
            task,
            agent,
            crew,
            prompt,
            tools,
            tools_names,
            stop,
            max_iter,
            callbacks: Vec::new(),
            tools_handler,
            original_tools: Vec::new(),
            step_callback: None,
            tools_description,
            function_calling_llm: None,
            respect_context_window: false,
            request_within_rpm_limit: None,
            response_model: None,
            ask_for_human_input: false,
            messages: Vec::new(),
            iterations: 0,
            log_error_after: 3,
        }
    }

    /// Check whether stop words are being used.
    pub fn use_stop_words(&self) -> bool {
        // In a full implementation, this would check llm.supports_stop_words()
        !self.stop.is_empty()
    }

    /// Execute the agent with given inputs (synchronous).
    ///
    /// This is a stub implementation that shows the execution structure.
    /// The full implementation would:
    /// 1. Set up messages from the prompt templates
    /// 2. Run the invoke loop (ReAct or native tools)
    /// 3. Handle human feedback if requested
    /// 4. Create memory entries
    /// 5. Return the output
    pub fn invoke(
        &mut self,
        inputs: HashMap<String, String>,
    ) -> Result<HashMap<String, Value>, Box<dyn std::error::Error + Send + Sync>> {
        self.setup_messages(&inputs);
        self.ask_for_human_input = inputs
            .get("ask_for_human_input")
            .map(|v| v == "true")
            .unwrap_or(false);

        let formatted_answer = self.invoke_loop()?;

        let mut output = HashMap::new();
        output.insert("output".to_string(), formatted_answer.output.clone());
        Ok(output)
    }

    /// Execute the agent with given inputs (asynchronous).
    ///
    /// Stub implementation that delegates to the sync version.
    pub async fn ainvoke(
        &mut self,
        inputs: HashMap<String, String>,
    ) -> Result<HashMap<String, Value>, Box<dyn std::error::Error + Send + Sync>> {
        // In a full async implementation, this would use async LLM calls
        self.invoke(inputs)
    }

    /// Set up messages for the agent execution from prompt templates.
    fn setup_messages(&mut self, inputs: &HashMap<String, String>) {
        self.messages.clear();

        if let Some(system_prompt) = self.prompt.get("system") {
            let formatted_system = Self::format_prompt(system_prompt, inputs);
            let user_prompt = self.prompt.get("user").cloned().unwrap_or_default();
            let formatted_user = Self::format_prompt(&user_prompt, inputs);

            let mut system_msg = HashMap::new();
            system_msg.insert("role".to_string(), Value::String("system".to_string()));
            system_msg.insert("content".to_string(), Value::String(formatted_system));
            self.messages.push(system_msg);

            let mut user_msg = HashMap::new();
            user_msg.insert("role".to_string(), Value::String("user".to_string()));
            user_msg.insert("content".to_string(), Value::String(formatted_user));
            self.messages.push(user_msg);
        } else if let Some(prompt) = self.prompt.get("prompt") {
            let formatted = Self::format_prompt(prompt, inputs);
            let mut msg = HashMap::new();
            msg.insert("role".to_string(), Value::String("user".to_string()));
            msg.insert("content".to_string(), Value::String(formatted));
            self.messages.push(msg);
        }
    }

    /// Execute agent loop until completion.
    ///
    /// Checks if the LLM supports native function calling and uses that
    /// approach if available, otherwise falls back to the ReAct text pattern.
    ///
    /// This is a stub implementation showing the loop structure.
    fn invoke_loop(&mut self) -> Result<AgentFinish, Box<dyn std::error::Error + Send + Sync>> {
        // Stub: In a full implementation, this would:
        // 1. Check if the LLM supports native function calling
        // 2. If yes, use _invoke_loop_native_tools
        // 3. If no, use _invoke_loop_react

        // For now, return a placeholder indicating the executor structure is in place
        Err("CrewAgentExecutor invoke_loop not yet fully implemented. \
             The executor structure is in place but requires LLM integration."
            .into())
    }

    /// Execute agent loop using ReAct text-based pattern.
    ///
    /// The traditional approach where tool definitions are embedded in the
    /// prompt and the LLM outputs Action/Action Input text that is parsed
    /// to execute tools.
    fn invoke_loop_react(
        &mut self,
    ) -> Result<AgentFinish, Box<dyn std::error::Error + Send + Sync>> {
        let formatted_answer: Option<AgentFinish> = None;

        while formatted_answer.is_none() {
            if self.iterations >= self.max_iter {
                return Err(format!(
                    "Agent exceeded maximum iterations ({})",
                    self.max_iter
                )
                .into());
            }

            // In a full implementation:
            // 1. Enforce RPM limit
            // 2. Call LLM with messages
            // 3. Parse response into AgentAction or AgentFinish
            // 4. If AgentAction, execute tool and append result
            // 5. If AgentFinish, return

            self.iterations += 1;
        }

        formatted_answer.ok_or_else(|| {
            "Agent execution ended without reaching a final answer".into()
        })
    }

    /// Execute agent loop using native function calling.
    ///
    /// Uses the LLM's native tool/function calling capability instead of
    /// the text-based ReAct pattern.
    fn invoke_loop_native_tools(
        &mut self,
    ) -> Result<AgentFinish, Box<dyn std::error::Error + Send + Sync>> {
        // Stub: Similar to invoke_loop_react but uses native tool calls
        Err("Native tool calling not yet implemented".into())
    }

    /// Append a message to the conversation history.
    fn append_message(&mut self, text: &str, role: &str) {
        let mut msg = HashMap::new();
        msg.insert("role".to_string(), Value::String(role.to_string()));
        msg.insert("content".to_string(), Value::String(text.to_string()));
        self.messages.push(msg);
    }

    /// Format a prompt template by replacing placeholders with input values.
    fn format_prompt(prompt: &str, inputs: &HashMap<String, String>) -> String {
        let mut result = prompt.to_string();
        if let Some(input) = inputs.get("input") {
            result = result.replace("{input}", input);
        }
        if let Some(tool_names) = inputs.get("tool_names") {
            result = result.replace("{tool_names}", tool_names);
        }
        if let Some(tools) = inputs.get("tools") {
            result = result.replace("{tools}", tools);
        }
        result
    }

    /// Invoke the optional step callback.
    fn invoke_step_callback(&self, _answer: &dyn Any) {
        if let Some(ref _callback) = self.step_callback {
            // _callback(_answer);  // Would need proper type handling
        }
    }
}
