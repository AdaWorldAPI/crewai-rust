//! crewai-rust HTTP server binary.
//!
//! Starts an axum HTTP server that exposes the unified execution contract
//! endpoints for integration with n8n-rs and ladybug-rs.
//!
//! # Environment Variables
//!
//! - `PORT` — HTTP port (default: 8080)
//! - `CREWAI_STORE` — Storage backend: "memory" (default) or "postgres"
//! - `DATABASE_URL` — PostgreSQL connection string (required if CREWAI_STORE=postgres)
//! - `RUST_LOG` — Tracing filter (default: "info")
//!
//! # Usage
//!
//! ```bash
//! cargo run --bin server
//! # or with postgres:
//! cargo run --bin server --features postgres
//! ```

use crewai::server::{app_router, AppState};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,crewai=debug".into()),
        )
        .init();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_addr = format!("0.0.0.0:{}", port);

    // Build app state
    let state = AppState::new();

    // Optional: PostgreSQL migration
    #[cfg(feature = "postgres")]
    {
        if std::env::var("CREWAI_STORE").as_deref() == Ok("postgres") {
            if let Ok(database_url) = std::env::var("DATABASE_URL") {
                tracing::info!("Connecting to PostgreSQL...");
                match sqlx::PgPool::connect(&database_url).await {
                    Ok(pool) => {
                        let store = crewai::contract::pg_store::PgStore::new(pool);
                        if let Err(e) = store.migrate().await {
                            tracing::error!("Failed to run migrations: {}", e);
                        } else {
                            tracing::info!("PostgreSQL migrations complete");
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to connect to PostgreSQL: {}", e);
                    }
                }
            } else {
                tracing::warn!("CREWAI_STORE=postgres but DATABASE_URL not set");
            }
        }
    }

    let app = app_router(state);

    tracing::info!("crewai-rust server starting on {}", bind_addr);
    tracing::info!("Endpoints:");
    tracing::info!("  GET  /health  — liveness probe");
    tracing::info!("  POST /execute — crew.* step delegation");

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}
