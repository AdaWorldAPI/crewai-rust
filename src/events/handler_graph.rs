//! Dependency graph resolution for event handlers.
//!
//! Corresponds to `crewai/events/handler_graph.py`.
//!
//! Resolves handler dependencies into execution levels using topological
//! sort, ensuring handlers execute in the correct order while maximising
//! parallelism within each level.

use std::collections::{HashMap, HashSet, VecDeque};

use super::event_bus::{Depends, ExecutionPlan, HandlerId};

// ---------------------------------------------------------------------------
// CircularDependencyError
// ---------------------------------------------------------------------------

/// Error raised when circular dependencies are detected among event handlers.
///
/// Corresponds to `crewai/events/handler_graph.py::CircularDependencyError`.
#[derive(Debug, thiserror::Error)]
#[error("Circular dependency detected in event handlers: {}", handler_names(.handlers))]
pub struct CircularDependencyError {
    /// The handlers involved in the circular dependency.
    pub handlers: Vec<HandlerId>,
}

fn handler_names(handlers: &[HandlerId]) -> String {
    handlers
        .iter()
        .take(5)
        .map(|h| h.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

// ---------------------------------------------------------------------------
// HandlerGraph
// ---------------------------------------------------------------------------

/// Resolves handler dependencies into parallel execution levels.
///
/// Handlers are organised into levels where:
/// - Level 0: handlers with no dependencies (can run first).
/// - Level N: handlers that depend on handlers in levels 0..N-1.
///
/// Handlers within the same level can execute in parallel.
///
/// Corresponds to `crewai/events/handler_graph.py::HandlerGraph`.
pub struct HandlerGraph {
    /// Ordered execution levels.
    pub levels: ExecutionPlan,
}

impl HandlerGraph {
    /// Build the dependency graph and resolve it into execution levels.
    ///
    /// # Panics
    ///
    /// Panics (via `CircularDependencyError`) if a circular dependency is
    /// detected.
    pub fn new(handlers: &HashMap<HandlerId, Vec<Depends>>) -> Self {
        let levels = Self::resolve(handlers);
        Self { levels }
    }

    /// Resolve dependencies into execution levels using Kahn's algorithm
    /// (topological sort).
    fn resolve(handlers: &HashMap<HandlerId, Vec<Depends>>) -> ExecutionPlan {
        // Build adjacency: dependents[dep] = set of handlers that depend on dep.
        let mut dependents: HashMap<HandlerId, HashSet<HandlerId>> = HashMap::new();
        let mut in_degree: HashMap<HandlerId, usize> = HashMap::new();

        // Initialise in-degree for every handler.
        for handler_id in handlers.keys() {
            in_degree.entry(handler_id.clone()).or_insert(0);
        }

        for (handler_id, deps) in handlers {
            *in_degree.entry(handler_id.clone()).or_insert(0) = deps.len();
            for dep in deps {
                dependents
                    .entry(dep.handler_id.clone())
                    .or_default()
                    .insert(handler_id.clone());
            }
        }

        // Seed queue with handlers that have no dependencies.
        let mut queue: VecDeque<HandlerId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut levels: ExecutionPlan = Vec::new();

        while !queue.is_empty() {
            let mut current_level: HashSet<HandlerId> = HashSet::new();
            let level_size = queue.len();

            for _ in 0..level_size {
                let handler_id = queue.pop_front().unwrap();
                current_level.insert(handler_id.clone());

                if let Some(deps) = dependents.get(&handler_id) {
                    for dependent in deps {
                        if let Some(deg) = in_degree.get_mut(dependent) {
                            *deg -= 1;
                            if *deg == 0 {
                                queue.push_back(dependent.clone());
                            }
                        }
                    }
                }
            }

            if !current_level.is_empty() {
                levels.push(current_level);
            }
        }

        // Detect circular dependencies: any handler with in_degree > 0.
        let remaining: Vec<HandlerId> = in_degree
            .into_iter()
            .filter(|(_, deg)| *deg > 0)
            .map(|(id, _)| id)
            .collect();

        if !remaining.is_empty() {
            panic!(
                "{}",
                CircularDependencyError {
                    handlers: remaining
                }
            );
        }

        levels
    }

    /// Get the ordered execution plan.
    pub fn get_execution_plan(&self) -> &ExecutionPlan {
        &self.levels
    }
}

// ---------------------------------------------------------------------------
// build_execution_plan – convenience function
// ---------------------------------------------------------------------------

/// Build an execution plan from a list of handler IDs and their dependencies.
///
/// Corresponds to `crewai/events/handler_graph.py::build_execution_plan`.
///
/// # Panics
///
/// Panics if circular dependencies are detected.
pub fn build_execution_plan(
    handler_ids: &[HandlerId],
    dependencies: &HashMap<HandlerId, Vec<Depends>>,
) -> ExecutionPlan {
    let handler_dict: HashMap<HandlerId, Vec<Depends>> = handler_ids
        .iter()
        .map(|h| {
            let deps = dependencies.get(h).cloned().unwrap_or_default();
            (h.clone(), deps)
        })
        .collect();

    let graph = HandlerGraph::new(&handler_dict);
    graph.levels
}
