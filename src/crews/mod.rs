//! Crew sub-modules for crew output and utility functions.
//!
//! Corresponds to `crewai/crews/`.
//!
//! This module contains the `CrewOutput` struct that represents execution
//! results, and utility functions for preparing crew kickoff, managing
//! task execution, streaming, and conditional task logic.

pub mod crew_output;
pub mod utils;

pub use crew_output::CrewOutput;
