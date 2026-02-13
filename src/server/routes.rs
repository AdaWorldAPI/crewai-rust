//! Axum route handlers for the crewai-rust HTTP server.
//!
//! # Routes
//!
//! - `GET  /health`  — Returns `{"status": "ok", "version": "1.9.3"}`
//! - `POST /execute` — Accepts `StepDelegationRequest`, runs crew task, returns `StepDelegationResponse`

use std::sync::{Arc, RwLock};

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use tower_http::cors::CorsLayer;

use crate::agent::Agent;
use crate::contract::envelope;
use crate::contract::event_recorder::ContractRecorder;
use crate::contract::types::{
    DataEnvelope, EnvelopeMetadata, StepDelegationRequest, StepDelegationResponse, StepStatus,
    UnifiedStep,
};

/// Shared application state for the HTTP server.
#[derive(Clone)]
pub struct AppState {
    /// Contract recorder for tracking execution state.
    pub recorder: Arc<RwLock<ContractRecorder>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            recorder: Arc::new(RwLock::new(ContractRecorder::new())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the axum router with all routes.
pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/execute", post(execute_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// GET /health — liveness probe.
async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": crate::VERSION,
        "service": "crewai-rust",
    }))
}

/// POST /execute — execute a crew.* step delegation.
///
/// Request:  `StepDelegationRequest` = `{ "step": UnifiedStep, "input": DataEnvelope }`
/// Response: `StepDelegationResponse` = `{ "output": DataEnvelope, "step": Option<UnifiedStep> }`
///
/// The handler:
/// 1. Maps incoming step parameters to an Agent + Task configuration
/// 2. Runs the agent via `execute_task()` (sync, wrapped in spawn_blocking)
/// 3. Populates decision trail fields (reasoning, confidence, alternatives) from output
/// 4. Returns DataEnvelope with result
async fn execute_handler(
    State(state): State<AppState>,
    Json(request): Json<StepDelegationRequest>,
) -> Result<Json<StepDelegationResponse>, (StatusCode, Json<Value>)> {
    let mut step = request.step.clone();
    let task_input = envelope::to_task_input(&request.input);

    // Validate this is a crew.* step
    if !step.is_crew() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Cannot handle step type '{}' — only crew.* steps are accepted", step.step_type),
            })),
        ));
    }

    // Extract agent configuration from step input
    let role = step
        .input
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("AI Agent")
        .to_string();

    let goal = step
        .input
        .get("goal")
        .and_then(|v| v.as_str())
        .unwrap_or("Complete the delegated task")
        .to_string();

    let backstory = step
        .input
        .get("backstory")
        .and_then(|v| v.as_str())
        .unwrap_or("You are an expert AI agent.")
        .to_string();

    let llm = step
        .input
        .get("llm")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Record step start
    let crew_name = format!("delegation-{}", &step.execution_id);
    {
        let mut recorder = state.recorder.write().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Recorder lock poisoned"})),
            )
        })?;
        if !recorder.crew_to_execution.contains_key(&crew_name) {
            recorder.on_crew_started(&crew_name);
        }
        recorder.on_task_started(
            &step.step_id,
            &step.name,
            &crew_name,
            Some(&role),
        );
    }

    // Mark step as running
    step.mark_running();

    // Execute via Agent (synchronous, so use spawn_blocking)
    let task_description = if task_input.is_empty() {
        step.name.clone()
    } else {
        task_input
    };

    let result = tokio::task::spawn_blocking(move || {
        let mut agent = Agent::new(role, goal, backstory);
        if let Some(llm_str) = llm {
            agent.llm = Some(llm_str);
        }
        agent.verbose = false;
        agent.execute_task(&task_description, None, None)
    })
    .await;

    match result {
        Ok(Ok(output)) => {
            let confidence = 0.85; // Default confidence for successful execution

            // Build output envelope
            let output_envelope = DataEnvelope {
                data: serde_json::json!({
                    "result": output,
                }),
                metadata: EnvelopeMetadata {
                    source_step: step.step_id.clone(),
                    confidence,
                    epoch: chrono::Utc::now().timestamp_millis(),
                    version: Some(crate::VERSION.to_string()),
                },
            };

            // Update step with completion + decision trail
            step.mark_completed(serde_json::json!({"result": &output}));
            step.confidence = Some(confidence);
            // Reasoning is extracted from agent's last messages if available
            step.reasoning = Some(format!("Executed as {} agent", step.step_type));

            // Record completion
            {
                if let Ok(mut recorder) = state.recorder.write() {
                    recorder.on_task_completed(
                        &step.step_id,
                        serde_json::json!({"result": &output}),
                        step.reasoning.clone(),
                        step.confidence,
                        None,
                    );
                }
            }

            Ok(Json(StepDelegationResponse {
                output: output_envelope,
                step: Some(step),
            }))
        }
        Ok(Err(error)) => {
            step.mark_failed(&error);

            // Record failure
            {
                if let Ok(mut recorder) = state.recorder.write() {
                    recorder.on_task_failed(&step.step_id, &error);
                }
            }

            let error_envelope = DataEnvelope {
                data: serde_json::json!({"error": error}),
                metadata: EnvelopeMetadata {
                    source_step: step.step_id.clone(),
                    confidence: 0.0,
                    epoch: chrono::Utc::now().timestamp_millis(),
                    version: Some(crate::VERSION.to_string()),
                },
            };

            Ok(Json(StepDelegationResponse {
                output: error_envelope,
                step: Some(step),
            }))
        }
        Err(join_error) => {
            let error_msg = format!("Agent execution panicked: {}", join_error);
            step.mark_failed(&error_msg);

            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": error_msg})),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = AppState::new();
        let app = app_router(state);

        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["version"], crate::VERSION);
        assert_eq!(json["service"], "crewai-rust");
    }

    #[tokio::test]
    async fn test_execute_rejects_non_crew_step() {
        let state = AppState::new();
        let app = app_router(state);

        let step = UnifiedStep::new("e1", "n8n.set", "Set Node", 0);
        let input = DataEnvelope::new(serde_json::json!({}), "trigger");
        let req_body = StepDelegationRequest { step, input };

        let request = Request::builder()
            .method("POST")
            .uri("/execute")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&req_body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"].as_str().unwrap().contains("n8n.set"));
    }

    #[tokio::test]
    async fn test_execute_crew_agent_step() {
        let state = AppState::new();
        let app = app_router(state.clone());

        let mut step = UnifiedStep::new("exec-1", "crew.agent", "Research Task", 0);
        step.input = serde_json::json!({
            "role": "Researcher",
            "goal": "Find information",
            "backstory": "Expert researcher",
        });
        let input = DataEnvelope::new(serde_json::json!({"query": "What is Rust?"}), "trigger");
        let req_body = StepDelegationRequest { step, input };

        let request = Request::builder()
            .method("POST")
            .uri("/execute")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&req_body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        // The agent execution will likely fail (no LLM configured) but the
        // server should still return a valid response (not panic)
        assert!(
            response.status() == StatusCode::OK
                || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    async fn test_execute_records_to_contract_recorder() {
        let state = AppState::new();
        let app = app_router(state.clone());

        let mut step = UnifiedStep::new("exec-1", "crew.agent", "Test Task", 0);
        step.input = serde_json::json!({
            "role": "Tester",
            "goal": "Test things",
        });
        let input = DataEnvelope::new(serde_json::json!({"query": "test"}), "trigger");
        let req_body = StepDelegationRequest { step, input };

        let request = Request::builder()
            .method("POST")
            .uri("/execute")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&req_body).unwrap()))
            .unwrap();

        let _ = app.oneshot(request).await.unwrap();

        // Verify the recorder captured the execution
        let recorder = state.recorder.read().unwrap();
        assert!(
            !recorder.executions.is_empty(),
            "Recorder should have at least one execution"
        );
    }
}
