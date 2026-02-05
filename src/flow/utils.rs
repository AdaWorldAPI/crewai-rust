//! Flow utility functions for visualization and dependency analysis.
//!
//! Corresponds to `crewai/flow/utils.py`.
//! Provides core functionality for analyzing and manipulating flow structures,
//! including node level calculation, ancestor tracking, and condition normalization.

use std::collections::{HashMap, HashSet, VecDeque};

use serde_json::Value;

/// Constants for condition types matching Python's `constants.py`.
pub const AND_CONDITION: &str = "AND";
pub const OR_CONDITION: &str = "OR";

/// A flow condition can be either a simple string (method name) or a nested
/// condition dictionary.
///
/// Corresponds to Python's `FlowCondition` TypedDict.
#[derive(Debug, Clone, PartialEq)]
pub enum FlowCondition {
    /// A single method name.
    MethodName(String),
    /// A compound condition with type ("AND"/"OR") and sub-conditions.
    Compound {
        condition_type: String,
        conditions: Vec<FlowCondition>,
    },
}

impl FlowCondition {
    /// Create an OR condition from method names.
    pub fn or_condition(methods: Vec<String>) -> Self {
        FlowCondition::Compound {
            condition_type: OR_CONDITION.to_string(),
            conditions: methods.into_iter().map(FlowCondition::MethodName).collect(),
        }
    }

    /// Create an AND condition from method names.
    pub fn and_condition(methods: Vec<String>) -> Self {
        FlowCondition::Compound {
            condition_type: AND_CONDITION.to_string(),
            conditions: methods.into_iter().map(FlowCondition::MethodName).collect(),
        }
    }
}

/// A simple flow condition: (condition_type, list_of_method_names).
///
/// Corresponds to Python's `SimpleFlowCondition`.
#[derive(Debug, Clone)]
pub struct SimpleFlowCondition {
    pub condition_type: String,
    pub methods: Vec<String>,
}

/// Listener condition: either simple tuple form or nested condition tree.
///
/// Corresponds to Python's `SimpleFlowCondition | FlowCondition` union.
#[derive(Debug, Clone)]
pub enum ListenerCondition {
    /// Simple tuple condition: (condition_type, methods).
    Simple(SimpleFlowCondition),
    /// Nested condition tree.
    Nested(FlowCondition),
}

/// Check if a string is a valid flow method name.
pub fn is_flow_method_name(s: &str) -> bool {
    !s.is_empty()
}

/// Normalize a condition to standard format with a "conditions" key.
///
/// Corresponds to Python's `_normalize_condition`.
pub fn normalize_condition(condition: &FlowCondition) -> FlowCondition {
    match condition {
        FlowCondition::MethodName(name) => FlowCondition::Compound {
            condition_type: OR_CONDITION.to_string(),
            conditions: vec![FlowCondition::MethodName(name.clone())],
        },
        FlowCondition::Compound { .. } => condition.clone(),
    }
}

/// Extract ALL method names from a condition tree recursively.
///
/// Used for visualization and debugging. Extracts every method name regardless of nesting.
///
/// Corresponds to Python's `_extract_all_methods_recursive`.
pub fn extract_all_methods_recursive(
    condition: &FlowCondition,
    known_methods: Option<&HashSet<String>>,
) -> Vec<String> {
    match condition {
        FlowCondition::MethodName(name) => {
            if let Some(methods) = known_methods {
                if methods.contains(name) {
                    vec![name.clone()]
                } else {
                    vec![]
                }
            } else {
                vec![name.clone()]
            }
        }
        FlowCondition::Compound { conditions, .. } => {
            let mut result = Vec::new();
            for sub in conditions {
                result.extend(extract_all_methods_recursive(sub, known_methods));
            }
            result
        }
    }
}

/// Extract method names that must complete for AND conditions.
///
/// For AND conditions, extracts methods that must ALL complete.
/// For OR conditions nested inside AND, we do not extract their methods
/// since only one branch of the OR needs to trigger.
///
/// Corresponds to Python's `_extract_all_methods`.
pub fn extract_all_methods(condition: &FlowCondition) -> Vec<String> {
    match condition {
        FlowCondition::MethodName(name) => vec![name.clone()],
        FlowCondition::Compound {
            condition_type,
            conditions,
        } => {
            if condition_type == AND_CONDITION {
                conditions
                    .iter()
                    .filter_map(|c| {
                        if let FlowCondition::MethodName(name) = c {
                            Some(name.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            }
        }
    }
}

/// Extract method names from a `ListenerCondition`.
pub fn extract_methods_from_listener(condition: &ListenerCondition) -> Vec<String> {
    match condition {
        ListenerCondition::Simple(simple) => simple.methods.clone(),
        ListenerCondition::Nested(nested) => extract_all_methods_recursive(nested, None),
    }
}

/// Get the condition type from a `ListenerCondition`.
pub fn get_condition_type(condition: &ListenerCondition) -> &str {
    match condition {
        ListenerCondition::Simple(simple) => &simple.condition_type,
        ListenerCondition::Nested(nested) => match nested {
            FlowCondition::Compound {
                condition_type, ..
            } => condition_type,
            FlowCondition::MethodName(_) => OR_CONDITION,
        },
    }
}

/// Calculate the hierarchical level of each node in the flow.
///
/// Performs a breadth-first traversal of the flow graph to assign levels
/// to nodes, starting with start methods at level 0.
///
/// Corresponds to Python's `calculate_node_levels`.
pub fn calculate_node_levels(
    methods: &HashSet<String>,
    start_methods: &[String],
    listeners: &HashMap<String, ListenerCondition>,
    routers: &HashSet<String>,
    router_paths: &HashMap<String, Vec<String>>,
) -> HashMap<String, usize> {
    let mut levels: HashMap<String, usize> = HashMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut pending_and: HashMap<String, HashSet<String>> = HashMap::new();

    // Start methods at level 0
    for method_name in start_methods {
        levels.insert(method_name.clone(), 0);
        queue.push_back(method_name.clone());
    }

    // Precompute listener dependencies
    let mut or_listeners: HashMap<String, Vec<String>> = HashMap::new();
    let mut and_listeners: HashMap<String, HashSet<String>> = HashMap::new();

    for (listener_name, condition_data) in listeners {
        let condition_type = get_condition_type(condition_data);
        let trigger_methods = extract_methods_from_listener(condition_data);

        if condition_type == OR_CONDITION {
            for method in &trigger_methods {
                or_listeners
                    .entry(method.clone())
                    .or_default()
                    .push(listener_name.clone());
            }
        } else if condition_type == AND_CONDITION {
            and_listeners.insert(
                listener_name.clone(),
                trigger_methods.into_iter().collect(),
            );
        }
    }

    // Breadth-first traversal
    while let Some(current) = queue.pop_front() {
        let current_level = *levels.get(&current).unwrap_or(&0);
        visited.insert(current.clone());

        // Process OR listeners
        if let Some(or_listener_names) = or_listeners.get(&current) {
            for listener_name in or_listener_names {
                let new_level = current_level + 1;
                if !levels.contains_key(listener_name) || levels[listener_name] > new_level {
                    levels.insert(listener_name.clone(), new_level);
                    if !visited.contains(listener_name) {
                        queue.push_back(listener_name.clone());
                    }
                }
            }
        }

        // Process AND listeners
        for (listener_name, required_methods) in &and_listeners {
            if required_methods.contains(&current) {
                let entry = pending_and.entry(listener_name.clone()).or_default();
                entry.insert(current.clone());

                if *required_methods == *entry {
                    let new_level = current_level + 1;
                    if !levels.contains_key(listener_name) || levels[listener_name] > new_level {
                        levels.insert(listener_name.clone(), new_level);
                        if !visited.contains(listener_name) {
                            queue.push_back(listener_name.clone());
                        }
                    }
                }
            }
        }

        // Process router paths
        if routers.contains(&current) {
            if let Some(paths) = router_paths.get(&current) {
                for path in paths {
                    for (listener_name, condition_data) in listeners {
                        let trigger_methods = extract_methods_from_listener(condition_data);
                        if trigger_methods.contains(path) {
                            let new_level = current_level + 1;
                            if !levels.contains_key(listener_name)
                                || levels[listener_name] > new_level
                            {
                                levels.insert(listener_name.clone(), new_level);
                                queue.push_back(listener_name.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Assign unreachable methods to max_level + 1
    let max_level = levels.values().copied().max().unwrap_or(0);
    for method_name in methods {
        levels.entry(method_name.clone()).or_insert(max_level + 1);
    }

    levels
}

/// Count the number of outgoing edges for each method.
///
/// Corresponds to Python's `count_outgoing_edges`.
pub fn count_outgoing_edges(
    methods: &HashSet<String>,
    listeners: &HashMap<String, ListenerCondition>,
) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for method in methods {
        counts.insert(method.clone(), 0);
    }

    for condition_data in listeners.values() {
        let trigger_methods = extract_methods_from_listener(condition_data);
        for trigger in trigger_methods {
            if methods.contains(&trigger) {
                *counts.entry(trigger).or_insert(0) += 1;
            }
        }
    }

    counts
}

/// Build a dictionary mapping each node to its ancestor nodes.
///
/// Corresponds to Python's `build_ancestor_dict`.
pub fn build_ancestor_dict(
    methods: &HashSet<String>,
    listeners: &HashMap<String, ListenerCondition>,
    routers: &HashSet<String>,
    router_paths: &HashMap<String, Vec<String>>,
) -> HashMap<String, HashSet<String>> {
    let mut ancestors: HashMap<String, HashSet<String>> = HashMap::new();
    for node in methods {
        ancestors.insert(node.clone(), HashSet::new());
    }
    let mut visited: HashSet<String> = HashSet::new();

    for node in methods {
        if !visited.contains(node) {
            dfs_ancestors(
                node,
                &mut ancestors,
                &mut visited,
                listeners,
                routers,
                router_paths,
            );
        }
    }

    ancestors
}

/// Depth-first search to build ancestor relationships.
///
/// Corresponds to Python's `dfs_ancestors`.
fn dfs_ancestors(
    node: &str,
    ancestors: &mut HashMap<String, HashSet<String>>,
    visited: &mut HashSet<String>,
    listeners: &HashMap<String, ListenerCondition>,
    routers: &HashSet<String>,
    router_paths: &HashMap<String, Vec<String>>,
) {
    if visited.contains(node) {
        return;
    }
    visited.insert(node.to_string());

    for (listener_name, condition_data) in listeners {
        let trigger_methods = extract_methods_from_listener(condition_data);
        if trigger_methods.contains(&node.to_string()) {
            // Get ancestors of node before inserting
            let node_ancestors: HashSet<String> =
                ancestors.get(node).cloned().unwrap_or_default();

            let entry = ancestors.entry(listener_name.clone()).or_default();
            entry.insert(node.to_string());
            entry.extend(node_ancestors);

            dfs_ancestors(
                listener_name,
                ancestors,
                visited,
                listeners,
                routers,
                router_paths,
            );
        }
    }

    if routers.contains(node) {
        if let Some(paths) = router_paths.get(node) {
            for path in paths {
                for (listener_name, condition_data) in listeners {
                    let trigger_methods = extract_methods_from_listener(condition_data);
                    if trigger_methods.contains(path) {
                        let node_ancestors: HashSet<String> =
                            ancestors.get(node).cloned().unwrap_or_default();
                        let entry = ancestors.entry(listener_name.clone()).or_default();
                        entry.extend(node_ancestors);

                        dfs_ancestors(
                            listener_name,
                            ancestors,
                            visited,
                            listeners,
                            routers,
                            router_paths,
                        );
                    }
                }
            }
        }
    }
}

/// Check if one node is an ancestor of another.
///
/// Corresponds to Python's `is_ancestor`.
pub fn is_ancestor(
    node: &str,
    ancestor_candidate: &str,
    ancestors: &HashMap<String, HashSet<String>>,
) -> bool {
    ancestors
        .get(node)
        .map(|a| a.contains(ancestor_candidate))
        .unwrap_or(false)
}

/// Build a dictionary mapping parent nodes to their children.
///
/// Corresponds to Python's `build_parent_children_dict`.
pub fn build_parent_children_dict(
    listeners: &HashMap<String, ListenerCondition>,
    router_paths: &HashMap<String, Vec<String>>,
) -> HashMap<String, Vec<String>> {
    let mut parent_children: HashMap<String, Vec<String>> = HashMap::new();

    for (listener_name, condition_data) in listeners {
        let trigger_methods = extract_methods_from_listener(condition_data);
        for trigger in trigger_methods {
            let children = parent_children.entry(trigger).or_default();
            if !children.contains(listener_name) {
                children.push(listener_name.clone());
            }
        }
    }

    for (router_method_name, paths) in router_paths {
        for path in paths {
            for (listener_name, condition_data) in listeners {
                let trigger_methods = extract_methods_from_listener(condition_data);
                if trigger_methods.contains(path) {
                    let children = parent_children
                        .entry(router_method_name.clone())
                        .or_default();
                    if !children.contains(listener_name) {
                        children.push(listener_name.clone());
                    }
                }
            }
        }
    }

    parent_children
}

/// Get the index of a child node in its parent's sorted children list.
///
/// Corresponds to Python's `get_child_index`.
pub fn get_child_index(
    parent: &str,
    child: &str,
    parent_children: &HashMap<String, Vec<String>>,
) -> Option<usize> {
    if let Some(children) = parent_children.get(parent) {
        let mut sorted = children.clone();
        sorted.sort();
        sorted.iter().position(|c| c == child)
    } else {
        None
    }
}

/// Check if a `Value` represents a valid flow condition dictionary.
///
/// Corresponds to Python's `is_flow_condition_dict`.
pub fn is_flow_condition_dict(value: &Value) -> bool {
    if let Some(obj) = value.as_object() {
        let type_val = obj.get("type").and_then(|v| v.as_str());
        match type_val {
            Some("AND") | Some("OR") => {
                // Check conditions field if present
                if let Some(conditions) = obj.get("conditions") {
                    if !conditions.is_array() {
                        return false;
                    }
                }
                // Check methods field if present
                if let Some(methods) = obj.get("methods") {
                    if let Some(arr) = methods.as_array() {
                        if !arr.iter().all(|m| m.is_string()) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    } else {
        false
    }
}

/// Parse a `serde_json::Value` into a `FlowCondition`.
pub fn parse_flow_condition(value: &Value) -> Option<FlowCondition> {
    if let Some(s) = value.as_str() {
        return Some(FlowCondition::MethodName(s.to_string()));
    }

    if let Some(obj) = value.as_object() {
        let condition_type = obj.get("type")?.as_str()?.to_string();

        // Try "conditions" key first, then "methods" key
        let sub_values = obj
            .get("conditions")
            .or_else(|| obj.get("methods"))
            .and_then(|v| v.as_array())?;

        let conditions: Vec<FlowCondition> = sub_values
            .iter()
            .filter_map(|v| parse_flow_condition(v))
            .collect();

        return Some(FlowCondition::Compound {
            condition_type,
            conditions,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_condition_or() {
        let cond = FlowCondition::or_condition(vec!["a".into(), "b".into()]);
        let methods = extract_all_methods_recursive(&cond, None);
        assert_eq!(methods, vec!["a", "b"]);
    }

    #[test]
    fn test_flow_condition_and() {
        let cond = FlowCondition::and_condition(vec!["x".into(), "y".into()]);
        let methods = extract_all_methods(&cond);
        assert_eq!(methods, vec!["x", "y"]);
    }

    #[test]
    fn test_is_flow_condition_dict() {
        let valid = serde_json::json!({"type": "OR", "conditions": ["a", "b"]});
        assert!(is_flow_condition_dict(&valid));

        let invalid = serde_json::json!({"type": "MAYBE"});
        assert!(!is_flow_condition_dict(&invalid));
    }

    #[test]
    fn test_parse_flow_condition() {
        let value = serde_json::json!({
            "type": "AND",
            "conditions": ["method_a", "method_b"]
        });
        let cond = parse_flow_condition(&value).unwrap();
        let methods = extract_all_methods(&cond);
        assert_eq!(methods, vec!["method_a", "method_b"]);
    }
}
