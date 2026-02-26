//! HTTP server for the unified execution contract.
//!
//! Exposes crewai-rust as an HTTP service that n8n-rs and ladybug-rs can
//! delegate `crew.*` steps to.
//!
//! # Endpoints
//!
//! - `GET  /health`  — Liveness probe
//! - `POST /execute` — Execute a `crew.*` step delegation
//! - `POST /barrier/check-outbound` — 4-layer barrier check (outbound)
//! - `POST /barrier/check-inbound`  — 4-layer barrier check (inbound)
//! - `GET  /barrier/topology`       — Triune facet intensities
//! - `POST /barrier/feedback`       — Success/failure feedback
//! - `GET  /barrier/stats`          — Markov barrier statistics
//! - `GET  /.well-known/agent.json` — A2A agent card discovery
//! - `POST /a2a`                    — A2A JSON-RPC 2.0 dispatcher

pub mod a2a_routes;
pub mod barrier_routes;
pub mod routes;

pub use a2a_routes::{a2a_router, A2AState};
pub use barrier_routes::{barrier_router, BarrierState};
pub use routes::{app_router, AppState};
