//! Lifecycle hook traits for cross-system integration.
//!
//! These traits allow external systems (openclaw-rs, n8n-rs, ladybug-rs) to
//! observe and intercept crewai-rust agent activity without tight coupling.
//!
//! # Design Principles
//!
//! - All methods have default no-op implementations → implementors pick what they need.
//! - `&self` receivers → hooks are shared-immutable, interior mutability via atomics/channels.
//! - `Send + Sync + 'static` bounds → hooks can be registered once and used from any task.
//! - Return types carry `Action` enums to allow interception (approve/deny/modify).

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use crate::llms::base_llm::LLMMessage;

// ---------------------------------------------------------------------------
// Action enums (returned by hooks to control execution flow)
// ---------------------------------------------------------------------------

/// What should happen after a tool call request is intercepted.
#[derive(Debug, Clone)]
pub enum ToolAction {
    /// Allow the tool call to proceed.
    Allow,
    /// Deny the tool call with a reason (returned to the agent as a tool error).
    Deny(String),
    /// Modify the tool arguments before proceeding.
    Modify(Value),
    /// Require human approval before proceeding (async flow).
    RequireApproval,
}

/// What should happen after a step routing decision is intercepted.
#[derive(Debug, Clone)]
pub enum StepAction {
    /// Continue with the step as planned.
    Continue,
    /// Skip this step entirely.
    Skip,
    /// Replace the step input with modified data.
    ReplaceInput(Value),
}

// ---------------------------------------------------------------------------
// AgentHook — observe/intercept agent thinking and tool use
// ---------------------------------------------------------------------------

/// Hook trait for observing agent lifecycle events.
///
/// Implement this to integrate with the crewai-rust agent pipeline.
/// All methods have default no-op implementations.
///
/// # Example
///
/// ```ignore
/// struct OpenClawAgentHook { /* ... */ }
///
/// impl AgentHook for OpenClawAgentHook {
///     fn before_think(&self, agent_id: &str, messages: &[LLMMessage]) -> Result<(), HookError> {
///         // Log to channel, update typing indicator, etc.
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait AgentHook: Send + Sync + 'static {
    /// Called before the agent sends messages to the LLM.
    ///
    /// Can be used to:
    /// - Update typing indicators on channels
    /// - Log the conversation state
    /// - Inject additional context
    fn before_think(
        &self,
        _agent_id: &str,
        _messages: &[LLMMessage],
    ) -> Result<(), HookError> {
        Ok(())
    }

    /// Called after the agent receives an LLM response.
    ///
    /// The response string can be observed (not modified — use a guardrail for that).
    fn after_think(
        &self,
        _agent_id: &str,
        _response: &str,
    ) -> Result<(), HookError> {
        Ok(())
    }

    /// Called when the agent requests a tool call.
    ///
    /// Returns a `ToolAction` to allow, deny, or modify the call.
    /// This is the integration point for openclaw-rs execution approval workflows.
    fn on_tool_request(
        &self,
        _agent_id: &str,
        _tool_name: &str,
        _tool_args: &Value,
    ) -> Result<ToolAction, HookError> {
        Ok(ToolAction::Allow)
    }

    /// Called after a tool call completes (success or failure).
    fn after_tool_call(
        &self,
        _agent_id: &str,
        _tool_name: &str,
        _result: &Value,
        _success: bool,
    ) -> Result<(), HookError> {
        Ok(())
    }

    /// Called when an agent starts executing a task.
    fn on_task_start(
        &self,
        _agent_id: &str,
        _task_description: &str,
    ) -> Result<(), HookError> {
        Ok(())
    }

    /// Called when an agent finishes a task.
    fn on_task_complete(
        &self,
        _agent_id: &str,
        _task_description: &str,
        _output: &str,
    ) -> Result<(), HookError> {
        Ok(())
    }

    /// Called when an agent delegates to another agent.
    fn on_delegation(
        &self,
        _from_agent: &str,
        _to_agent: &str,
        _task_description: &str,
    ) -> Result<(), HookError> {
        Ok(())
    }

    /// Called when a streaming chunk is produced.
    ///
    /// This is the integration point for openclaw-rs block-streaming
    /// and live-edit channel support.
    fn on_stream_chunk(
        &self,
        _agent_id: &str,
        _chunk: &str,
        _is_final: bool,
    ) -> Result<(), HookError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MemoryHook — observe memory store/recall operations
// ---------------------------------------------------------------------------

/// Hook trait for observing memory operations.
///
/// Implement this to bridge crewai-rust memory with external systems
/// (e.g., ladybug-rs cognitive substrate, openclaw-rs memory system).
#[async_trait]
pub trait MemoryHook: Send + Sync + 'static {
    /// Called when a memory entry is stored.
    fn on_store(
        &self,
        _category: &str,
        _content: &str,
        _metadata: &HashMap<String, Value>,
    ) -> Result<(), HookError> {
        Ok(())
    }

    /// Called when memories are recalled.
    fn on_recall(
        &self,
        _query: &str,
        _results: &[Value],
    ) -> Result<(), HookError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// StepHook — observe/intercept unified step execution
// ---------------------------------------------------------------------------

/// Hook trait for observing unified execution step lifecycle.
///
/// Implement this to bridge crewai-rust with n8n-rs workflow engine
/// or openclaw-rs hook system.
#[async_trait]
pub trait StepHook: Send + Sync + 'static {
    /// Called before a step begins execution.
    ///
    /// Returns a `StepAction` to control whether the step proceeds.
    fn before_step(
        &self,
        _step_type: &str,
        _step_name: &str,
        _input: &Value,
    ) -> Result<StepAction, HookError> {
        Ok(StepAction::Continue)
    }

    /// Called after a step completes (success or failure).
    fn after_step(
        &self,
        _step_type: &str,
        _step_name: &str,
        _output: &Value,
        _success: bool,
    ) -> Result<(), HookError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ModelHook — observe model provider operations
// ---------------------------------------------------------------------------

/// Hook trait for observing LLM provider operations.
///
/// Implement this for usage tracking, cost monitoring, and model failover
/// in openclaw-rs.
#[async_trait]
pub trait ModelHook: Send + Sync + 'static {
    /// Called before an LLM API call is made.
    fn before_llm_call(
        &self,
        _model: &str,
        _provider: &str,
        _message_count: usize,
    ) -> Result<(), HookError> {
        Ok(())
    }

    /// Called after an LLM API call completes.
    fn after_llm_call(
        &self,
        _model: &str,
        _provider: &str,
        _prompt_tokens: i64,
        _completion_tokens: i64,
        _success: bool,
    ) -> Result<(), HookError> {
        Ok(())
    }

    /// Called when a model provider fails and failover is attempted.
    fn on_provider_failover(
        &self,
        _from_provider: &str,
        _to_provider: &str,
        _reason: &str,
    ) -> Result<(), HookError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// HookError
// ---------------------------------------------------------------------------

/// Error type for hook invocations.
///
/// Hooks should not panic — they return `HookError` to signal problems.
/// The engine logs hook errors but continues execution (hooks are advisory).
#[derive(Debug, Clone)]
pub struct HookError {
    pub message: String,
}

impl HookError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HookError: {}", self.message)
    }
}

impl std::error::Error for HookError {}

impl From<String> for HookError {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for HookError {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

// ---------------------------------------------------------------------------
// HookRegistry — store registered hooks
// ---------------------------------------------------------------------------

/// Registry for managing lifecycle hooks.
///
/// Multiple hooks can be registered for each trait. They are invoked in
/// registration order. All hooks see the same data (no short-circuiting
/// except for `ToolAction::Deny`).
pub struct HookRegistry {
    pub agent_hooks: Vec<Box<dyn AgentHook>>,
    pub memory_hooks: Vec<Box<dyn MemoryHook>>,
    pub step_hooks: Vec<Box<dyn StepHook>>,
    pub model_hooks: Vec<Box<dyn ModelHook>>,
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl HookRegistry {
    /// Create an empty hook registry.
    pub fn new() -> Self {
        Self {
            agent_hooks: Vec::new(),
            memory_hooks: Vec::new(),
            step_hooks: Vec::new(),
            model_hooks: Vec::new(),
        }
    }

    /// Register an agent hook.
    pub fn register_agent_hook(&mut self, hook: impl AgentHook) {
        self.agent_hooks.push(Box::new(hook));
    }

    /// Register a memory hook.
    pub fn register_memory_hook(&mut self, hook: impl MemoryHook) {
        self.memory_hooks.push(Box::new(hook));
    }

    /// Register a step hook.
    pub fn register_step_hook(&mut self, hook: impl StepHook) {
        self.step_hooks.push(Box::new(hook));
    }

    /// Register a model hook.
    pub fn register_model_hook(&mut self, hook: impl ModelHook) {
        self.model_hooks.push(Box::new(hook));
    }

    /// Invoke all agent hooks: before_think.
    pub fn invoke_before_think(
        &self,
        agent_id: &str,
        messages: &[LLMMessage],
    ) {
        for hook in &self.agent_hooks {
            if let Err(e) = hook.before_think(agent_id, messages) {
                log::warn!("AgentHook.before_think error: {}", e);
            }
        }
    }

    /// Invoke all agent hooks: after_think.
    pub fn invoke_after_think(
        &self,
        agent_id: &str,
        response: &str,
    ) {
        for hook in &self.agent_hooks {
            if let Err(e) = hook.after_think(agent_id, response) {
                log::warn!("AgentHook.after_think error: {}", e);
            }
        }
    }

    /// Invoke all agent hooks: on_tool_request.
    ///
    /// Returns the most restrictive action: if any hook returns Deny, the
    /// overall result is Deny. RequireApproval takes precedence over Allow.
    pub fn invoke_on_tool_request(
        &self,
        agent_id: &str,
        tool_name: &str,
        tool_args: &Value,
    ) -> ToolAction {
        let mut result = ToolAction::Allow;
        for hook in &self.agent_hooks {
            match hook.on_tool_request(agent_id, tool_name, tool_args) {
                Ok(ToolAction::Deny(reason)) => return ToolAction::Deny(reason),
                Ok(ToolAction::RequireApproval) => result = ToolAction::RequireApproval,
                Ok(ToolAction::Modify(args)) => {
                    if matches!(result, ToolAction::Allow) {
                        result = ToolAction::Modify(args);
                    }
                }
                Ok(ToolAction::Allow) => {}
                Err(e) => {
                    log::warn!("AgentHook.on_tool_request error: {}", e);
                }
            }
        }
        result
    }

    /// Invoke all agent hooks: on_stream_chunk.
    pub fn invoke_on_stream_chunk(
        &self,
        agent_id: &str,
        chunk: &str,
        is_final: bool,
    ) {
        for hook in &self.agent_hooks {
            if let Err(e) = hook.on_stream_chunk(agent_id, chunk, is_final) {
                log::warn!("AgentHook.on_stream_chunk error: {}", e);
            }
        }
    }

    /// Invoke all step hooks: before_step.
    ///
    /// Returns Skip if any hook returns Skip.
    pub fn invoke_before_step(
        &self,
        step_type: &str,
        step_name: &str,
        input: &Value,
    ) -> StepAction {
        for hook in &self.step_hooks {
            match hook.before_step(step_type, step_name, input) {
                Ok(StepAction::Skip) => return StepAction::Skip,
                Ok(StepAction::ReplaceInput(v)) => return StepAction::ReplaceInput(v),
                Ok(StepAction::Continue) => {}
                Err(e) => {
                    log::warn!("StepHook.before_step error: {}", e);
                }
            }
        }
        StepAction::Continue
    }

    /// Invoke all model hooks: before_llm_call.
    pub fn invoke_before_llm_call(
        &self,
        model: &str,
        provider: &str,
        message_count: usize,
    ) {
        for hook in &self.model_hooks {
            if let Err(e) = hook.before_llm_call(model, provider, message_count) {
                log::warn!("ModelHook.before_llm_call error: {}", e);
            }
        }
    }

    /// Invoke all model hooks: after_llm_call.
    pub fn invoke_after_llm_call(
        &self,
        model: &str,
        provider: &str,
        prompt_tokens: i64,
        completion_tokens: i64,
        success: bool,
    ) {
        for hook in &self.model_hooks {
            if let Err(e) = hook.after_llm_call(model, provider, prompt_tokens, completion_tokens, success) {
                log::warn!("ModelHook.after_llm_call error: {}", e);
            }
        }
    }
}

impl std::fmt::Debug for HookRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookRegistry")
            .field("agent_hooks", &self.agent_hooks.len())
            .field("memory_hooks", &self.memory_hooks.len())
            .field("step_hooks", &self.step_hooks.len())
            .field("model_hooks", &self.model_hooks.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    struct TestAgentHook {
        deny_tool: Option<String>,
    }

    impl AgentHook for TestAgentHook {
        fn on_tool_request(
            &self,
            _agent_id: &str,
            tool_name: &str,
            _tool_args: &Value,
        ) -> Result<ToolAction, HookError> {
            if let Some(ref denied) = self.deny_tool {
                if tool_name == denied {
                    return Ok(ToolAction::Deny("Blocked by test hook".into()));
                }
            }
            Ok(ToolAction::Allow)
        }
    }

    struct NoOpAgentHook;
    impl AgentHook for NoOpAgentHook {}

    struct NoOpMemoryHook;
    impl MemoryHook for NoOpMemoryHook {}

    struct NoOpStepHook;
    impl StepHook for NoOpStepHook {}

    struct NoOpModelHook;
    impl ModelHook for NoOpModelHook {}

    #[test]
    fn test_hook_registry_new() {
        let registry = HookRegistry::new();
        assert_eq!(registry.agent_hooks.len(), 0);
        assert_eq!(registry.memory_hooks.len(), 0);
        assert_eq!(registry.step_hooks.len(), 0);
        assert_eq!(registry.model_hooks.len(), 0);
    }

    #[test]
    fn test_hook_registry_register() {
        let mut registry = HookRegistry::new();
        registry.register_agent_hook(NoOpAgentHook);
        registry.register_memory_hook(NoOpMemoryHook);
        registry.register_step_hook(NoOpStepHook);
        registry.register_model_hook(NoOpModelHook);
        assert_eq!(registry.agent_hooks.len(), 1);
        assert_eq!(registry.memory_hooks.len(), 1);
        assert_eq!(registry.step_hooks.len(), 1);
        assert_eq!(registry.model_hooks.len(), 1);
    }

    #[test]
    fn test_tool_action_deny() {
        let mut registry = HookRegistry::new();
        registry.register_agent_hook(TestAgentHook {
            deny_tool: Some("dangerous_tool".into()),
        });

        let action = registry.invoke_on_tool_request(
            "agent-1",
            "dangerous_tool",
            &Value::Null,
        );
        assert!(matches!(action, ToolAction::Deny(_)));

        let action = registry.invoke_on_tool_request(
            "agent-1",
            "safe_tool",
            &Value::Null,
        );
        assert!(matches!(action, ToolAction::Allow));
    }

    #[test]
    fn test_before_think_no_panic() {
        let mut registry = HookRegistry::new();
        registry.register_agent_hook(NoOpAgentHook);
        registry.invoke_before_think("agent-1", &[]);
    }

    #[test]
    fn test_after_think_no_panic() {
        let mut registry = HookRegistry::new();
        registry.register_agent_hook(NoOpAgentHook);
        registry.invoke_after_think("agent-1", "some response");
    }

    #[test]
    fn test_before_step_skip() {
        struct SkipStep;
        impl StepHook for SkipStep {
            fn before_step(
                &self,
                _step_type: &str,
                _step_name: &str,
                _input: &Value,
            ) -> Result<StepAction, HookError> {
                Ok(StepAction::Skip)
            }
        }

        let mut registry = HookRegistry::new();
        registry.register_step_hook(SkipStep);
        let action = registry.invoke_before_step("oc.channel.receive", "receive", &Value::Null);
        assert!(matches!(action, StepAction::Skip));
    }

    #[test]
    fn test_hook_error() {
        let err = HookError::new("test error");
        assert_eq!(err.to_string(), "HookError: test error");

        let err: HookError = "from str".into();
        assert_eq!(err.message, "from str");

        let err: HookError = String::from("from string").into();
        assert_eq!(err.message, "from string");
    }

    #[test]
    fn test_hook_registry_debug() {
        let registry = HookRegistry::new();
        let debug = format!("{:?}", registry);
        assert!(debug.contains("HookRegistry"));
        assert!(debug.contains("agent_hooks: 0"));
    }
}
