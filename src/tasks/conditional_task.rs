//! Conditional task execution based on previous task output.
//!
//! Corresponds to `crewai/tasks/conditional_task.py`.

use serde::{Deserialize, Serialize};

use super::output_format::OutputFormat;
use super::task_output::TaskOutput;

/// A callback type that evaluates a [`TaskOutput`] and returns whether execution should proceed.
pub type ConditionFn = Box<dyn Fn(&TaskOutput) -> bool + Send + Sync>;

/// A task that can be conditionally executed based on the output of another task.
///
/// This task type allows for dynamic workflow execution based on the results of
/// previous tasks in the crew execution chain.
///
/// # Notes
///
/// - Cannot be the only task in your crew
/// - Cannot be the first task since it needs context from the previous task
#[derive(Serialize, Deserialize)]
pub struct ConditionalTask {
    /// Description of the task.
    pub description: String,
    /// Expected output of the task.
    pub expected_output: String,
    /// Agent role assigned to this task.
    pub agent_role: Option<String>,
    /// Whether the task should be executed asynchronously.
    pub async_execution: bool,

    /// The condition function that determines whether the task should execute.
    /// Skipped during serialization (not serializable).
    #[serde(skip)]
    pub condition: Option<ConditionFn>,
}

impl std::fmt::Debug for ConditionalTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConditionalTask")
            .field("description", &self.description)
            .field("expected_output", &self.expected_output)
            .field("agent_role", &self.agent_role)
            .field("async_execution", &self.async_execution)
            .field("condition", &self.condition.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

impl Clone for ConditionalTask {
    fn clone(&self) -> Self {
        Self {
            description: self.description.clone(),
            expected_output: self.expected_output.clone(),
            agent_role: self.agent_role.clone(),
            async_execution: self.async_execution,
            // Condition functions cannot be cloned; set to None on clone.
            condition: None,
        }
    }
}

impl ConditionalTask {
    /// Create a new ConditionalTask.
    pub fn new(
        description: String,
        expected_output: String,
        condition: Option<ConditionFn>,
    ) -> Self {
        Self {
            description,
            expected_output,
            agent_role: None,
            async_execution: false,
            condition,
        }
    }

    /// Determines whether the conditional task should be executed based on the provided context.
    ///
    /// # Errors
    ///
    /// Returns an error if no condition function is set.
    pub fn should_execute(&self, context: &TaskOutput) -> Result<bool, String> {
        match &self.condition {
            Some(cond) => Ok(cond(context)),
            None => Err("No condition function set for conditional task".to_string()),
        }
    }

    /// Generate a TaskOutput for when the conditional task is skipped.
    pub fn get_skipped_task_output(&self) -> TaskOutput {
        TaskOutput {
            description: self.description.clone(),
            name: None,
            expected_output: None,
            summary: None,
            raw: String::new(),
            pydantic: None,
            json_dict: None,
            agent: self.agent_role.clone().unwrap_or_default(),
            output_format: OutputFormat::Raw,
            messages: Vec::new(),
        }
    }
}
