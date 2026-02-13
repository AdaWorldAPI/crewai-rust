//! Module system errors.

use thiserror::Error;

/// Errors that can occur during module loading, activation, or runtime.
#[derive(Debug, Error)]
pub enum ModuleError {
    /// YAML parsing or serialization failed.
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// File I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Module definition validation failed.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Interface conversion failed.
    #[error("Interface error: {0}")]
    Interface(String),

    /// OpenAPI spec parsing failed.
    #[error("OpenAPI parsing error: {0}")]
    OpenApi(String),

    /// Module not found.
    #[error("Module not found: {0}")]
    NotFound(String),

    /// Module already activated.
    #[error("Module already active: {0}")]
    AlreadyActive(String),

    /// RBAC policy violation.
    #[error("Policy violation: {0}")]
    PolicyViolation(String),

    /// Runtime error during module execution.
    #[error("Runtime error: {0}")]
    Runtime(String),
}
