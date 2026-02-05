//! Main Crew struct for CrewAI.
//!
//! Corresponds to `crewai/crew.py`.

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::crews::crew_output::CrewOutput;
use crate::process::Process;
use crate::security::security_config::SecurityConfig;
use crate::task::Task;
use crate::tasks::task_output::TaskOutput;
use crate::types::usage_metrics::UsageMetrics;

/// Represents a group of agents, defining how they should collaborate and the
/// tasks they should perform.
///
/// Corresponds to `crewai.crew.Crew`.
///
/// # Fields
///
/// See field documentation below for complete details on all attributes from the
/// Python implementation.
#[derive(Serialize, Deserialize)]
pub struct Crew {
    // ---- Identity ----
    /// Optional name for the crew.
    pub name: Option<String>,
    /// Unique identifier for the crew instance.
    pub id: Uuid,

    // ---- Cache ----
    /// Whether the crew should use a cache to store the results of tool execution.
    pub cache: bool,

    // ---- Tasks and Agents ----
    /// List of tasks assigned to the crew.
    pub tasks: Vec<Task>,
    /// List of agent role strings part of this crew.
    pub agents: Vec<String>,

    // ---- Process ----
    /// The process flow that the crew will follow.
    pub process: Process,

    // ---- Verbosity ----
    /// Indicates the verbosity level for logging during execution.
    pub verbose: bool,

    // ---- Memory ----
    /// Whether the crew should use memory to store memories of its execution.
    pub memory: bool,
    /// Short-term memory configuration (stored as optional config map).
    pub short_term_memory: Option<HashMap<String, serde_json::Value>>,
    /// Long-term memory configuration.
    pub long_term_memory: Option<HashMap<String, serde_json::Value>>,
    /// Entity memory configuration.
    pub entity_memory: Option<HashMap<String, serde_json::Value>>,
    /// External memory configuration.
    pub external_memory: Option<HashMap<String, serde_json::Value>>,

    // ---- Embedder ----
    /// Configuration for the embedder to be used for the crew.
    pub embedder: Option<HashMap<String, serde_json::Value>>,

    // ---- Usage metrics ----
    /// Metrics for the LLM usage during all tasks execution.
    pub usage_metrics: Option<UsageMetrics>,

    // ---- Manager ----
    /// Language model identifier that will run the manager agent.
    pub manager_llm: Option<String>,
    /// Custom manager agent role (if any).
    pub manager_agent: Option<String>,

    // ---- Function calling LLM ----
    /// Language model for tool calling for all agents.
    pub function_calling_llm: Option<String>,

    // ---- Config ----
    /// Configuration settings for the crew.
    pub config: Option<HashMap<String, serde_json::Value>>,

    // ---- Sharing ----
    /// Whether you want to share the complete crew information and execution with crewAI.
    pub share_crew: bool,

    // ---- Callbacks (not serialized) ----
    /// Callback to be executed after each step for all agents execution.
    #[serde(skip)]
    pub step_callback: Option<Box<dyn Fn(&str) + Send + Sync>>,
    /// Callback to be executed after each task for all agents execution.
    #[serde(skip)]
    pub task_callback: Option<Box<dyn Fn(&TaskOutput) + Send + Sync>>,
    /// List of callbacks to be executed before crew kickoff.
    #[serde(skip)]
    pub before_kickoff_callbacks:
        Vec<Box<dyn Fn(Option<HashMap<String, String>>) -> Option<HashMap<String, String>> + Send + Sync>>,
    /// List of callbacks to be executed after crew kickoff.
    #[serde(skip)]
    pub after_kickoff_callbacks: Vec<Box<dyn Fn(CrewOutput) -> CrewOutput + Send + Sync>>,

    // ---- Streaming ----
    /// Whether to stream output from the crew execution.
    pub stream: bool,

    // ---- RPM ----
    /// Maximum number of requests per minute for the crew execution.
    pub max_rpm: Option<i32>,

    // ---- Planning ----
    /// Plan the crew execution and add the plan to the crew.
    pub planning: bool,
    /// Language model that will run the AgentPlanner if planning is true.
    pub planning_llm: Option<String>,

    // ---- Execution logs ----
    /// List of execution logs for tasks.
    pub execution_logs: Vec<HashMap<String, serde_json::Value>>,

    // ---- Knowledge ----
    /// Knowledge sources for the crew (stored as config maps).
    pub knowledge_sources: Option<Vec<HashMap<String, serde_json::Value>>>,
    /// Knowledge for the crew.
    pub knowledge: Option<HashMap<String, serde_json::Value>>,

    // ---- Security ----
    /// Security configuration for the crew, including fingerprinting.
    pub security_config: SecurityConfig,

    // ---- Token usage ----
    /// Metrics for the LLM usage during all tasks execution.
    pub token_usage: Option<UsageMetrics>,

    // ---- Tracing ----
    /// Whether to enable tracing for the crew.
    pub tracing: Option<bool>,

    // ---- Prompt file ----
    /// Path to the prompt json file to be used for the crew.
    pub prompt_file: Option<String>,

    // ---- Output log file ----
    /// Path to the log file to be saved.
    pub output_log_file: Option<String>,

    // ---- Chat LLM ----
    /// LLM used to handle chatting with the crew.
    pub chat_llm: Option<String>,

    // ---- Private state (not serialized) ----
    /// Inputs provided during kickoff.
    #[serde(skip)]
    _inputs: Option<HashMap<String, String>>,
}

impl std::fmt::Debug for Crew {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Crew")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("process", &self.process)
            .field("agents", &self.agents)
            .field("tasks", &self.tasks)
            .field("verbose", &self.verbose)
            .field("memory", &self.memory)
            .field("stream", &self.stream)
            .field("planning", &self.planning)
            .finish_non_exhaustive()
    }
}

impl Crew {
    /// Create a new Crew with required fields.
    pub fn new(tasks: Vec<Task>, agents: Vec<String>) -> Self {
        Self {
            name: Some("crew".to_string()),
            id: Uuid::new_v4(),
            cache: true,
            tasks,
            agents,
            process: Process::default(),
            verbose: false,
            memory: false,
            short_term_memory: None,
            long_term_memory: None,
            entity_memory: None,
            external_memory: None,
            embedder: None,
            usage_metrics: None,
            manager_llm: None,
            manager_agent: None,
            function_calling_llm: None,
            config: None,
            share_crew: false,
            step_callback: None,
            task_callback: None,
            before_kickoff_callbacks: Vec::new(),
            after_kickoff_callbacks: Vec::new(),
            stream: false,
            max_rpm: None,
            planning: false,
            planning_llm: None,
            execution_logs: Vec::new(),
            knowledge_sources: None,
            knowledge: None,
            security_config: SecurityConfig::default(),
            token_usage: None,
            tracing: None,
            prompt_file: None,
            output_log_file: None,
            chat_llm: None,
            _inputs: None,
        }
    }

    /// Compute the key property (MD5 hash of agent keys + task keys).
    pub fn key(&self) -> String {
        let mut source: Vec<String> = self.agents.clone();
        for task in &self.tasks {
            source.push(task.key());
        }
        let combined = source.join("|");
        let mut hasher = Md5::new();
        hasher.update(combined.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Execute the crew's workflow.
    ///
    /// # Arguments
    ///
    /// * `inputs` - Optional input dictionary for task interpolation.
    ///
    /// # Returns
    ///
    /// `CrewOutput` with the results of the crew execution.
    pub fn kickoff(
        &mut self,
        inputs: Option<HashMap<String, String>>,
    ) -> Result<CrewOutput, String> {
        // Run before_kickoff callbacks
        let mut current_inputs = inputs;
        for callback in &self.before_kickoff_callbacks {
            current_inputs = callback(current_inputs);
        }

        // Store inputs
        self._inputs = current_inputs.clone();

        // Interpolate inputs into tasks
        if let Some(ref inp) = current_inputs {
            self.interpolate_inputs(inp);
        }

        // Execute based on process
        let result = match self.process {
            Process::Sequential => self.run_sequential_process()?,
            Process::Hierarchical => self.run_hierarchical_process()?,
        };

        // Run after_kickoff callbacks
        let mut final_result = result;
        for callback in &self.after_kickoff_callbacks {
            final_result = callback(final_result);
        }

        // Calculate usage metrics
        self.usage_metrics = Some(self.calculate_usage_metrics());

        Ok(final_result)
    }

    /// Async version of kickoff.
    pub async fn kickoff_async(
        &mut self,
        inputs: Option<HashMap<String, String>>,
    ) -> Result<CrewOutput, String> {
        // For now, delegate to sync kickoff.
        // In the full implementation this would use native async task execution.
        self.kickoff(inputs)
    }

    /// Creates a deep copy of the Crew instance.
    pub fn copy(&self) -> Crew {
        Crew {
            name: self.name.clone(),
            id: Uuid::new_v4(), // New ID on copy
            cache: self.cache,
            tasks: self.tasks.clone(),
            agents: self.agents.clone(),
            process: self.process,
            verbose: self.verbose,
            memory: self.memory,
            short_term_memory: self.short_term_memory.clone(),
            long_term_memory: self.long_term_memory.clone(),
            entity_memory: self.entity_memory.clone(),
            external_memory: self.external_memory.clone(),
            embedder: self.embedder.clone(),
            usage_metrics: None,
            manager_llm: self.manager_llm.clone(),
            manager_agent: self.manager_agent.clone(),
            function_calling_llm: self.function_calling_llm.clone(),
            config: self.config.clone(),
            share_crew: self.share_crew,
            step_callback: None, // Callbacks can't be cloned
            task_callback: None,
            before_kickoff_callbacks: Vec::new(),
            after_kickoff_callbacks: Vec::new(),
            stream: self.stream,
            max_rpm: self.max_rpm,
            planning: self.planning,
            planning_llm: self.planning_llm.clone(),
            execution_logs: Vec::new(),
            knowledge_sources: self.knowledge_sources.clone(),
            knowledge: self.knowledge.clone(),
            security_config: self.security_config.clone(),
            token_usage: None,
            tracing: self.tracing,
            prompt_file: self.prompt_file.clone(),
            output_log_file: self.output_log_file.clone(),
            chat_llm: self.chat_llm.clone(),
            _inputs: None,
        }
    }

    /// Reset specific or all memories for the crew.
    ///
    /// # Arguments
    ///
    /// * `command_type` - Type of memory to reset.
    ///   Valid options: "long", "short", "entity", "knowledge", "agent_knowledge",
    ///   "kickoff_outputs", "external", or "all".
    pub fn reset_memories(&mut self, command_type: &str) -> Result<(), String> {
        let valid_types = [
            "long",
            "short",
            "entity",
            "knowledge",
            "agent_knowledge",
            "kickoff_outputs",
            "all",
            "external",
        ];

        if !valid_types.contains(&command_type) {
            return Err(format!(
                "Invalid command type. Must be one of: {}",
                valid_types.join(", ")
            ));
        }

        match command_type {
            "all" => {
                self.short_term_memory = None;
                self.long_term_memory = None;
                self.entity_memory = None;
                self.external_memory = None;
                self.knowledge = None;
                self.execution_logs.clear();
                log::info!("All memories have been reset");
            }
            "short" => {
                self.short_term_memory = None;
                log::info!("Short Term memory has been reset");
            }
            "long" => {
                self.long_term_memory = None;
                log::info!("Long Term memory has been reset");
            }
            "entity" => {
                self.entity_memory = None;
                log::info!("Entity memory has been reset");
            }
            "external" => {
                self.external_memory = None;
                log::info!("External memory has been reset");
            }
            "knowledge" | "agent_knowledge" => {
                self.knowledge = None;
                log::info!("Knowledge has been reset");
            }
            "kickoff_outputs" => {
                self.execution_logs.clear();
                log::info!("Kickoff outputs have been reset");
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    /// Calculate and return usage metrics.
    pub fn calculate_usage_metrics(&self) -> UsageMetrics {
        // TODO: Iterate over agents' LLM usage summaries and aggregate.
        // For now, return the existing usage metrics or default.
        self.usage_metrics.clone().unwrap_or_default()
    }

    /// Interpolate inputs into tasks and agents.
    fn interpolate_inputs(&mut self, inputs: &HashMap<String, String>) {
        for task in &mut self.tasks {
            task.interpolate_inputs(inputs);
        }
        // TODO: Interpolate inputs into agents as well.
    }

    /// Execute tasks sequentially and return the final output.
    fn run_sequential_process(&mut self) -> Result<CrewOutput, String> {
        self.execute_tasks()
    }

    /// Create and assign a manager agent to complete the tasks.
    fn run_hierarchical_process(&mut self) -> Result<CrewOutput, String> {
        // TODO: Create manager agent and delegate tasks.
        self.execute_tasks()
    }

    /// Execute tasks and return the crew output.
    fn execute_tasks(&mut self) -> Result<CrewOutput, String> {
        let mut task_outputs: Vec<TaskOutput> = Vec::new();

        for task in &mut self.tasks {
            let context = if !task_outputs.is_empty() {
                Some(
                    task_outputs
                        .iter()
                        .map(|o| o.raw.clone())
                        .collect::<Vec<String>>()
                        .join("\n"),
                )
            } else {
                None
            };

            // Clone agent before mutable borrow to avoid borrow conflict.
            let agent_role = task.agent.clone();
            let task_output = task.execute_sync(
                agent_role.as_deref(),
                context.as_deref(),
                None,
            )?;
            task_outputs.push(task_output);
        }

        self.create_crew_output(task_outputs)
    }

    /// Create CrewOutput from task outputs.
    fn create_crew_output(&mut self, task_outputs: Vec<TaskOutput>) -> Result<CrewOutput, String> {
        if task_outputs.is_empty() {
            return Err("No task outputs available to create crew output.".to_string());
        }

        let valid_outputs: Vec<&TaskOutput> =
            task_outputs.iter().filter(|t| !t.raw.is_empty()).collect();
        if valid_outputs.is_empty() {
            return Err("No valid task outputs available to create crew output.".to_string());
        }

        let final_task_output = valid_outputs.last().unwrap();
        let token_usage = self.calculate_usage_metrics();
        self.token_usage = Some(token_usage.clone());

        Ok(CrewOutput {
            raw: final_task_output.raw.clone(),
            pydantic: final_task_output.pydantic.clone(),
            json_dict: final_task_output.json_dict.clone(),
            tasks_output: task_outputs,
            token_usage,
        })
    }
}

impl std::fmt::Display for Crew {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Crew(id={}, process={}, number_of_agents={}, number_of_tasks={})",
            self.id,
            self.process,
            self.agents.len(),
            self.tasks.len()
        )
    }
}
