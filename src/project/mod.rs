//! Project module with crew base and annotation decorators.
//!
//! Corresponds to `crewai/project/`.
//!
//! In Rust, Python decorator patterns are represented as marker types
//! and builder patterns rather than function wrappers.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Marker types for crew component annotations
// ---------------------------------------------------------------------------

/// Marker indicating a method defines an agent.
#[derive(Debug, Clone)]
pub struct AgentMarker;

/// Marker indicating a method defines a task.
#[derive(Debug, Clone)]
pub struct TaskMarker;

/// Marker indicating a method defines an LLM.
#[derive(Debug, Clone)]
pub struct LLMMarker;

/// Marker indicating a method defines a tool.
#[derive(Debug, Clone)]
pub struct ToolMarker;

/// Marker indicating a method defines a callback.
#[derive(Debug, Clone)]
pub struct CallbackMarker;

/// Marker indicating a method defines a cache handler.
#[derive(Debug, Clone)]
pub struct CacheHandlerMarker;

/// Marker for before-kickoff hooks.
#[derive(Debug, Clone)]
pub struct BeforeKickoffMarker;

/// Marker for after-kickoff hooks.
#[derive(Debug, Clone)]
pub struct AfterKickoffMarker;

/// Marker for JSON output format.
#[derive(Debug, Clone)]
pub struct OutputJsonMarker;

/// Marker for Pydantic (structured) output format.
#[derive(Debug, Clone)]
pub struct OutputPydanticMarker;

// ---------------------------------------------------------------------------
// Crew metadata
// ---------------------------------------------------------------------------

/// Metadata collected from annotated methods during crew class setup.
///
/// Corresponds to `__crew_metadata__` in the Python `CrewBase`.
#[derive(Debug, Clone, Default)]
pub struct CrewMetadata {
    /// Agent method names in declaration order.
    pub agents: Vec<String>,
    /// Task method names in declaration order.
    pub tasks: Vec<String>,
    /// Before-kickoff callback names.
    pub before_kickoff: Vec<String>,
    /// After-kickoff callback names.
    pub after_kickoff: Vec<String>,
    /// LLM provider names.
    pub llms: Vec<String>,
    /// Tool names.
    pub tools: Vec<String>,
    /// Callback names.
    pub callbacks: Vec<String>,
    /// Cache handler names.
    pub cache_handlers: Vec<String>,
}

// ---------------------------------------------------------------------------
// CrewBase
// ---------------------------------------------------------------------------

/// Base for crew project classes.
///
/// In the Python version this is a metaclass (`CrewBase`) that introspects
/// decorated methods. In Rust this is a regular struct that collects the
/// same metadata via builder methods.
#[derive(Debug, Clone)]
pub struct CrewBase {
    /// Path to the agents YAML config file.
    pub agents_config: Option<String>,
    /// Path to the tasks YAML config file.
    pub tasks_config: Option<String>,
    /// Collected metadata about crew components.
    pub metadata: CrewMetadata,
    /// Instantiated agents (by role).
    pub agents: Vec<String>,
    /// Instantiated tasks (by description/name).
    pub tasks: Vec<String>,
}

impl Default for CrewBase {
    fn default() -> Self {
        Self {
            agents_config: Some("config/agents.yaml".to_string()),
            tasks_config: Some("config/tasks.yaml".to_string()),
            metadata: CrewMetadata::default(),
            agents: Vec::new(),
            tasks: Vec::new(),
        }
    }
}

impl CrewBase {
    /// Create a new `CrewBase`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an agent name.
    pub fn register_agent(&mut self, name: impl Into<String>) {
        self.metadata.agents.push(name.into());
    }

    /// Register a task name.
    pub fn register_task(&mut self, name: impl Into<String>) {
        self.metadata.tasks.push(name.into());
    }

    /// Register a before-kickoff callback.
    pub fn register_before_kickoff(&mut self, name: impl Into<String>) {
        self.metadata.before_kickoff.push(name.into());
    }

    /// Register an after-kickoff callback.
    pub fn register_after_kickoff(&mut self, name: impl Into<String>) {
        self.metadata.after_kickoff.push(name.into());
    }

    /// Register an LLM provider.
    pub fn register_llm(&mut self, name: impl Into<String>) {
        self.metadata.llms.push(name.into());
    }

    /// Register a tool.
    pub fn register_tool(&mut self, name: impl Into<String>) {
        self.metadata.tools.push(name.into());
    }
}

// ---------------------------------------------------------------------------
// Utility: memoize helper
// ---------------------------------------------------------------------------

/// Simple memoization cache for project utility functions.
///
/// Corresponds to `crewai/project/utils.py::memoize`.
#[derive(Debug, Clone, Default)]
pub struct MemoCache {
    cache: HashMap<String, String>,
}

impl MemoCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a cached value by key.
    pub fn get(&self, key: &str) -> Option<&String> {
        self.cache.get(key)
    }

    /// Insert a value into the cache.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.cache.insert(key.into(), value.into());
    }

    /// Clear all cached entries.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}
