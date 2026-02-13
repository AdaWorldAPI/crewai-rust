//! Event bus integration for contract recording.
//!
//! Listens to the CrewAI event bus and records crew/task lifecycle events
//! as unified execution steps.  When `CREWAI_STORE=postgres` is set,
//! events are also persisted to PostgreSQL.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde_json::Value;

use super::types::{StepStatus, UnifiedExecution, UnifiedStep};

/// In-memory recorder that tracks execution and step state.
///
/// Designed to be wrapped in `Arc<RwLock<>>` and shared with event bus
/// handlers.
#[derive(Debug)]
pub struct ContractRecorder {
    /// Active executions keyed by execution_id.
    pub executions: HashMap<String, UnifiedExecution>,
    /// Active steps keyed by step_id.
    pub steps: HashMap<String, UnifiedStep>,
    /// Mapping from crew_name -> execution_id.
    pub crew_to_execution: HashMap<String, String>,
    /// Mapping from task_id -> step_id.
    pub task_to_step: HashMap<String, String>,
    /// Step sequence counter per execution.
    sequence_counters: HashMap<String, i32>,
}

impl ContractRecorder {
    /// Create a new empty recorder.
    pub fn new() -> Self {
        Self {
            executions: HashMap::new(),
            steps: HashMap::new(),
            crew_to_execution: HashMap::new(),
            task_to_step: HashMap::new(),
            sequence_counters: HashMap::new(),
        }
    }

    /// Record a crew kickoff (creates a UnifiedExecution).
    pub fn on_crew_started(&mut self, crew_name: &str) -> String {
        let mut exec = UnifiedExecution::new(crew_name);
        exec.mark_running();
        let execution_id = exec.execution_id.clone();
        self.crew_to_execution
            .insert(crew_name.to_string(), execution_id.clone());
        self.sequence_counters.insert(execution_id.clone(), 0);
        self.executions.insert(execution_id.clone(), exec);
        execution_id
    }

    /// Record a task start (creates a UnifiedStep).
    pub fn on_task_started(
        &mut self,
        task_id: &str,
        task_name: &str,
        crew_name: &str,
        agent_role: Option<&str>,
    ) -> Option<String> {
        let execution_id = self.crew_to_execution.get(crew_name)?.clone();

        let seq = self.sequence_counters.get_mut(&execution_id)?;
        let sequence = *seq;
        *seq += 1;

        // Determine step_type based on agent role
        let step_type = if let Some(role) = agent_role {
            format!("crew.agent.{}", role.to_lowercase().replace(' ', "_"))
        } else {
            "crew.task".to_string()
        };

        let mut step = UnifiedStep::new(&execution_id, &step_type, task_name, sequence);
        step.mark_running();

        let step_id = step.step_id.clone();
        self.task_to_step
            .insert(task_id.to_string(), step_id.clone());
        self.steps.insert(step_id.clone(), step);

        // Add step to execution
        if let Some(exec) = self.executions.get_mut(&execution_id) {
            if let Some(step) = self.steps.get(&step_id) {
                exec.steps.push(step.clone());
            }
        }

        Some(step_id)
    }

    /// Record a task completion with output and decision trail.
    pub fn on_task_completed(
        &mut self,
        task_id: &str,
        output: Value,
        reasoning: Option<String>,
        confidence: Option<f64>,
        alternatives: Option<Value>,
    ) {
        if let Some(step_id) = self.task_to_step.get(task_id).cloned() {
            if let Some(step) = self.steps.get_mut(&step_id) {
                step.mark_completed(output.clone());
                step.reasoning = reasoning;
                step.confidence = confidence;
                step.alternatives = alternatives;

                // Update the step in the execution's steps vec too
                let exec_id = step.execution_id.clone();
                if let Some(exec) = self.executions.get_mut(&exec_id) {
                    if let Some(exec_step) = exec.steps.iter_mut().find(|s| s.step_id == step_id) {
                        *exec_step = step.clone();
                    }
                }
            }
        }
    }

    /// Record a task failure.
    pub fn on_task_failed(&mut self, task_id: &str, error: &str) {
        if let Some(step_id) = self.task_to_step.get(task_id).cloned() {
            if let Some(step) = self.steps.get_mut(&step_id) {
                step.mark_failed(error);

                let exec_id = step.execution_id.clone();
                if let Some(exec) = self.executions.get_mut(&exec_id) {
                    if let Some(exec_step) = exec.steps.iter_mut().find(|s| s.step_id == step_id) {
                        *exec_step = step.clone();
                    }
                }
            }
        }
    }

    /// Record crew completion.
    pub fn on_crew_completed(&mut self, crew_name: &str) {
        if let Some(execution_id) = self.crew_to_execution.get(crew_name).cloned() {
            if let Some(exec) = self.executions.get_mut(&execution_id) {
                exec.mark_completed();
            }
        }
    }

    /// Record crew failure.
    pub fn on_crew_failed(&mut self, crew_name: &str) {
        if let Some(execution_id) = self.crew_to_execution.get(crew_name).cloned() {
            if let Some(exec) = self.executions.get_mut(&execution_id) {
                exec.mark_failed();
            }
        }
    }

    /// Get a completed execution by crew name.
    pub fn get_execution(&self, crew_name: &str) -> Option<&UnifiedExecution> {
        self.crew_to_execution
            .get(crew_name)
            .and_then(|id| self.executions.get(id))
    }

    /// Get execution by ID.
    pub fn get_execution_by_id(&self, execution_id: &str) -> Option<&UnifiedExecution> {
        self.executions.get(execution_id)
    }

    /// Get all completed executions.
    pub fn all_executions(&self) -> Vec<&UnifiedExecution> {
        self.executions.values().collect()
    }

    /// Get a step by task_id.
    pub fn get_step(&self, task_id: &str) -> Option<&UnifiedStep> {
        self.task_to_step
            .get(task_id)
            .and_then(|id| self.steps.get(id))
    }
}

impl Default for ContractRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a shared, thread-safe recorder.
pub fn shared_recorder() -> Arc<RwLock<ContractRecorder>> {
    Arc::new(RwLock::new(ContractRecorder::new()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorder_crew_lifecycle() {
        let mut recorder = ContractRecorder::new();

        // Crew starts
        let exec_id = recorder.on_crew_started("my-crew");
        assert!(recorder.executions.contains_key(&exec_id));
        assert_eq!(
            recorder.executions[&exec_id].status,
            StepStatus::Running
        );

        // Task 1 starts
        let step_id = recorder
            .on_task_started("t1", "Research", "my-crew", Some("Researcher"))
            .unwrap();
        assert!(recorder.steps.contains_key(&step_id));
        assert_eq!(recorder.steps[&step_id].status, StepStatus::Running);
        assert_eq!(
            recorder.steps[&step_id].step_type,
            "crew.agent.researcher"
        );

        // Task 1 completes
        recorder.on_task_completed(
            "t1",
            serde_json::json!({"result": "Found 10 papers"}),
            Some("Searched Google Scholar".into()),
            Some(0.92),
            None,
        );
        let step = recorder.get_step("t1").unwrap();
        assert_eq!(step.status, StepStatus::Completed);
        assert_eq!(step.reasoning.as_deref(), Some("Searched Google Scholar"));
        assert_eq!(step.confidence, Some(0.92));

        // Task 2 starts and fails
        recorder
            .on_task_started("t2", "Write Report", "my-crew", Some("Writer"))
            .unwrap();
        recorder.on_task_failed("t2", "LLM timeout");
        let step2 = recorder.get_step("t2").unwrap();
        assert_eq!(step2.status, StepStatus::Failed);
        assert_eq!(step2.error.as_deref(), Some("LLM timeout"));

        // Crew completes
        recorder.on_crew_completed("my-crew");
        let exec = recorder.get_execution("my-crew").unwrap();
        assert_eq!(exec.status, StepStatus::Completed);
        assert_eq!(exec.steps.len(), 2);
    }

    #[test]
    fn test_recorder_crew_failure() {
        let mut recorder = ContractRecorder::new();
        recorder.on_crew_started("failing-crew");
        recorder.on_crew_failed("failing-crew");

        let exec = recorder.get_execution("failing-crew").unwrap();
        assert_eq!(exec.status, StepStatus::Failed);
    }

    #[test]
    fn test_recorder_task_without_agent() {
        let mut recorder = ContractRecorder::new();
        recorder.on_crew_started("crew-1");
        let step_id = recorder
            .on_task_started("t1", "Simple Task", "crew-1", None)
            .unwrap();

        let step = &recorder.steps[&step_id];
        assert_eq!(step.step_type, "crew.task");
    }

    #[test]
    fn test_recorder_sequence_numbering() {
        let mut recorder = ContractRecorder::new();
        recorder.on_crew_started("crew-1");

        recorder.on_task_started("t1", "Task A", "crew-1", None);
        recorder.on_task_started("t2", "Task B", "crew-1", None);
        recorder.on_task_started("t3", "Task C", "crew-1", None);

        let s1 = recorder.get_step("t1").unwrap();
        let s2 = recorder.get_step("t2").unwrap();
        let s3 = recorder.get_step("t3").unwrap();

        assert_eq!(s1.sequence, 0);
        assert_eq!(s2.sequence, 1);
        assert_eq!(s3.sequence, 2);
    }

    #[test]
    fn test_recorder_unknown_crew() {
        let mut recorder = ContractRecorder::new();
        // Task for unknown crew returns None
        assert!(recorder
            .on_task_started("t1", "Orphan", "unknown", None)
            .is_none());
    }

    #[test]
    fn test_shared_recorder() {
        let recorder = shared_recorder();
        {
            let mut r = recorder.write().unwrap();
            r.on_crew_started("shared-crew");
        }
        {
            let r = recorder.read().unwrap();
            assert!(r.get_execution("shared-crew").is_some());
        }
    }

    #[test]
    fn test_all_executions() {
        let mut recorder = ContractRecorder::new();
        recorder.on_crew_started("crew-a");
        recorder.on_crew_started("crew-b");
        assert_eq!(recorder.all_executions().len(), 2);
    }
}
