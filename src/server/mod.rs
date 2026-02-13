//! HTTP server for the unified execution contract.
//!
//! Exposes crewai-rust as an HTTP service that n8n-rs and ladybug-rs can
//! delegate `crew.*` steps to.
//!
//! # Endpoints
//!
//! - `GET  /health`  — Liveness probe
//! - `POST /execute` — Execute a `crew.*` step delegation

pub mod routes;

pub use routes::{app_router, AppState};
