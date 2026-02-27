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

/// In-process POST dispatch for MCP / single-binary mode.
///
/// Fast path for `/execute` calls `execute_step` directly (zero HTTP overhead).
/// All other routes (chat, barrier, a2a, modules management) delegate to the
/// full Axum router so every endpoint in the orchestration stack is reachable.
pub async fn handle_request_body(
    path: &str,
    body: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let path_trimmed = path.trim_start_matches('/');
    match path_trimmed {
        // Fast path — direct dispatch, no HTTP types
        "execute" => {
            let state = AppState::new();
            let request: crate::contract::types::StepDelegationRequest =
                serde_json::from_str(body)?;
            let response = routes::execute_step(&state, request)
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })?;
            Ok(serde_json::to_string(&response)?)
        }
        // All other routes: chat, barrier/*, a2a, modules/*/activate|deactivate|gate-check
        _ => route_via_axum("POST", path, Some(body)).await,
    }
}

/// In-process GET dispatch for MCP / single-binary mode.
pub async fn handle_get(
    path: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let path_trimmed = path.trim_start_matches('/');
    match path_trimmed {
        "health" => Ok(serde_json::json!({
            "status": "ok",
            "version": crate::VERSION,
            "service": "crewai-rust",
        })
        .to_string()),
        // All other routes: modules, modules/:id, barrier/*, .well-known/agent.json
        _ => route_via_axum("GET", path, None).await,
    }
}

/// Dispatch through the full Axum router for routes not in the fast path.
///
/// This ensures every endpoint in the orchestration stack (chat, barrier,
/// a2a, module management) remains reachable via MCP dispatch.
async fn route_via_axum(
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let app = app_router(AppState::new());
    let uri = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let request = match body {
        Some(b) => Request::builder()
            .method(method)
            .uri(&uri)
            .header("content-type", "application/json")
            .body(Body::from(b.to_owned()))?,
        None => Request::builder()
            .method(method)
            .uri(&uri)
            .body(Body::empty())?,
    };
    let response = app.oneshot(request).await?;
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024 * 1024).await?;
    let text = String::from_utf8(bytes.to_vec())?;
    if status.is_success() {
        Ok(text)
    } else {
        Err(format!("crewai returned {status}: {text}").into())
    }
}
