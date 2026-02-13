//! PostgreSQL persistence for unified executions and steps.
//!
//! Requires the `postgres` feature flag:
//! ```toml
//! [dependencies]
//! crewai = { features = ["postgres"] }
//! ```

#[cfg(feature = "postgres")]
mod inner {
    use crate::contract::types::{StepStatus, UnifiedExecution, UnifiedStep};
    use sqlx::PgPool;
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum PgStoreError {
        #[error("Database error: {0}")]
        Sqlx(#[from] sqlx::Error),
    }

    /// PostgreSQL store for unified execution data.
    #[derive(Clone)]
    pub struct PgStore {
        pool: PgPool,
    }

    impl PgStore {
        pub fn new(pool: PgPool) -> Self {
            Self { pool }
        }

        /// Run migrations to create/update the unified_executions and unified_steps tables.
        pub async fn migrate(&self) -> Result<(), PgStoreError> {
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS unified_executions (
                    execution_id TEXT PRIMARY KEY,
                    workflow_name TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending',
                    started_at TIMESTAMPTZ,
                    finished_at TIMESTAMPTZ,
                    fork_id TEXT,
                    fork_parent TEXT
                )
                "#,
            )
            .execute(&self.pool)
            .await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS unified_steps (
                    step_id TEXT PRIMARY KEY,
                    execution_id TEXT NOT NULL REFERENCES unified_executions(execution_id),
                    step_type TEXT NOT NULL,
                    name TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending',
                    sequence INTEGER NOT NULL DEFAULT 0,
                    input JSONB NOT NULL DEFAULT 'null'::jsonb,
                    output JSONB NOT NULL DEFAULT 'null'::jsonb,
                    error TEXT,
                    started_at TIMESTAMPTZ,
                    finished_at TIMESTAMPTZ,
                    reasoning TEXT,
                    confidence REAL,
                    alternatives JSONB
                )
                "#,
            )
            .execute(&self.pool)
            .await?;

            // Additive migrations for existing tables (idempotent).
            sqlx::query(
                r#"
                ALTER TABLE unified_executions ADD COLUMN IF NOT EXISTS fork_id TEXT;
                ALTER TABLE unified_executions ADD COLUMN IF NOT EXISTS fork_parent TEXT;
                ALTER TABLE unified_steps ADD COLUMN IF NOT EXISTS reasoning TEXT;
                ALTER TABLE unified_steps ADD COLUMN IF NOT EXISTS confidence REAL;
                ALTER TABLE unified_steps ADD COLUMN IF NOT EXISTS alternatives JSONB;
                "#,
            )
            .execute(&self.pool)
            .await?;

            log::debug!("Unified contract tables migrated");
            Ok(())
        }

        /// Insert or upsert an execution.
        pub async fn write_execution(&self, exec: &UnifiedExecution) -> Result<(), PgStoreError> {
            sqlx::query(
                r#"
                INSERT INTO unified_executions
                    (execution_id, workflow_name, status, started_at, finished_at, fork_id, fork_parent)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (execution_id) DO UPDATE SET
                    status = EXCLUDED.status,
                    started_at = EXCLUDED.started_at,
                    finished_at = EXCLUDED.finished_at,
                    fork_id = EXCLUDED.fork_id,
                    fork_parent = EXCLUDED.fork_parent
                "#,
            )
            .bind(&exec.execution_id)
            .bind(&exec.workflow_name)
            .bind(status_to_str(exec.status))
            .bind(exec.started_at)
            .bind(exec.finished_at)
            .bind(&exec.fork_id)
            .bind(&exec.fork_parent)
            .execute(&self.pool)
            .await?;

            Ok(())
        }

        /// Insert or upsert a step.
        pub async fn write_step(&self, step: &UnifiedStep) -> Result<(), PgStoreError> {
            sqlx::query(
                r#"
                INSERT INTO unified_steps
                    (step_id, execution_id, step_type, name, status, sequence,
                     input, output, error, started_at, finished_at,
                     reasoning, confidence, alternatives)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                ON CONFLICT (step_id) DO UPDATE SET
                    status = EXCLUDED.status,
                    output = EXCLUDED.output,
                    error = EXCLUDED.error,
                    started_at = EXCLUDED.started_at,
                    finished_at = EXCLUDED.finished_at,
                    reasoning = EXCLUDED.reasoning,
                    confidence = EXCLUDED.confidence,
                    alternatives = EXCLUDED.alternatives
                "#,
            )
            .bind(&step.step_id)
            .bind(&step.execution_id)
            .bind(&step.step_type)
            .bind(&step.name)
            .bind(status_to_str(step.status))
            .bind(step.sequence)
            .bind(&step.input)
            .bind(&step.output)
            .bind(&step.error)
            .bind(step.started_at)
            .bind(step.finished_at)
            .bind(&step.reasoning)
            .bind(step.confidence.map(|c| c as f32))
            .bind(&step.alternatives)
            .execute(&self.pool)
            .await?;

            Ok(())
        }

        /// Update just the status of an execution.
        pub async fn update_status(
            &self,
            execution_id: &str,
            status: StepStatus,
        ) -> Result<(), PgStoreError> {
            let now = chrono::Utc::now();
            let finished = matches!(status, StepStatus::Completed | StepStatus::Failed);

            sqlx::query(
                r#"
                UPDATE unified_executions
                SET status = $1,
                    finished_at = CASE WHEN $2 THEN $3 ELSE finished_at END
                WHERE execution_id = $4
                "#,
            )
            .bind(status_to_str(status))
            .bind(finished)
            .bind(now)
            .bind(execution_id)
            .execute(&self.pool)
            .await?;

            Ok(())
        }
    }

    fn status_to_str(s: StepStatus) -> &'static str {
        match s {
            StepStatus::Pending => "pending",
            StepStatus::Running => "running",
            StepStatus::Completed => "completed",
            StepStatus::Failed => "failed",
            StepStatus::Skipped => "skipped",
        }
    }
}

#[cfg(feature = "postgres")]
pub use inner::*;
