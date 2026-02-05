//! Wrapper types for flow decorated methods with type-safe metadata.
//!
//! Corresponds to `crewai/flow/flow_wrappers.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A type-safe method name for flow methods.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FlowMethodName(pub String);

impl FlowMethodName {
    /// Create a new FlowMethodName.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl std::fmt::Display for FlowMethodName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for FlowMethodName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for FlowMethodName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Flow condition type -- how trigger conditions combine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlowConditionType {
    /// Any trigger firing will activate the method.
    #[serde(rename = "OR")]
    OR,
    /// All triggers must fire before the method activates.
    #[serde(rename = "AND")]
    AND,
}

impl std::fmt::Display for FlowConditionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlowConditionType::OR => write!(f, "OR"),
            FlowConditionType::AND => write!(f, "AND"),
        }
    }
}

/// A simple flow condition (type + method names).
///
/// Corresponds to `SimpleFlowCondition` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleFlowCondition {
    /// The condition type (OR/AND).
    pub condition_type: FlowConditionType,
    /// The method names that form the condition.
    pub methods: Vec<FlowMethodName>,
}

/// Type definition for flow trigger conditions.
///
/// This is a recursive structure where conditions can contain nested FlowConditions.
///
/// Corresponds to `FlowCondition` TypedDict in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowCondition {
    /// The type of the condition (OR/AND).
    #[serde(rename = "type")]
    pub condition_type: FlowConditionType,
    /// Nested conditions (can be method names or sub-conditions).
    #[serde(default)]
    pub conditions: Vec<FlowConditionItem>,
    /// Direct method name triggers.
    #[serde(default)]
    pub methods: Vec<FlowMethodName>,
}

/// An item in a FlowCondition's conditions list.
///
/// Can be either a method name or a nested FlowCondition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FlowConditionItem {
    /// A method name reference.
    MethodName(FlowMethodName),
    /// A nested flow condition.
    Condition(FlowCondition),
}

/// Metadata for a flow method registration.
///
/// Corresponds to the attributes set by FlowMethod, StartMethod, ListenMethod,
/// and RouterMethod in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowMethodMeta {
    /// Whether this is a start method.
    pub is_start_method: bool,
    /// Trigger methods.
    pub trigger_methods: Option<Vec<FlowMethodName>>,
    /// Condition type.
    pub condition_type: Option<FlowConditionType>,
    /// Full trigger condition (for nested conditions).
    pub trigger_condition: Option<FlowCondition>,
    /// Whether this is a router method.
    pub is_router: bool,
    /// Possible router paths.
    pub router_paths: Option<Vec<String>>,
    /// Human feedback config name (if any).
    pub human_feedback_config: Option<String>,
}

impl Default for FlowMethodMeta {
    fn default() -> Self {
        Self {
            is_start_method: false,
            trigger_methods: None,
            condition_type: None,
            trigger_condition: None,
            is_router: false,
            router_paths: None,
            human_feedback_config: None,
        }
    }
}

/// StartMethod metadata builder.
pub fn start_method_meta() -> FlowMethodMeta {
    FlowMethodMeta {
        is_start_method: true,
        ..Default::default()
    }
}

/// ListenMethod metadata builder.
pub fn listen_method_meta(
    trigger_methods: Vec<FlowMethodName>,
    condition_type: FlowConditionType,
) -> FlowMethodMeta {
    FlowMethodMeta {
        trigger_methods: Some(trigger_methods),
        condition_type: Some(condition_type),
        ..Default::default()
    }
}

/// RouterMethod metadata builder.
pub fn router_method_meta(
    trigger_methods: Vec<FlowMethodName>,
    condition_type: FlowConditionType,
    router_paths: Option<Vec<String>>,
) -> FlowMethodMeta {
    FlowMethodMeta {
        is_router: true,
        trigger_methods: Some(trigger_methods),
        condition_type: Some(condition_type),
        router_paths,
        ..Default::default()
    }
}

/// Helper to create an OR condition.
pub fn or_(methods: Vec<FlowMethodName>) -> FlowCondition {
    FlowCondition {
        condition_type: FlowConditionType::OR,
        conditions: Vec::new(),
        methods,
    }
}

/// Helper to create an AND condition.
pub fn and_(methods: Vec<FlowMethodName>) -> FlowCondition {
    FlowCondition {
        condition_type: FlowConditionType::AND,
        conditions: Vec::new(),
        methods,
    }
}
