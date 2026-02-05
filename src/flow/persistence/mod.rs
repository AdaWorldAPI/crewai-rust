//! Flow state persistence with SQLite backend.
//!
//! Corresponds to `crewai/flow/persistence/`.
//!
//! Provides the `FlowPersistence` trait (abstract base) and a concrete
//! `SQLiteFlowPersistence` implementation for persisting flow states,
//! including support for async human feedback pending contexts.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde_json::Value;
use std::path::Path;
use std::sync::Mutex;

use super::async_feedback::PendingFeedbackContext;

/// Abstract base trait for flow state persistence.
///
/// This trait defines the interface that all persistence implementations must follow.
/// It supports both structured and unstructured states (serialized as JSON `Value`).
///
/// For async human feedback support, implementations can optionally override:
/// - `save_pending_feedback()`: Saves state with pending feedback context
/// - `load_pending_feedback()`: Loads state and pending feedback context
/// - `clear_pending_feedback()`: Clears pending feedback after resume
///
/// Corresponds to `crewai.flow.persistence.base.FlowPersistence`.
pub trait FlowPersistence: Send + Sync + std::fmt::Debug {
    /// Initialize the persistence backend.
    ///
    /// This method should handle any necessary setup, such as:
    /// - Creating tables
    /// - Establishing connections
    /// - Setting up indexes
    fn init_db(&self) -> Result<(), anyhow::Error>;

    /// Persist the flow state after method completion.
    ///
    /// # Arguments
    ///
    /// * `flow_uuid` - Unique identifier for the flow instance.
    /// * `method_name` - Name of the method that just completed.
    /// * `state_data` - Current state data as a JSON value.
    fn save_state(
        &self,
        flow_uuid: &str,
        method_name: &str,
        state_data: &Value,
    ) -> Result<(), anyhow::Error>;

    /// Load the most recent state for a given flow UUID.
    ///
    /// # Returns
    ///
    /// The most recent state as a JSON value, or None if no state exists.
    fn load_state(&self, flow_uuid: &str) -> Result<Option<Value>, anyhow::Error>;

    /// Save state with a pending feedback marker.
    ///
    /// Called when a flow is paused waiting for async human feedback.
    /// The default implementation just saves the state without the pending context.
    fn save_pending_feedback(
        &self,
        flow_uuid: &str,
        context: &PendingFeedbackContext,
        state_data: &Value,
    ) -> Result<(), anyhow::Error> {
        // Default: just save the state without pending context.
        self.save_state(flow_uuid, &context.method_name, state_data)
    }

    /// Load state and pending feedback context.
    ///
    /// Called when resuming a paused flow.
    ///
    /// # Returns
    ///
    /// Tuple of (state_data, pending_context) if pending feedback exists,
    /// None otherwise.
    fn load_pending_feedback(
        &self,
        flow_uuid: &str,
    ) -> Result<Option<(Value, PendingFeedbackContext)>, anyhow::Error> {
        let _ = flow_uuid;
        Ok(None)
    }

    /// Clear the pending feedback marker after successful resume.
    ///
    /// Called after feedback is received and the flow resumes.
    fn clear_pending_feedback(&self, flow_uuid: &str) -> Result<(), anyhow::Error> {
        let _ = flow_uuid;
        Ok(())
    }
}

/// SQLite-based implementation of flow state persistence.
///
/// This class provides a simple, file-based persistence implementation using SQLite.
/// It is suitable for development, testing, or production use cases with
/// moderate performance requirements.
///
/// Supports async human feedback by storing pending feedback context in a
/// separate table. When a flow is paused waiting for feedback, use
/// `save_pending_feedback()` to persist the context. Later, use
/// `load_pending_feedback()` to retrieve it when resuming.
///
/// Corresponds to `crewai.flow.persistence.sqlite.SQLiteFlowPersistence`.
///
/// # Example
///
/// ```rust,no_run
/// use crewai::flow::persistence::SQLiteFlowPersistence;
///
/// let persistence = SQLiteFlowPersistence::new(Some("flows.db".to_string()));
/// ```
#[derive(Debug)]
pub struct SQLiteFlowPersistence {
    /// Path to the SQLite database file.
    pub db_path: String,
    /// Connection guarded by a mutex for thread safety.
    conn: Mutex<Connection>,
}

impl SQLiteFlowPersistence {
    /// Create a new SQLiteFlowPersistence.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Optional path to the SQLite database file.
    ///   If None, uses `flow_states.db` in the current directory.
    pub fn new(db_path: Option<String>) -> Self {
        let path = db_path.unwrap_or_else(|| "flow_states.db".to_string());

        // Ensure parent directory exists.
        if let Some(parent) = Path::new(&path).parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let conn = Connection::open(&path)
            .unwrap_or_else(|e| panic!("Failed to open SQLite database at '{}': {}", path, e));

        let persistence = Self {
            db_path: path,
            conn: Mutex::new(conn),
        };

        // Initialize the database.
        if let Err(e) = persistence.init_db() {
            log::warn!("Failed to initialize SQLite persistence: {}", e);
        }

        persistence
    }
}

impl FlowPersistence for SQLiteFlowPersistence {
    fn init_db(&self) -> Result<(), anyhow::Error> {
        let conn = self.conn.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire database lock: {}", e)
        })?;

        // Main state table.
        conn.execute(
            "CREATE TABLE IF NOT EXISTS flow_states (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                flow_uuid TEXT NOT NULL,
                method_name TEXT NOT NULL,
                timestamp DATETIME NOT NULL,
                state_json TEXT NOT NULL
            )",
            [],
        )?;

        // Index for faster UUID lookups.
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_flow_states_uuid
             ON flow_states(flow_uuid)",
            [],
        )?;

        // Pending feedback table for async HITL.
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pending_feedback (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                flow_uuid TEXT NOT NULL UNIQUE,
                context_json TEXT NOT NULL,
                state_json TEXT NOT NULL,
                created_at DATETIME NOT NULL
            )",
            [],
        )?;

        // Index for faster UUID lookups on pending feedback.
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pending_feedback_uuid
             ON pending_feedback(flow_uuid)",
            [],
        )?;

        Ok(())
    }

    fn save_state(
        &self,
        flow_uuid: &str,
        method_name: &str,
        state_data: &Value,
    ) -> Result<(), anyhow::Error> {
        let conn = self.conn.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire database lock: {}", e)
        })?;

        let state_json = serde_json::to_string(state_data)?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO flow_states (flow_uuid, method_name, timestamp, state_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![flow_uuid, method_name, now, state_json],
        )?;

        log::debug!(
            "SQLiteFlowPersistence::save_state: flow_uuid={}, method={}",
            flow_uuid,
            method_name
        );

        Ok(())
    }

    fn load_state(&self, flow_uuid: &str) -> Result<Option<Value>, anyhow::Error> {
        let conn = self.conn.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire database lock: {}", e)
        })?;

        let mut stmt = conn.prepare(
            "SELECT state_json FROM flow_states
             WHERE flow_uuid = ?1
             ORDER BY id DESC
             LIMIT 1",
        )?;

        let result: Option<String> = stmt
            .query_row(params![flow_uuid], |row| row.get(0))
            .ok();

        match result {
            Some(json_str) => {
                let value: Value = serde_json::from_str(&json_str)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    fn save_pending_feedback(
        &self,
        flow_uuid: &str,
        context: &PendingFeedbackContext,
        state_data: &Value,
    ) -> Result<(), anyhow::Error> {
        // Also save to regular state table for consistency.
        self.save_state(flow_uuid, &context.method_name, state_data)?;

        let conn = self.conn.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire database lock: {}", e)
        })?;

        let context_json = serde_json::to_string(&context.to_dict())?;
        let state_json = serde_json::to_string(state_data)?;
        let now = Utc::now().to_rfc3339();

        // Use INSERT OR REPLACE to handle re-triggering feedback on same flow.
        conn.execute(
            "INSERT OR REPLACE INTO pending_feedback
             (flow_uuid, context_json, state_json, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![flow_uuid, context_json, state_json, now],
        )?;

        log::debug!(
            "SQLiteFlowPersistence::save_pending_feedback: flow_uuid={}",
            flow_uuid
        );

        Ok(())
    }

    fn load_pending_feedback(
        &self,
        flow_uuid: &str,
    ) -> Result<Option<(Value, PendingFeedbackContext)>, anyhow::Error> {
        let conn = self.conn.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire database lock: {}", e)
        })?;

        let mut stmt = conn.prepare(
            "SELECT state_json, context_json FROM pending_feedback
             WHERE flow_uuid = ?1",
        )?;

        let result: Option<(String, String)> = stmt
            .query_row(params![flow_uuid], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .ok();

        match result {
            Some((state_json, context_json)) => {
                let state_value: Value = serde_json::from_str(&state_json)?;
                let context_map: std::collections::HashMap<String, Value> =
                    serde_json::from_str(&context_json)?;
                let context = PendingFeedbackContext::from_dict(&context_map)
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize context: {}", e))?;
                Ok(Some((state_value, context)))
            }
            None => Ok(None),
        }
    }

    fn clear_pending_feedback(&self, flow_uuid: &str) -> Result<(), anyhow::Error> {
        let conn = self.conn.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire database lock: {}", e)
        })?;

        conn.execute(
            "DELETE FROM pending_feedback WHERE flow_uuid = ?1",
            params![flow_uuid],
        )?;

        log::debug!(
            "SQLiteFlowPersistence::clear_pending_feedback: flow_uuid={}",
            flow_uuid
        );

        Ok(())
    }
}

/// Persistence decorator helper.
///
/// In Python, `@persist` is a decorator that automatically saves state after
/// method execution. In Rust, this is a helper that can be called after
/// method execution to persist state.
///
/// Corresponds to `crewai.flow.persistence.decorators.PersistenceDecorator`.
pub struct PersistenceDecorator;

impl PersistenceDecorator {
    /// Persist flow state with proper error handling and logging.
    ///
    /// # Arguments
    ///
    /// * `flow_uuid` - The flow's unique identifier.
    /// * `method_name` - Name of the method that triggered persistence.
    /// * `state_data` - Current state data to persist.
    /// * `persistence` - The persistence backend to use.
    /// * `verbose` - Whether to log persistence operations.
    pub fn persist_state(
        flow_uuid: &str,
        method_name: &str,
        state_data: &Value,
        persistence: &dyn FlowPersistence,
        verbose: bool,
    ) -> Result<(), anyhow::Error> {
        if verbose {
            log::info!("Saving flow state to memory for ID: {}", flow_uuid);
        }

        persistence
            .save_state(flow_uuid, method_name, state_data)
            .map_err(|e| {
                log::error!(
                    "Failed to persist state for method {}: {}",
                    method_name,
                    e
                );
                anyhow::anyhow!("State persistence failed: {}", e)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_persistence_init() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        let persistence = SQLiteFlowPersistence::new(Some(path));
        assert!(persistence.init_db().is_ok());
    }

    #[test]
    fn test_sqlite_persistence_save_load() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        let persistence = SQLiteFlowPersistence::new(Some(path));

        let state = serde_json::json!({"id": "test-uuid", "counter": 42});
        persistence
            .save_state("test-uuid", "start_method", &state)
            .unwrap();

        let loaded = persistence.load_state("test-uuid").unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded["counter"], 42);
    }

    #[test]
    fn test_sqlite_persistence_load_nonexistent() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        let persistence = SQLiteFlowPersistence::new(Some(path));

        let loaded = persistence.load_state("nonexistent").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_sqlite_persistence_pending_feedback() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        let persistence = SQLiteFlowPersistence::new(Some(path));

        let context = PendingFeedbackContext::new(
            "flow-123".to_string(),
            "MyFlow".to_string(),
            "review_step".to_string(),
            serde_json::json!({"text": "Review this"}),
            "Please review".to_string(),
        );

        let state = serde_json::json!({"id": "flow-123", "data": "test"});
        persistence
            .save_pending_feedback("flow-123", &context, &state)
            .unwrap();

        let loaded = persistence.load_pending_feedback("flow-123").unwrap();
        assert!(loaded.is_some());

        let (loaded_state, loaded_context) = loaded.unwrap();
        assert_eq!(loaded_context.flow_id, "flow-123");
        assert_eq!(loaded_context.method_name, "review_step");

        // Clear and verify.
        persistence.clear_pending_feedback("flow-123").unwrap();
        let loaded = persistence.load_pending_feedback("flow-123").unwrap();
        assert!(loaded.is_none());
    }
}
