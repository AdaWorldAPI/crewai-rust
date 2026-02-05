//! Utility functions for agent task execution.
//!
//! Corresponds to `crewai/agent/utils.py`.
//!
//! This module contains shared logic extracted from the Agent's execute_task
//! and aexecute_task methods to reduce code duplication. Functions handle
//! reasoning, prompt building, knowledge retrieval, training data, tool
//! results processing, and message management.

use std::collections::HashMap;

/// Handle reasoning/chain-of-thought for an agent's task execution.
///
/// If the agent has reasoning enabled, this generates a plan using the
/// agent's LLM before actual task execution begins.
///
/// # Arguments
///
/// * `agent_role` - The role of the agent performing reasoning.
/// * `task_description` - The task to reason about.
/// * `max_attempts` - Maximum number of reasoning iterations.
///
/// # Returns
///
/// The reasoning result string.
pub fn handle_reasoning(
    agent_role: &str,
    task_description: &str,
    max_attempts: i32,
) -> Result<String, String> {
    log::debug!(
        "handle_reasoning: agent='{}', task='{}', max_attempts={}",
        agent_role,
        task_description,
        max_attempts
    );
    // TODO: Implement iterative reasoning with LLM via AgentReasoning handler.
    // In Python this delegates to:
    //   reasoning_handler = AgentReasoning(task=task, agent=agent)
    //   reasoning_output = reasoning_handler.handle_agent_reasoning()
    //   task.description += f"\n\nReasoning Plan:\n{reasoning_output.plan.plan}"
    Ok(format!(
        "[Reasoning result for '{}' on task: {}]",
        agent_role, task_description
    ))
}

/// Build the task prompt with schema information for structured output.
///
/// If the task has output_json or output_pydantic requirements but no
/// response_model, this augments the prompt with JSON schema instructions.
///
/// # Arguments
///
/// * `task_prompt` - The base task prompt.
/// * `output_schema` - Optional JSON schema for structured output.
///
/// # Returns
///
/// The formatted task prompt with schema instructions.
pub fn build_task_prompt_with_schema(
    task_prompt: &str,
    output_schema: Option<&serde_json::Value>,
) -> String {
    match output_schema {
        Some(schema) => {
            format!(
                "{}\n\nYour output MUST conform to the following JSON schema:\n{}",
                task_prompt,
                serde_json::to_string_pretty(schema).unwrap_or_default()
            )
        }
        None => task_prompt.to_string(),
    }
}

/// Format a task prompt with context if provided.
///
/// Uses the i18n "task_with_context" template to combine the task prompt
/// and context string.
///
/// # Arguments
///
/// * `task_prompt` - The task prompt.
/// * `context` - Optional context string.
///
/// # Returns
///
/// The task prompt formatted with context if provided.
pub fn format_task_with_context(task_prompt: &str, context: Option<&str>) -> String {
    match context {
        Some(ctx) if !ctx.is_empty() => {
            format!(
                "Task: {}\n\nThis is the context you're working with:\n{}",
                task_prompt, ctx
            )
        }
        _ => task_prompt.to_string(),
    }
}

/// Get knowledge configuration from agent.
///
/// Extracts the knowledge configuration (limits, thresholds, etc.) from
/// the agent's knowledge_config field.
///
/// # Arguments
///
/// * `knowledge_config` - Agent's knowledge config.
///
/// # Returns
///
/// Knowledge configuration map.
pub fn get_knowledge_config(
    knowledge_config: Option<&HashMap<String, serde_json::Value>>,
) -> HashMap<String, serde_json::Value> {
    knowledge_config.cloned().unwrap_or_default()
}

/// Handle knowledge retrieval for task execution.
///
/// Queries both agent-specific and crew-specific knowledge bases to
/// augment the task prompt with relevant context.
///
/// # Arguments
///
/// * `query` - The query to search for.
/// * `knowledge_config` - Configuration for the knowledge source.
///
/// # Returns
///
/// Relevant knowledge context string.
pub fn handle_knowledge_retrieval(
    query: &str,
    _knowledge_config: &HashMap<String, serde_json::Value>,
) -> Result<String, String> {
    log::debug!("handle_knowledge_retrieval: query='{}'", query);
    // TODO: Implement actual knowledge retrieval:
    //   1. Generate search query via LLM
    //   2. Query agent knowledge
    //   3. Query crew knowledge
    //   4. Extract and combine knowledge context
    //   5. Emit knowledge retrieval events
    Ok(String::new())
}

/// Async version of handle_knowledge_retrieval.
pub async fn ahandle_knowledge_retrieval(
    query: &str,
    knowledge_config: &HashMap<String, serde_json::Value>,
) -> Result<String, String> {
    handle_knowledge_retrieval(query, knowledge_config)
}

/// Apply training data to the task prompt.
///
/// If the crew is in training mode, applies training handler data.
/// Otherwise, applies previously trained agent data.
///
/// # Arguments
///
/// * `agent_role` - The role of the agent.
/// * `task_prompt` - The current task prompt.
/// * `is_training` - Whether the crew is in training mode.
///
/// # Returns
///
/// The task prompt with training data applied.
pub fn apply_training_data(
    agent_role: &str,
    task_prompt: &str,
    is_training: bool,
) -> String {
    log::debug!(
        "apply_training_data: agent='{}', training={}",
        agent_role,
        is_training
    );
    // TODO: Implement training data application:
    //   - If training: load from TRAINING_DATA_FILE, append human_feedbacks
    //   - If not training: load from TRAINED_AGENTS_DATA_FILE, append suggestions
    task_prompt.to_string()
}

/// Process tool results after execution.
///
/// Checks all tool results and returns the `result_as_answer` value
/// if any tool has that flag set.
///
/// # Arguments
///
/// * `tools_results` - Results of the tools used by the agent.
/// * `result` - The current result string.
///
/// # Returns
///
/// The final result, potentially overridden by tool result_as_answer.
pub fn process_tool_results(
    tools_results: &[HashMap<String, serde_json::Value>],
    result: &str,
) -> String {
    for tool_result in tools_results {
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
    result.to_string()
}

/// Save last messages from agent executor.
///
/// Sanitizes messages to be compatible with TaskOutput's LLMMessage type,
/// which accepts 'user', 'assistant', 'system', and 'tool' roles.
/// Preserves tool_call_id/name for tool messages and tool_calls for
/// assistant messages.
///
/// # Arguments
///
/// * `messages` - Messages to save.
pub fn save_last_messages(messages: &[HashMap<String, String>]) {
    log::debug!("save_last_messages: {} messages", messages.len());
    // TODO: Implement message persistence with sanitization:
    //   - Filter roles to user/assistant/system/tool
    //   - Preserve tool_call_id and name for tool messages
    //   - Preserve tool_calls for assistant messages
}

/// Prepare tools for task execution and create agent executor.
///
/// Merges task-specific tools with agent tools and initializes the
/// agent executor.
///
/// # Arguments
///
/// * `agent_tools` - Tools available to the agent.
/// * `task_tools` - Tools specific to the task.
///
/// # Returns
///
/// Merged list of tool names.
pub fn prepare_tools(agent_tools: &[String], task_tools: &[String]) -> Vec<String> {
    let mut tools = agent_tools.to_vec();
    for tool in task_tools {
        if !tools.contains(tool) {
            tools.push(tool.clone());
        }
    }
    tools
}

/// Validate max_execution_time parameter.
///
/// Ensures the max execution time is a positive integer if provided.
///
/// # Arguments
///
/// * `max_execution_time` - The maximum execution time to validate.
///
/// # Returns
///
/// `Ok(())` if valid, `Err` with message if invalid.
pub fn validate_max_execution_time(max_execution_time: Option<i64>) -> Result<(), String> {
    if let Some(t) = max_execution_time {
        if t <= 0 {
            return Err(
                "Max Execution time must be a positive integer greater than zero".to_string(),
            );
        }
    }
    Ok(())
}

/// Combine agent and crew knowledge contexts into a single string.
///
/// # Arguments
///
/// * `agent_context` - The agent's knowledge context.
/// * `crew_context` - The crew's knowledge context.
///
/// # Returns
///
/// Combined knowledge context string.
pub fn combine_knowledge_context(
    agent_context: Option<&str>,
    crew_context: Option<&str>,
) -> String {
    let agent_ctx = agent_context.unwrap_or("");
    let crew_ctx = crew_context.unwrap_or("");
    let separator = if !agent_ctx.is_empty() && !crew_ctx.is_empty() {
        "\n"
    } else {
        ""
    };
    format!("{}{}{}", agent_ctx, separator, crew_ctx)
}
