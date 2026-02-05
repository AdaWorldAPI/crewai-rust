//! Utility functions for crew operations.
//!
//! Corresponds to `crewai/crews/utils.py`.
//!
//! Provides helper functions for preparing crew kickoff, managing task
//! execution flow, streaming context management, agent setup, conditional
//! task handling, and input file extraction.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::tasks::task_output::TaskOutput;

/// Data container for prepared task execution information.
///
/// Holds the agent, tools, and skip status for a single task execution.
#[derive(Debug, Clone)]
pub struct TaskExecutionData {
    /// The agent role to use for task execution (None if skipped).
    pub agent: Option<String>,
    /// Prepared tools for the task (tool names).
    pub tools: Vec<String>,
    /// Whether the task should be skipped (replay).
    pub should_skip: bool,
}

impl TaskExecutionData {
    /// Create a new TaskExecutionData.
    pub fn new(agent: Option<String>, tools: Vec<String>, should_skip: bool) -> Self {
        Self {
            agent,
            tools,
            should_skip,
        }
    }

    /// Create a TaskExecutionData that signals skipping.
    pub fn skip() -> Self {
        Self {
            agent: None,
            tools: Vec::new(),
            should_skip: true,
        }
    }
}

/// Task info for streaming context.
///
/// Contains metadata about the currently executing task for streaming
/// and progress tracking purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    /// Task index in the crew.
    pub index: usize,
    /// Task name.
    pub name: String,
    /// Task unique ID.
    pub id: String,
    /// Agent role handling this task.
    pub agent_role: String,
    /// Agent unique ID.
    pub agent_id: String,
}

impl Default for TaskInfo {
    fn default() -> Self {
        Self {
            index: 0,
            name: String::new(),
            id: String::new(),
            agent_role: String::new(),
            agent_id: String::new(),
        }
    }
}

/// Container for streaming state and holders used during crew execution.
///
/// Manages the streaming state including result holders, current task info,
/// and async/sync mode flags.
#[derive(Debug)]
pub struct StreamingContext {
    /// Results from crew execution.
    pub result_holder: Vec<serde_json::Value>,
    /// Current task info.
    pub current_task_info: TaskInfo,
    /// Whether to use async streaming mode.
    pub use_async: bool,
}

impl StreamingContext {
    /// Create a new StreamingContext.
    pub fn new(use_async: bool) -> Self {
        Self {
            result_holder: Vec::new(),
            current_task_info: TaskInfo::default(),
            use_async,
        }
    }
}

/// Container for streaming state used in for_each crew execution methods.
#[derive(Debug)]
pub struct ForEachStreamingContext {
    /// Results from all crew executions (one list per execution).
    pub result_holder: Vec<Vec<serde_json::Value>>,
    /// Current task info.
    pub current_task_info: TaskInfo,
}

impl ForEachStreamingContext {
    /// Create a new ForEachStreamingContext.
    pub fn new() -> Self {
        Self {
            result_holder: vec![Vec::new()],
            current_task_info: TaskInfo::default(),
        }
    }
}

impl Default for ForEachStreamingContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Enable streaming on all agents that have an LLM configured.
///
/// # Arguments
///
/// * `agent_roles` - List of agent role strings (placeholder for agent refs).
pub fn enable_agent_streaming(agent_roles: &[String]) {
    for role in agent_roles {
        log::debug!("Enabling streaming for agent '{}'", role);
        // TODO: Set agent.llm.stream = true for each agent.
    }
}

/// Set up agents for crew execution.
///
/// Assigns the crew reference, sets knowledge, configures function_calling_llm,
/// step_callback, and creates the agent executor for each agent.
///
/// # Arguments
///
/// * `agents` - Agent role strings (placeholder for agent refs).
/// * `crew_embedder` - Crew's embedder configuration.
/// * `function_calling_llm` - Default function calling LLM for agents.
pub fn setup_agents(
    agents: &[String],
    _crew_embedder: Option<&HashMap<String, serde_json::Value>>,
    _function_calling_llm: Option<&str>,
) {
    for role in agents {
        log::debug!("Setting up agent '{}'", role);
        // TODO: Implement full agent setup:
        //   1. agent.crew = crew
        //   2. agent.set_knowledge(crew_embedder)
        //   3. Set function_calling_llm if not already set
        //   4. Set step_callback if not already set
        //   5. agent.create_agent_executor()
    }
}

/// Prepare crew for kickoff execution.
///
/// Handles before callbacks, event emission, task handler reset, input
/// interpolation, task callbacks, agent setup, and planning.
///
/// # Arguments
///
/// * `inputs` - Optional input dictionary to pass to the crew.
///
/// # Returns
///
/// The potentially modified inputs dictionary after before callbacks.
pub fn prepare_kickoff(
    inputs: Option<HashMap<String, String>>,
) -> Option<HashMap<String, String>> {
    // TODO: Implement full kickoff preparation logic:
    // 1. Normalize inputs to dict
    // 2. Run before_kickoff callbacks
    // 3. Emit CrewKickoffStartedEvent
    // 4. Reset task output handler
    // 5. Extract file inputs
    // 6. Store files
    // 7. Interpolate inputs into tasks and agents
    // 8. Set tasks callbacks
    // 9. Set allow_crewai_trigger_context for first task
    // 10. Setup agents (crew, knowledge, function_calling_llm, step_callback)
    // 11. Handle planning if enabled
    log::debug!("prepare_kickoff called with inputs: {:?}", inputs);
    inputs
}

/// Prepare a task for execution.
///
/// Handles replay skip logic and agent/tool setup.
///
/// # Arguments
///
/// * `task_index` - Index of the current task.
/// * `start_index` - Index to start execution from (for replay).
/// * `task_outputs` - Current list of task outputs.
///
/// # Returns
///
/// The task execution data describing how to execute the task.
pub fn prepare_task_execution(
    task_index: usize,
    start_index: Option<usize>,
    task_outputs: &[TaskOutput],
) -> TaskExecutionData {
    // Handle replay skip
    if let Some(start) = start_index {
        if task_index < start {
            return TaskExecutionData::skip();
        }
    }

    // TODO: Implement full task preparation:
    //   1. Get agent_to_use from crew._get_agent_to_use(task)
    //   2. Get tools_for_task from task.tools or agent.tools
    //   3. Prepare tools via crew._prepare_tools
    //   4. Log task start
    let _ = task_outputs;
    TaskExecutionData::new(None, Vec::new(), false)
}

/// Check if a conditional task should be skipped.
///
/// Evaluates the condition function against the previous task output
/// to determine if a conditional task should execute or be skipped.
///
/// # Arguments
///
/// * `task_outputs` - List of previous task outputs.
/// * `task_index` - Index of the current task.
/// * `was_replayed` - Whether this is a replayed execution.
///
/// # Returns
///
/// The skipped task output if the task should be skipped, None otherwise.
pub fn check_conditional_skip(
    task_outputs: &[TaskOutput],
    _task_index: usize,
    _was_replayed: bool,
) -> Option<TaskOutput> {
    // TODO: Implement full conditional skip logic:
    //   1. Get previous output from task_outputs
    //   2. Call task.should_execute(previous_output)
    //   3. If false, return task.get_skipped_task_output()
    //   4. If not replayed, store execution log
    let _ = task_outputs;
    None
}
