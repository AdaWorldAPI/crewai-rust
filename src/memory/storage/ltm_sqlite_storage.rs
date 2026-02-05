//! SQLite storage class for long-term memory data.
//!
//! Port of crewai/memory/storage/ltm_sqlite_storage.py

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use serde_json::Value;

/// SQLite storage class for long-term memory data.
///
/// Stores task descriptions, metadata, datetime, and quality scores
/// in a SQLite database for persistent long-term memory.
pub struct LTMSQLiteStorage {
    /// Path to the SQLite database file.
    pub db_path: PathBuf,
    /// Whether to print error messages.
    verbose: bool,
}

impl LTMSQLiteStorage {
    /// Initialize the SQLite storage.
    ///
    /// # Arguments
    /// * `db_path` - Optional path to the database file.
    ///   Defaults to `<db_storage_path>/long_term_memory_storage.db`.
    /// * `verbose` - Whether to print error messages.
    pub fn new(db_path: Option<PathBuf>, verbose: bool) -> Result<Self, anyhow::Error> {
        let db_path = db_path.unwrap_or_else(|| {
            let base = crate::utilities::paths::db_storage_path();
            PathBuf::from(base).join("long_term_memory_storage.db")
        });

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let storage = Self { db_path, verbose };
        storage.initialize_db()?;
        Ok(storage)
    }

    /// Initialize the SQLite database and create the LTM table.
    fn initialize_db(&self) -> Result<(), anyhow::Error> {
        match Connection::open(&self.db_path) {
            Ok(conn) => {
                conn.execute(
                    "CREATE TABLE IF NOT EXISTS long_term_memories (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        task_description TEXT,
                        metadata TEXT,
                        datetime TEXT,
                        score REAL
                    )",
                    [],
                )?;
                Ok(())
            }
            Err(e) => {
                if self.verbose {
                    log::error!(
                        "MEMORY ERROR: An error occurred during database initialization: {}",
                        e
                    );
                }
                Err(e.into())
            }
        }
    }

    /// Save data to the LTM table.
    ///
    /// # Arguments
    /// * `task_description` - Description of the task.
    /// * `metadata` - Metadata associated with the memory.
    /// * `datetime` - Timestamp of the memory.
    /// * `score` - Quality score of the memory.
    pub fn save(
        &self,
        task_description: &str,
        metadata: &HashMap<String, Value>,
        datetime: &str,
        score: f64,
    ) -> Result<(), anyhow::Error> {
        let metadata_json = serde_json::to_string(metadata)?;
        match Connection::open(&self.db_path) {
            Ok(conn) => {
                conn.execute(
                    "INSERT INTO long_term_memories (task_description, metadata, datetime, score)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![task_description, metadata_json, datetime, score],
                )?;
                Ok(())
            }
            Err(e) => {
                if self.verbose {
                    log::error!(
                        "MEMORY ERROR: An error occurred while saving to LTM: {}",
                        e
                    );
                }
                Err(e.into())
            }
        }
    }

    /// Save data to the LTM table asynchronously.
    ///
    /// Note: In Rust, rusqlite is synchronous. This method wraps the sync
    /// operation in a tokio blocking task for async compatibility.
    pub async fn asave(
        &self,
        task_description: &str,
        metadata: &HashMap<String, Value>,
        datetime: &str,
        score: f64,
    ) -> Result<(), anyhow::Error> {
        let db_path = self.db_path.clone();
        let task_description = task_description.to_string();
        let metadata_json = serde_json::to_string(metadata)?;
        let datetime = datetime.to_string();
        let verbose = self.verbose;

        tokio::task::spawn_blocking(move || {
            match Connection::open(&db_path) {
                Ok(conn) => {
                    conn.execute(
                        "INSERT INTO long_term_memories (task_description, metadata, datetime, score)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![task_description, metadata_json, datetime, score],
                    )?;
                    Ok(())
                }
                Err(e) => {
                    if verbose {
                        log::error!(
                            "MEMORY ERROR: An error occurred while saving to LTM: {}",
                            e
                        );
                    }
                    Err(anyhow::anyhow!(e))
                }
            }
        })
        .await?
    }

    /// Query the LTM table by task description.
    ///
    /// # Arguments
    /// * `task_description` - Description of the task to search for.
    /// * `latest_n` - Maximum number of results to return.
    ///
    /// # Returns
    /// A vector of matching memory entries, or None if an error occurs.
    pub fn load(
        &self,
        task_description: &str,
        latest_n: usize,
    ) -> Result<Option<Vec<HashMap<String, Value>>>, anyhow::Error> {
        match Connection::open(&self.db_path) {
            Ok(conn) => {
                let mut stmt = conn.prepare(&format!(
                    "SELECT metadata, datetime, score
                     FROM long_term_memories
                     WHERE task_description = ?1
                     ORDER BY datetime DESC, score ASC
                     LIMIT {}",
                    latest_n
                ))?;

                let rows = stmt.query_map(params![task_description], |row| {
                    let metadata_str: String = row.get(0)?;
                    let datetime: String = row.get(1)?;
                    let score: f64 = row.get(2)?;
                    Ok((metadata_str, datetime, score))
                })?;

                let mut results = Vec::new();
                for row in rows {
                    let (metadata_str, datetime, score) = row?;
                    let metadata: Value =
                        serde_json::from_str(&metadata_str).unwrap_or(Value::Null);
                    let mut entry = HashMap::new();
                    entry.insert("metadata".to_string(), metadata);
                    entry.insert("datetime".to_string(), Value::String(datetime));
                    entry.insert(
                        "score".to_string(),
                        serde_json::to_value(score)?,
                    );
                    results.push(entry);
                }

                if results.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(results))
                }
            }
            Err(e) => {
                if self.verbose {
                    log::error!(
                        "MEMORY ERROR: An error occurred while querying LTM: {}",
                        e
                    );
                }
                Ok(None)
            }
        }
    }

    /// Query the LTM table by task description asynchronously.
    pub async fn aload(
        &self,
        task_description: &str,
        latest_n: usize,
    ) -> Result<Option<Vec<HashMap<String, Value>>>, anyhow::Error> {
        let db_path = self.db_path.clone();
        let task_description = task_description.to_string();
        let verbose = self.verbose;

        tokio::task::spawn_blocking(move || {
            match Connection::open(&db_path) {
                Ok(conn) => {
                    let mut stmt = conn.prepare(&format!(
                        "SELECT metadata, datetime, score
                         FROM long_term_memories
                         WHERE task_description = ?1
                         ORDER BY datetime DESC, score ASC
                         LIMIT {}",
                        latest_n
                    ))?;

                    let rows = stmt.query_map(params![&task_description], |row| {
                        let metadata_str: String = row.get(0)?;
                        let datetime: String = row.get(1)?;
                        let score: f64 = row.get(2)?;
                        Ok((metadata_str, datetime, score))
                    })?;

                    let mut results = Vec::new();
                    for row in rows {
                        let (metadata_str, datetime, score) = row?;
                        let metadata: Value =
                            serde_json::from_str(&metadata_str).unwrap_or(Value::Null);
                        let mut entry = HashMap::new();
                        entry.insert("metadata".to_string(), metadata);
                        entry.insert("datetime".to_string(), Value::String(datetime));
                        entry.insert(
                            "score".to_string(),
                            serde_json::to_value(score).unwrap_or(Value::Null),
                        );
                        results.push(entry);
                    }

                    if results.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(results))
                    }
                }
                Err(e) => {
                    if verbose {
                        log::error!(
                            "MEMORY ERROR: An error occurred while querying LTM: {}",
                            e
                        );
                    }
                    Ok(None)
                }
            }
        })
        .await?
    }

    /// Reset the LTM table by deleting all rows.
    pub fn reset(&self) -> Result<(), anyhow::Error> {
        match Connection::open(&self.db_path) {
            Ok(conn) => {
                conn.execute("DELETE FROM long_term_memories", [])?;
                Ok(())
            }
            Err(e) => {
                if self.verbose {
                    log::error!(
                        "MEMORY ERROR: An error occurred while deleting all rows in LTM: {}",
                        e
                    );
                }
                Err(e.into())
            }
        }
    }

    /// Reset the LTM table asynchronously.
    pub async fn areset(&self) -> Result<(), anyhow::Error> {
        let db_path = self.db_path.clone();
        let verbose = self.verbose;

        tokio::task::spawn_blocking(move || {
            match Connection::open(&db_path) {
                Ok(conn) => {
                    conn.execute("DELETE FROM long_term_memories", [])?;
                    Ok(())
                }
                Err(e) => {
                    if verbose {
                        log::error!(
                            "MEMORY ERROR: An error occurred while deleting all rows in LTM: {}",
                            e
                        );
                    }
                    Err(anyhow::anyhow!(e))
                }
            }
        })
        .await?
    }
}
