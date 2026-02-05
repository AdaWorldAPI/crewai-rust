//! Core flow execution framework with method registration and state management.
//!
//! Corresponds to `crewai/flow/flow.py`.
//!
//! This module provides the `Flow` struct and associated types for building
//! event-driven workflows with start, listen, and router method semantics,
//! conditional execution, state management, persistence, and human-in-the-loop
//! feedback support.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::async_feedback::{HumanFeedbackPending, PendingFeedbackContext};
use super::flow_events::*;
use super::flow_wrappers::{
    FlowCondition, FlowConditionItem, FlowConditionType, FlowMethodMeta, FlowMethodName,
    SimpleFlowCondition,
};
use super::human_feedback::HumanFeedbackResult;
use super::persistence::FlowPersistence;
use super::utils::{extract_all_methods, normalize_condition};

/// Constant for OR condition type (matches Python `OR_CONDITION`).
pub const OR_CONDITION: &str = "OR";
/// Constant for AND condition type (matches Python `AND_CONDITION`).
pub const AND_CONDITION: &str = "AND";

/// Base model for all flow states, ensuring each state has a unique ID.
///
/// Corresponds to `crewai.flow.flow.FlowState`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowState {
    /// Unique identifier for the flow state.
    pub id: String,
    /// Arbitrary state data stored as key-value pairs.
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}

impl Default for FlowState {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            data: HashMap::new(),
        }
    }
}

impl FlowState {
    /// Create a new FlowState with a generated UUID.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new FlowState with specific initial data.
    pub fn with_data(data: HashMap<String, Value>) -> Self {
        let mut state = Self {
            id: Uuid::new_v4().to_string(),
            data,
        };
        // Remove `id` from data if present and use it as the top-level id.
        if let Some(id_val) = state.data.remove("id") {
            if let Some(id_str) = id_val.as_str() {
                if !id_str.is_empty() {
                    state.id = id_str.to_string();
                }
            }
        }
        state
    }

    /// Get a value from the state.
    pub fn get(&self, key: &str) -> Option<&Value> {
        if key == "id" {
            return None; // id is at the top level, not in data
        }
        self.data.get(key)
    }

    /// Set a value in the state.
    pub fn set(&mut self, key: String, value: Value) {
        if key == "id" {
            if let Some(s) = value.as_str() {
                self.id = s.to_string();
            }
        } else {
            self.data.insert(key, value);
        }
    }

    /// Convert the state to a flat dictionary including id.
    pub fn to_dict(&self) -> HashMap<String, Value> {
        let mut map = self.data.clone();
        map.insert("id".to_string(), Value::String(self.id.clone()));
        map
    }

    /// Create a FlowState from a dictionary.
    pub fn from_dict(data: HashMap<String, Value>) -> Self {
        Self::with_data(data)
    }
}

/// Method execution type marker (analogous to Python decorators).
///
/// Represents the role a method plays in a flow:
/// - `Start`: An entry point for flow execution (`@start`).
/// - `Listen`: A method triggered by other method completions (`@listen`).
/// - `Router`: A method that routes execution based on its return value (`@router`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowMethodType {
    /// A starting method (@start decorator).
    Start,
    /// A listener method (@listen decorator).
    Listen,
    /// A router method (@router decorator).
    Router,
}

impl std::fmt::Display for FlowMethodType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlowMethodType::Start => write!(f, "start"),
            FlowMethodType::Listen => write!(f, "listen"),
            FlowMethodType::Router => write!(f, "router"),
        }
    }
}

/// Type alias for a flow method callback.
///
/// In Rust, we represent flow methods as boxed async closures that take
/// a mutable reference to the flow state, an optional trigger result,
/// and return a `Value` result.
pub type FlowMethodFn =
    Box<dyn Fn(&mut FlowState, Option<Value>) -> futures::future::BoxFuture<'_, Result<Value, anyhow::Error>> + Send + Sync>;

/// Registration info for a flow method (analogous to Python decorator metadata).
///
/// Corresponds to the combined metadata from `FlowMethodMeta` and `FlowMethodRegistration`
/// in the Python implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowMethodRegistration {
    /// Name of the method.
    pub name: FlowMethodName,
    /// Type of the flow method (start, listen, router).
    pub method_type: FlowMethodType,
    /// Whether this is a start method.
    pub is_start_method: bool,
    /// Trigger methods that cause this method to execute.
    pub trigger_methods: Option<Vec<FlowMethodName>>,
    /// Condition type for triggering (OR/AND).
    pub condition_type: Option<FlowConditionType>,
    /// Full trigger condition (for compound/nested conditions).
    pub trigger_condition: Option<FlowCondition>,
    /// Whether this method is a router.
    pub is_router: bool,
    /// Possible router paths (return values that trigger listeners).
    pub router_paths: Option<Vec<String>>,
}

impl FlowMethodRegistration {
    /// Create a new registration from a FlowMethodMeta.
    pub fn from_meta(name: FlowMethodName, meta: &FlowMethodMeta) -> Self {
        let method_type = if meta.is_start_method {
            FlowMethodType::Start
        } else if meta.is_router {
            FlowMethodType::Router
        } else {
            FlowMethodType::Listen
        };

        Self {
            name,
            method_type,
            is_start_method: meta.is_start_method,
            trigger_methods: meta.trigger_methods.clone(),
            condition_type: meta.condition_type,
            trigger_condition: meta.trigger_condition.clone(),
            is_router: meta.is_router,
            router_paths: meta.router_paths.clone(),
        }
    }
}

/// Flow method data for serialization/introspection.
///
/// Corresponds to `crewai.flow.types.FlowMethodData`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowMethodData {
    /// The name of the flow method.
    pub name: String,
    /// Whether this method is a starting point for the flow.
    #[serde(default)]
    pub starting_point: bool,
}

/// Completed method data.
///
/// Corresponds to `crewai.flow.types.CompletedMethodData`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedMethodData {
    /// The flow method information.
    pub flow_method: FlowMethodData,
    /// The completion status.
    pub status: String,
}

/// Execution method data with timing and state information.
///
/// Corresponds to `crewai.flow.types.ExecutionMethodData`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMethodData {
    /// The flow method information.
    pub flow_method: FlowMethodData,
    /// ISO timestamp when the method started execution.
    pub started_at: String,
    /// Current status of the method execution.
    pub status: String,
    /// ISO timestamp when the method finished execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// The state before method execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_state: Option<Value>,
    /// The state after method execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_state: Option<Value>,
    /// Details about any error that occurred during execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<Value>,
}

/// Flow execution data for tracking and resuming execution.
///
/// Corresponds to `crewai.flow.types.FlowExecutionData`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlowExecutionData {
    /// Unique identifier for the flow execution.
    pub id: String,
    /// Flow structure metadata.
    pub flow: Option<Value>,
    /// Input data provided to the flow.
    #[serde(default)]
    pub inputs: HashMap<String, Value>,
    /// List of methods that have been completed.
    #[serde(default)]
    pub completed_methods: Vec<CompletedMethodData>,
    /// Detailed execution history for all methods.
    #[serde(default)]
    pub execution_methods: Vec<ExecutionMethodData>,
}

/// Listener condition type: either a simple condition or a compound FlowCondition.
///
/// Corresponds to `SimpleFlowCondition | FlowCondition` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ListenerCondition {
    /// A simple condition with a type and list of method names.
    Simple(SimpleFlowCondition),
    /// A compound/nested flow condition.
    Compound(FlowCondition),
}

/// Main Flow struct for orchestrating event-driven workflows.
///
/// In the Python implementation, `Flow` is generic over a state type T
/// (`dict[str, Any]` or `BaseModel`). Here, we use `FlowState` as the
/// state container, with a `HashMap<String, Value>` for arbitrary data.
///
/// Corresponds to `crewai.flow.flow.Flow`.
pub struct Flow {
    /// The flow's mutable state.
    pub state: FlowState,
    /// The initial state (preserved for reset).
    initial_state: FlowState,
    /// Unique flow identifier (from state.id).
    flow_id: String,
    /// Human-readable name of the flow.
    pub name: Option<String>,

    // --- Method registry (analogous to Python's FlowMeta metaclass) ---
    /// All registered flow methods and their metadata.
    pub methods: Vec<FlowMethodRegistration>,
    /// Registered start method names.
    start_methods: Vec<FlowMethodName>,
    /// Listener conditions, keyed by listener method name.
    listeners: HashMap<FlowMethodName, ListenerCondition>,
    /// Router method names.
    routers: HashSet<FlowMethodName>,
    /// Router paths: for each router method, the list of possible return values.
    router_paths: HashMap<FlowMethodName, Vec<FlowMethodName>>,

    // --- Execution tracking ---
    /// Execution data for the flow.
    pub execution_data: FlowExecutionData,
    /// Method execution counts.
    method_execution_counts: HashMap<FlowMethodName, usize>,
    /// Set of completed methods.
    completed_methods: HashSet<FlowMethodName>,
    /// Pending AND listener state: tracks which triggers have fired.
    pending_and_listeners: HashMap<String, HashSet<FlowMethodName>>,
    /// OR listeners that have already fired (for deduplication).
    fired_or_listeners: HashSet<FlowMethodName>,
    /// All method outputs in execution order.
    pub method_outputs: Vec<Value>,
    /// Method results keyed by method name.
    method_results: HashMap<String, Value>,

    // --- Human feedback ---
    /// Human feedback history.
    pub human_feedback_history: Vec<HumanFeedbackResult>,
    /// Last human feedback result.
    pub last_human_feedback: Option<HumanFeedbackResult>,
    /// Pending feedback context (when flow is paused for HITL).
    pending_feedback_context: Option<PendingFeedbackContext>,

    // --- Configuration ---
    /// Persistence backend (not serialized).
    pub persistence: Option<Box<dyn FlowPersistence>>,
    /// Whether the flow execution is resuming from a paused state.
    is_execution_resuming: bool,
    /// Whether to suppress flow event emissions (internal use).
    pub suppress_flow_events: bool,
    /// Whether to enable streaming.
    pub stream: bool,
    /// Whether to enable tracing.
    pub tracing: Option<bool>,
    /// Request ID for tracing.
    pub request_id: Option<String>,
    /// Registered method callbacks (not serialized).
    method_callbacks: HashMap<FlowMethodName, Arc<FlowMethodFn>>,
    /// Thread-safe state lock.
    state_lock: Arc<Mutex<()>>,
}

impl Default for Flow {
    fn default() -> Self {
        let state = FlowState::new();
        let flow_id = state.id.clone();
        Self {
            state: state.clone(),
            initial_state: state,
            flow_id: flow_id.clone(),
            name: None,
            methods: Vec::new(),
            start_methods: Vec::new(),
            listeners: HashMap::new(),
            routers: HashSet::new(),
            router_paths: HashMap::new(),
            execution_data: FlowExecutionData {
                id: flow_id,
                ..Default::default()
            },
            method_execution_counts: HashMap::new(),
            completed_methods: HashSet::new(),
            pending_and_listeners: HashMap::new(),
            fired_or_listeners: HashSet::new(),
            method_outputs: Vec::new(),
            method_results: HashMap::new(),
            human_feedback_history: Vec::new(),
            last_human_feedback: None,
            pending_feedback_context: None,
            persistence: None,
            is_execution_resuming: false,
            suppress_flow_events: false,
            stream: false,
            tracing: None,
            request_id: None,
            method_callbacks: HashMap::new(),
            state_lock: Arc::new(Mutex::new(())),
        }
    }
}

impl Flow {
    /// Create a new Flow with default state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new Flow with a specific name.
    pub fn with_name(name: &str) -> Self {
        let mut flow = Self::default();
        flow.name = Some(name.to_string());
        flow
    }

    /// Create a new Flow with specific initial state.
    pub fn with_state(state: FlowState) -> Self {
        let flow_id = state.id.clone();
        Self {
            state: state.clone(),
            initial_state: state,
            flow_id: flow_id.clone(),
            execution_data: FlowExecutionData {
                id: flow_id,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Builder: set persistence backend.
    pub fn with_persistence(mut self, persistence: Box<dyn FlowPersistence>) -> Self {
        self.persistence = Some(persistence);
        self
    }

    /// Builder: set tracing.
    pub fn with_tracing(mut self, tracing: bool) -> Self {
        self.tracing = Some(tracing);
        self
    }

    /// Get the flow's unique identifier.
    pub fn flow_id(&self) -> &str {
        &self.flow_id
    }

    /// Get the flow name (falls back to "Flow" if not set).
    pub fn flow_name(&self) -> &str {
        self.name.as_deref().unwrap_or("Flow")
    }

    /// Get the pending feedback context, if the flow is paused.
    pub fn pending_feedback(&self) -> Option<&PendingFeedbackContext> {
        self.pending_feedback_context.as_ref()
    }

    // -----------------------------------------------------------------------
    // Method registration (equivalent to FlowMeta metaclass processing)
    // -----------------------------------------------------------------------

    /// Register a method with the flow using metadata.
    ///
    /// This is the primary method registration API. It records the method's
    /// name, type, trigger conditions, and router paths.
    pub fn register_method(&mut self, registration: FlowMethodRegistration) {
        let name = registration.name.clone();

        // Track start methods.
        if registration.is_start_method {
            if !self.start_methods.contains(&name) {
                self.start_methods.push(name.clone());
            }
        }

        // Track listeners and their conditions.
        if let Some(ref triggers) = registration.trigger_methods {
            if !triggers.is_empty() {
                if let Some(ref trigger_condition) = registration.trigger_condition {
                    // Compound condition.
                    self.listeners.insert(
                        name.clone(),
                        ListenerCondition::Compound(trigger_condition.clone()),
                    );
                } else {
                    // Simple condition.
                    let cond_type = registration
                        .condition_type
                        .unwrap_or(FlowConditionType::OR);
                    self.listeners.insert(
                        name.clone(),
                        ListenerCondition::Simple(SimpleFlowCondition {
                            condition_type: cond_type,
                            methods: triggers.clone(),
                        }),
                    );
                }
            }
        }

        // Track routers.
        if registration.is_router {
            self.routers.insert(name.clone());
            if let Some(ref paths) = registration.router_paths {
                self.router_paths.insert(
                    name.clone(),
                    paths.iter().map(|p| FlowMethodName::new(p.as_str())).collect(),
                );
            } else {
                self.router_paths.insert(name.clone(), Vec::new());
            }
        }

        self.methods.push(registration);
    }

    /// Register a method using FlowMethodMeta (convenience).
    pub fn register_method_meta(&mut self, name: &str, meta: &FlowMethodMeta) {
        let registration =
            FlowMethodRegistration::from_meta(FlowMethodName::new(name), meta);
        self.register_method(registration);
    }

    /// Register a method callback.
    ///
    /// In Rust, since we cannot use Python-style decorators, callers register
    /// async functions that will be called during flow execution.
    pub fn register_callback(&mut self, name: &str, callback: FlowMethodFn) {
        self.method_callbacks
            .insert(FlowMethodName::new(name), Arc::new(callback));
    }

    // -----------------------------------------------------------------------
    // OR listener deduplication
    // -----------------------------------------------------------------------

    /// Mark an OR listener as fired atomically.
    ///
    /// Returns `true` if this was the first call to fire the listener.
    fn mark_or_listener_fired(&mut self, listener_name: &FlowMethodName) -> bool {
        if self.fired_or_listeners.contains(listener_name) {
            return false;
        }
        self.fired_or_listeners.insert(listener_name.clone());
        true
    }

    /// Clear all fired OR listeners (for cyclic flows).
    fn clear_or_listeners(&mut self) {
        self.fired_or_listeners.clear();
    }

    // -----------------------------------------------------------------------
    // State management
    // -----------------------------------------------------------------------

    /// Initialize or update the flow state with new inputs.
    ///
    /// Corresponds to `Flow._initialize_state()` in Python.
    pub fn initialize_state(&mut self, inputs: HashMap<String, Value>) {
        let current_id = self.state.id.clone();
        let inputs_has_id = inputs.contains_key("id");

        for (k, v) in inputs {
            self.state.set(k, v);
        }

        // Preserve existing ID unless inputs explicitly provided one.
        if !inputs_has_id {
            self.state.id = current_id;
        }

        self.flow_id = self.state.id.clone();
    }

    /// Create a serializable copy of the current state.
    fn copy_and_serialize_state(&self) -> Value {
        serde_json::to_value(&self.state).unwrap_or(Value::Null)
    }

    // -----------------------------------------------------------------------
    // Kickoff (entry point)
    // -----------------------------------------------------------------------

    /// Start the flow execution (synchronous wrapper).
    ///
    /// Corresponds to `Flow.kickoff()` in Python. Finds all `@start` methods,
    /// executes them, then propagates results to listeners and routers.
    ///
    /// # Returns
    ///
    /// The final result of the flow execution.
    pub fn kickoff(&mut self) -> Result<Value, anyhow::Error> {
        let rt = tokio::runtime::Handle::try_current();
        match rt {
            Ok(_) => {
                // Already in an async context -- callers should use kickoff_async.
                Err(anyhow::anyhow!(
                    "kickoff() cannot be called from within an async context. \
                     Use 'flow.kickoff_async().await' instead."
                ))
            }
            Err(_) => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(self.kickoff_async())
            }
        }
    }

    /// Start the flow execution (async).
    ///
    /// Corresponds to `Flow.kickoff_async()` in Python.
    pub async fn kickoff_async(&mut self) -> Result<Value, anyhow::Error> {
        log::debug!("Flow::kickoff_async starting for flow_id={}", self.flow_id);

        // Emit flow started event.
        let flow_name = self.flow_name().to_string();

        // Find start methods.
        let start_methods: Vec<FlowMethodRegistration> = self
            .methods
            .iter()
            .filter(|m| m.is_start_method)
            .cloned()
            .collect();

        if start_methods.is_empty() {
            return Err(anyhow::anyhow!(
                "No start methods defined in this flow. \
                 Register at least one method with is_start_method=true."
            ));
        }

        // Execute all start methods.
        let mut last_result = Value::Null;

        for method in &start_methods {
            let method_name = method.name.clone();
            log::debug!("Executing start method: {}", method_name);

            match self.execute_method(&method_name).await {
                Ok(result) => {
                    self.method_outputs.push(result.clone());
                    self.method_results
                        .insert(method_name.0.clone(), result.clone());
                    self.completed_methods.insert(method_name.clone());

                    // Propagate to listeners.
                    if let Err(e) = self.execute_listeners(&method_name, &result).await {
                        // Check if it's a HumanFeedbackPending pause.
                        let err_str = format!("{}", e);
                        if err_str.contains("HumanFeedbackPending") {
                            log::info!("Flow paused for human feedback at method {}", method_name);
                            return Ok(Value::String(err_str));
                        }
                        return Err(e);
                    }

                    last_result = result;
                }
                Err(e) => {
                    log::error!(
                        "Start method {} failed: {}",
                        method_name,
                        e
                    );
                    return Err(e);
                }
            }
        }

        log::debug!(
            "Flow::kickoff_async finished for flow_id={}",
            self.flow_id
        );

        Ok(last_result)
    }

    // -----------------------------------------------------------------------
    // Resume (after human feedback pause)
    // -----------------------------------------------------------------------

    /// Resume flow execution with human feedback (synchronous wrapper).
    ///
    /// Corresponds to `Flow.resume()` in Python.
    pub fn resume(&mut self, feedback: &str) -> Result<Value, anyhow::Error> {
        let rt = tokio::runtime::Handle::try_current();
        match rt {
            Ok(_) => Err(anyhow::anyhow!(
                "resume() cannot be called from within an async context. \
                 Use 'flow.resume_async(feedback).await' instead."
            )),
            Err(_) => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(self.resume_async(feedback))
            }
        }
    }

    /// Resume flow execution with human feedback (async).
    ///
    /// Corresponds to `Flow.resume_async()` in Python.
    pub async fn resume_async(&mut self, feedback: &str) -> Result<Value, anyhow::Error> {
        let context = self
            .pending_feedback_context
            .take()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No pending feedback context. \
                     Use from_pending() to restore a paused flow."
                )
            })?;

        let emit = context.emit.clone();
        let default_outcome = context.default_outcome.clone();
        let _llm = context.llm.clone();

        // Determine outcome.
        let collapsed_outcome: Option<String> = if feedback.trim().is_empty() {
            if let Some(ref default) = default_outcome {
                Some(default.clone())
            } else if let Some(ref emit_opts) = emit {
                emit_opts.first().cloned()
            } else {
                None
            }
        } else if let Some(ref emit_opts) = emit {
            // In a full implementation, we would use the LLM to collapse feedback
            // to one of the emit options. For now, use the first option.
            emit_opts.first().cloned()
        } else {
            None
        };

        // Create result.
        let result = HumanFeedbackResult {
            output: context.method_output.clone(),
            feedback: feedback.to_string(),
            outcome: collapsed_outcome.clone(),
            timestamp: chrono::Utc::now(),
            method_name: context.method_name.clone(),
            metadata: context.metadata.clone(),
        };

        // Store in flow instance.
        self.human_feedback_history.push(result.clone());
        self.last_human_feedback = Some(result.clone());

        // Clear pending feedback from persistence.
        if let Some(ref persistence) = self.persistence {
            let _ = persistence.clear_pending_feedback(&context.flow_id);
        }

        // Clear resumption flag.
        self.is_execution_resuming = false;

        // Trigger downstream listeners.
        let trigger_name = if let (Some(ref emit_opts), Some(ref outcome)) =
            (&emit, &collapsed_outcome)
        {
            self.method_outputs
                .push(Value::String(outcome.clone()));
            FlowMethodName::new(outcome.as_str())
        } else {
            FlowMethodName::new(context.method_name.as_str())
        };

        let result_value = serde_json::to_value(&result).unwrap_or(Value::Null);
        self.execute_listeners(&trigger_name, &result_value)
            .await?;

        Ok(result_value)
    }

    /// Create a Flow instance from a pending feedback state.
    ///
    /// Corresponds to `Flow.from_pending()` classmethod in Python.
    pub fn from_pending(
        flow_id: &str,
        persistence: Box<dyn FlowPersistence>,
    ) -> Result<Self, anyhow::Error> {
        let loaded = persistence
            .load_pending_feedback(flow_id)?
            .ok_or_else(|| {
                anyhow::anyhow!("No pending feedback found for flow_id: {}", flow_id)
            })?;

        let (state_data, pending_context) = loaded;

        let mut flow = Self::default();
        flow.persistence = Some(persistence);

        // Restore state from persisted data.
        if let Some(state_map) = state_data.as_object() {
            let map: HashMap<String, Value> = state_map
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            flow.initialize_state(map);
        }

        // Store pending context for resume.
        flow.pending_feedback_context = Some(pending_context.clone());
        flow.is_execution_resuming = true;
        flow.completed_methods
            .insert(FlowMethodName::new(pending_context.method_name.as_str()));

        Ok(flow)
    }

    // -----------------------------------------------------------------------
    // Method execution
    // -----------------------------------------------------------------------

    /// Execute a single method by name.
    async fn execute_method(
        &mut self,
        method_name: &FlowMethodName,
    ) -> Result<Value, anyhow::Error> {
        log::debug!("Executing method: {}", method_name);

        // Look up the callback.
        let callback = self
            .method_callbacks
            .get(method_name)
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No callback registered for method '{}'. \
                     Register callbacks with flow.register_callback().",
                    method_name
                )
            })?;

        // Get the last result from the triggering method.
        let trigger_result = self.method_outputs.last().cloned();

        // Execute the method callback.
        let result = callback(&mut self.state, trigger_result).await?;

        // Track execution count.
        let count = self
            .method_execution_counts
            .entry(method_name.clone())
            .or_insert(0);
        *count += 1;

        Ok(result)
    }

    /// Execute all listeners triggered by a method's completion.
    ///
    /// Corresponds to `Flow._execute_listeners()` in Python.
    async fn execute_listeners(
        &mut self,
        completed_method: &FlowMethodName,
        result: &Value,
    ) -> Result<(), anyhow::Error> {
        // Collect listeners that should be triggered.
        // We collect keys first to avoid borrowing self immutably while calling should_trigger.
        let listener_keys: Vec<(FlowMethodName, ListenerCondition)> = self
            .listeners
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let mut triggered: Vec<FlowMethodName> = Vec::new();
        for (listener_name, condition) in &listener_keys {
            if self.should_trigger(listener_name, condition, completed_method) {
                triggered.push(listener_name.clone());
            }
        }

        if triggered.is_empty() {
            return Ok(());
        }

        log::debug!(
            "Method {} triggered listeners: {:?}",
            completed_method,
            triggered
        );

        // Execute triggered listeners.
        for listener_name in &triggered {
            // Skip if already resuming and method was completed before pause.
            if self.is_execution_resuming && self.completed_methods.contains(listener_name) {
                continue;
            }

            match self.execute_method(listener_name).await {
                Ok(listener_result) => {
                    self.method_outputs.push(listener_result.clone());
                    self.method_results
                        .insert(listener_name.0.clone(), listener_result.clone());
                    self.completed_methods.insert(listener_name.clone());

                    // Persist state after method completion if persistence is configured.
                    if let Some(ref persistence) = self.persistence {
                        let state_data = self.copy_and_serialize_state();
                        let _ = persistence.save_state(
                            &self.flow_id,
                            &listener_name.0,
                            &state_data,
                        );
                    }

                    // If the listener is a router, route based on its return value.
                    if self.routers.contains(listener_name) {
                        if let Some(route_str) = listener_result.as_str() {
                            let route_name = FlowMethodName::new(route_str);
                            // Recursively trigger listeners for the route value.
                            Box::pin(self.execute_listeners(&route_name, &listener_result))
                                .await?;
                        }
                    } else {
                        // Recursively trigger downstream listeners.
                        Box::pin(
                            self.execute_listeners(listener_name, &listener_result),
                        )
                        .await?;
                    }
                }
                Err(e) => {
                    log::error!(
                        "Listener {} failed: {}",
                        listener_name,
                        e
                    );
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Determine if a listener should be triggered by a completed method.
    fn should_trigger(
        &mut self,
        listener_name: &FlowMethodName,
        condition: &ListenerCondition,
        completed_method: &FlowMethodName,
    ) -> bool {
        match condition {
            ListenerCondition::Simple(simple) => {
                let methods = &simple.methods;
                match simple.condition_type {
                    FlowConditionType::OR => {
                        // OR: trigger if any method in the list matches.
                        if methods.iter().any(|m| m == completed_method) {
                            self.mark_or_listener_fired(listener_name)
                        } else {
                            false
                        }
                    }
                    FlowConditionType::AND => {
                        // AND: track which triggers have fired, trigger only when all have.
                        if !methods.contains(completed_method) {
                            return false;
                        }
                        let key = format!("{}:{}", listener_name, "default");
                        let pending = self
                            .pending_and_listeners
                            .entry(key.clone())
                            .or_insert_with(HashSet::new);
                        pending.insert(completed_method.clone());

                        // Check if all required methods have completed.
                        let all_methods: HashSet<FlowMethodName> =
                            methods.iter().cloned().collect();
                        if pending.is_superset(&all_methods) {
                            // Reset for potential re-triggering.
                            self.pending_and_listeners.remove(&key);
                            true
                        } else {
                            false
                        }
                    }
                }
            }
            ListenerCondition::Compound(condition) => {
                // Evaluate compound conditions recursively.
                self.evaluate_compound_condition(
                    listener_name,
                    condition,
                    completed_method,
                )
            }
        }
    }

    /// Evaluate a compound (nested) flow condition.
    fn evaluate_compound_condition(
        &mut self,
        listener_name: &FlowMethodName,
        condition: &FlowCondition,
        completed_method: &FlowMethodName,
    ) -> bool {
        // Check if the completed method is mentioned anywhere in this condition.
        let all_methods = extract_all_methods_from_condition(condition);
        if !all_methods.contains(&completed_method.0) {
            return false;
        }

        match condition.condition_type {
            FlowConditionType::OR => {
                // OR: any sub-condition being satisfied triggers.
                for item in &condition.conditions {
                    match item {
                        FlowConditionItem::MethodName(m) => {
                            if m == completed_method {
                                return self.mark_or_listener_fired(listener_name);
                            }
                        }
                        FlowConditionItem::Condition(sub) => {
                            if self.evaluate_compound_condition(
                                listener_name,
                                sub,
                                completed_method,
                            ) {
                                return true;
                            }
                        }
                    }
                }
                // Also check direct methods list.
                if condition.methods.contains(completed_method) {
                    return self.mark_or_listener_fired(listener_name);
                }
                false
            }
            FlowConditionType::AND => {
                // AND: all sub-conditions must be satisfied.
                let key = format!("{}:compound", listener_name);
                let pending = self
                    .pending_and_listeners
                    .entry(key.clone())
                    .or_insert_with(HashSet::new);
                pending.insert(completed_method.clone());

                let all_required: HashSet<FlowMethodName> = all_methods
                    .into_iter()
                    .map(|s| FlowMethodName::new(s))
                    .collect();

                if pending.is_superset(&all_required) {
                    self.pending_and_listeners.remove(&key);
                    true
                } else {
                    false
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Visualization / Plot
    // -----------------------------------------------------------------------

    /// Plot the flow structure as an interactive HTML visualization.
    ///
    /// Corresponds to `Flow.plot()` in Python.
    pub fn plot(&self, filename: Option<&str>) -> Result<String, anyhow::Error> {
        let filename = filename.unwrap_or("flow_plot");
        log::debug!("Flow::plot for flow_id={}", self.flow_id);

        let structure =
            super::visualization::build_flow_structure(&self.methods);
        super::visualization::render_interactive(&structure, filename)
    }

    // -----------------------------------------------------------------------
    // Reset
    // -----------------------------------------------------------------------

    /// Reset the flow state to initial.
    pub fn reset(&mut self) {
        self.state = self.initial_state.clone();
        self.flow_id = self.state.id.clone();
        self.execution_data = FlowExecutionData {
            id: self.flow_id.clone(),
            ..Default::default()
        };
        self.method_execution_counts.clear();
        self.completed_methods.clear();
        self.pending_and_listeners.clear();
        self.fired_or_listeners.clear();
        self.method_outputs.clear();
        self.method_results.clear();
        self.human_feedback_history.clear();
        self.last_human_feedback = None;
        self.pending_feedback_context = None;
        self.is_execution_resuming = false;
    }
}

impl std::fmt::Debug for Flow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Flow")
            .field("flow_id", &self.flow_id)
            .field("name", &self.name)
            .field("methods", &self.methods.len())
            .field("completed_methods", &self.completed_methods.len())
            .field("persistence", &self.persistence.is_some())
            .finish()
    }
}

impl std::fmt::Display for Flow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Flow(name={}, id={}, methods={}, completed={})",
            self.flow_name(),
            self.flow_id,
            self.methods.len(),
            self.completed_methods.len()
        )
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Extract all method names from a FlowCondition recursively.
fn extract_all_methods_from_condition(condition: &FlowCondition) -> HashSet<String> {
    let mut result = HashSet::new();
    for item in &condition.conditions {
        match item {
            FlowConditionItem::MethodName(m) => {
                result.insert(m.0.clone());
            }
            FlowConditionItem::Condition(sub) => {
                result.extend(extract_all_methods_from_condition(sub));
            }
        }
    }
    for m in &condition.methods {
        result.insert(m.0.clone());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_state_default() {
        let state = FlowState::new();
        assert!(!state.id.is_empty());
        assert!(state.data.is_empty());
    }

    #[test]
    fn test_flow_state_with_data() {
        let mut data = HashMap::new();
        data.insert(
            "counter".to_string(),
            Value::Number(serde_json::Number::from(0)),
        );
        let state = FlowState::with_data(data);
        assert!(!state.id.is_empty());
        assert_eq!(
            state.get("counter"),
            Some(&Value::Number(serde_json::Number::from(0)))
        );
    }

    #[test]
    fn test_flow_state_with_data_preserves_id() {
        let mut data = HashMap::new();
        data.insert(
            "id".to_string(),
            Value::String("custom-id-123".to_string()),
        );
        data.insert("key".to_string(), Value::String("value".to_string()));
        let state = FlowState::with_data(data);
        assert_eq!(state.id, "custom-id-123");
        assert_eq!(
            state.get("key"),
            Some(&Value::String("value".to_string()))
        );
    }

    #[test]
    fn test_flow_state_to_dict() {
        let mut state = FlowState::new();
        state.set("name".to_string(), Value::String("test".to_string()));
        let dict = state.to_dict();
        assert_eq!(dict.get("id"), Some(&Value::String(state.id.clone())));
        assert_eq!(
            dict.get("name"),
            Some(&Value::String("test".to_string()))
        );
    }

    #[test]
    fn test_flow_default() {
        let flow = Flow::new();
        assert!(!flow.flow_id().is_empty());
        assert!(flow.methods.is_empty());
        assert_eq!(flow.flow_name(), "Flow");
    }

    #[test]
    fn test_flow_with_name() {
        let flow = Flow::with_name("MyWorkflow");
        assert_eq!(flow.flow_name(), "MyWorkflow");
    }

    #[test]
    fn test_flow_register_method() {
        let mut flow = Flow::new();
        let meta = super::super::flow_wrappers::FlowMethodMeta {
            is_start_method: true,
            ..Default::default()
        };
        flow.register_method_meta("begin", &meta);
        assert_eq!(flow.methods.len(), 1);
        assert_eq!(flow.start_methods.len(), 1);
        assert_eq!(flow.start_methods[0], FlowMethodName::new("begin"));
    }

    #[test]
    fn test_flow_register_router() {
        let mut flow = Flow::new();
        let meta = super::super::flow_wrappers::FlowMethodMeta {
            is_router: true,
            trigger_methods: Some(vec![FlowMethodName::new("begin")]),
            condition_type: Some(FlowConditionType::OR),
            router_paths: Some(vec!["path_a".to_string(), "path_b".to_string()]),
            ..Default::default()
        };
        flow.register_method_meta("route_decision", &meta);
        assert!(flow.routers.contains(&FlowMethodName::new("route_decision")));
        assert!(flow.router_paths.contains_key(&FlowMethodName::new("route_decision")));
    }

    #[test]
    fn test_flow_reset() {
        let mut flow = Flow::new();
        flow.method_outputs.push(Value::String("test".to_string()));
        flow.completed_methods
            .insert(FlowMethodName::new("some_method"));
        flow.reset();
        assert!(flow.method_outputs.is_empty());
        assert!(flow.completed_methods.is_empty());
    }

    #[test]
    fn test_flow_initialize_state() {
        let mut flow = Flow::new();
        let original_id = flow.flow_id().to_string();

        let mut inputs = HashMap::new();
        inputs.insert(
            "counter".to_string(),
            Value::Number(serde_json::Number::from(42)),
        );
        flow.initialize_state(inputs);

        // ID should be preserved.
        assert_eq!(flow.flow_id(), original_id);
        assert_eq!(
            flow.state.get("counter"),
            Some(&Value::Number(serde_json::Number::from(42)))
        );
    }

    #[test]
    fn test_flow_display() {
        let flow = Flow::with_name("TestFlow");
        let display = format!("{}", flow);
        assert!(display.contains("TestFlow"));
        assert!(display.contains("methods=0"));
        assert!(display.contains("completed=0"));
    }
}
