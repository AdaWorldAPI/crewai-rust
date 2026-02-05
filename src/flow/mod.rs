//! Flow module for CrewAI event-driven workflows.
//!
//! Corresponds to `crewai/flow/`.
//!
//! This module provides the core `Flow` abstraction for building event-driven
//! workflows with `@start`, `@listen`, and `@router` method decorators
//! (represented as method metadata in Rust), conditional execution logic,
//! state management, persistence, visualization, and human-in-the-loop feedback.

pub mod async_feedback;
pub mod flow;
pub mod flow_config;
pub mod flow_events;
pub mod flow_trackable;
pub mod flow_wrappers;
pub mod human_feedback;
pub mod persistence;
pub mod utils;
pub mod visualization;

// Re-export the main Flow type and FlowState.
pub use self::flow::{Flow, FlowState};

// Re-export decorator-style helpers.
pub use self::flow_wrappers::{
    and_, or_, FlowCondition, FlowConditionItem, FlowConditionType, FlowMethodMeta,
    FlowMethodName, SimpleFlowCondition,
};

// Re-export flow events.
pub use self::flow_events::FlowEvent;

// Re-export visualization entry points.
pub use self::visualization::{build_flow_structure, render_interactive, FlowStructure};
