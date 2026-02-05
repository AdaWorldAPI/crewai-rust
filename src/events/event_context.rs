//! Event context management for parent-child relationship tracking.
//!
//! Corresponds to `crewai/events/event_context.py`.
//!
//! Maintains a thread-local stack of `(event_id, event_type)` tuples that
//! allow the event bus to automatically assign `parent_event_id` and detect
//! mismatched start/end event pairs.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// MismatchBehavior
// ---------------------------------------------------------------------------

/// Behaviour when event start/end pairs do not match.
///
/// Corresponds to `crewai/events/event_context.py::MismatchBehavior`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MismatchBehavior {
    /// Log a warning.
    Warn,
    /// Raise (panic) on mismatch.
    Raise,
    /// Silently ignore the mismatch.
    Silent,
}

// ---------------------------------------------------------------------------
// EventContextConfig
// ---------------------------------------------------------------------------

/// Configuration for event context behaviour.
///
/// Corresponds to `crewai/events/event_context.py::EventContextConfig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContextConfig {
    /// Maximum allowed scope-stack depth. 0 means unlimited.
    pub max_stack_depth: usize,
    /// Behaviour on mismatched event pairs.
    pub mismatch_behavior: MismatchBehavior,
    /// Behaviour when popping from an empty stack.
    pub empty_pop_behavior: MismatchBehavior,
}

impl Default for EventContextConfig {
    fn default() -> Self {
        Self {
            max_stack_depth: 100,
            mismatch_behavior: MismatchBehavior::Warn,
            empty_pop_behavior: MismatchBehavior::Warn,
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Raised when stack depth limit is exceeded.
#[derive(Debug, thiserror::Error)]
#[error("Event stack depth limit ({limit}) exceeded. This usually indicates missing ending events.")]
pub struct StackDepthExceededError {
    /// The configured limit that was exceeded.
    pub limit: usize,
}

/// Raised when event start/end pairs do not match.
#[derive(Debug, thiserror::Error)]
#[error("Event pairing mismatch: {message}")]
pub struct EventPairingError {
    /// Human-readable description of the mismatch.
    pub message: String,
}

/// Raised when popping from an empty stack.
#[derive(Debug, thiserror::Error)]
#[error("Empty scope stack: {message}")]
pub struct EmptyStackError {
    /// Human-readable description.
    pub message: String,
}

// ---------------------------------------------------------------------------
// Thread-local state
// ---------------------------------------------------------------------------

thread_local! {
    /// Stack of `(event_id, event_type)` for hierarchical scope tracking.
    static EVENT_ID_STACK: RefCell<Vec<(String, String)>> = RefCell::new(Vec::new());

    /// Per-thread configuration override (falls back to DEFAULT_CONFIG).
    static EVENT_CONTEXT_CONFIG: RefCell<Option<EventContextConfig>> = const { RefCell::new(None) };

    /// The event_id of the most recently emitted event (linear chain).
    static LAST_EVENT_ID: RefCell<Option<String>> = const { RefCell::new(None) };

    /// The event_id that causally triggered the current execution context.
    static TRIGGERING_EVENT_ID: RefCell<Option<String>> = const { RefCell::new(None) };
}

static DEFAULT_CONFIG: Lazy<EventContextConfig> = Lazy::new(EventContextConfig::default);

fn with_config<R>(f: impl FnOnce(&EventContextConfig) -> R) -> R {
    EVENT_CONTEXT_CONFIG.with(|cell| {
        let borrow = cell.borrow();
        match borrow.as_ref() {
            Some(cfg) => f(cfg),
            None => f(&DEFAULT_CONFIG),
        }
    })
}

// ---------------------------------------------------------------------------
// Public API – scope stack
// ---------------------------------------------------------------------------

/// Get the current parent event ID from the top of the stack.
pub fn get_current_parent_id() -> Option<String> {
    EVENT_ID_STACK.with(|stack| {
        let s = stack.borrow();
        s.last().map(|(id, _)| id.clone())
    })
}

/// Get the parent of the current scope (`stack[-2]`).
pub fn get_enclosing_parent_id() -> Option<String> {
    EVENT_ID_STACK.with(|stack| {
        let s = stack.borrow();
        if s.len() >= 2 {
            Some(s[s.len() - 2].0.clone())
        } else {
            None
        }
    })
}

/// Push an event ID and type onto the scope stack.
pub fn push_event_scope(event_id: String, event_type: String) {
    let limit = with_config(|c| c.max_stack_depth);
    EVENT_ID_STACK.with(|stack| {
        let mut s = stack.borrow_mut();
        if limit > 0 && s.len() >= limit {
            panic!(
                "Event stack depth limit ({}) exceeded. This usually indicates missing ending events.",
                limit
            );
        }
        s.push((event_id, event_type));
    });
}

/// Pop an event entry from the scope stack.
///
/// Returns `Some((event_id, event_type))` or `None` if the stack is empty.
pub fn pop_event_scope() -> Option<(String, String)> {
    EVENT_ID_STACK.with(|stack| {
        let mut s = stack.borrow_mut();
        s.pop()
    })
}

/// Handle a pop attempt on an empty stack.
pub fn handle_empty_pop(event_type_name: &str) {
    let msg = format!(
        "Ending event '{}' emitted with empty scope stack. Missing starting event?",
        event_type_name
    );
    let behavior = with_config(|c| c.empty_pop_behavior);
    match behavior {
        MismatchBehavior::Raise => panic!("[CrewAIEventsBus] {}", msg),
        MismatchBehavior::Warn => log::warn!("[CrewAIEventsBus] Warning: {}", msg),
        MismatchBehavior::Silent => {}
    }
}

/// Handle a mismatched event pair.
pub fn handle_mismatch(event_type_name: &str, popped_type: &str, expected_start: &str) {
    let msg = format!(
        "Event pairing mismatch. '{}' closed '{}' (expected '{}')",
        event_type_name, popped_type, expected_start
    );
    let behavior = with_config(|c| c.mismatch_behavior);
    match behavior {
        MismatchBehavior::Raise => panic!("[CrewAIEventsBus] {}", msg),
        MismatchBehavior::Warn => log::warn!("[CrewAIEventsBus] Warning: {}", msg),
        MismatchBehavior::Silent => {}
    }
}

// ---------------------------------------------------------------------------
// Public API – linear chain tracking
// ---------------------------------------------------------------------------

/// Get the ID of the last emitted event for linear chain tracking.
pub fn get_last_event_id() -> Option<String> {
    LAST_EVENT_ID.with(|cell| cell.borrow().clone())
}

/// Reset the last event ID to `None`.
pub fn reset_last_event_id() {
    LAST_EVENT_ID.with(|cell| *cell.borrow_mut() = None);
}

/// Set the ID of the last emitted event.
pub fn set_last_event_id(event_id: String) {
    LAST_EVENT_ID.with(|cell| *cell.borrow_mut() = Some(event_id));
}

/// Get the ID of the event that triggered the current execution.
pub fn get_triggering_event_id() -> Option<String> {
    TRIGGERING_EVENT_ID.with(|cell| cell.borrow().clone())
}

/// Set the triggering event ID for causal chain tracking.
pub fn set_triggering_event_id(event_id: Option<String>) {
    TRIGGERING_EVENT_ID.with(|cell| *cell.borrow_mut() = event_id);
}

// ---------------------------------------------------------------------------
// Event scope guard (RAII equivalent of Python contextmanager)
// ---------------------------------------------------------------------------

/// RAII guard that pushes an event scope on creation and pops it on drop.
///
/// Corresponds to `crewai/events/event_context.py::event_scope`.
pub struct EventScopeGuard {
    owned: bool,
}

impl EventScopeGuard {
    /// Create a new scope guard, pushing `event_id` onto the stack if it is
    /// not already present.
    pub fn new(event_id: String, event_type: String) -> Self {
        let already = EVENT_ID_STACK.with(|stack| {
            let s = stack.borrow();
            s.iter().any(|(id, _)| *id == event_id)
        });
        if !already {
            push_event_scope(event_id, event_type);
        }
        Self { owned: !already }
    }
}

impl Drop for EventScopeGuard {
    fn drop(&mut self) {
        if self.owned {
            pop_event_scope();
        }
    }
}

/// RAII guard for triggered-by scope.
///
/// Corresponds to `crewai/events/event_context.py::triggered_by_scope`.
pub struct TriggeredByScopeGuard {
    previous: Option<String>,
}

impl TriggeredByScopeGuard {
    /// Enter a triggered-by scope.
    pub fn new(event_id: String) -> Self {
        let previous = get_triggering_event_id();
        set_triggering_event_id(Some(event_id));
        Self { previous }
    }
}

impl Drop for TriggeredByScopeGuard {
    fn drop(&mut self) {
        set_triggering_event_id(self.previous.take());
    }
}

// ---------------------------------------------------------------------------
// Scope-starting / scope-ending event sets & valid pairs
// ---------------------------------------------------------------------------

/// Set of event type names that start a new scope.
pub static SCOPE_STARTING_EVENTS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert("flow_started");
    s.insert("method_execution_started");
    s.insert("crew_kickoff_started");
    s.insert("crew_train_started");
    s.insert("crew_test_started");
    s.insert("agent_execution_started");
    s.insert("agent_evaluation_started");
    s.insert("lite_agent_execution_started");
    s.insert("task_started");
    s.insert("llm_call_started");
    s.insert("llm_guardrail_started");
    s.insert("tool_usage_started");
    s.insert("mcp_connection_started");
    s.insert("mcp_tool_execution_started");
    s.insert("memory_retrieval_started");
    s.insert("memory_save_started");
    s.insert("memory_query_started");
    s.insert("knowledge_query_started");
    s.insert("knowledge_search_query_started");
    s.insert("a2a_delegation_started");
    s.insert("a2a_conversation_started");
    s.insert("a2a_server_task_started");
    s.insert("a2a_parallel_delegation_started");
    s.insert("agent_reasoning_started");
    s
});

/// Set of event type names that end a scope.
pub static SCOPE_ENDING_EVENTS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert("flow_finished");
    s.insert("flow_paused");
    s.insert("method_execution_finished");
    s.insert("method_execution_failed");
    s.insert("method_execution_paused");
    s.insert("crew_kickoff_completed");
    s.insert("crew_kickoff_failed");
    s.insert("crew_train_completed");
    s.insert("crew_train_failed");
    s.insert("crew_test_completed");
    s.insert("crew_test_failed");
    s.insert("agent_execution_completed");
    s.insert("agent_execution_error");
    s.insert("agent_evaluation_completed");
    s.insert("agent_evaluation_failed");
    s.insert("lite_agent_execution_completed");
    s.insert("lite_agent_execution_error");
    s.insert("task_completed");
    s.insert("task_failed");
    s.insert("llm_call_completed");
    s.insert("llm_call_failed");
    s.insert("llm_guardrail_completed");
    s.insert("llm_guardrail_failed");
    s.insert("tool_usage_finished");
    s.insert("tool_usage_error");
    s.insert("mcp_connection_completed");
    s.insert("mcp_connection_failed");
    s.insert("mcp_tool_execution_completed");
    s.insert("mcp_tool_execution_failed");
    s.insert("memory_retrieval_completed");
    s.insert("memory_retrieval_failed");
    s.insert("memory_save_completed");
    s.insert("memory_save_failed");
    s.insert("memory_query_completed");
    s.insert("memory_query_failed");
    s.insert("knowledge_query_completed");
    s.insert("knowledge_query_failed");
    s.insert("knowledge_search_query_completed");
    s.insert("knowledge_search_query_failed");
    s.insert("a2a_delegation_completed");
    s.insert("a2a_conversation_completed");
    s.insert("a2a_server_task_completed");
    s.insert("a2a_server_task_canceled");
    s.insert("a2a_server_task_failed");
    s.insert("a2a_parallel_delegation_completed");
    s.insert("agent_reasoning_completed");
    s.insert("agent_reasoning_failed");
    s
});

/// Mapping from ending event type name to its expected starting event type name.
pub static VALID_EVENT_PAIRS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("flow_finished", "flow_started");
    m.insert("flow_paused", "flow_started");
    m.insert("method_execution_finished", "method_execution_started");
    m.insert("method_execution_failed", "method_execution_started");
    m.insert("method_execution_paused", "method_execution_started");
    m.insert("crew_kickoff_completed", "crew_kickoff_started");
    m.insert("crew_kickoff_failed", "crew_kickoff_started");
    m.insert("crew_train_completed", "crew_train_started");
    m.insert("crew_train_failed", "crew_train_started");
    m.insert("crew_test_completed", "crew_test_started");
    m.insert("crew_test_failed", "crew_test_started");
    m.insert("agent_execution_completed", "agent_execution_started");
    m.insert("agent_execution_error", "agent_execution_started");
    m.insert("agent_evaluation_completed", "agent_evaluation_started");
    m.insert("agent_evaluation_failed", "agent_evaluation_started");
    m.insert("lite_agent_execution_completed", "lite_agent_execution_started");
    m.insert("lite_agent_execution_error", "lite_agent_execution_started");
    m.insert("task_completed", "task_started");
    m.insert("task_failed", "task_started");
    m.insert("llm_call_completed", "llm_call_started");
    m.insert("llm_call_failed", "llm_call_started");
    m.insert("llm_guardrail_completed", "llm_guardrail_started");
    m.insert("llm_guardrail_failed", "llm_guardrail_started");
    m.insert("tool_usage_finished", "tool_usage_started");
    m.insert("tool_usage_error", "tool_usage_started");
    m.insert("mcp_connection_completed", "mcp_connection_started");
    m.insert("mcp_connection_failed", "mcp_connection_started");
    m.insert("mcp_tool_execution_completed", "mcp_tool_execution_started");
    m.insert("mcp_tool_execution_failed", "mcp_tool_execution_started");
    m.insert("memory_retrieval_completed", "memory_retrieval_started");
    m.insert("memory_retrieval_failed", "memory_retrieval_started");
    m.insert("memory_save_completed", "memory_save_started");
    m.insert("memory_save_failed", "memory_save_started");
    m.insert("memory_query_completed", "memory_query_started");
    m.insert("memory_query_failed", "memory_query_started");
    m.insert("knowledge_query_completed", "knowledge_query_started");
    m.insert("knowledge_query_failed", "knowledge_query_started");
    m.insert("knowledge_search_query_completed", "knowledge_search_query_started");
    m.insert("knowledge_search_query_failed", "knowledge_search_query_started");
    m.insert("a2a_delegation_completed", "a2a_delegation_started");
    m.insert("a2a_conversation_completed", "a2a_conversation_started");
    m.insert("a2a_server_task_completed", "a2a_server_task_started");
    m.insert("a2a_server_task_canceled", "a2a_server_task_started");
    m.insert("a2a_server_task_failed", "a2a_server_task_started");
    m.insert("a2a_parallel_delegation_completed", "a2a_parallel_delegation_started");
    m.insert("agent_reasoning_completed", "agent_reasoning_started");
    m.insert("agent_reasoning_failed", "agent_reasoning_started");
    m
});
