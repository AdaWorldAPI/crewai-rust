//! CrewAI events system for monitoring and extending agent behaviour.
//!
//! Corresponds to `crewai/events/__init__.py`.
//!
//! This module provides the event infrastructure that allows users to:
//! - Monitor agent, task, and crew execution
//! - Track memory operations and performance
//! - Build custom logging and analytics
//! - Extend CrewAI with custom event handlers
//! - Declare handler dependencies for ordered execution

// ---------------------------------------------------------------------------
// Core infrastructure modules
// ---------------------------------------------------------------------------

/// Base event trait and data types.
pub mod base_event;

/// Abstract event listener trait.
pub mod base_event_listener;

/// Singleton event bus implementation.
pub mod event_bus;

/// Event context management for parent-child relationship tracking.
pub mod event_context;

/// Dependency graph resolution for handler execution ordering.
pub mod handler_graph;

// ---------------------------------------------------------------------------
// Facade / convenience modules
// ---------------------------------------------------------------------------

/// Combined re-export of base event types + listener trait + `Listener` struct.
///
/// Prefer importing from this module for new code.
pub mod event_listener;

/// Re-export of the event bus singleton under the canonical `crewai_event_bus` name.
///
/// Prefer importing from this module for new code.
pub mod crewai_event_bus;

// ---------------------------------------------------------------------------
// Event type definitions
// ---------------------------------------------------------------------------

/// Domain-specific event type structs.
pub mod types;

// ---------------------------------------------------------------------------
// Convenience re-exports
// ---------------------------------------------------------------------------

// Core types
pub use base_event::{BaseEvent, BaseEventData};
pub use base_event_listener::BaseEventListener;
pub use event_bus::{CrewAIEventsBus, Depends, HandlerId, CREWAI_EVENT_BUS};
pub use event_listener::{CrewAIBaseEvent, Listener};
pub use handler_graph::CircularDependencyError;

// Agent events
pub use types::agent_events::{
    AgentEvaluationCompletedEvent, AgentEvaluationFailedEvent, AgentEvaluationStartedEvent,
    AgentExecutionCompletedEvent, AgentExecutionErrorEvent, AgentExecutionStartedEvent,
    LiteAgentExecutionCompletedEvent, LiteAgentExecutionErrorEvent,
    LiteAgentExecutionStartedEvent,
};

// Crew events
pub use types::crew_events::{
    CrewKickoffCompletedEvent, CrewKickoffFailedEvent, CrewKickoffStartedEvent,
    CrewTestCompletedEvent, CrewTestFailedEvent, CrewTestResultEvent, CrewTestStartedEvent,
    CrewTrainCompletedEvent, CrewTrainFailedEvent, CrewTrainStartedEvent,
};

// Task events
pub use types::task_events::{
    TaskCompletedEvent, TaskEvaluationEvent, TaskFailedEvent, TaskStartedEvent,
};

// Tool events
pub use types::tool_events::{
    ToolExecutionErrorEvent, ToolSelectionErrorEvent, ToolUsageErrorEvent, ToolUsageEvent,
    ToolUsageFinishedEvent, ToolUsageStartedEvent, ToolValidateInputErrorEvent,
};

// LLM events
pub use types::llm_events::{
    LLMCallCompletedEvent, LLMCallFailedEvent, LLMCallStartedEvent, LLMCallType,
    LLMStreamChunkEvent,
};

// Flow events
pub use types::flow_events::{
    FlowCreatedEvent, FlowFinishedEvent, FlowPausedEvent, FlowPlotEvent, FlowStartedEvent,
    HumanFeedbackReceivedEvent, HumanFeedbackRequestedEvent, MethodExecutionFailedEvent,
    MethodExecutionFinishedEvent, MethodExecutionPausedEvent, MethodExecutionStartedEvent,
};

// Knowledge events
pub use types::knowledge_events::{
    KnowledgeQueryCompletedEvent, KnowledgeQueryFailedEvent, KnowledgeQueryStartedEvent,
    KnowledgeRetrievalCompletedEvent, KnowledgeRetrievalStartedEvent,
    KnowledgeSearchQueryFailedEvent,
};

// Memory events
pub use types::memory_events::{
    MemoryQueryCompletedEvent, MemoryQueryFailedEvent, MemoryQueryStartedEvent,
    MemoryRetrievalCompletedEvent, MemoryRetrievalFailedEvent, MemoryRetrievalStartedEvent,
    MemorySaveCompletedEvent, MemorySaveFailedEvent, MemorySaveStartedEvent,
};
