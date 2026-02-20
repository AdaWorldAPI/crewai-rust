//! Execution pipeline — orchestrates Blackboard + Phase + Router + A2A.
//!
//! The pipeline runs a [`UnifiedExecution`] through its steps, using the
//! [`StepRouter`] for dispatch and the [`Blackboard`] for zero-serde data flow.
//! Each step runs inside a [`Phase`], which provides scoped `&mut` access
//! and automatic trace logging.
//!
//! # One-Binary Architecture
//!
//! Since all crates compile into one binary:
//! - No IPC between subsystems
//! - No serialization for internal data flow
//! - Phase-based `&mut` discipline prevents data races at compile time
//! - The A2A registry provides agent discovery without message passing

use super::router::{StepDomain, StepHandler, StepResult, StepRouter};
use super::types::{StepStatus, UnifiedExecution, UnifiedStep};
use crate::blackboard::{Blackboard, Phase};
use crate::hooks::lifecycle::HookRegistry;

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Execution pipeline that runs a full workflow using Blackboard + Router.
///
/// # Example
///
/// ```
/// use crewai::contract::pipeline::Pipeline;
/// use crewai::contract::router::{StepDomain, StepHandler, StepResult, StepRouter};
/// use crewai::contract::types::{UnifiedExecution, UnifiedStep};
/// use crewai::blackboard::Blackboard;
///
/// // Register handlers
/// struct EchoHandler;
/// impl StepHandler for EchoHandler {
///     fn handle(&self, step: &mut UnifiedStep, bb: &mut Blackboard) -> StepResult {
///         step.mark_running();
///         bb.put_typed(
///             format!("out:{}", step.sequence),
///             step.input.clone(),
///             "echo",
///             &step.step_type,
///         );
///         step.mark_completed(step.input.clone());
///         Ok(())
///     }
///     fn domain(&self) -> StepDomain { StepDomain::Crew }
/// }
///
/// let mut router = StepRouter::new();
/// router.register(Box::new(EchoHandler));
///
/// let mut pipeline = Pipeline::new(router);
///
/// let mut exec = UnifiedExecution::new("test");
/// exec.steps.push(UnifiedStep::new("e1", "crew.agent", "Echo", 0));
///
/// let bb = pipeline.run(&mut exec).unwrap();
/// assert_eq!(exec.status, crewai::StepStatus::Completed);
/// ```
pub struct Pipeline {
    router: StepRouter,
    hooks: Option<HookRegistry>,
}

impl Pipeline {
    /// Create a new pipeline with the given router.
    pub fn new(router: StepRouter) -> Self {
        Self {
            router,
            hooks: None,
        }
    }

    /// Create a pipeline with router and hook registry.
    pub fn with_hooks(router: StepRouter, hooks: HookRegistry) -> Self {
        Self {
            router,
            hooks: Some(hooks),
        }
    }

    /// Run a full execution, returning the final blackboard state.
    ///
    /// Each step is wrapped in a [`Phase`] for trace logging and
    /// borrow discipline. Steps are executed sequentially in order.
    /// The execution stops on the first failed step.
    pub fn run(
        &self,
        execution: &mut UnifiedExecution,
    ) -> Result<Blackboard, Box<dyn std::error::Error + Send + Sync>> {
        let mut bb = Blackboard::with_capacity(execution.steps.len() * 2);
        self.run_with_blackboard(execution, &mut bb)?;
        Ok(bb)
    }

    /// Run a full execution using an existing blackboard.
    ///
    /// This is useful when you want to pre-populate the blackboard
    /// with context (e.g., conversation history, agent registry).
    pub fn run_with_blackboard(
        &self,
        execution: &mut UnifiedExecution,
        bb: &mut Blackboard,
    ) -> StepResult {
        execution.mark_running();

        log::info!(
            "Pipeline: starting execution '{}' ({}) with {} steps",
            execution.workflow_name,
            execution.execution_id,
            execution.steps.len(),
        );

        for i in 0..execution.steps.len() {
            let step = &execution.steps[i];
            if step.status != StepStatus::Pending {
                continue;
            }

            let step_type = step.step_type.clone();
            let step_name = step.name.clone();

            // Run the step inside a phase for trace discipline
            let result = {
                let mut phase = Phase::begin(bb, &step_type);
                let step_mut = &mut execution.steps[i];
                self.router.dispatch(step_mut, phase.bb())
            }; // Phase dropped here — trace entry recorded

            match result {
                Ok(()) => {
                    log::debug!(
                        "Pipeline: step {} '{}' ({}) completed",
                        i,
                        step_name,
                        step_type,
                    );
                }
                Err(e) => {
                    log::error!(
                        "Pipeline: step {} '{}' ({}) failed: {}",
                        i,
                        step_name,
                        step_type,
                        e,
                    );
                    execution.mark_failed();
                    return Err(e);
                }
            }
        }

        execution.mark_completed();
        log::info!(
            "Pipeline: execution '{}' completed ({} steps)",
            execution.workflow_name,
            execution.steps.len(),
        );

        Ok(())
    }

    /// Get a reference to the router.
    pub fn router(&self) -> &StepRouter {
        &self.router
    }

    /// Get a mutable reference to the router (to register new handlers).
    pub fn router_mut(&mut self) -> &mut StepRouter {
        &mut self.router
    }

    /// Get the hook registry.
    pub fn hooks(&self) -> Option<&HookRegistry> {
        self.hooks.as_ref()
    }
}

impl std::fmt::Debug for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pipeline")
            .field("router", &self.router)
            .field("has_hooks", &self.hooks.is_some())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    // Simple handler that writes typed data to blackboard
    struct TypedWriteHandler;

    impl StepHandler for TypedWriteHandler {
        fn handle(&self, step: &mut UnifiedStep, bb: &mut Blackboard) -> StepResult {
            step.mark_running();

            // Read input from blackboard if available
            let input_key = format!("in:{}", step.sequence);
            let context = bb.get_typed::<String>(&input_key).cloned();

            // Write output (zero-serde, typed)
            let output_key = format!("out:{}", step.sequence);
            let output = format!(
                "Processed by {} (context: {:?})",
                step.name,
                context,
            );
            bb.put_typed(
                output_key,
                output.clone(),
                &step.step_type,
                &step.step_type,
            );

            step.mark_completed(serde_json::json!({"output": output}));
            Ok(())
        }

        fn domain(&self) -> StepDomain {
            StepDomain::Crew
        }
    }

    struct OcHandler;

    impl StepHandler for OcHandler {
        fn handle(&self, step: &mut UnifiedStep, bb: &mut Blackboard) -> StepResult {
            step.mark_running();
            let key = format!("oc:{}", step.sequence);
            bb.put_typed(key, format!("oc-output-{}", step.sequence), "oc", &step.step_type);
            step.mark_completed(Value::String("done".into()));
            Ok(())
        }

        fn domain(&self) -> StepDomain {
            StepDomain::OpenClaw
        }
    }

    #[test]
    fn test_pipeline_basic_execution() {
        let mut router = StepRouter::new();
        router.register(Box::new(TypedWriteHandler));

        let pipeline = Pipeline::new(router);

        let mut exec = UnifiedExecution::new("test-workflow");
        exec.steps.push(UnifiedStep::new("e1", "crew.agent", "Step A", 0));
        exec.steps.push(UnifiedStep::new("e1", "crew.agent", "Step B", 1));

        let bb = pipeline.run(&mut exec).unwrap();

        assert_eq!(exec.status, StepStatus::Completed);
        assert!(exec.started_at.is_some());
        assert!(exec.finished_at.is_some());

        // Both steps completed
        assert_eq!(exec.steps[0].status, StepStatus::Completed);
        assert_eq!(exec.steps[1].status, StepStatus::Completed);

        // Blackboard has typed outputs
        let out0 = bb.get_typed::<String>("out:0").unwrap();
        assert!(out0.contains("Step A"));

        // Trace has phase markers
        let trace = bb.trace();
        assert!(trace.iter().any(|t| t.contains("phase:")));
    }

    #[test]
    fn test_pipeline_multi_domain() {
        let mut router = StepRouter::new();
        router.register(Box::new(TypedWriteHandler));
        router.register(Box::new(OcHandler));

        let pipeline = Pipeline::new(router);

        let mut exec = UnifiedExecution::new("cross-domain");
        exec.steps.push(UnifiedStep::new("e1", "crew.agent", "Research", 0));
        exec.steps.push(UnifiedStep::new("e1", "oc.channel.send", "Send", 1));

        let bb = pipeline.run(&mut exec).unwrap();
        assert_eq!(exec.status, StepStatus::Completed);

        // crew handler wrote to out:0
        assert!(bb.get_typed::<String>("out:0").is_some());
        // oc handler wrote to oc:1
        assert!(bb.get_typed::<String>("oc:1").is_some());
    }

    #[test]
    fn test_pipeline_with_pre_populated_blackboard() {
        let mut router = StepRouter::new();
        router.register(Box::new(TypedWriteHandler));

        let pipeline = Pipeline::new(router);

        let mut exec = UnifiedExecution::new("context-test");
        exec.steps.push(UnifiedStep::new("e1", "crew.agent", "Agent", 0));

        let mut bb = Blackboard::new();
        // Pre-populate context
        bb.put_typed("in:0", "pre-existing context".to_string(), "setup", "setup");

        pipeline.run_with_blackboard(&mut exec, &mut bb).unwrap();

        let out = bb.get_typed::<String>("out:0").unwrap();
        assert!(out.contains("pre-existing context"));
    }

    #[test]
    fn test_pipeline_with_a2a_registry() {
        let mut router = StepRouter::new();
        router.register(Box::new(TypedWriteHandler));

        let pipeline = Pipeline::new(router);

        let mut exec = UnifiedExecution::new("a2a-test");
        exec.steps.push(UnifiedStep::new("e1", "crew.agent", "Agent", 0));

        let mut bb = Blackboard::new();
        // Register agents in A2A
        bb.a2a.register("agent-1", "Researcher", "research", vec!["search".into()]);
        bb.a2a.register("agent-2", "Writer", "writing", vec!["write".into()]);

        pipeline.run_with_blackboard(&mut exec, &mut bb).unwrap();

        // A2A registry persists through execution
        assert_eq!(bb.a2a.len(), 2);
        assert!(bb.a2a.by_capability("search").len() == 1);
    }

    #[test]
    fn test_pipeline_failure_marks_execution_failed() {
        struct FailingHandler;
        impl StepHandler for FailingHandler {
            fn handle(&self, step: &mut UnifiedStep, _bb: &mut Blackboard) -> StepResult {
                step.mark_running();
                step.mark_failed("boom");
                Err("boom".into())
            }
            fn domain(&self) -> StepDomain {
                StepDomain::Crew
            }
        }

        let mut router = StepRouter::new();
        router.register(Box::new(FailingHandler));

        let pipeline = Pipeline::new(router);

        let mut exec = UnifiedExecution::new("fail-test");
        exec.steps.push(UnifiedStep::new("e1", "crew.agent", "Boom", 0));
        exec.steps.push(UnifiedStep::new("e1", "crew.agent", "Never", 1));

        let result = pipeline.run(&mut exec);
        assert!(result.is_err());
        assert_eq!(exec.status, StepStatus::Failed);

        // Second step never ran
        assert_eq!(exec.steps[1].status, StepStatus::Pending);
    }

    #[test]
    fn test_pipeline_skips_completed_steps() {
        let mut router = StepRouter::new();
        router.register(Box::new(TypedWriteHandler));

        let pipeline = Pipeline::new(router);

        let mut exec = UnifiedExecution::new("skip-test");
        let mut done = UnifiedStep::new("e1", "crew.agent", "AlreadyDone", 0);
        done.mark_completed(serde_json::json!({"pre": true}));
        exec.steps.push(done);
        exec.steps.push(UnifiedStep::new("e1", "crew.agent", "RunMe", 1));

        let bb = pipeline.run(&mut exec).unwrap();

        // First step kept its original output
        assert_eq!(exec.steps[0].output["pre"], true);
        // Second step was processed
        assert_eq!(exec.steps[1].status, StepStatus::Completed);
        // Only second step wrote to blackboard
        assert!(bb.get_typed::<String>("out:0").is_none());
        assert!(bb.get_typed::<String>("out:1").is_some());
    }
}
