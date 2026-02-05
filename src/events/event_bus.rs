//! Singleton event bus for managing and dispatching events in CrewAI.
//!
//! Corresponds to `crewai/events/event_bus.py`.
//!
//! Provides a global, thread-safe event bus that allows registration and
//! handling of events throughout the CrewAI system. Handlers are dispatched
//! via a background Tokio runtime. Dependency-aware execution ordering is
//! supported through [`Depends`] and the [`handler_graph`] module.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

use crate::events::base_event::{get_next_emission_sequence, BaseEvent};
use crate::events::event_context::{
    get_current_parent_id, get_enclosing_parent_id, get_last_event_id, get_triggering_event_id,
    handle_empty_pop, handle_mismatch, pop_event_scope, push_event_scope, set_last_event_id,
    SCOPE_ENDING_EVENTS, SCOPE_STARTING_EVENTS, VALID_EVENT_PAIRS,
};
use crate::events::handler_graph::build_execution_plan;

// ---------------------------------------------------------------------------
// Global singleton
// ---------------------------------------------------------------------------

/// Global singleton event bus instance.
///
/// Access via `CREWAI_EVENT_BUS.get()` after the first call to
/// `CrewAIEventsBus::global()`, or call `CrewAIEventsBus::global()` directly.
pub static CREWAI_EVENT_BUS: OnceLock<CrewAIEventsBus> = OnceLock::new();

// ---------------------------------------------------------------------------
// Handler types
// ---------------------------------------------------------------------------

/// A synchronous event handler function.
///
/// Receives:
/// - `source`: the object that emitted the event (type-erased).
/// - `event`: the concrete event reference (type-erased via `dyn BaseEvent`).
pub type SyncHandler = Arc<dyn Fn(&dyn Any, &dyn BaseEvent) + Send + Sync>;

/// Unique identifier for a handler, used for deduplication and dependency tracking.
#[derive(Clone)]
pub struct HandlerId {
    /// Human-readable name.
    pub name: String,
    /// Unique numeric ID (monotonically increasing).
    id: u64,
}

impl fmt::Debug for HandlerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HandlerId({}:{})", self.id, self.name)
    }
}

impl PartialEq for HandlerId {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for HandlerId {}

impl std::hash::Hash for HandlerId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

static HANDLER_ID_COUNTER: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

impl HandlerId {
    /// Create a new handler ID with the given human-readable name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: HANDLER_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }
}

// ---------------------------------------------------------------------------
// Depends – handler dependency declaration
// ---------------------------------------------------------------------------

/// Declares a dependency on another event handler.
///
/// Corresponds to `crewai/events/depends.py::Depends`.
#[derive(Clone)]
pub struct Depends {
    /// The handler this dependency refers to.
    pub handler_id: HandlerId,
}

impl Depends {
    /// Create a new dependency referencing the given handler.
    pub fn new(handler_id: HandlerId) -> Self {
        Self { handler_id }
    }
}

impl fmt::Debug for Depends {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Depends({:?})", self.handler_id)
    }
}

// ---------------------------------------------------------------------------
// Internal handler entry
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct HandlerEntry {
    id: HandlerId,
    handler: SyncHandler,
    dependencies: Vec<Depends>,
}

// ---------------------------------------------------------------------------
// Execution plan type alias
// ---------------------------------------------------------------------------

/// An ordered list of handler sets. Each set can execute in parallel; sets
/// must execute sequentially in order.
pub type ExecutionPlan = Vec<HashSet<HandlerId>>;

// ---------------------------------------------------------------------------
// CrewAIEventsBus
// ---------------------------------------------------------------------------

/// Singleton event bus for handling events in CrewAI.
///
/// This struct manages event registration and emission. Handlers are
/// dispatched on a dedicated Tokio runtime running in a background thread.
///
/// Corresponds to `crewai/events/event_bus.py::CrewAIEventsBus`.
pub struct CrewAIEventsBus {
    /// Handlers keyed by event `TypeId`.
    handlers: RwLock<HashMap<TypeId, Vec<HandlerEntry>>>,

    /// Cached execution plans keyed by event `TypeId`.
    execution_plan_cache: RwLock<HashMap<TypeId, ExecutionPlan>>,

    /// Background Tokio runtime for async handler dispatch.
    runtime: Runtime,

    /// Pending join handles for in-flight handler tasks.
    pending: Mutex<Vec<JoinHandle<()>>>,

    /// Flag indicating the bus is shutting down.
    shutting_down: RwLock<bool>,
}

// SAFETY: All interior state is protected by locks.
unsafe impl Send for CrewAIEventsBus {}
unsafe impl Sync for CrewAIEventsBus {}

impl CrewAIEventsBus {
    /// Obtain a reference to the global event bus singleton, initialising it
    /// on first call.
    pub fn global() -> &'static CrewAIEventsBus {
        CREWAI_EVENT_BUS.get_or_init(|| {
            let runtime = Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("crewai-events")
                .enable_all()
                .build()
                .expect("failed to create CrewAI events runtime");

            CrewAIEventsBus {
                handlers: RwLock::new(HashMap::new()),
                execution_plan_cache: RwLock::new(HashMap::new()),
                runtime,
                pending: Mutex::new(Vec::new()),
                shutting_down: RwLock::new(false),
            }
        })
    }

    // -----------------------------------------------------------------------
    // Registration
    // -----------------------------------------------------------------------

    /// Register a handler for the given event type `E`.
    ///
    /// Returns the [`HandlerId`] assigned to the handler, which can later be
    /// used with [`Depends`] or [`off`](Self::off).
    pub fn on<E: BaseEvent + 'static>(
        &self,
        name: impl Into<String>,
        handler: impl Fn(&dyn Any, &dyn BaseEvent) + Send + Sync + 'static,
        dependencies: Option<Vec<Depends>>,
    ) -> HandlerId {
        let id = HandlerId::new(name);
        let entry = HandlerEntry {
            id: id.clone(),
            handler: Arc::new(handler),
            dependencies: dependencies.unwrap_or_default(),
        };

        let type_id = TypeId::of::<E>();
        {
            let mut map = self.handlers.write().unwrap();
            map.entry(type_id).or_default().push(entry);
        }
        // Invalidate cached plan.
        {
            let mut cache = self.execution_plan_cache.write().unwrap();
            cache.remove(&type_id);
        }
        id
    }

    /// Register a handler (convenience wrapper accepting an `Arc<dyn Fn>`).
    pub fn register_handler<E: BaseEvent + 'static>(
        &self,
        name: impl Into<String>,
        handler: SyncHandler,
    ) -> HandlerId {
        let id = HandlerId::new(name);
        let entry = HandlerEntry {
            id: id.clone(),
            handler,
            dependencies: Vec::new(),
        };

        let type_id = TypeId::of::<E>();
        {
            let mut map = self.handlers.write().unwrap();
            map.entry(type_id).or_default().push(entry);
        }
        {
            let mut cache = self.execution_plan_cache.write().unwrap();
            cache.remove(&type_id);
        }
        id
    }

    /// Unregister a handler by its [`HandlerId`].
    pub fn off<E: BaseEvent + 'static>(&self, handler_id: &HandlerId) {
        let type_id = TypeId::of::<E>();
        {
            let mut map = self.handlers.write().unwrap();
            if let Some(entries) = map.get_mut(&type_id) {
                entries.retain(|e| e.id != *handler_id);
                if entries.is_empty() {
                    map.remove(&type_id);
                }
            }
        }
        {
            let mut cache = self.execution_plan_cache.write().unwrap();
            cache.remove(&type_id);
        }
    }

    // -----------------------------------------------------------------------
    // Emission
    // -----------------------------------------------------------------------

    /// Emit an event to all registered handlers.
    ///
    /// Handles scope tracking (parent/previous/triggered-by) exactly like
    /// the Python implementation, then dispatches handlers on the background
    /// runtime.
    pub fn emit<E: BaseEvent + 'static>(
        &self,
        source: Arc<dyn Any + Send + Sync>,
        event: &mut E,
    ) {
        // -- chain tracking ------------------------------------------------
        event.set_previous_event_id(get_last_event_id());
        event.set_triggered_by_event_id(get_triggering_event_id());
        event.set_emission_sequence(Some(get_next_emission_sequence()));

        if event.parent_event_id().is_none() {
            let event_type_name = event.event_type().to_string();

            if SCOPE_ENDING_EVENTS.contains(event_type_name.as_str()) {
                event.set_parent_event_id(get_enclosing_parent_id());
                let popped = pop_event_scope();
                match popped {
                    None => handle_empty_pop(&event_type_name),
                    Some((_, ref popped_type)) => {
                        if let Some(expected_start) = VALID_EVENT_PAIRS.get(event_type_name.as_str())
                        {
                            if !popped_type.is_empty() && popped_type != expected_start {
                                handle_mismatch(
                                    &event_type_name,
                                    popped_type,
                                    expected_start,
                                );
                            }
                        }
                    }
                }
            } else if SCOPE_STARTING_EVENTS.contains(event_type_name.as_str()) {
                event.set_parent_event_id(get_current_parent_id());
                push_event_scope(event.event_id().to_string(), event_type_name);
            } else {
                event.set_parent_event_id(get_current_parent_id());
            }
        }

        set_last_event_id(event.event_id().to_string());

        // -- dispatch ------------------------------------------------------
        let type_id = TypeId::of::<E>();

        if *self.shutting_down.read().unwrap() {
            log::warn!("[CrewAIEventsBus] Attempted to emit event during shutdown. Ignoring.");
            return;
        }

        let entries: Vec<HandlerEntry> = {
            let map = self.handlers.read().unwrap();
            match map.get(&type_id) {
                Some(v) => v.clone(),
                None => return,
            }
        };

        if entries.is_empty() {
            return;
        }

        // Check whether we need dependency-aware dispatch.
        let has_deps = entries.iter().any(|e| !e.dependencies.is_empty());

        if has_deps {
            self.emit_with_dependencies(source, event, &entries);
        } else {
            self.emit_simple(source, event, &entries);
        }
    }

    /// Simple dispatch: all handlers can run concurrently.
    fn emit_simple(
        &self,
        source: Arc<dyn Any + Send + Sync>,
        event: &dyn BaseEvent,
        entries: &[HandlerEntry],
    ) {
        // Serialize event data to JSON for sending across threads.
        // We use a simple wrapper to make BaseEvent data sendable.
        let event_data = serialize_event(event);
        for entry in entries {
            let handler = entry.handler.clone();
            let src = source.clone();
            let evt = event_data.clone();
            let handle = self.runtime.spawn(async move {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    handler(src.as_ref(), evt.as_ref());
                }));
                if let Err(e) = result {
                    log::error!(
                        "[CrewAIEventsBus] Handler panic: {:?}",
                        e
                    );
                }
            });
            self.track_handle(handle);
        }
    }

    /// Dependency-aware dispatch: handlers execute in topological levels.
    fn emit_with_dependencies(
        &self,
        source: Arc<dyn Any + Send + Sync>,
        event: &dyn BaseEvent,
        entries: &[HandlerEntry],
    ) {
        let handler_ids: Vec<HandlerId> = entries.iter().map(|e| e.id.clone()).collect();
        let deps_map: HashMap<HandlerId, Vec<Depends>> = entries
            .iter()
            .map(|e| (e.id.clone(), e.dependencies.clone()))
            .collect();

        let plan = build_execution_plan(&handler_ids, &deps_map);
        let handler_map: HashMap<HandlerId, SyncHandler> = entries
            .iter()
            .map(|e| (e.id.clone(), e.handler.clone()))
            .collect();

        let event_data = serialize_event(event);

        for level in &plan {
            let mut handles = Vec::new();
            for handler_id in level {
                if let Some(handler) = handler_map.get(handler_id) {
                    let h = handler.clone();
                    let src = source.clone();
                    let evt = event_data.clone();
                    let jh = self.runtime.spawn(async move {
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            h(src.as_ref(), evt.as_ref());
                        }));
                        if let Err(e) = result {
                            log::error!(
                                "[CrewAIEventsBus] Handler panic: {:?}",
                                e
                            );
                        }
                    });
                    handles.push(jh);
                }
            }
            // Block until this level completes before starting the next.
            for jh in handles {
                let _ = self.runtime.block_on(jh);
            }
        }
    }

    /// Track a spawned task handle.
    fn track_handle(&self, handle: JoinHandle<()>) {
        let mut pending = self.pending.lock().unwrap();
        pending.push(handle);
    }

    // -----------------------------------------------------------------------
    // Flush / shutdown
    // -----------------------------------------------------------------------

    /// Block until all pending event handlers complete.
    ///
    /// Returns `true` if all handlers completed, `false` if errors occurred.
    pub fn flush(&self) -> bool {
        let handles: Vec<JoinHandle<()>> = {
            let mut pending = self.pending.lock().unwrap();
            std::mem::take(&mut *pending)
        };

        if handles.is_empty() {
            return true;
        }

        let mut all_ok = true;
        for handle in handles {
            match self.runtime.block_on(handle) {
                Ok(()) => {}
                Err(e) => {
                    log::error!("[CrewAIEventsBus] Handler exception during flush: {e}");
                    all_ok = false;
                }
            }
        }
        all_ok
    }

    /// Gracefully shut down the event bus.
    ///
    /// If `wait` is true, flushes all pending handlers first.
    pub fn shutdown(&self, wait: bool) {
        if wait {
            self.flush();
        }

        {
            let mut flag = self.shutting_down.write().unwrap();
            *flag = true;
        }

        {
            let mut map = self.handlers.write().unwrap();
            map.clear();
        }
        {
            let mut cache = self.execution_plan_cache.write().unwrap();
            cache.clear();
        }
    }

    /// Validate all registered handler dependencies.
    ///
    /// Builds execution plans for all event types that have dependencies,
    /// detecting circular or unresolved references eagerly.
    pub fn validate_dependencies(&self) -> Result<(), crate::events::handler_graph::CircularDependencyError> {
        let map = self.handlers.read().unwrap();
        for (_type_id, entries) in map.iter() {
            let has_deps = entries.iter().any(|e| !e.dependencies.is_empty());
            if !has_deps {
                continue;
            }
            let handler_ids: Vec<HandlerId> = entries.iter().map(|e| e.id.clone()).collect();
            let deps_map: HashMap<HandlerId, Vec<Depends>> = entries
                .iter()
                .map(|e| (e.id.clone(), e.dependencies.clone()))
                .collect();
            build_execution_plan(&handler_ids, &deps_map);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

use crate::events::base_event::BaseEventData;

/// Serialize a `&dyn BaseEvent` into a sendable `Arc<BaseEventData>`.
fn serialize_event(event: &dyn BaseEvent) -> Arc<BaseEventData> {
    Arc::new(BaseEventData {
        event_id: event.event_id().to_string(),
        timestamp: event.timestamp(),
        event_type: event.event_type().to_string(),
        source_fingerprint: event.source_fingerprint().map(|s| s.to_string()),
        source_type: event.source_type().map(|s| s.to_string()),
        fingerprint_metadata: event.fingerprint_metadata().cloned(),
        task_id: event.task_id().map(|s| s.to_string()),
        task_name: event.task_name().map(|s| s.to_string()),
        agent_id: event.agent_id().map(|s| s.to_string()),
        agent_role: event.agent_role().map(|s| s.to_string()),
        parent_event_id: event.parent_event_id().map(|s| s.to_string()),
        previous_event_id: event.previous_event_id().map(|s| s.to_string()),
        triggered_by_event_id: event.triggered_by_event_id().map(|s| s.to_string()),
        emission_sequence: event.emission_sequence(),
    })
}
