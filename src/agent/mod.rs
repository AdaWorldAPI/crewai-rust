//! Agent module for CrewAI.
//!
//! Corresponds to `crewai/agent/` package.
//!
//! This module contains the main `Agent` struct which extends `BaseAgent`
//! with execution capabilities, MCP tool integration, knowledge handling,
//! reasoning, guardrails, and the standalone `kickoff` execution mode.

pub mod core;
pub mod internal;
pub mod utils;

// Re-export the main Agent type.
pub use self::core::Agent;
