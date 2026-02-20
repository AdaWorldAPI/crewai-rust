//! Cross-crate integration traits for subsystem plug-in.
//!
//! These traits define the interface that ladybug-rs, n8n-rs, and openclaw-rs
//! implement to register themselves with the unified execution pipeline.
//! Since all crates compile into one binary, there's no IPC — just trait
//! objects resolved at startup.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────┐
//! │                    Pipeline                       │
//! │  ┌────────────────────────────────────────────┐  │
//! │  │              StepRouter                     │  │
//! │  │  crew.* → CrewSubsystem.step_handler()      │  │
//! │  │  lb.*   → LadybugSubsystem.step_handler()   │  │
//! │  │  n8n.*  → N8nSubsystem.step_handler()       │  │
//! │  │  oc.*   → OpenClawSubsystem.step_handler()  │  │
//! │  └────────────────────────────────────────────┘  │
//! │  ┌──────────────────┐ ┌──────────────────────┐  │
//! │  │   Blackboard     │ │    A2A Registry       │  │
//! │  │  (typed slots)   │ │  (agent presence)     │  │
//! │  └──────────────────┘ └──────────────────────┘  │
//! └──────────────────────────────────────────────────┘
//! ```
//!
//! Each subsystem:
//! 1. Implements [`Subsystem`] to provide metadata and initialization
//! 2. Returns a [`StepHandler`] for routing
//! 3. Can register agents in the [`A2ARegistry`]
//! 4. Can install lifecycle hooks in the [`HookRegistry`]

use super::router::{StepDomain, StepHandler};
use crate::blackboard::Blackboard;
use crate::hooks::lifecycle::HookRegistry;

// ---------------------------------------------------------------------------
// Subsystem trait
// ---------------------------------------------------------------------------

/// Trait for subsystems that plug into the unified pipeline.
///
/// Each crate (ladybug-rs, n8n-rs, openclaw-rs) implements this trait
/// to register itself with the execution pipeline. The pipeline calls
/// these methods during initialization.
///
/// # Lifecycle
///
/// 1. `name()` / `domain()` — metadata for logging and routing
/// 2. `step_handler()` — returns the handler for this domain's steps
/// 3. `init_blackboard()` — pre-populate the blackboard (optional)
/// 4. `install_hooks()` — register lifecycle hooks (optional)
/// 5. `register_agents()` — register agents in A2A (optional)
pub trait Subsystem: Send + Sync {
    /// Human-readable subsystem name (e.g., "ladybug-rs", "n8n-rs").
    fn name(&self) -> &str;

    /// Which domain this subsystem handles.
    fn domain(&self) -> StepDomain;

    /// Version string for this subsystem.
    fn version(&self) -> &str {
        "0.0.0"
    }

    /// Create the step handler for this subsystem.
    ///
    /// Called once during pipeline initialization. The returned handler
    /// is registered with the [`StepRouter`].
    fn step_handler(&self) -> Box<dyn StepHandler>;

    /// Initialize the blackboard with subsystem-specific state.
    ///
    /// Called once before the first execution. Use this to pre-populate
    /// typed slots, register default data, etc.
    fn init_blackboard(&self, _bb: &mut Blackboard) {
        // Default: no-op
    }

    /// Install lifecycle hooks.
    ///
    /// Called once during pipeline initialization. Use this to add
    /// hooks for observation, interception, or cross-cutting concerns.
    fn install_hooks(&self, _hooks: &mut HookRegistry) {
        // Default: no-op
    }

    /// Register agents in the A2A registry.
    ///
    /// Called once during pipeline initialization. Each subsystem can
    /// register its agents so other subsystems can discover them.
    fn register_agents(&self, _bb: &mut Blackboard) {
        // Default: no-op
    }

    /// Shutdown hook — called when the pipeline is torn down.
    fn shutdown(&self) {
        // Default: no-op
    }
}

// ---------------------------------------------------------------------------
// SubsystemRegistry
// ---------------------------------------------------------------------------

/// Registry of all subsystems in the unified binary.
///
/// Collects subsystems and wires them into the pipeline.
pub struct SubsystemRegistry {
    subsystems: Vec<Box<dyn Subsystem>>,
}

impl SubsystemRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            subsystems: Vec::new(),
        }
    }

    /// Register a subsystem.
    pub fn register(&mut self, subsystem: Box<dyn Subsystem>) {
        log::info!(
            "SubsystemRegistry: registered '{}' v{} for domain '{}'",
            subsystem.name(),
            subsystem.version(),
            subsystem.domain(),
        );
        self.subsystems.push(subsystem);
    }

    /// Build a pipeline from all registered subsystems.
    ///
    /// This:
    /// 1. Creates a `StepRouter` with all step handlers
    /// 2. Creates a `Blackboard` and calls `init_blackboard` for each
    /// 3. Creates a `HookRegistry` and calls `install_hooks` for each
    /// 4. Calls `register_agents` for each
    pub fn build(
        &self,
    ) -> (
        super::pipeline::Pipeline,
        Blackboard,
    ) {
        use super::pipeline::Pipeline;
        use super::router::StepRouter;

        let mut router = StepRouter::new();
        let mut hooks = HookRegistry::new();
        let mut bb = Blackboard::new();

        for sub in &self.subsystems {
            // Register step handler
            router.register(sub.step_handler());

            // Initialize blackboard
            sub.init_blackboard(&mut bb);

            // Install hooks
            sub.install_hooks(&mut hooks);

            // Register agents
            sub.register_agents(&mut bb);
        }

        let pipeline = Pipeline::with_hooks(router, hooks);
        (pipeline, bb)
    }

    /// Get all registered subsystem names.
    pub fn names(&self) -> Vec<&str> {
        self.subsystems.iter().map(|s| s.name()).collect()
    }

    /// Get the number of registered subsystems.
    pub fn len(&self) -> usize {
        self.subsystems.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.subsystems.is_empty()
    }

    /// Shutdown all subsystems.
    pub fn shutdown(&self) {
        for sub in &self.subsystems {
            log::info!("SubsystemRegistry: shutting down '{}'", sub.name());
            sub.shutdown();
        }
    }
}

impl Default for SubsystemRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::router::StepResult;
    use super::super::types::UnifiedStep;
    use crate::blackboard::a2a::AgentState;

    // A test subsystem implementing all hooks
    struct TestCrewSubsystem;

    impl Subsystem for TestCrewSubsystem {
        fn name(&self) -> &str {
            "test-crew"
        }

        fn domain(&self) -> StepDomain {
            StepDomain::Crew
        }

        fn version(&self) -> &str {
            "1.0.0"
        }

        fn step_handler(&self) -> Box<dyn StepHandler> {
            struct Handler;
            impl StepHandler for Handler {
                fn handle(&self, step: &mut UnifiedStep, bb: &mut Blackboard) -> StepResult {
                    step.mark_running();
                    bb.put_typed(
                        format!("crew:{}", step.sequence),
                        "crew-result".to_string(),
                        "crew",
                        &step.step_type,
                    );
                    step.mark_completed(serde_json::json!({"crew": true}));
                    Ok(())
                }
                fn domain(&self) -> StepDomain {
                    StepDomain::Crew
                }
            }
            Box::new(Handler)
        }

        fn init_blackboard(&self, bb: &mut Blackboard) {
            bb.put_typed("crew:config", "initialized".to_string(), "crew", "crew.init");
        }

        fn register_agents(&self, bb: &mut Blackboard) {
            bb.a2a.register("crew-researcher", "Researcher", "research", vec!["search".into()]);
            bb.a2a.set_state("crew-researcher", AgentState::Active);
        }
    }

    struct TestOcSubsystem;

    impl Subsystem for TestOcSubsystem {
        fn name(&self) -> &str {
            "test-openclaw"
        }

        fn domain(&self) -> StepDomain {
            StepDomain::OpenClaw
        }

        fn step_handler(&self) -> Box<dyn StepHandler> {
            struct Handler;
            impl StepHandler for Handler {
                fn handle(&self, step: &mut UnifiedStep, bb: &mut Blackboard) -> StepResult {
                    step.mark_running();
                    bb.put_typed(
                        format!("oc:{}", step.sequence),
                        "oc-result".to_string(),
                        "oc",
                        &step.step_type,
                    );
                    step.mark_completed(serde_json::json!({"oc": true}));
                    Ok(())
                }
                fn domain(&self) -> StepDomain {
                    StepDomain::OpenClaw
                }
            }
            Box::new(Handler)
        }

        fn register_agents(&self, bb: &mut Blackboard) {
            bb.a2a.register("oc-assistant", "Assistant", "chat", vec!["chat".into(), "search".into()]);
        }
    }

    #[test]
    fn test_subsystem_registry_build() {
        let mut registry = SubsystemRegistry::new();
        registry.register(Box::new(TestCrewSubsystem));
        registry.register(Box::new(TestOcSubsystem));

        assert_eq!(registry.len(), 2);
        assert_eq!(registry.names(), vec!["test-crew", "test-openclaw"]);

        let (pipeline, bb) = registry.build();

        // Router has both handlers
        assert!(pipeline.router().has_handler(StepDomain::Crew));
        assert!(pipeline.router().has_handler(StepDomain::OpenClaw));

        // Blackboard was initialized
        assert_eq!(
            bb.get_typed::<String>("crew:config").unwrap(),
            "initialized"
        );

        // A2A registry has agents from both subsystems
        assert_eq!(bb.a2a.len(), 2);
        assert!(bb.a2a.get("crew-researcher").is_some());
        assert!(bb.a2a.get("oc-assistant").is_some());
    }

    #[test]
    fn test_subsystem_full_pipeline_run() {
        use super::super::types::UnifiedExecution;

        let mut registry = SubsystemRegistry::new();
        registry.register(Box::new(TestCrewSubsystem));
        registry.register(Box::new(TestOcSubsystem));

        let (pipeline, mut bb) = registry.build();

        let mut exec = UnifiedExecution::new("cross-subsystem");
        exec.steps.push(UnifiedStep::new("e1", "crew.agent", "Research", 0));
        exec.steps.push(UnifiedStep::new("e1", "oc.channel.send", "Send", 1));

        pipeline.run_with_blackboard(&mut exec, &mut bb).unwrap();

        assert_eq!(exec.status, super::super::types::StepStatus::Completed);

        // Each subsystem wrote typed data
        assert_eq!(bb.get_typed::<String>("crew:0").unwrap(), "crew-result");
        assert_eq!(bb.get_typed::<String>("oc:1").unwrap(), "oc-result");

        // Pre-initialized data still there
        assert_eq!(bb.get_typed::<String>("crew:config").unwrap(), "initialized");

        // A2A agents still registered
        assert_eq!(bb.a2a.len(), 2);
        let searchers = bb.a2a.by_capability("search");
        assert_eq!(searchers.len(), 2); // Both crew-researcher and oc-assistant have search
    }

    #[test]
    fn test_subsystem_a2a_cross_discovery() {
        let mut registry = SubsystemRegistry::new();
        registry.register(Box::new(TestCrewSubsystem));
        registry.register(Box::new(TestOcSubsystem));

        let (_pipeline, bb) = registry.build();

        // OpenClaw subsystem can discover crew agents
        let crew_agents: Vec<&str> = bb
            .a2a
            .iter()
            .filter(|(_, a)| a.role == "research")
            .map(|(id, _)| id)
            .collect();
        assert_eq!(crew_agents, vec!["crew-researcher"]);

        // Crew subsystem can discover OpenClaw agents
        let oc_agents: Vec<&str> = bb
            .a2a
            .iter()
            .filter(|(_, a)| a.role == "chat")
            .map(|(id, _)| id)
            .collect();
        assert_eq!(oc_agents, vec!["oc-assistant"]);
    }
}
