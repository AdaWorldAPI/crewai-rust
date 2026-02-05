//! Process types for crew execution.
//!
//! Corresponds to `crewai/process.py`.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the different processes that can be used to tackle tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Process {
    /// Tasks are executed one after another in order.
    Sequential,
    /// A manager agent delegates tasks to other agents.
    Hierarchical,
    // TODO: Consensual
}

impl fmt::Display for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Process::Sequential => write!(f, "sequential"),
            Process::Hierarchical => write!(f, "hierarchical"),
        }
    }
}

impl Default for Process {
    fn default() -> Self {
        Process::Sequential
    }
}
