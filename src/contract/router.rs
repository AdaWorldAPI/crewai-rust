//! Typed step router — prefix-based dispatch without runtime string matching.
//!
//! Instead of `if step.is_crew() { ... } else if step.is_n8n() { ... }`,
//! subsystems register as [`StepHandler`] implementations with the
//! [`StepRouter`]. The router parses the prefix once and dispatches via
//! a `HashMap<StepDomain, Box<dyn StepHandler>>` — O(1) lookup, zero string
//! comparisons at execution time.
//!
//! # One-Binary Design
//!
//! All handlers live in the same process. The router holds `Box<dyn StepHandler>`
//! which the borrow checker treats as owned objects — no IPC, no serialization.
//! The handler receives `&mut Blackboard` for zero-serde data flow.
//!
//! # Example
//!
//! ```
//! use crewai::contract::router::{StepDomain, StepRouter, StepHandler, StepResult};
//! use crewai::blackboard::Blackboard;
//! use crewai::contract::types::UnifiedStep;
//!
//! struct MyCrewHandler;
//!
//! impl StepHandler for MyCrewHandler {
//!     fn handle(&self, step: &mut UnifiedStep, bb: &mut Blackboard) -> StepResult {
//!         step.mark_completed(serde_json::json!({"done": true}));
//!         Ok(())
//!     }
//!     fn domain(&self) -> StepDomain { StepDomain::Crew }
//! }
//!
//! let mut router = StepRouter::new();
//! router.register(Box::new(MyCrewHandler));
//! assert!(router.has_handler(StepDomain::Crew));
//! ```

use std::collections::HashMap;

use super::types::UnifiedStep;
use crate::blackboard::Blackboard;

// ---------------------------------------------------------------------------
// StepDomain — typed routing prefix
// ---------------------------------------------------------------------------

/// Typed routing domain — replaces runtime `starts_with("crew.")` checks.
///
/// Each domain maps 1:1 to a step_type prefix. The router parses the prefix
/// once via [`StepDomain::from_step_type`] and uses it as a HashMap key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepDomain {
    /// `crew.*` — crewai-rust agent orchestration.
    Crew,
    /// `lb.*` — ladybug-rs cognitive database.
    Ladybug,
    /// `n8n.*` — n8n-rs workflow automation.
    N8n,
    /// `oc.*` — openclaw-rs multi-channel assistant.
    OpenClaw,
    /// `chess.*` — chess engine (when enabled).
    Chess,
}

impl StepDomain {
    /// Parse a step_type string into a domain.
    ///
    /// Returns `None` for unrecognized prefixes. This is the single
    /// point where string matching happens — after this, dispatch is
    /// by enum variant (O(1) HashMap lookup).
    pub fn from_step_type(step_type: &str) -> Option<Self> {
        let prefix = step_type.split('.').next()?;
        match prefix {
            "crew" => Some(Self::Crew),
            "lb" => Some(Self::Ladybug),
            "n8n" => Some(Self::N8n),
            "oc" => Some(Self::OpenClaw),
            "chess" => Some(Self::Chess),
            _ => None,
        }
    }

    /// Get the string prefix for this domain.
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Crew => "crew",
            Self::Ladybug => "lb",
            Self::N8n => "n8n",
            Self::OpenClaw => "oc",
            Self::Chess => "chess",
        }
    }

    /// Get the sub-type (everything after the prefix dot).
    ///
    /// For `oc.channel.receive`, returns `"channel.receive"`.
    pub fn sub_type(step_type: &str) -> &str {
        match step_type.find('.') {
            Some(pos) => &step_type[pos + 1..],
            None => step_type,
        }
    }
}

impl std::fmt::Display for StepDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.prefix())
    }
}

// ---------------------------------------------------------------------------
// StepHandler trait
// ---------------------------------------------------------------------------

/// Result type for step handlers.
pub type StepResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Trait for subsystems that handle steps of a particular domain.
///
/// Implement this for each subsystem (crew engine, ladybug, n8n, openclaw).
/// The handler receives `&mut Blackboard` for zero-serde data flow —
/// no serialization needed since everything is in one binary.
///
/// # Contract
///
/// - The handler MUST update `step.status` (mark_running → mark_completed or mark_failed).
/// - The handler SHOULD write its output to the blackboard, not just to `step.output`.
/// - The handler MAY read inputs from the blackboard (written by previous phases).
pub trait StepHandler: Send + Sync {
    /// Execute a step within the given blackboard context.
    ///
    /// The step is passed mutably so the handler can update its status,
    /// output, reasoning, and confidence fields.
    fn handle(&self, step: &mut UnifiedStep, bb: &mut Blackboard) -> StepResult;

    /// Which domain this handler serves.
    fn domain(&self) -> StepDomain;

    /// Human-readable name for logging.
    fn name(&self) -> &str {
        self.domain().prefix()
    }
}

// ---------------------------------------------------------------------------
// StepRouter
// ---------------------------------------------------------------------------

/// The step router dispatches steps to registered handlers by domain.
///
/// Prefix is parsed once via [`StepDomain::from_step_type`], then the
/// handler is looked up via O(1) HashMap access. No repeated string
/// comparisons.
pub struct StepRouter {
    handlers: HashMap<StepDomain, Box<dyn StepHandler>>,
}

impl StepRouter {
    /// Create a new empty router.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for its declared domain.
    ///
    /// Replaces any previously registered handler for that domain.
    pub fn register(&mut self, handler: Box<dyn StepHandler>) {
        let domain = handler.domain();
        self.handlers.insert(domain, handler);
    }

    /// Check if a handler is registered for a domain.
    pub fn has_handler(&self, domain: StepDomain) -> bool {
        self.handlers.contains_key(&domain)
    }

    /// Dispatch a step to the appropriate handler.
    ///
    /// Returns an error if no handler is registered for the step's domain,
    /// or if the handler itself returns an error.
    pub fn dispatch(
        &self,
        step: &mut UnifiedStep,
        bb: &mut Blackboard,
    ) -> StepResult {
        let domain = StepDomain::from_step_type(&step.step_type).ok_or_else(|| {
            format!(
                "Unknown step domain for step_type '{}'. Known domains: crew, lb, n8n, oc, chess",
                step.step_type
            )
        })?;

        let handler = self.handlers.get(&domain).ok_or_else(|| {
            format!(
                "No handler registered for domain '{}' (step_type: '{}')",
                domain, step.step_type
            )
        })?;

        log::debug!(
            "Router: dispatching step '{}' ({}) to handler '{}'",
            step.name,
            step.step_type,
            handler.name(),
        );

        handler.handle(step, bb)
    }

    /// Dispatch all steps in an execution sequentially.
    ///
    /// Stops at the first failure. Each step gets a fresh phase in the
    /// blackboard trace.
    pub fn dispatch_all(
        &self,
        steps: &mut [UnifiedStep],
        bb: &mut Blackboard,
    ) -> StepResult {
        for step in steps.iter_mut() {
            if step.status != super::types::StepStatus::Pending {
                continue; // Skip already-processed steps
            }
            self.dispatch(step, bb)?;
        }
        Ok(())
    }

    /// Get all registered domains.
    pub fn registered_domains(&self) -> Vec<StepDomain> {
        self.handlers.keys().copied().collect()
    }
}

impl Default for StepRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for StepRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StepRouter")
            .field("handlers", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // A simple test handler that marks steps completed
    struct TestHandler {
        domain: StepDomain,
    }

    impl StepHandler for TestHandler {
        fn handle(&self, step: &mut UnifiedStep, bb: &mut Blackboard) -> StepResult {
            step.mark_running();
            // Write output to blackboard (zero-serde)
            let key = format!("{}:{}", step.step_type, step.sequence);
            bb.put_typed(key, format!("output from {}", step.name), &step.step_type, &step.step_type);
            step.mark_completed(serde_json::json!({"handled": true}));
            Ok(())
        }

        fn domain(&self) -> StepDomain {
            self.domain
        }
    }

    // A handler that always fails
    struct FailHandler;

    impl StepHandler for FailHandler {
        fn handle(&self, step: &mut UnifiedStep, _bb: &mut Blackboard) -> StepResult {
            step.mark_running();
            step.mark_failed("Intentional failure");
            Err("Intentional failure".into())
        }

        fn domain(&self) -> StepDomain {
            StepDomain::N8n
        }
    }

    #[test]
    fn test_step_domain_from_step_type() {
        assert_eq!(StepDomain::from_step_type("crew.agent"), Some(StepDomain::Crew));
        assert_eq!(StepDomain::from_step_type("lb.resonate"), Some(StepDomain::Ladybug));
        assert_eq!(StepDomain::from_step_type("n8n.set"), Some(StepDomain::N8n));
        assert_eq!(StepDomain::from_step_type("oc.channel.receive"), Some(StepDomain::OpenClaw));
        assert_eq!(StepDomain::from_step_type("chess.evaluate"), Some(StepDomain::Chess));
        assert_eq!(StepDomain::from_step_type("unknown.thing"), None);
        assert_eq!(StepDomain::from_step_type(""), None);
    }

    #[test]
    fn test_step_domain_sub_type() {
        assert_eq!(StepDomain::sub_type("oc.channel.receive"), "channel.receive");
        assert_eq!(StepDomain::sub_type("crew.agent"), "agent");
        assert_eq!(StepDomain::sub_type("lb.resonate"), "resonate");
    }

    #[test]
    fn test_step_domain_prefix() {
        assert_eq!(StepDomain::Crew.prefix(), "crew");
        assert_eq!(StepDomain::OpenClaw.prefix(), "oc");
        assert_eq!(StepDomain::Ladybug.prefix(), "lb");
    }

    #[test]
    fn test_router_register_and_dispatch() {
        let mut router = StepRouter::new();
        router.register(Box::new(TestHandler { domain: StepDomain::Crew }));
        router.register(Box::new(TestHandler { domain: StepDomain::OpenClaw }));

        assert!(router.has_handler(StepDomain::Crew));
        assert!(router.has_handler(StepDomain::OpenClaw));
        assert!(!router.has_handler(StepDomain::N8n));

        let mut bb = Blackboard::new();
        let mut step = UnifiedStep::new("e1", "crew.agent", "Research", 0);

        router.dispatch(&mut step, &mut bb).unwrap();
        assert_eq!(step.status, super::super::types::StepStatus::Completed);

        // Check that the handler wrote to the blackboard
        let output = bb.get_typed::<String>("crew.agent:0").unwrap();
        assert_eq!(output, "output from Research");
    }

    #[test]
    fn test_router_unknown_domain() {
        let router = StepRouter::new();
        let mut bb = Blackboard::new();
        let mut step = UnifiedStep::new("e1", "alien.probe", "Probe", 0);

        let result = router.dispatch(&mut step, &mut bb);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown step domain"));
    }

    #[test]
    fn test_router_no_handler() {
        let router = StepRouter::new();
        let mut bb = Blackboard::new();
        let mut step = UnifiedStep::new("e1", "crew.agent", "Research", 0);

        let result = router.dispatch(&mut step, &mut bb);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No handler registered"));
    }

    #[test]
    fn test_router_dispatch_all() {
        let mut router = StepRouter::new();
        router.register(Box::new(TestHandler { domain: StepDomain::Crew }));
        router.register(Box::new(TestHandler { domain: StepDomain::OpenClaw }));

        let mut bb = Blackboard::new();
        let mut steps = vec![
            UnifiedStep::new("e1", "crew.agent", "Research", 0),
            UnifiedStep::new("e1", "oc.channel.send", "Send", 1),
        ];

        router.dispatch_all(&mut steps, &mut bb).unwrap();

        assert_eq!(steps[0].status, super::super::types::StepStatus::Completed);
        assert_eq!(steps[1].status, super::super::types::StepStatus::Completed);
        assert_eq!(bb.total_len(), 2);
    }

    #[test]
    fn test_router_dispatch_all_stops_on_failure() {
        let mut router = StepRouter::new();
        router.register(Box::new(FailHandler)); // n8n handler that fails
        router.register(Box::new(TestHandler { domain: StepDomain::Crew }));

        let mut bb = Blackboard::new();
        let mut steps = vec![
            UnifiedStep::new("e1", "n8n.set", "Set", 0),
            UnifiedStep::new("e1", "crew.agent", "Research", 1),
        ];

        let result = router.dispatch_all(&mut steps, &mut bb);
        assert!(result.is_err());

        // First step failed
        assert_eq!(steps[0].status, super::super::types::StepStatus::Failed);
        // Second step never ran
        assert_eq!(steps[1].status, super::super::types::StepStatus::Pending);
    }

    #[test]
    fn test_router_skips_non_pending() {
        let mut router = StepRouter::new();
        router.register(Box::new(TestHandler { domain: StepDomain::Crew }));

        let mut bb = Blackboard::new();
        let mut already_done = UnifiedStep::new("e1", "crew.agent", "Done", 0);
        already_done.mark_completed(serde_json::json!({"already": true}));

        let mut pending = UnifiedStep::new("e1", "crew.agent", "New", 1);
        let mut steps = vec![already_done, pending];

        router.dispatch_all(&mut steps, &mut bb).unwrap();

        // First step kept its original completed state
        assert_eq!(steps[0].output["already"], true);
        // Second step was processed
        assert_eq!(steps[1].status, super::super::types::StepStatus::Completed);
    }
}
