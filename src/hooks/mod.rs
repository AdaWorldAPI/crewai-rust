//! Hooks module for intercepting LLM and tool calls.
//!
//! Corresponds to `crewai/hooks/`.
//!
//! Provides context structs and global hook registries for before/after
//! interception of LLM calls and tool invocations.
//!
//! # Lifecycle Hooks
//!
//! The [`lifecycle`] submodule provides trait-based hooks for cross-system
//! integration (openclaw-rs, n8n-rs, ladybug-rs). These are complementary
//! to the callback-based global hooks below.

pub mod lifecycle;

use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Context types
// ---------------------------------------------------------------------------

/// Context object passed to LLM call hooks.
///
/// Provides hooks with complete access to the execution state, allowing
/// modification of messages, responses, and executor attributes.
#[derive(Debug, Clone)]
pub struct LLMCallHookContext {
    /// Direct reference to messages (mutable list).
    pub messages: Vec<HashMap<String, String>>,
    /// Agent role or identifier (may be empty for direct LLM calls).
    pub agent: Option<String>,
    /// Task description (if applicable).
    pub task: Option<String>,
    /// Crew identifier (if applicable).
    pub crew: Option<String>,
    /// LLM model identifier.
    pub llm: Option<String>,
    /// Current iteration count (0 for direct LLM calls).
    pub iterations: usize,
    /// LLM response string (only set for after_llm_call hooks).
    pub response: Option<String>,
}

impl LLMCallHookContext {
    /// Create a new `LLMCallHookContext`.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            agent: None,
            task: None,
            crew: None,
            llm: None,
            iterations: 0,
            response: None,
        }
    }
}

impl Default for LLMCallHookContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Context object passed to tool call hooks.
///
/// Provides hooks with access to the tool being called, its input,
/// the agent/task/crew context, and the result (for after hooks).
#[derive(Debug, Clone)]
pub struct ToolCallHookContext {
    /// Name of the tool being called.
    pub tool_name: String,
    /// Tool input parameters (mutable map).
    pub tool_input: HashMap<String, Value>,
    /// Agent role or identifier (may be `None`).
    pub agent: Option<String>,
    /// Task description (may be `None`).
    pub task: Option<String>,
    /// Crew identifier (may be `None`).
    pub crew: Option<String>,
    /// Tool execution result (only set for after_tool_call hooks).
    pub tool_result: Option<String>,
}

impl ToolCallHookContext {
    /// Create a new `ToolCallHookContext`.
    pub fn new(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            tool_input: HashMap::new(),
            agent: None,
            task: None,
            crew: None,
            tool_result: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Hook type aliases
// ---------------------------------------------------------------------------

/// Before-LLM-call hook: receives context, returns `false` to block execution.
pub type BeforeLLMCallHook = Box<dyn Fn(&mut LLMCallHookContext) -> Option<bool> + Send + Sync>;

/// After-LLM-call hook: receives context, returns replacement response or `None`.
pub type AfterLLMCallHook =
    Box<dyn Fn(&mut LLMCallHookContext) -> Option<String> + Send + Sync>;

/// Before-tool-call hook: receives context, returns `false` to block execution.
pub type BeforeToolCallHook =
    Box<dyn Fn(&mut ToolCallHookContext) -> Option<bool> + Send + Sync>;

/// After-tool-call hook: receives context, returns replacement result or `None`.
pub type AfterToolCallHook =
    Box<dyn Fn(&mut ToolCallHookContext) -> Option<String> + Send + Sync>;

// ---------------------------------------------------------------------------
// Global hook registries
// ---------------------------------------------------------------------------

static BEFORE_LLM_HOOKS: Lazy<Mutex<Vec<BeforeLLMCallHook>>> =
    Lazy::new(|| Mutex::new(Vec::new()));
static AFTER_LLM_HOOKS: Lazy<Mutex<Vec<AfterLLMCallHook>>> =
    Lazy::new(|| Mutex::new(Vec::new()));
static BEFORE_TOOL_HOOKS: Lazy<Mutex<Vec<BeforeToolCallHook>>> =
    Lazy::new(|| Mutex::new(Vec::new()));
static AFTER_TOOL_HOOKS: Lazy<Mutex<Vec<AfterToolCallHook>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

/// Register a global before-LLM-call hook.
///
/// Global hooks are invoked for every LLM call across all executors.
/// The hook receives a mutable [`LLMCallHookContext`] and may return
/// `Some(false)` to block execution.
pub fn register_before_llm_call_hook(hook: BeforeLLMCallHook) {
    BEFORE_LLM_HOOKS.lock().unwrap().push(hook);
}

/// Register a global after-LLM-call hook.
///
/// The hook receives a mutable [`LLMCallHookContext`] (with `response` set)
/// and may return a replacement response string.
pub fn register_after_llm_call_hook(hook: AfterLLMCallHook) {
    AFTER_LLM_HOOKS.lock().unwrap().push(hook);
}

/// Register a global before-tool-call hook.
///
/// The hook receives a mutable [`ToolCallHookContext`] and may return
/// `Some(false)` to block tool execution.
pub fn register_before_tool_call_hook(hook: BeforeToolCallHook) {
    BEFORE_TOOL_HOOKS.lock().unwrap().push(hook);
}

/// Register a global after-tool-call hook.
///
/// The hook receives a mutable [`ToolCallHookContext`] (with `tool_result` set)
/// and may return a replacement result string.
pub fn register_after_tool_call_hook(hook: AfterToolCallHook) {
    AFTER_TOOL_HOOKS.lock().unwrap().push(hook);
}

/// Retrieve copies of all registered before-LLM-call hook count.
pub fn before_llm_call_hook_count() -> usize {
    BEFORE_LLM_HOOKS.lock().unwrap().len()
}

/// Retrieve the count of all registered after-LLM-call hooks.
pub fn after_llm_call_hook_count() -> usize {
    AFTER_LLM_HOOKS.lock().unwrap().len()
}

/// Clear all registered global before-LLM-call hooks. Returns count cleared.
pub fn clear_before_llm_call_hooks() -> usize {
    let mut hooks = BEFORE_LLM_HOOKS.lock().unwrap();
    let count = hooks.len();
    hooks.clear();
    count
}

/// Clear all registered global after-LLM-call hooks. Returns count cleared.
pub fn clear_after_llm_call_hooks() -> usize {
    let mut hooks = AFTER_LLM_HOOKS.lock().unwrap();
    let count = hooks.len();
    hooks.clear();
    count
}

/// Clear all registered global before-tool-call hooks. Returns count cleared.
pub fn clear_before_tool_call_hooks() -> usize {
    let mut hooks = BEFORE_TOOL_HOOKS.lock().unwrap();
    let count = hooks.len();
    hooks.clear();
    count
}

/// Clear all registered global after-tool-call hooks. Returns count cleared.
pub fn clear_after_tool_call_hooks() -> usize {
    let mut hooks = AFTER_TOOL_HOOKS.lock().unwrap();
    let count = hooks.len();
    hooks.clear();
    count
}

/// Clear **all** registered global hooks (LLM + tool, before + after).
///
/// Returns `(before_llm, after_llm, before_tool, after_tool)` counts.
pub fn clear_all_global_hooks() -> (usize, usize, usize, usize) {
    let bl = clear_before_llm_call_hooks();
    let al = clear_after_llm_call_hooks();
    let bt = clear_before_tool_call_hooks();
    let at = clear_after_tool_call_hooks();
    (bl, al, bt, at)
}

/// Execute all before-LLM-call hooks on the given context.
///
/// Returns `false` if any hook blocks execution.
pub fn run_before_llm_call_hooks(ctx: &mut LLMCallHookContext) -> bool {
    let hooks = BEFORE_LLM_HOOKS.lock().unwrap();
    for hook in hooks.iter() {
        if let Some(false) = hook(ctx) {
            return false;
        }
    }
    true
}

/// Execute all after-LLM-call hooks on the given context.
///
/// If a hook returns a replacement response, it updates `ctx.response`.
pub fn run_after_llm_call_hooks(ctx: &mut LLMCallHookContext) {
    let hooks = AFTER_LLM_HOOKS.lock().unwrap();
    for hook in hooks.iter() {
        if let Some(replacement) = hook(ctx) {
            ctx.response = Some(replacement);
        }
    }
}

/// Execute all before-tool-call hooks on the given context.
///
/// Returns `false` if any hook blocks execution.
pub fn run_before_tool_call_hooks(ctx: &mut ToolCallHookContext) -> bool {
    let hooks = BEFORE_TOOL_HOOKS.lock().unwrap();
    for hook in hooks.iter() {
        if let Some(false) = hook(ctx) {
            return false;
        }
    }
    true
}

/// Execute all after-tool-call hooks on the given context.
///
/// If a hook returns a replacement result, it updates `ctx.tool_result`.
pub fn run_after_tool_call_hooks(ctx: &mut ToolCallHookContext) {
    let hooks = AFTER_TOOL_HOOKS.lock().unwrap();
    for hook in hooks.iter() {
        if let Some(replacement) = hook(ctx) {
            ctx.tool_result = Some(replacement);
        }
    }
}
