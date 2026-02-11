//! Main Task struct for CrewAI.
//!
//! Corresponds to `crewai/task.py`.

use chrono::{DateTime, Utc};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::security::security_config::SecurityConfig;
use crate::tasks::output_format::OutputFormat;
use crate::tasks::task_output::TaskOutput;

/// Type alias for a guardrail callback.
///
/// Returns `(success, result_or_error_message)`.
pub type GuardrailFn = Box<dyn Fn(&TaskOutput) -> (bool, String) + Send + Sync>;

/// Type alias for a task completion callback.
pub type TaskCallback = Box<dyn Fn(&TaskOutput) + Send + Sync>;

/// Type alias for an agent executor callback.
///
/// Takes the task prompt and context, returns the agent's response.
/// Parameters: (task_prompt, context, tools_names)
/// Returns: Result<(raw_output, messages), error_message>
pub type AgentExecutorFn = Box<dyn Fn(&str, Option<&str>, &[String]) -> Result<(String, Vec<crate::tasks::task_output::LLMMessage>), String> + Send + Sync>;

/// Represents a task to be executed.
///
/// Each task must have a description, an expected output, and an agent responsible
/// for execution. Corresponds to `crewai.task.Task`.
///
/// # Fields
///
/// See field documentation below for complete details.
#[derive(Serialize, Deserialize)]
pub struct Task {
    // ---- Counters ----
    /// Number of tools used during task execution.
    pub used_tools: i32,
    /// Number of tool errors during task execution.
    pub tools_errors: i32,
    /// Number of delegation operations during task execution.
    pub delegations: i32,

    // ---- Core fields ----
    /// Optional name for the task.
    pub name: Option<String>,
    /// Additional prompt context for the task.
    pub prompt_context: Option<String>,
    /// Descriptive text detailing the task's purpose and execution.
    pub description: String,
    /// Clear definition of expected task outcome.
    pub expected_output: String,

    // ---- Configuration ----
    /// Dictionary containing task-specific configuration parameters.
    pub config: Option<HashMap<String, serde_json::Value>>,

    // ---- Agent reference (stored as role string) ----
    /// Agent role responsible for execution. In the full implementation this would
    /// be a reference to a BaseAgent; here we store the agent role string.
    pub agent: Option<String>,

    // ---- Context tasks (stored as task IDs) ----
    /// IDs of other tasks providing context or input data.
    pub context: Option<Vec<Uuid>>,

    // ---- Execution mode ----
    /// Whether the task should be executed asynchronously.
    pub async_execution: bool,

    // ---- Output format ----
    /// Schema name for JSON output format.
    pub output_json: Option<String>,
    /// Schema name for Pydantic output format.
    pub output_pydantic: Option<String>,
    /// Schema name for structured LLM output using native provider features.
    pub response_model: Option<String>,

    // ---- File output ----
    /// File path for storing task output.
    pub output_file: Option<String>,
    /// Whether to create the directory for output_file if it doesn't exist.
    pub create_directory: bool,

    // ---- Task output ----
    /// Task output, the final result after being executed.
    pub output: Option<TaskOutput>,

    // ---- Tools (stored as tool names) ----
    /// Tools the agent is limited to use for this task (stored as tool names).
    pub tools: Vec<String>,

    // ---- Input files ----
    /// Named input files for this task. Keys are reference names, values are paths.
    pub input_files: HashMap<String, String>,

    // ---- Security ----
    /// Security configuration for the task.
    pub security_config: SecurityConfig,

    // ---- Identity ----
    /// Unique identifier for the task.
    pub id: Uuid,

    // ---- Human input ----
    /// Whether the task should have a human review the final answer of the agent.
    pub human_input: bool,

    // ---- Markdown ----
    /// Whether the task should instruct the agent to return the final answer formatted in Markdown.
    pub markdown: bool,

    // ---- Guardrails ----
    /// Single guardrail description (string) or None.
    pub guardrail: Option<String>,
    /// List of guardrail descriptions or None.
    pub guardrails: Option<Vec<String>>,
    /// Maximum number of retries when guardrail fails.
    pub guardrail_max_retries: i32,
    /// Current number of retries.
    pub retry_count: i32,

    // ---- Timing ----
    /// Start time of the task execution.
    pub start_time: Option<DateTime<Utc>>,
    /// End time of the task execution.
    pub end_time: Option<DateTime<Utc>>,

    // ---- Tracking ----
    /// Set of agent roles that have processed this task.
    pub processed_by_agents: HashSet<String>,

    /// Whether this task should append trigger payload to description.
    pub allow_crewai_trigger_context: Option<bool>,

    // ---- Private / non-serialized fields ----
    /// The compiled guardrail callback (not serialized).
    #[serde(skip)]
    pub guardrail_fn: Option<GuardrailFn>,

    /// The compiled guardrail callbacks list (not serialized).
    #[serde(skip)]
    pub guardrails_fns: Vec<GuardrailFn>,

    /// Task completion callback (not serialized).
    #[serde(skip)]
    pub callback: Option<TaskCallback>,

    /// Agent executor callback (not serialized).
    /// Set by the Crew to execute the task via the assigned agent.
    #[serde(skip)]
    pub agent_executor: Option<AgentExecutorFn>,

    /// Original description before interpolation.
    #[serde(skip)]
    original_description: Option<String>,
    /// Original expected output before interpolation.
    #[serde(skip)]
    original_expected_output: Option<String>,
    /// Original output file before interpolation.
    #[serde(skip)]
    original_output_file: Option<String>,
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .field("description", &self.description)
            .field("expected_output", &self.expected_output)
            .field("agent", &self.agent)
            .field("async_execution", &self.async_execution)
            .finish_non_exhaustive()
    }
}

impl Clone for Task {
    fn clone(&self) -> Self {
        Self {
            used_tools: self.used_tools,
            tools_errors: self.tools_errors,
            delegations: self.delegations,
            name: self.name.clone(),
            prompt_context: self.prompt_context.clone(),
            description: self.description.clone(),
            expected_output: self.expected_output.clone(),
            config: self.config.clone(),
            agent: self.agent.clone(),
            context: self.context.clone(),
            async_execution: self.async_execution,
            output_json: self.output_json.clone(),
            output_pydantic: self.output_pydantic.clone(),
            response_model: self.response_model.clone(),
            output_file: self.output_file.clone(),
            create_directory: self.create_directory,
            output: self.output.clone(),
            tools: self.tools.clone(),
            input_files: self.input_files.clone(),
            security_config: self.security_config.clone(),
            id: Uuid::new_v4(), // New ID on clone, matching Python behavior
            human_input: self.human_input,
            markdown: self.markdown,
            guardrail: self.guardrail.clone(),
            guardrails: self.guardrails.clone(),
            guardrail_max_retries: self.guardrail_max_retries,
            retry_count: 0,
            start_time: None,
            end_time: None,
            processed_by_agents: HashSet::new(),
            allow_crewai_trigger_context: self.allow_crewai_trigger_context,
            // Non-cloneable fields
            guardrail_fn: None,
            guardrails_fns: Vec::new(),
            callback: None,
            agent_executor: None,
            original_description: self.original_description.clone(),
            original_expected_output: self.original_expected_output.clone(),
            original_output_file: self.original_output_file.clone(),
        }
    }
}

impl Task {
    /// Create a new Task with required fields.
    pub fn new(description: String, expected_output: String) -> Self {
        Self {
            used_tools: 0,
            tools_errors: 0,
            delegations: 0,
            name: None,
            prompt_context: None,
            description,
            expected_output,
            config: None,
            callback: None,
            agent: None,
            context: None,
            async_execution: false,
            output_json: None,
            output_pydantic: None,
            response_model: None,
            output_file: None,
            create_directory: true,
            output: None,
            tools: Vec::new(),
            input_files: HashMap::new(),
            security_config: SecurityConfig::default(),
            id: Uuid::new_v4(),
            human_input: false,
            markdown: false,
            guardrail: None,
            guardrails: None,
            guardrail_max_retries: 3,
            retry_count: 0,
            start_time: None,
            end_time: None,
            processed_by_agents: HashSet::new(),
            allow_crewai_trigger_context: None,
            guardrail_fn: None,
            guardrails_fns: Vec::new(),
            agent_executor: None,
            original_description: None,
            original_expected_output: None,
            original_output_file: None,
        }
    }

    /// Set the agent executor callback.
    pub fn set_agent_executor<F>(&mut self, executor: F)
    where
        F: Fn(&str, Option<&str>, &[String]) -> Result<(String, Vec<crate::tasks::task_output::LLMMessage>), String> + Send + Sync + 'static,
    {
        self.agent_executor = Some(Box::new(executor));
    }

    /// Execute the task synchronously.
    ///
    /// In the full implementation this would delegate to an agent executor.
    /// Currently a stub that sets start_time and returns a placeholder.
    pub fn execute_sync(
        &mut self,
        agent: Option<&str>,
        context: Option<&str>,
        _tools: Option<&[String]>,
    ) -> Result<TaskOutput, String> {
        self.start_time = Some(Utc::now());

        let agent_role = agent
            .or(self.agent.as_deref())
            .ok_or_else(|| {
                format!(
                    "The task '{}' has no agent assigned, therefore it can't be executed directly \
                     and should be executed in a Crew using a specific process that supports that, \
                     like hierarchical.",
                    self.description
                )
            })?
            .to_string();

        if let Some(ctx) = context {
            self.prompt_context = Some(ctx.to_string());
        }

        self.processed_by_agents.insert(agent_role.clone());

        // Build the task prompt
        let task_prompt = self.prompt();

        // Collect tool names
        let tool_names: Vec<String> = self.tools.clone();

        // Execute via the agent executor callback if set
        let (result, messages) = if let Some(ref executor) = self.agent_executor {
            executor(&task_prompt, context, &tool_names)?
        } else {
            // Fallback: return placeholder if no executor configured
            log::warn!("No agent_executor configured for task, returning placeholder");
            (format!("[Task execution placeholder for: {}]", self.description), Vec::new())
        };

        let task_output = TaskOutput {
            description: self.description.clone(),
            name: self.name.clone().or_else(|| Some(self.description.clone())),
            expected_output: Some(self.expected_output.clone()),
            summary: Some(
                self.description
                    .split_whitespace()
                    .take(10)
                    .collect::<Vec<&str>>()
                    .join(" ")
                    + "...",
            ),
            raw: result,
            pydantic: None,
            json_dict: None,
            agent: agent_role,
            output_format: self.get_output_format(),
            messages,
        };

        self.output = Some(task_output.clone());
        self.end_time = Some(Utc::now());

        if let Some(ref cb) = self.callback {
            cb(&task_output);
        }

        Ok(task_output)
    }

    /// Execute the task asynchronously (spawns a background tokio task).
    ///
    /// Returns a JoinHandle that resolves to the TaskOutput.
    pub fn execute_async(
        &mut self,
        agent: Option<String>,
        context: Option<String>,
        tools: Option<Vec<String>>,
    ) -> tokio::task::JoinHandle<Result<TaskOutput, String>> {
        let mut task_clone = self.clone();
        tokio::spawn(async move {
            task_clone.execute_sync(
                agent.as_deref(),
                context.as_deref(),
                tools.as_deref(),
            )
        })
    }

    /// Generate the task prompt.
    ///
    /// When the markdown attribute is true, instructions for formatting the
    /// response in Markdown syntax will be added to the prompt.
    pub fn prompt(&self) -> String {
        let mut tasks_slices = vec![self.description.clone()];

        let output = format!(
            "Expected Output: {}",
            self.expected_output
        );
        tasks_slices.push(output);

        if self.markdown {
            let markdown_instruction = "\
Your final answer MUST be formatted in Markdown syntax.\n\
Follow these guidelines:\n\
- Use # for headers\n\
- Use ** for bold text\n\
- Use * for italic text\n\
- Use - or * for bullet points\n\
- Use `code` for inline code\n\
- Use ```language for code blocks";
            tasks_slices.push(markdown_instruction.to_string());
        }

        tasks_slices.join("\n")
    }

    /// Compute the key property (MD5 hash of description|expected_output).
    pub fn key(&self) -> String {
        let desc = self
            .original_description
            .as_deref()
            .unwrap_or(&self.description);
        let expected = self
            .original_expected_output
            .as_deref()
            .unwrap_or(&self.expected_output);

        let source = format!("{}|{}", desc, expected);
        let mut hasher = Md5::new();
        hasher.update(source.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get the execution duration in seconds, if both start and end times are set.
    pub fn execution_duration(&self) -> Option<f64> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => {
                Some((end - start).num_milliseconds() as f64 / 1000.0)
            }
            _ => None,
        }
    }

    /// Interpolate inputs into the task description, expected output, and output file path.
    pub fn interpolate_inputs(&mut self, inputs: &HashMap<String, String>) {
        if self.original_description.is_none() {
            self.original_description = Some(self.description.clone());
        }
        if self.original_expected_output.is_none() {
            self.original_expected_output = Some(self.expected_output.clone());
        }
        if self.output_file.is_some() && self.original_output_file.is_none() {
            self.original_output_file = self.output_file.clone();
        }

        if inputs.is_empty() {
            return;
        }

        // Simple interpolation: replace {key} with value
        if let Some(ref orig_desc) = self.original_description {
            self.description = interpolate_string(orig_desc, inputs);
        }
        if let Some(ref orig_expected) = self.original_expected_output {
            self.expected_output = interpolate_string(orig_expected, inputs);
        }
        if let Some(ref orig_file) = self.original_output_file {
            self.output_file = Some(interpolate_string(orig_file, inputs));
        }
    }

    /// Increment the tools errors counter.
    pub fn increment_tools_errors(&mut self) {
        self.tools_errors += 1;
    }

    /// Increment the delegations counter.
    pub fn increment_delegations(&mut self, agent_name: Option<&str>) {
        if let Some(name) = agent_name {
            self.processed_by_agents.insert(name.to_string());
        }
        self.delegations += 1;
    }

    /// Get the output format based on task configuration.
    fn get_output_format(&self) -> OutputFormat {
        if self.output_json.is_some() {
            OutputFormat::JSON
        } else if self.output_pydantic.is_some() {
            OutputFormat::Pydantic
        } else {
            OutputFormat::Raw
        }
    }

    /// Save task output to a file.
    pub fn save_file(&self, result: &str) -> Result<(), String> {
        let output_file = self
            .output_file
            .as_ref()
            .ok_or("output_file is not set")?;

        let path = std::path::Path::new(output_file);

        if self.create_directory {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            }
        }

        std::fs::write(path, result).map_err(|e| format!("Failed to save output file: {}", e))
    }
}

impl std::fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Task(description={}, expected_output={})",
            self.description, self.expected_output
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
