//! Error types for CrewAI utilities.
//!
//! Corresponds to `crewai/utilities/errors.py`.

use thiserror::Error;

/// Errors related to database operations.
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// A generic database operation error.
    #[error("Database operation error: {message}")]
    OperationError { message: String },

    /// Connection error.
    #[error("Database connection error: {message}")]
    ConnectionError { message: String },

    /// Query error.
    #[error("Database query error: {message}")]
    QueryError { message: String },
}

/// Errors from the agent repository.
#[derive(Debug, Error)]
pub enum AgentRepositoryError {
    /// Agent not found.
    #[error("Agent not found: {agent_id}")]
    NotFound { agent_id: String },

    /// Storage error.
    #[error("Agent repository storage error: {message}")]
    StorageError { message: String },

    /// Underlying database error.
    #[error(transparent)]
    Database(#[from] DatabaseError),
}

/// Alias matching the Python `DatabaseOperationError`.
pub type DatabaseOperationError = DatabaseError;
