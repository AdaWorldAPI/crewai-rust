//! Core Agent struct for CrewAI.
//!
//! Corresponds to `crewai/agent/core.py`.
//!
//! This is the main concrete Agent implementation that extends the BaseAgent
//! trait with full execution capabilities including MCP tool integration,
//! knowledge retrieval, reasoning, guardrails, code execution, and both
//! crew-based (`execute_task`) and standalone (`kickoff`) execution modes.

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::agents::crew_agent_executor::CrewAgentExecutor;
use crate::agents::tools_handler::ToolsHandler;
use crate::llms::base_llm::{BaseLLM, BaseLLMState, LLMMessage};
use crate::llms::providers::anthropic::AnthropicCompletion;
use crate::llms::providers::openai::OpenAICompletion;
use crate::llms::providers::xai::XAICompletion;
use crate::security::security_config::SecurityConfig;

/// MCP connection timeout in seconds.
pub const MCP_CONNECTION_TIMEOUT: u64 = 10;
/// MCP tool execution timeout in seconds.
pub const MCP_TOOL_EXECUTION_TIMEOUT: u64 = 30;
/// MCP discovery timeout in seconds.
pub const MCP_DISCOVERY_TIMEOUT: u64 = 15;
/// MCP maximum retries for discovery.
pub const MCP_MAX_RETRIES: u32 = 3;
/// MCP schema cache TTL in seconds (5 minutes).
pub const MCP_CACHE_TTL: u64 = 300;

/// Type alias for a step callback function.
pub type StepCallback = Box<dyn Fn(&str) + Send + Sync>;

/// Code execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodeExecutionMode {
    /// Safe mode using Docker for code execution.
    Safe,
    /// Unsafe mode using direct execution.
    Unsafe,
}

impl Default for CodeExecutionMode {
    fn default() -> Self {
        CodeExecutionMode::Safe
    }
}

/// Represents an agent in the CrewAI system.
///
/// Each agent has a role, a goal, a backstory, and an optional language model (llm).
/// The agent can also have memory, can operate in verbose mode, and can delegate
/// tasks to other agents.
///
/// Corresponds to `crewai.agent.core.Agent` (extending `BaseAgent`).
///
/// # Fields
///
/// All fields from both `BaseAgent` and the concrete `Agent` class are included.
/// See field documentation below for complete details.
#[derive(Serialize, Deserialize)]
pub struct Agent {
    // ---- Identity (from BaseAgent) ----
    /// Unique identifier for the agent.
    pub id: Uuid,
    /// Role of the agent.
    pub role: String,
    /// Objective of the agent.
    pub goal: String,
    /// Backstory of the agent.
    pub backstory: String,

    // ---- Configuration (from BaseAgent) ----
    /// Configuration for the agent.
    pub config: Option<HashMap<String, serde_json::Value>>,
    /// Whether the agent should use a cache for tool usage.
    pub cache: bool,
    /// Verbose mode for the Agent Execution.
    pub verbose: bool,
    /// Maximum number of requests per minute.
    pub max_rpm: Option<i32>,
    /// Enable agent to delegate and ask questions among each other.
    pub allow_delegation: bool,
    /// Tools at agents' disposal (stored as tool names).
    pub tools: Vec<String>,
    /// Maximum iterations for an agent to execute a task.
    pub max_iter: i32,
    /// Language model identifier that will run the agent.
    pub llm: Option<String>,
    /// Maximum number of tokens for the agent to generate in a response.
    pub max_tokens: Option<i32>,
    /// Results of the tools used by the agent.
    pub tools_results: Vec<HashMap<String, serde_json::Value>>,

    // ---- Security (from BaseAgent) ----
    /// Security configuration for the agent, including fingerprinting.
    pub security_config: SecurityConfig,

    // ---- BaseAgent fields (additional) ----
    /// Whether the agent is adapted (e.g., via an adapter).
    pub adapted_agent: bool,
    /// Knowledge configuration (limits, threshold, etc.).
    pub knowledge_config: Option<HashMap<String, serde_json::Value>>,
    /// Platform apps the agent can access through CrewAI AMP Tools.
    pub apps: Option<Vec<String>>,
    /// MCP server references for tool integration.
    pub mcps: Option<Vec<String>>,

    // ---- Agent-specific fields (from Agent, extending BaseAgent) ----
    /// Maximum execution time for an agent to execute a task (seconds).
    pub max_execution_time: Option<i64>,

    /// Callback to be executed after each step of the agent execution.
    #[serde(skip)]
    pub step_callback: Option<StepCallback>,

    /// Use system prompt for the agent.
    pub use_system_prompt: bool,

    /// Language model that will handle tool calling for this agent.
    pub function_calling_llm: Option<String>,

    /// System format for the agent.
    pub system_template: Option<String>,
    /// Prompt format for the agent.
    pub prompt_template: Option<String>,
    /// Response format for the agent.
    pub response_template: Option<String>,

    /// Enable code execution for the agent.
    pub allow_code_execution: bool,

    /// Keep messages under the context window size by summarizing content.
    pub respect_context_window: bool,

    /// Maximum number of retries for an agent when an error occurs.
    pub max_retry_limit: i32,

    /// Whether the agent is multimodal (deprecated, will be removed in v2.0).
    pub multimodal: bool,

    /// Whether to automatically inject the current date into tasks.
    pub inject_date: bool,
    /// Format string for date when inject_date is enabled.
    pub date_format: String,

    /// Mode for code execution: 'safe' (using Docker) or 'unsafe' (direct execution).
    pub code_execution_mode: CodeExecutionMode,

    /// Whether the agent should reflect and create a plan before executing a task.
    pub reasoning: bool,
    /// Maximum number of reasoning attempts before executing the task.
    pub max_reasoning_attempts: Option<i32>,

    /// Embedder configuration for the agent.
    pub embedder: Option<HashMap<String, serde_json::Value>>,

    /// Agent knowledge context (injected before task execution).
    pub agent_knowledge_context: Option<String>,
    /// Crew knowledge context (injected before task execution).
    pub crew_knowledge_context: Option<String>,
    /// Knowledge search query dynamically generated by the agent.
    pub knowledge_search_query: Option<String>,

    /// The Agent's role to be loaded from a repository.
    pub from_repository: Option<String>,

    /// Guardrail description or callable for validating agent output.
    pub guardrail: Option<String>,
    /// Maximum number of retries when guardrail fails.
    pub guardrail_max_retries: i32,

    /// A2A (Agent-to-Agent) configuration.
    pub a2a: Option<serde_json::Value>,

    /// Executor class name (for custom agent executors).
    pub executor_class: Option<String>,

    /// Knowledge sources for the agent (stored as config maps).
    pub knowledge_sources: Option<Vec<HashMap<String, serde_json::Value>>>,
    /// Knowledge storage configuration.
    pub knowledge_storage: Option<serde_json::Value>,
    /// Knowledge instance for the agent.
    pub knowledge: Option<serde_json::Value>,

    /// Crew reference (not serialized).
    #[serde(skip)]
    pub crew: Option<String>,

    // ---- Private state ----
    /// Number of times the agent has been executed (for retry tracking).
    #[serde(skip)]
    times_executed: i32,
    /// Original role before interpolation.
    #[serde(skip)]
    original_role: Option<String>,
    /// Original goal before interpolation.
    #[serde(skip)]
    original_goal: Option<String>,
    /// Original backstory before interpolation.
    #[serde(skip)]
    original_backstory: Option<String>,
    /// Last messages from the agent's LLM interaction.
    #[serde(skip)]
    pub last_messages: Vec<HashMap<String, String>>,
    /// MCP client references for cleanup.
    #[serde(skip)]
    mcp_clients: Vec<serde_json::Value>,
}

impl std::fmt::Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent")
            .field("id", &self.id)
            .field("role", &self.role)
            .field("goal", &self.goal)
            .field("llm", &self.llm)
            .field("verbose", &self.verbose)
            .finish_non_exhaustive()
    }
}

impl Clone for Agent {
    fn clone(&self) -> Self {
        Self {
            id: Uuid::new_v4(), // New ID on clone
            role: self.role.clone(),
            goal: self.goal.clone(),
            backstory: self.backstory.clone(),
            config: self.config.clone(),
            cache: self.cache,
            verbose: self.verbose,
            max_rpm: self.max_rpm,
            allow_delegation: self.allow_delegation,
            tools: self.tools.clone(),
            max_iter: self.max_iter,
            llm: self.llm.clone(),
            max_tokens: self.max_tokens,
            tools_results: Vec::new(),
            security_config: self.security_config.clone(),
            adapted_agent: self.adapted_agent,
            knowledge_config: self.knowledge_config.clone(),
            apps: self.apps.clone(),
            mcps: self.mcps.clone(),
            max_execution_time: self.max_execution_time,
            step_callback: None, // Can't clone closures
            use_system_prompt: self.use_system_prompt,
            function_calling_llm: self.function_calling_llm.clone(),
            system_template: self.system_template.clone(),
            prompt_template: self.prompt_template.clone(),
            response_template: self.response_template.clone(),
            allow_code_execution: self.allow_code_execution,
            respect_context_window: self.respect_context_window,
            max_retry_limit: self.max_retry_limit,
            multimodal: self.multimodal,
            inject_date: self.inject_date,
            date_format: self.date_format.clone(),
            code_execution_mode: self.code_execution_mode,
            reasoning: self.reasoning,
            max_reasoning_attempts: self.max_reasoning_attempts,
            embedder: self.embedder.clone(),
            agent_knowledge_context: self.agent_knowledge_context.clone(),
            crew_knowledge_context: self.crew_knowledge_context.clone(),
            knowledge_search_query: self.knowledge_search_query.clone(),
            from_repository: self.from_repository.clone(),
            guardrail: self.guardrail.clone(),
            guardrail_max_retries: self.guardrail_max_retries,
            a2a: self.a2a.clone(),
            executor_class: self.executor_class.clone(),
            knowledge_sources: self.knowledge_sources.clone(),
            knowledge_storage: self.knowledge_storage.clone(),
            knowledge: self.knowledge.clone(),
            crew: self.crew.clone(),
            times_executed: 0,
            original_role: self.original_role.clone(),
            original_goal: self.original_goal.clone(),
            original_backstory: self.original_backstory.clone(),
            last_messages: Vec::new(),
            mcp_clients: Vec::new(),
        }
    }
}

impl Agent {
    /// Create a new Agent with required fields.
    pub fn new(role: String, goal: String, backstory: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            goal,
            backstory,
            config: None,
            cache: true,
            verbose: false,
            max_rpm: None,
            allow_delegation: false,
            tools: Vec::new(),
            max_iter: 25,
            llm: None,
            max_tokens: None,
            tools_results: Vec::new(),
            security_config: SecurityConfig::default(),
            adapted_agent: false,
            knowledge_config: None,
            apps: None,
            mcps: None,
            max_execution_time: None,
            step_callback: None,
            use_system_prompt: true,
            function_calling_llm: None,
            system_template: None,
            prompt_template: None,
            response_template: None,
            allow_code_execution: false,
            respect_context_window: true,
            max_retry_limit: 2,
            multimodal: false,
            inject_date: false,
            date_format: "%Y-%m-%d".to_string(),
            code_execution_mode: CodeExecutionMode::default(),
            reasoning: false,
            max_reasoning_attempts: None,
            embedder: None,
            agent_knowledge_context: None,
            crew_knowledge_context: None,
            knowledge_search_query: None,
            from_repository: None,
            guardrail: None,
            guardrail_max_retries: 3,
            a2a: None,
            executor_class: None,
            knowledge_sources: None,
            knowledge_storage: None,
            knowledge: None,
            crew: None,
            times_executed: 0,
            original_role: None,
            original_goal: None,
            original_backstory: None,
            last_messages: Vec::new(),
            mcp_clients: Vec::new(),
        }
    }

    /// Set knowledge for the agent with optional crew embedder configuration.
    ///
    /// Corresponds to `Agent.set_knowledge()` in Python.
    pub fn set_knowledge(
        &mut self,
        crew_embedder: Option<&HashMap<String, serde_json::Value>>,
    ) {
        if self.embedder.is_none() {
            if let Some(embedder) = crew_embedder {
                self.embedder = Some(embedder.clone());
            }
        }
        // TODO: Initialize Knowledge instance from knowledge_sources if present.
        log::debug!("set_knowledge called for agent '{}'", self.role);
    }

    /// Check if any memory is available through the crew.
    fn is_any_available_memory(&self) -> bool {
        // In the full implementation, this checks the crew's memory attributes.
        // For now, return false as we don't have a crew reference with memory.
        false
    }

    /// Check if the LLM supports native function calling with the given tools.
    fn supports_native_tool_calling(&self, tools: &[String]) -> bool {
        // TODO: Check LLM capabilities for native function calling.
        !tools.is_empty() && self.llm.is_some()
    }

    /// Execute a task with the agent.
    ///
    /// # Arguments
    ///
    /// * `task_description` - Description of the task to execute.
    /// * `context` - Optional context string.
    /// * `tools` - Optional list of tool names.
    ///
    /// # Returns
    ///
    /// The output string from the agent execution.
    pub fn execute_task(
        &mut self,
        task_description: &str,
        context: Option<&str>,
        tools: Option<&[String]>,
    ) -> Result<String, String> {
        log::debug!(
            "Agent '{}' executing task: {}",
            self.role,
            task_description
        );

        // Handle reasoning if enabled
        if self.reasoning {
            let _ = super::utils::handle_reasoning(&self.role, task_description, self.max_reasoning_attempts.unwrap_or(3));
        }

        // Inject date if enabled
        let task_desc = if self.inject_date {
            self.inject_date_to_description(task_description)
        } else {
            task_description.to_string()
        };

        // Build task prompt with schema
        let task_prompt = super::utils::build_task_prompt_with_schema(&task_desc, None);

        // Format with context
        let task_prompt = if let Some(ctx) = context {
            format!("{}\n\nContext:\n{}", task_prompt, ctx)
        } else {
            task_prompt
        };

        // Validate max execution time
        super::utils::validate_max_execution_time(self.max_execution_time)?;

        // Execute (with or without timeout)
        let result = if let Some(timeout) = self.max_execution_time {
            self.execute_with_timeout(&task_prompt, timeout)?
        } else {
            self.execute_without_timeout(&task_prompt)?
        };

        // Process tool results
        let result = self.process_tool_results_internal(result);

        // Save last messages
        super::utils::save_last_messages(&self.last_messages);

        // Cleanup MCP clients
        self.cleanup_mcp_clients();

        Ok(result)
    }

    /// Async version of execute_task.
    pub async fn aexecute_task(
        &mut self,
        task_description: &str,
        context: Option<&str>,
        tools: Option<&[String]>,
    ) -> Result<String, String> {
        // Delegate to sync for now; full async implementation would use
        // async LLM calls and async MCP discovery.
        self.execute_task(task_description, context, tools)
    }

    /// Execute a task with a timeout.
    fn execute_with_timeout(
        &mut self,
        task_prompt: &str,
        timeout: i64,
    ) -> Result<String, String> {
        // TODO: Implement actual timeout using tokio::time::timeout or threads.
        log::debug!("Executing with timeout: {}s", timeout);
        self.execute_without_timeout(task_prompt)
    }

    /// Execute a task without a timeout.
    ///
    /// Builds a `CrewAgentExecutor` with the agent's LLM and tools, then
    /// runs the invoke loop (ReAct or native function calling) to produce
    /// the final answer.
    fn execute_without_timeout(&mut self, task_prompt: &str) -> Result<String, String> {
        // 1. Create the LLM instance from agent config
        let llm = self.create_llm_instance()
            .map_err(|e| format!("Failed to create LLM instance: {}", e))?;

        // 2. Build system + user prompt
        let system_prompt = format!(
            "You are {}.\n{}\n\nYour goal: {}\n\nAvailable tools: {}\n\n\
             You MUST use the following format:\n\n\
             Thought: you should always think about what to do\n\
             Action: the action to take, one of [{}]\n\
             Action Input: the input to the action\n\
             Observation: the result of the action\n\
             ... (this Thought/Action/Action Input/Observation can repeat N times)\n\
             Thought: I now know the final answer\n\
             Final Answer: the final answer to the original input question",
            self.role,
            self.backstory,
            self.goal,
            self.tools.join(", "),
            self.tools.join(", "),
        );

        let mut prompt = HashMap::new();
        prompt.insert("system".to_string(), system_prompt);
        prompt.insert("user".to_string(), task_prompt.to_string());

        // 3. Build the executor
        let tools_names = self.tools.join(", ");
        let tools_description = self.tools.iter()
            .map(|t| format!("- {}: A tool named {}", t, t))
            .collect::<Vec<_>>()
            .join("\n");

        let mut executor = CrewAgentExecutor::new(
            Box::new(()),                          // llm placeholder (we use callback)
            Box::new(()),                          // task placeholder
            Box::new(()),                          // agent placeholder
            Box::new(()),                          // crew placeholder
            prompt,
            self.max_iter as u32,
            Vec::new(),                            // structured tools
            tools_names.clone(),
            vec!["Observation:".to_string()],      // stop words
            tools_description,
            ToolsHandler::new(None),
        );

        // 4. Set the LLM call callback using the real LLM instance
        let llm_arc: std::sync::Arc<dyn BaseLLM> = std::sync::Arc::from(llm);
        let llm_for_call = llm_arc.clone();
        executor.set_llm_call(move |messages: &[crate::agents::crew_agent_executor::LLMMessage], tools: Option<&[serde_json::Value]>| {
            let msgs: Vec<LLMMessage> = messages.iter().map(|m| {
                m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            }).collect();

            let tools_vec = tools.map(|t| t.to_vec());

            let result = llm_for_call.call(msgs, tools_vec, None)?;

            // Extract text from the LLM Value response
            match result {
                serde_json::Value::String(s) => Ok(s),
                other => Ok(other.to_string()),
            }
        });

        // 5. Set a basic tool executor (logs tool calls, returns stub for now)
        executor.set_tool_executor(|tool_name: &str, tool_input: &str| {
            log::info!("Tool call: {}({})", tool_name, tool_input);
            Ok(format!("Tool '{}' executed with input: {}", tool_name, tool_input))
        });

        // 6. Run the executor
        let mut inputs = HashMap::new();
        inputs.insert("input".to_string(), task_prompt.to_string());
        inputs.insert("tool_names".to_string(), tools_names);

        let result = executor.invoke(inputs)
            .map_err(|e| format!("Agent execution failed: {}", e))?;

        // 7. Extract the output
        let output = result.get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(output)
    }

    /// Create an LLM instance based on the agent's `llm` configuration string.
    ///
    /// Parses strings like `"openai/gpt-4o"`, `"anthropic/claude-3-5-sonnet"`,
    /// `"xai/grok-3"`, or bare model names like `"gpt-4o-mini"` (defaults to OpenAI).
    ///
    /// Corresponds to `Agent._create_llm` in Python.
    pub fn create_llm_instance(&self) -> Result<Box<dyn BaseLLM>, String> {
        let llm_str = self.llm.as_deref().unwrap_or("openai/gpt-4o-mini");
        let (provider, model) = if let Some(idx) = llm_str.find('/') {
            (&llm_str[..idx], &llm_str[idx + 1..])
        } else {
            // Infer provider from model name
            let lower = llm_str.to_lowercase();
            if lower.starts_with("claude") {
                ("anthropic", llm_str)
            } else if lower.starts_with("grok") {
                ("xai", llm_str)
            } else if lower.starts_with("gemini") {
                ("gemini", llm_str)
            } else {
                ("openai", llm_str)
            }
        };

        log::debug!("Creating LLM instance: provider={}, model={}", provider, model);

        match provider.to_lowercase().as_str() {
            "openai" => {
                Ok(Box::new(OpenAICompletion::new(model, None, None)))
            }
            "anthropic" => {
                Ok(Box::new(AnthropicCompletion::new(model, None, None)))
            }
            "xai" | "grok" => {
                Ok(Box::new(XAICompletion::new(model, None, None)))
            }
            other => {
                // Default to OpenAI-compatible with the full string as model
                log::warn!("Unknown provider '{}', falling back to OpenAI-compatible", other);
                Ok(Box::new(OpenAICompletion::new(llm_str, None, None)))
            }
        }
    }

    /// Create the agent executor.
    ///
    /// In the full implementation this sets up the CrewAgentExecutor or AgentExecutor
    /// with tools, prompts, stop words, and RPM limits.
    pub fn create_agent_executor(&mut self) {
        log::debug!("Creating agent executor for '{}'", self.role);
        // The executor is now built on-demand in execute_without_timeout().
    }

    /// Get delegation tools for the specified agents.
    ///
    /// Returns tool names for delegating to other agents.
    pub fn get_delegation_tools(&self, agents: &[Agent]) -> Vec<String> {
        agents
            .iter()
            .flat_map(|a| {
                vec![
                    format!("Delegate work to co-worker '{}'", a.role),
                    format!("Ask question to co-worker '{}'", a.role),
                ]
            })
            .collect()
    }

    /// Get platform tools for the specified list of applications.
    ///
    /// # Arguments
    ///
    /// * `apps` - List of platform app names or app/action strings.
    pub fn get_platform_tools(&self, _apps: &[String]) -> Vec<String> {
        // TODO: Implement platform tools integration via CrewAI AMP.
        log::debug!("get_platform_tools called for agent '{}'", self.role);
        Vec::new()
    }

    /// Get MCP tools from server references/configs.
    ///
    /// Supports both string references (backwards compatible) and structured
    /// configuration objects.
    ///
    /// # Arguments
    ///
    /// * `mcps` - List of MCP server reference strings.
    pub fn get_mcp_tools(&self, mcps: &[String]) -> Vec<String> {
        let mut all_tools = Vec::new();
        for mcp_ref in mcps {
            if mcp_ref.starts_with("crewai-amp:") {
                let tools = self.get_amp_mcp_tools(mcp_ref);
                all_tools.extend(tools);
            } else if mcp_ref.starts_with("https://") {
                let tools = self.get_external_mcp_tools(mcp_ref);
                all_tools.extend(tools);
            }
        }
        all_tools
    }

    /// Get tools from external HTTPS MCP server.
    fn get_external_mcp_tools(&self, _mcp_ref: &str) -> Vec<String> {
        // TODO: Implement MCP tool discovery via HTTP/SSE transport.
        Vec::new()
    }

    /// Get tools from CrewAI AMP MCP marketplace.
    fn get_amp_mcp_tools(&self, _amp_ref: &str) -> Vec<String> {
        // TODO: Implement AMP API call to discover MCP servers.
        Vec::new()
    }

    /// Cleanup MCP client connections after task execution.
    fn cleanup_mcp_clients(&mut self) {
        self.mcp_clients.clear();
    }

    /// Extract server name from URL for tool prefixing.
    pub fn extract_server_name(server_url: &str) -> String {
        // Simple extraction: replace dots and slashes with underscores
        let url = server_url
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        url.replace('.', "_")
            .replace('/', "_")
            .trim_matches('_')
            .to_string()
    }

    /// Get code execution tools.
    pub fn get_code_execution_tools(&self) -> Vec<String> {
        if !self.allow_code_execution {
            return Vec::new();
        }
        // TODO: Integrate with CodeInterpreterTool.
        vec!["code_interpreter".to_string()]
    }

    /// Get multimodal tools.
    pub fn get_multimodal_tools() -> Vec<String> {
        vec!["add_image".to_string()]
    }

    /// Compute the key property (MD5 hash of role|goal|backstory).
    pub fn key(&self) -> String {
        let role = self.original_role.as_deref().unwrap_or(&self.role);
        let goal = self.original_goal.as_deref().unwrap_or(&self.goal);
        let backstory = self
            .original_backstory
            .as_deref()
            .unwrap_or(&self.backstory);

        let source = format!("{}|{}|{}", role, goal, backstory);
        let mut hasher = Md5::new();
        hasher.update(source.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Interpolate inputs into the agent role, goal, and backstory.
    pub fn interpolate_inputs(&mut self, inputs: &HashMap<String, String>) {
        if self.original_role.is_none() {
            self.original_role = Some(self.role.clone());
        }
        if self.original_goal.is_none() {
            self.original_goal = Some(self.goal.clone());
        }
        if self.original_backstory.is_none() {
            self.original_backstory = Some(self.backstory.clone());
        }

        if inputs.is_empty() {
            return;
        }

        if let Some(ref orig) = self.original_role {
            self.role = interpolate_string(orig, inputs);
        }
        if let Some(ref orig) = self.original_goal {
            self.goal = interpolate_string(orig, inputs);
        }
        if let Some(ref orig) = self.original_backstory {
            self.backstory = interpolate_string(orig, inputs);
        }
    }

    /// Simple kickoff for standalone agent execution.
    ///
    /// Executes the agent with the given messages without requiring a Crew.
    /// Supports tools, response formatting, guardrails, and file inputs.
    ///
    /// # Arguments
    ///
    /// * `query` - The query or messages string to execute.
    pub fn kickoff(&mut self, query: &str) -> Result<String, String> {
        log::debug!("Agent '{}' kickoff with query: {}", self.role, query);

        // TODO: Implement full standalone execution:
        // 1. Process platform apps and MCP tools
        // 2. Parse tools
        // 3. Build prompts
        // 4. Create AgentExecutor
        // 5. Execute and build output
        // 6. Process guardrails
        // 7. Return LiteAgentOutput

        Ok(format!("[Agent '{}' response to: {}]", self.role, query))
    }

    /// Async version of kickoff.
    pub async fn kickoff_async(&mut self, query: &str) -> Result<String, String> {
        self.kickoff(query)
    }

    /// Inject the current date into a task description if inject_date is enabled.
    fn inject_date_to_description(&self, description: &str) -> String {
        // Use chrono for date formatting in the full implementation.
        // For now, use a placeholder format.
        let date_str = chrono::Local::now().format(&self.date_format).to_string();
        format!("{}\n\nCurrent Date: {}", description, date_str)
    }

    /// Process tool results, returning result_as_answer if applicable.
    fn process_tool_results_internal(&self, result: String) -> String {
        for tool_result in &self.tools_results {
            if tool_result
                .get("result_as_answer")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                if let Some(serde_json::Value::String(r)) = tool_result.get("result") {
                    return r.clone();
                }
            }
        }
        result
    }

    /// Get the agent's fingerprint.
    pub fn fingerprint(&self) -> &crate::security::security_config::SecurityConfig {
        &self.security_config
    }

    /// Set the agent's fingerprint.
    pub fn set_fingerprint(&mut self, security_config: SecurityConfig) {
        self.security_config = security_config;
    }

    /// Validate Docker installation for code execution.
    fn validate_docker_installation(&self) -> Result<(), String> {
        // TODO: Check if Docker is installed and running.
        if self.allow_code_execution {
            log::debug!(
                "Validating Docker installation for agent '{}'",
                self.role
            );
        }
        Ok(())
    }
}

impl std::fmt::Display for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Agent(role={}, goal={}, backstory={})",
            self.role, self.goal, self.backstory
        )
    }
}

/// Simple string interpolation: replace `{key}` with corresponding value.
fn interpolate_string(template: &str, inputs: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in inputs {
        let pattern = format!("{{{}}}", key);
        result = result.replace(&pattern, value);
    }
    result
}
