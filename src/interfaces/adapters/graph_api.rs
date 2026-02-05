//! Microsoft Graph API adapter — connects to Microsoft 365 (mail, calendar, teams, etc.)
//!
//! This adapter enables AI agents to interact with Microsoft 365 tenants:
//! - Read and send emails
//! - Manage calendar events
//! - Access Teams channels
//! - Manage SharePoint files
//! - Interact with OneDrive
//!
//! ## Configuration
//!
//! ```yaml
//! interface:
//!   protocol: ms_graph
//!   config:
//!     tenant_id: "${AZURE_TENANT_ID}"
//!     client_id: "${AZURE_CLIENT_ID}"
//!     client_secret: "${AZURE_CLIENT_SECRET}"
//!     scopes:
//!       - "https://graph.microsoft.com/.default"
//!     api_version: "v1.0"
//! ```
//!
//! ## RBAC
//!
//! Access is gated by Microsoft Graph permissions (scopes). The PolicyEngine
//! enforces that agents can only access the scopes declared in their capability.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::super::adapter::{
    AdapterError, AdapterHealth, AdapterOperation, InterfaceAdapter,
};
use super::super::gateway::AdapterFactory;

/// Microsoft Graph API adapter
pub struct GraphApiAdapter {
    tenant_id: String,
    client_id: String,
    client_secret: String,
    scopes: Vec<String>,
    api_version: String,
    access_token: Option<String>,
    token_expires_at: Option<std::time::Instant>,
    client: Option<reqwest::Client>,
    connected: bool,
}

impl GraphApiAdapter {
    pub fn new() -> Self {
        Self {
            tenant_id: String::new(),
            client_id: String::new(),
            client_secret: String::new(),
            scopes: vec!["https://graph.microsoft.com/.default".to_string()],
            api_version: "v1.0".to_string(),
            access_token: None,
            token_expires_at: None,
            client: None,
            connected: false,
        }
    }

    /// Resolve environment variable references
    fn resolve_env(s: &str) -> String {
        if s.starts_with("${") && s.ends_with('}') {
            let var_name = &s[2..s.len() - 1];
            std::env::var(var_name).unwrap_or_default()
        } else {
            s.to_string()
        }
    }

    /// Acquire an OAuth2 access token using client credentials flow
    async fn acquire_token(&mut self) -> Result<(), AdapterError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| AdapterError::ConnectionFailed("HTTP client not initialized".to_string()))?;

        let token_url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id
        );

        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("scope", &self.scopes.join(" ")),
        ];

        let response = client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                AdapterError::AuthenticationFailed(format!("Token request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AdapterError::AuthenticationFailed(format!(
                "Token acquisition failed: {}",
                body
            )));
        }

        let token_response: Value = response
            .json()
            .await
            .map_err(|e| AdapterError::AuthenticationFailed(e.to_string()))?;

        self.access_token = token_response
            .get("access_token")
            .and_then(|v| v.as_str())
            .map(String::from);

        let expires_in = token_response
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);

        self.token_expires_at = Some(
            std::time::Instant::now() + std::time::Duration::from_secs(expires_in.saturating_sub(60)),
        );

        if self.access_token.is_none() {
            return Err(AdapterError::AuthenticationFailed(
                "No access token in response".to_string(),
            ));
        }

        Ok(())
    }

    /// Ensure we have a valid access token
    async fn ensure_token(&mut self) -> Result<(), AdapterError> {
        let needs_refresh = match self.token_expires_at {
            Some(expires) => std::time::Instant::now() >= expires,
            None => true,
        };

        if needs_refresh || self.access_token.is_none() {
            self.acquire_token().await?;
        }

        Ok(())
    }

    /// Make a Graph API request
    async fn graph_request(
        &self,
        method: &str,
        endpoint: &str,
        body: Option<&Value>,
    ) -> Result<Value, AdapterError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| AdapterError::ConnectionFailed("Not connected".to_string()))?;

        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| AdapterError::AuthenticationFailed("No access token".to_string()))?;

        let url = format!(
            "https://graph.microsoft.com/{}/{}",
            self.api_version, endpoint
        );

        let mut request = match method.to_uppercase().as_str() {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PATCH" => client.patch(&url),
            "DELETE" => client.delete(&url),
            "PUT" => client.put(&url),
            _ => {
                return Err(AdapterError::OperationNotSupported(format!(
                    "HTTP method: {}",
                    method
                )))
            }
        };

        request = request.header("Authorization", format!("Bearer {}", token));
        request = request.header("Content-Type", "application/json");

        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| AdapterError::ExecutionFailed(e.to_string()))?;

        let status = response.status().as_u16();

        if status == 401 {
            return Err(AdapterError::AuthenticationFailed(
                "Token expired or invalid".to_string(),
            ));
        }

        if status == 403 {
            return Err(AdapterError::PermissionDenied(
                "Insufficient Graph API permissions".to_string(),
            ));
        }

        let body_text = response
            .text()
            .await
            .map_err(|e| AdapterError::ExecutionFailed(e.to_string()))?;

        let body_value: Value = if body_text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&body_text).unwrap_or(Value::String(body_text))
        };

        Ok(serde_json::json!({
            "status": status,
            "body": body_value,
        }))
    }
}

#[async_trait]
impl InterfaceAdapter for GraphApiAdapter {
    fn name(&self) -> &str {
        "Microsoft Graph API"
    }

    fn protocol(&self) -> &str {
        "ms_graph"
    }

    async fn connect(&mut self, config: &HashMap<String, Value>) -> Result<(), AdapterError> {
        self.tenant_id = config
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .map(Self::resolve_env)
            .ok_or_else(|| AdapterError::InvalidConfig("tenant_id is required".to_string()))?;

        self.client_id = config
            .get("client_id")
            .and_then(|v| v.as_str())
            .map(Self::resolve_env)
            .ok_or_else(|| AdapterError::InvalidConfig("client_id is required".to_string()))?;

        self.client_secret = config
            .get("client_secret")
            .and_then(|v| v.as_str())
            .map(Self::resolve_env)
            .ok_or_else(|| {
                AdapterError::InvalidConfig("client_secret is required".to_string())
            })?;

        if let Some(scopes) = config.get("scopes").and_then(|v| v.as_array()) {
            self.scopes = scopes
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }

        self.api_version = config
            .get("api_version")
            .and_then(|v| v.as_str())
            .unwrap_or("v1.0")
            .to_string();

        self.client = Some(
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?,
        );

        // Acquire initial token
        self.acquire_token().await?;

        self.connected = true;
        Ok(())
    }

    async fn execute(&self, tool_name: &str, args: &Value) -> Result<Value, AdapterError> {
        if !self.connected {
            return Err(AdapterError::ConnectionFailed("Not connected".to_string()));
        }

        match tool_name {
            // ── Mail operations ──
            "list_messages" | "mail_list" => {
                let folder = args
                    .get("folder")
                    .and_then(|v| v.as_str())
                    .unwrap_or("inbox");
                let top = args.get("top").and_then(|v| v.as_u64()).unwrap_or(10);
                let endpoint = format!("me/mailFolders/{}/messages?$top={}", folder, top);
                self.graph_request("GET", &endpoint, None).await
            }

            "read_message" | "mail_read" => {
                let message_id = args
                    .get("message_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AdapterError::InvalidConfig("message_id is required".to_string())
                    })?;
                let endpoint = format!("me/messages/{}", message_id);
                self.graph_request("GET", &endpoint, None).await
            }

            "send_message" | "mail_send" => {
                let body = serde_json::json!({
                    "message": {
                        "subject": args.get("subject").and_then(|v| v.as_str()).unwrap_or(""),
                        "body": {
                            "contentType": "HTML",
                            "content": args.get("body").and_then(|v| v.as_str()).unwrap_or("")
                        },
                        "toRecipients": args.get("to").and_then(|v| v.as_array())
                            .map(|arr| arr.iter().map(|r| {
                                serde_json::json!({
                                    "emailAddress": {
                                        "address": r.as_str().unwrap_or("")
                                    }
                                })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default()
                    },
                    "saveToSentItems": true
                });
                self.graph_request("POST", "me/sendMail", Some(&body)).await
            }

            // ── Calendar operations ──
            "list_events" | "calendar_list" => {
                let top = args.get("top").and_then(|v| v.as_u64()).unwrap_or(10);
                let endpoint = format!("me/events?$top={}&$orderby=start/dateTime", top);
                self.graph_request("GET", &endpoint, None).await
            }

            "create_event" | "calendar_create" => {
                let body = serde_json::json!({
                    "subject": args.get("subject").and_then(|v| v.as_str()).unwrap_or(""),
                    "body": {
                        "contentType": "HTML",
                        "content": args.get("body").and_then(|v| v.as_str()).unwrap_or("")
                    },
                    "start": {
                        "dateTime": args.get("start_time").and_then(|v| v.as_str()).unwrap_or(""),
                        "timeZone": args.get("timezone").and_then(|v| v.as_str()).unwrap_or("UTC")
                    },
                    "end": {
                        "dateTime": args.get("end_time").and_then(|v| v.as_str()).unwrap_or(""),
                        "timeZone": args.get("timezone").and_then(|v| v.as_str()).unwrap_or("UTC")
                    },
                    "attendees": args.get("attendees").and_then(|v| v.as_array())
                        .map(|arr| arr.iter().map(|a| {
                            serde_json::json!({
                                "emailAddress": {
                                    "address": a.as_str().unwrap_or("")
                                },
                                "type": "required"
                            })
                        }).collect::<Vec<_>>())
                        .unwrap_or_default()
                });
                self.graph_request("POST", "me/events", Some(&body)).await
            }

            // ── Teams operations ──
            "list_teams" => {
                self.graph_request("GET", "me/joinedTeams", None).await
            }

            "list_channels" => {
                let team_id = args
                    .get("team_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AdapterError::InvalidConfig("team_id is required".to_string())
                    })?;
                let endpoint = format!("teams/{}/channels", team_id);
                self.graph_request("GET", &endpoint, None).await
            }

            "send_channel_message" => {
                let team_id = args.get("team_id").and_then(|v| v.as_str()).ok_or_else(|| {
                    AdapterError::InvalidConfig("team_id is required".to_string())
                })?;
                let channel_id = args
                    .get("channel_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AdapterError::InvalidConfig("channel_id is required".to_string())
                    })?;
                let body = serde_json::json!({
                    "body": {
                        "content": args.get("message").and_then(|v| v.as_str()).unwrap_or("")
                    }
                });
                let endpoint = format!("teams/{}/channels/{}/messages", team_id, channel_id);
                self.graph_request("POST", &endpoint, Some(&body)).await
            }

            // ── User/Directory operations ──
            "get_user" => {
                let user_id = args
                    .get("user_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("me");
                let endpoint = format!("users/{}", user_id);
                self.graph_request("GET", &endpoint, None).await
            }

            "list_users" => {
                let top = args.get("top").and_then(|v| v.as_u64()).unwrap_or(25);
                let endpoint = format!("users?$top={}", top);
                self.graph_request("GET", &endpoint, None).await
            }

            // ── OneDrive operations ──
            "list_files" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("root");
                let endpoint = if path == "root" {
                    "me/drive/root/children".to_string()
                } else {
                    format!("me/drive/root:/{}:/children", path)
                };
                self.graph_request("GET", &endpoint, None).await
            }

            // ── Generic Graph API call ──
            "graph_request" => {
                let method = args
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("GET");
                let endpoint = args
                    .get("endpoint")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AdapterError::InvalidConfig("endpoint is required".to_string())
                    })?;
                let body = args.get("body");
                self.graph_request(method, endpoint, body).await
            }

            _ => Err(AdapterError::OperationNotSupported(format!(
                "Unknown Graph API operation: {}",
                tool_name
            ))),
        }
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
        self.access_token = None;
        self.token_expires_at = None;
        self.client = None;
        self.connected = false;
        Ok(())
    }

    async fn health_check(&self) -> Result<AdapterHealth, AdapterError> {
        if !self.connected {
            return Ok(AdapterHealth {
                connected: false,
                latency_ms: None,
                message: "Not connected".to_string(),
            });
        }

        let start = std::time::Instant::now();
        match self.graph_request("GET", "me", None).await {
            Ok(resp) => Ok(AdapterHealth {
                connected: true,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: format!(
                    "Connected to tenant {}",
                    resp.get("body")
                        .and_then(|b| b.get("displayName"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown")
                ),
            }),
            Err(e) => Ok(AdapterHealth {
                connected: false,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: e.to_string(),
            }),
        }
    }

    fn supported_operations(&self) -> Vec<AdapterOperation> {
        vec![
            AdapterOperation {
                name: "list_messages".to_string(),
                description: "List emails in a folder".to_string(),
                read_only: true,
                idempotent: true,
            },
            AdapterOperation {
                name: "read_message".to_string(),
                description: "Read a specific email".to_string(),
                read_only: true,
                idempotent: true,
            },
            AdapterOperation {
                name: "send_message".to_string(),
                description: "Send an email".to_string(),
                read_only: false,
                idempotent: false,
            },
            AdapterOperation {
                name: "list_events".to_string(),
                description: "List calendar events".to_string(),
                read_only: true,
                idempotent: true,
            },
            AdapterOperation {
                name: "create_event".to_string(),
                description: "Create a calendar event".to_string(),
                read_only: false,
                idempotent: false,
            },
            AdapterOperation {
                name: "list_teams".to_string(),
                description: "List joined Teams".to_string(),
                read_only: true,
                idempotent: true,
            },
            AdapterOperation {
                name: "list_channels".to_string(),
                description: "List channels in a Team".to_string(),
                read_only: true,
                idempotent: true,
            },
            AdapterOperation {
                name: "send_channel_message".to_string(),
                description: "Send a message to a Teams channel".to_string(),
                read_only: false,
                idempotent: false,
            },
            AdapterOperation {
                name: "get_user".to_string(),
                description: "Get user profile information".to_string(),
                read_only: true,
                idempotent: true,
            },
            AdapterOperation {
                name: "list_users".to_string(),
                description: "List directory users".to_string(),
                read_only: true,
                idempotent: true,
            },
            AdapterOperation {
                name: "list_files".to_string(),
                description: "List OneDrive files".to_string(),
                read_only: true,
                idempotent: true,
            },
            AdapterOperation {
                name: "graph_request".to_string(),
                description: "Make a generic Graph API request".to_string(),
                read_only: false,
                idempotent: false,
            },
        ]
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Factory for creating Microsoft Graph API adapters
pub struct GraphApiAdapterFactory;

#[async_trait]
impl AdapterFactory for GraphApiAdapterFactory {
    fn create(&self) -> Box<dyn InterfaceAdapter> {
        Box::new(GraphApiAdapter::new())
    }

    fn protocol(&self) -> &str {
        "ms_graph"
    }
}
