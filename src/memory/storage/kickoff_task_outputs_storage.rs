//! SQLite storage for kickoff task outputs.
//!
//! Port of crewai/memory/storage/kickoff_task_outputs_storage.py

use std::collections::HashMap;
use std::path::PathBuf;

use rusqlite::{params, Connection};
use serde_json::Value;

/// SQLite storage class for kickoff task outputs.
///
/// Stores task outputs including task_id, expected_output, output JSON,
/// task_index, inputs JSON, was_replayed flag, and timestamp.
pub struct KickoffTaskOutputsSQLiteStorage {
    /// Path to the SQLite database file.
    pub db_path: PathBuf,
}

impl KickoffTaskOutputsSQLiteStorage {
    /// Initialize the SQLite storage for kickoff task outputs.
    ///
    /// # Arguments
    /// * `db_path` - Optional path to the database file.
    ///   Defaults to `<db_storage_path>/latest_kickoff_task_outputs.db`.
    pub fn new(db_path: Option<PathBuf>) -> Result<Self, anyhow::Error> {
        let db_path = db_path.unwrap_or_else(|| {
            let base = crate::utilities::paths::db_storage_path();
            PathBuf::from(base).join("latest_kickoff_task_outputs.db")
        });

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let storage = Self { db_path };
        storage.initialize_db()?;
        Ok(storage)
    }

    /// Initialize the SQLite database and create the latest_kickoff_task_outputs table.
    fn initialize_db(&self) -> Result<(), anyhow::Error> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS latest_kickoff_task_outputs (
                task_id TEXT PRIMARY KEY,
                expected_output TEXT,
                output JSON,
                task_index INTEGER,
                inputs JSON,
                was_replayed BOOLEAN,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        Ok(())
    }

    /// Add a new task output record to the database.
    ///
    /// # Arguments
    /// * `task_id` - Unique identifier for the task.
    /// * `expected_output` - The expected output description.
    /// * `output` - Dictionary containing the task's output data.
    /// * `task_index` - Integer index of the task in the sequence.
    /// * `was_replayed` - Whether this was a replay execution.
    /// * `inputs` - Optional dictionary of input parameters.
    pub fn add(
        &self,
        task_id: &str,
        expected_output: &str,
        output: &HashMap<String, Value>,
        task_index: i64,
        was_replayed: bool,
        inputs: Option<&HashMap<String, Value>>,
    ) -> Result<(), anyhow::Error> {
        let empty_map = HashMap::new();
        let inputs = inputs.unwrap_or(&empty_map);
        let output_json = serde_json::to_string(output)?;
        let inputs_json = serde_json::to_string(inputs)?;

        let conn = Connection::open(&self.db_path)?;
        conn.execute("BEGIN TRANSACTION", [])?;
        conn.execute(
            "INSERT OR REPLACE INTO latest_kickoff_task_outputs
             (task_id, expected_output, output, task_index, inputs, was_replayed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                task_id,
                expected_output,
                output_json,
                task_index,
                inputs_json,
                was_replayed
            ],
        )?;
        conn.execute("COMMIT", [])?;
        Ok(())
    }

    /// Update an existing task output record in the database.
    ///
    /// # Arguments
    /// * `task_index` - Integer index of the task to update.
    /// * `fields` - HashMap of field names to values to update.
    pub fn update(
        &self,
        task_index: i64,
        fields: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute("BEGIN TRANSACTION", [])?;

        let mut set_clauses = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        for (key, value) in fields {
            set_clauses.push(format!("{} = ?", key));
            match value {
                Value::Object(_) => {
                    values.push(Box::new(serde_json::to_string(value)?));
                }
                Value::String(s) => {
                    values.push(Box::new(s.clone()));
                }
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        values.push(Box::new(i));
                    } else if let Some(f) = n.as_f64() {
                        values.push(Box::new(f));
                    }
                }
                Value::Bool(b) => {
                    values.push(Box::new(*b));
                }
                _ => {
                    values.push(Box::new(value.to_string()));
                }
            }
        }

        values.push(Box::new(task_index));

        let query = format!(
            "UPDATE latest_kickoff_task_outputs SET {} WHERE task_index = ?",
            set_clauses.join(", ")
        );

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            values.iter().map(|v| v.as_ref()).collect();

        let rows_affected = conn.execute(&query, params_refs.as_slice())?;
        conn.execute("COMMIT", [])?;

        if rows_affected == 0 {
            log::warn!(
                "No row found with task_index {}. No update performed.",
                task_index
            );
        }

        Ok(())
    }

    /// Load all task output records from the database.
    ///
    /// # Returns
    /// Vector of HashMaps containing task output records, ordered by task_index.
    pub fn load(&self) -> Result<Vec<HashMap<String, Value>>, anyhow::Error> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT * FROM latest_kickoff_task_outputs ORDER BY task_index",
        )?;

        let rows = stmt.query_map([], |row| {
            let task_id: String = row.get(0)?;
            let expected_output: String = row.get(1)?;
            let output_str: String = row.get(2)?;
            let task_index: i64 = row.get(3)?;
            let inputs_str: String = row.get(4)?;
            let was_replayed: bool = row.get(5)?;
            let timestamp: String = row.get(6)?;
            Ok((
                task_id,
                expected_output,
                output_str,
                task_index,
                inputs_str,
                was_replayed,
                timestamp,
            ))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (task_id, expected_output, output_str, task_index, inputs_str, was_replayed, timestamp) =
                row?;
            let output: Value =
                serde_json::from_str(&output_str).unwrap_or(Value::Null);
            let inputs: Value =
                serde_json::from_str(&inputs_str).unwrap_or(Value::Null);

            let mut entry = HashMap::new();
            entry.insert("task_id".to_string(), Value::String(task_id));
            entry.insert(
                "expected_output".to_string(),
                Value::String(expected_output),
            );
            entry.insert("output".to_string(), output);
            entry.insert(
                "task_index".to_string(),
                serde_json::to_value(task_index)?,
            );
            entry.insert("inputs".to_string(), inputs);
            entry.insert("was_replayed".to_string(), Value::Bool(was_replayed));
            entry.insert("timestamp".to_string(), Value::String(timestamp));
            results.push(entry);
        }

        Ok(results)
    }

    /// Delete all task output records from the database.
    pub fn delete_all(&self) -> Result<(), anyhow::Error> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute("BEGIN TRANSACTION", [])?;
        conn.execute("DELETE FROM latest_kickoff_task_outputs", [])?;
        conn.execute("COMMIT", [])?;
        Ok(())
    }
}
