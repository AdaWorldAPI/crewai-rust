//! A2A utility functions.
//!
//! Corresponds to `crewai/a2a/utils/`.

use serde_json::Value;

/// Fetch and parse an agent card from a URL (stub).
///
/// In the full implementation, this makes an HTTP GET request to
/// `{endpoint}/.well-known/agent-card.json`.
pub async fn fetch_agent_card(endpoint: &str) -> Result<Value, String> {
    let url = format!("{}/.well-known/agent-card.json", endpoint.trim_end_matches('/'));
    // Stub: in production this would use reqwest
    Err(format!(
        "fetch_agent_card not yet implemented for URL: {}",
        url
    ))
}

/// Extract the agent name from an agent card.
pub fn get_agent_name(agent_card: &Value) -> Option<String> {
    agent_card.get("name").and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Extract skills from an agent card.
pub fn get_agent_skills(agent_card: &Value) -> Vec<Value> {
    agent_card
        .get("skills")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Build a delegation request message.
pub fn build_delegation_message(
    request_text: &str,
    context_id: Option<&str>,
    task_id: Option<&str>,
) -> Value {
    let mut msg = serde_json::Map::new();
    msg.insert("text".to_string(), Value::String(request_text.to_string()));
    if let Some(ctx) = context_id {
        msg.insert("contextId".to_string(), Value::String(ctx.to_string()));
    }
    if let Some(tid) = task_id {
        msg.insert("taskId".to_string(), Value::String(tid.to_string()));
    }
    Value::Object(msg)
}
