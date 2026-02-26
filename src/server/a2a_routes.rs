//! A2A (Agent-to-Agent) protocol server endpoints.
//!
//! Implements the server-side A2A JSON-RPC protocol:
//!
//! - `GET  /.well-known/agent.json` — Agent card discovery
//! - `POST /a2a`                    — JSON-RPC 2.0 dispatcher
//!
//! Supported JSON-RPC methods:
//! - `message/send`   — Send a message, get a task back
//! - `tasks/get`      — Get task status by ID
//! - `tasks/cancel`   — Cancel a running task

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;

use crate::a2a::client::{
    A2AMessage, A2ATask, A2ATaskState, A2ATaskStatus, AgentCapabilities, AgentCard, AgentProvider,
    AgentSkill,
};
use crate::a2a::errors::{A2AErrorCode, create_error_response};
use crate::a2a::types::PartsDict;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// In-memory task store for the A2A server.
#[derive(Clone)]
pub struct A2AState {
    /// Active tasks keyed by task ID.
    pub tasks: Arc<RwLock<HashMap<String, A2ATask>>>,
}

impl A2AState {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for A2AState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the A2A router with agent card and JSON-RPC endpoints.
pub fn a2a_router(state: A2AState) -> Router {
    Router::new()
        .route("/.well-known/agent.json", get(agent_card_handler))
        .route("/a2a", post(jsonrpc_handler))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// GET /.well-known/agent.json
// ---------------------------------------------------------------------------

/// Serve the agent card for discovery.
async fn agent_card_handler() -> impl IntoResponse {
    let card = build_agent_card();
    Json(serde_json::to_value(card).unwrap_or_default())
}

fn build_agent_card() -> AgentCard {
    AgentCard {
        name: "crewai-rust".to_string(),
        description: Some(
            "Rust-native CrewAI agent server with blood-brain barrier, \
             triune persona, Markov gating, and modular crew execution."
                .to_string(),
        ),
        url: "https://crewai-rust.up.railway.app/a2a".to_string(),
        version: Some(crate::VERSION.to_string()),
        capabilities: AgentCapabilities {
            streaming: false,
            push_notifications: false,
            multi_turn: true,
        },
        skills: vec![
            AgentSkill {
                id: "crew.execute".to_string(),
                name: "Crew Execution".to_string(),
                description: Some(
                    "Execute a crew.* step delegation with agent role, goal, and backstory"
                        .to_string(),
                ),
                input_modes: vec!["application/json".to_string()],
                output_modes: vec!["application/json".to_string()],
                tags: vec!["crew".to_string(), "agent".to_string(), "execution".to_string()],
            },
            AgentSkill {
                id: "barrier.check".to_string(),
                name: "Barrier Check".to_string(),
                description: Some(
                    "4-layer blood-brain barrier: NARS truth, Markov chain, \
                     triune persona, MUL gate"
                        .to_string(),
                ),
                input_modes: vec!["application/json".to_string()],
                output_modes: vec!["application/json".to_string()],
                tags: vec![
                    "barrier".to_string(),
                    "nars".to_string(),
                    "triune".to_string(),
                    "safety".to_string(),
                ],
            },
            AgentSkill {
                id: "barrier.topology".to_string(),
                name: "Triune Topology".to_string(),
                description: Some(
                    "Guardian/Driver/Catalyst facet intensities, strategy, and balance"
                        .to_string(),
                ),
                input_modes: vec!["application/json".to_string()],
                output_modes: vec!["application/json".to_string()],
                tags: vec!["barrier".to_string(), "triune".to_string(), "topology".to_string()],
            },
            AgentSkill {
                id: "chat".to_string(),
                name: "Chat".to_string(),
                description: Some(
                    "Conversational agent with identity seed and LLM backend".to_string(),
                ),
                input_modes: vec!["text/plain".to_string()],
                output_modes: vec!["text/plain".to_string()],
                tags: vec!["chat".to_string(), "conversation".to_string()],
            },
        ],
        provider: Some(AgentProvider {
            organization: "AdaWorldAPI".to_string(),
            url: Some("https://github.com/AdaWorldAPI".to_string()),
        }),
        default_input_modes: vec!["text/plain".to_string(), "application/json".to_string()],
        default_output_modes: vec!["text/plain".to_string(), "application/json".to_string()],
        security_schemes: vec![],
        extensions: vec![],
    }
}

// ---------------------------------------------------------------------------
// POST /a2a — JSON-RPC 2.0 dispatcher
// ---------------------------------------------------------------------------

async fn jsonrpc_handler(
    State(state): State<A2AState>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let request_id = body.get("id").cloned();

    // Validate JSON-RPC envelope
    let jsonrpc = body.get("jsonrpc").and_then(|v| v.as_str()).unwrap_or("");
    if jsonrpc != "2.0" {
        return Json(create_error_response(
            A2AErrorCode::InvalidRequest,
            Some("Missing or invalid jsonrpc version — expected \"2.0\""),
            None,
            request_id,
        ));
    }

    let method = match body.get("method").and_then(|v| v.as_str()) {
        Some(m) => m,
        None => {
            return Json(create_error_response(
                A2AErrorCode::InvalidRequest,
                Some("Missing \"method\" field"),
                None,
                request_id,
            ));
        }
    };

    let params = body.get("params").cloned().unwrap_or(Value::Object(Default::default()));

    match method {
        "message/send" => handle_message_send(&state, params, request_id),
        "tasks/get" => handle_tasks_get(&state, params, request_id),
        "tasks/cancel" => handle_tasks_cancel(&state, params, request_id),
        _ => Json(create_error_response(
            A2AErrorCode::MethodNotFound,
            Some(&format!("Unknown method: {}", method)),
            None,
            request_id,
        )),
    }
}

// ---------------------------------------------------------------------------
// method: message/send
// ---------------------------------------------------------------------------

fn handle_message_send(
    state: &A2AState,
    params: Value,
    request_id: Option<Value>,
) -> Json<Value> {
    // Parse the message from params
    let message = match params.get("message") {
        Some(msg_val) => {
            let role = msg_val
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("user")
                .to_string();
            let parts = msg_val
                .get("parts")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|p| {
                            p.get("text").and_then(|t| t.as_str()).map(|text| PartsDict {
                                text: text.to_string(),
                                metadata: None,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let metadata = msg_val
                .get("metadata")
                .and_then(|v| serde_json::from_value::<HashMap<String, Value>>(v.clone()).ok());

            A2AMessage {
                role,
                parts,
                metadata,
            }
        }
        None => {
            return Json(create_error_response(
                A2AErrorCode::InvalidParams,
                Some("Missing \"message\" in params"),
                None,
                request_id,
            ));
        }
    };

    // Generate task ID
    let task_id = uuid::Uuid::new_v4().to_string();
    let context_id = params
        .get("context_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Extract the text content for processing
    let input_text = message
        .parts
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // Process the message — for now, echo acknowledgment + barrier status hint
    let response_text = format!(
        "Received: \"{}\". Task {} created. \
         Use tasks/get to poll status or send follow-up messages.",
        truncate(&input_text, 120),
        &task_id[..8],
    );

    let response_message = A2AMessage {
        role: "agent".to_string(),
        parts: vec![PartsDict {
            text: response_text,
            metadata: None,
        }],
        metadata: None,
    };

    // Build the task
    let now = chrono::Utc::now().to_rfc3339();
    let task = A2ATask {
        id: task_id.clone(),
        context_id: context_id.clone(),
        status: A2ATaskStatus {
            state: A2ATaskState::Completed,
            message: Some(response_message.clone()),
            timestamp: Some(now),
        },
        history: vec![message, response_message],
        artifacts: vec![],
        metadata: None,
    };

    // Store the task
    if let Ok(mut tasks) = state.tasks.write() {
        tasks.insert(task_id.clone(), task.clone());
    }

    // Return JSON-RPC success
    let result = serde_json::to_value(&task).unwrap_or_default();
    Json(serde_json::json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "result": result,
    }))
}

// ---------------------------------------------------------------------------
// method: tasks/get
// ---------------------------------------------------------------------------

fn handle_tasks_get(
    state: &A2AState,
    params: Value,
    request_id: Option<Value>,
) -> Json<Value> {
    let task_id = match params.get("task_id").or_else(|| params.get("id")).and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return Json(create_error_response(
                A2AErrorCode::InvalidParams,
                Some("Missing \"task_id\" in params"),
                None,
                request_id,
            ));
        }
    };

    let tasks = match state.tasks.read() {
        Ok(t) => t,
        Err(_) => {
            return Json(create_error_response(
                A2AErrorCode::InternalError,
                Some("Task store lock poisoned"),
                None,
                request_id,
            ));
        }
    };

    match tasks.get(&task_id) {
        Some(task) => {
            let result = serde_json::to_value(task).unwrap_or_default();
            Json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": result,
            }))
        }
        None => Json(create_error_response(
            A2AErrorCode::TaskNotFound,
            Some(&format!("Task {} not found", task_id)),
            None,
            request_id,
        )),
    }
}

// ---------------------------------------------------------------------------
// method: tasks/cancel
// ---------------------------------------------------------------------------

fn handle_tasks_cancel(
    state: &A2AState,
    params: Value,
    request_id: Option<Value>,
) -> Json<Value> {
    let task_id = match params.get("task_id").or_else(|| params.get("id")).and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return Json(create_error_response(
                A2AErrorCode::InvalidParams,
                Some("Missing \"task_id\" in params"),
                None,
                request_id,
            ));
        }
    };

    let mut tasks = match state.tasks.write() {
        Ok(t) => t,
        Err(_) => {
            return Json(create_error_response(
                A2AErrorCode::InternalError,
                Some("Task store lock poisoned"),
                None,
                request_id,
            ));
        }
    };

    match tasks.get_mut(&task_id) {
        Some(task) => {
            // Only cancel if not already in a terminal state
            match task.status.state {
                A2ATaskState::Completed | A2ATaskState::Failed | A2ATaskState::Canceled => {
                    return Json(create_error_response(
                        A2AErrorCode::TaskNotCancelable,
                        Some(&format!(
                            "Task {} is already in terminal state: {:?}",
                            task_id, task.status.state
                        )),
                        None,
                        request_id,
                    ));
                }
                _ => {}
            }

            task.status.state = A2ATaskState::Canceled;
            task.status.timestamp = Some(chrono::Utc::now().to_rfc3339());
            task.status.message = Some(A2AMessage {
                role: "agent".to_string(),
                parts: vec![PartsDict {
                    text: "Task canceled".to_string(),
                    metadata: None,
                }],
                metadata: None,
            });

            let result = serde_json::to_value(&*task).unwrap_or_default();
            Json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": result,
            }))
        }
        None => Json(create_error_response(
            A2AErrorCode::TaskNotFound,
            Some(&format!("Task {} not found", task_id)),
            None,
            request_id,
        )),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
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

    fn test_state() -> A2AState {
        A2AState::new()
    }

    #[tokio::test]
    async fn test_agent_card() {
        let app = a2a_router(test_state());
        let req = Request::builder()
            .uri("/.well-known/agent.json")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["name"], "crewai-rust");
        assert_eq!(json["url"], "https://crewai-rust.up.railway.app/a2a");
        assert!(json["skills"].as_array().unwrap().len() >= 3);
    }

    #[tokio::test]
    async fn test_message_send() {
        let app = a2a_router(test_state());
        let rpc = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "test-1",
            "method": "message/send",
            "params": {
                "message": {
                    "role": "user",
                    "parts": [{"text": "hello from A2A test"}]
                }
            }
        });

        let req = Request::builder()
            .method("POST")
            .uri("/a2a")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&rpc).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert!(json.get("error").is_none());
        assert_eq!(json["result"]["status"]["state"], "completed");
    }

    #[tokio::test]
    async fn test_tasks_get_not_found() {
        let app = a2a_router(test_state());
        let rpc = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "test-2",
            "method": "tasks/get",
            "params": { "task_id": "nonexistent" }
        });

        let req = Request::builder()
            .method("POST")
            .uri("/a2a")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&rpc).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("error").is_some());
        assert_eq!(json["error"]["code"], -32001); // TaskNotFound
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let app = a2a_router(test_state());
        let rpc = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "test-3",
            "method": "bogus/method",
            "params": {}
        });

        let req = Request::builder()
            .method("POST")
            .uri("/a2a")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&rpc).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("error").is_some());
        assert_eq!(json["error"]["code"], -32601); // MethodNotFound
    }
}
