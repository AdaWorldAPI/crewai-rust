//! REST API adapter — connects to any HTTP REST endpoint via OpenAPI spec.
//!
//! This is the most general adapter: given an OpenAPI spec (or just a base URL
//! and endpoint mappings), it can call any REST API.
//!
//! ## Configuration
//!
//! ```yaml
//! interface:
//!   protocol: rest_api
//!   config:
//!     base_url: "https://api.example.com/v1"
//!     auth_header: "Authorization"
//!     auth_prefix: "Bearer"
//!     timeout_ms: 30000
//!     openapi_spec_url: "https://api.example.com/openapi.json"  # optional
//! ```

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::super::adapter::{
    AdapterError, AdapterHealth, AdapterOperation, InterfaceAdapter,
};
use super::super::gateway::AdapterFactory;

/// REST API adapter
pub struct RestApiAdapter {
    base_url: Option<String>,
    auth_header: Option<String>,
    auth_value: Option<String>,
    timeout_ms: u64,
    client: Option<reqwest::Client>,
    endpoints: HashMap<String, EndpointConfig>,
    connected: bool,
}

/// Configuration for a specific REST endpoint
#[derive(Debug, Clone)]
struct EndpointConfig {
    method: String,
    path: String,
    description: String,
    read_only: bool,
}

impl RestApiAdapter {
    pub fn new() -> Self {
        Self {
            base_url: None,
            auth_header: None,
            auth_value: None,
            timeout_ms: 30000,
            client: None,
            endpoints: HashMap::new(),
            connected: false,
        }
    }
}

#[async_trait]
impl InterfaceAdapter for RestApiAdapter {
    fn name(&self) -> &str {
        "REST API"
    }

    fn protocol(&self) -> &str {
        "rest_api"
    }

    async fn connect(&mut self, config: &HashMap<String, Value>) -> Result<(), AdapterError> {
        self.base_url = config
            .get("base_url")
            .and_then(|v| v.as_str())
            .map(String::from);

        if self.base_url.is_none() {
            return Err(AdapterError::InvalidConfig(
                "base_url is required for REST API adapter".to_string(),
            ));
        }

        self.auth_header = config
            .get("auth_header")
            .and_then(|v| v.as_str())
            .map(String::from);

        let auth_prefix = config
            .get("auth_prefix")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let auth_token = config
            .get("auth_token")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if !auth_prefix.is_empty() && !auth_token.is_empty() {
            self.auth_value = Some(format!("{} {}", auth_prefix, auth_token));
        } else if !auth_token.is_empty() {
            self.auth_value = Some(auth_token.to_string());
        }

        self.timeout_ms = config
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(30000);

        // Load endpoint mappings if provided
        if let Some(endpoints) = config.get("endpoints").and_then(|v| v.as_object()) {
            for (name, ep_config) in endpoints {
                if let Some(ep) = ep_config.as_object() {
                    let method = ep
                        .get("method")
                        .and_then(|v| v.as_str())
                        .unwrap_or("GET")
                        .to_string();
                    let path = ep
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let description = ep
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let read_only = ep
                        .get("read_only")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(method == "GET");

                    self.endpoints.insert(
                        name.clone(),
                        EndpointConfig {
                            method,
                            path,
                            description,
                            read_only,
                        },
                    );
                }
            }
        }

        self.client = Some(
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(self.timeout_ms))
                .build()
                .map_err(|e| AdapterError::ConnectionFailed(e.to_string()))?,
        );

        self.connected = true;
        Ok(())
    }

    async fn execute(&self, tool_name: &str, args: &Value) -> Result<Value, AdapterError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| AdapterError::ConnectionFailed("Not connected".to_string()))?;

        let base_url = self
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::InvalidConfig("No base URL".to_string()))?;

        // Look up endpoint config, or construct from args
        let (method, path) = if let Some(ep) = self.endpoints.get(tool_name) {
            (ep.method.clone(), ep.path.clone())
        } else {
            // Generic: use args to specify method and path
            let method = args
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("GET")
                .to_string();
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("/")
                .to_string();
            (method, path)
        };

        // Substitute path parameters from args
        let mut resolved_path = path.clone();
        if let Some(params) = args.get("path_params").and_then(|v| v.as_object()) {
            for (key, val) in params {
                if let Some(s) = val.as_str() {
                    resolved_path = resolved_path.replace(&format!("{{{}}}", key), s);
                }
            }
        }

        let url = format!("{}{}", base_url, resolved_path);

        let mut request = match method.to_uppercase().as_str() {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            "PATCH" => client.patch(&url),
            "DELETE" => client.delete(&url),
            _ => {
                return Err(AdapterError::OperationNotSupported(format!(
                    "HTTP method: {}",
                    method
                )))
            }
        };

        // Add auth header
        if let (Some(header), Some(value)) = (&self.auth_header, &self.auth_value) {
            request = request.header(header.as_str(), value.as_str());
        }

        // Add query params
        if let Some(query) = args.get("query_params").and_then(|v| v.as_object()) {
            let params: Vec<(String, String)> = query
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect();
            request = request.query(&params);
        }

        // Add body
        if let Some(body) = args.get("body") {
            request = request.json(body);
        }

        // Add custom headers
        if let Some(headers) = args.get("headers").and_then(|v| v.as_object()) {
            for (key, val) in headers {
                if let Some(s) = val.as_str() {
                    request = request.header(key.as_str(), s);
                }
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| AdapterError::ExecutionFailed(e.to_string()))?;

        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|e| AdapterError::ExecutionFailed(e.to_string()))?;

        // Parse response body as JSON if possible
        let body_value: Value = serde_json::from_str(&body).unwrap_or_else(|_| {
            Value::String(body)
        });

        Ok(serde_json::json!({
            "status": status,
            "body": body_value,
        }))
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
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
        let base_url = self
            .base_url
            .as_ref()
            .ok_or_else(|| AdapterError::InvalidConfig("No base URL".to_string()))?;

        let client = self
            .client
            .as_ref()
            .ok_or_else(|| AdapterError::ConnectionFailed("Not connected".to_string()))?;

        match client.head(base_url.as_str()).send().await {
            Ok(resp) => Ok(AdapterHealth {
                connected: true,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: format!("HTTP {}", resp.status()),
            }),
            Err(e) => Ok(AdapterHealth {
                connected: false,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: e.to_string(),
            }),
        }
    }

    fn supported_operations(&self) -> Vec<AdapterOperation> {
        self.endpoints
            .iter()
            .map(|(name, ep)| AdapterOperation {
                name: name.clone(),
                description: ep.description.clone(),
                read_only: ep.read_only,
                idempotent: ep.method == "GET" || ep.method == "PUT",
            })
            .collect()
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Factory for creating REST API adapters
pub struct RestApiAdapterFactory;

#[async_trait]
impl AdapterFactory for RestApiAdapterFactory {
    fn create(&self) -> Box<dyn InterfaceAdapter> {
        Box::new(RestApiAdapter::new())
    }

    fn protocol(&self) -> &str {
        "rest_api"
    }
}
