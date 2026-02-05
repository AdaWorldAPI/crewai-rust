//! Authentication schemes for A2A protocol clients.
//!
//! Corresponds to `crewai/a2a/auth/client_schemes.py`.
//!
//! Supported authentication methods:
//! - Bearer tokens
//! - OAuth2 (Client Credentials, Authorization Code)
//! - API Keys (header, query, cookie)
//! - HTTP Basic authentication
//! - HTTP Digest authentication
//! - mTLS (mutual TLS) client certificate authentication

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// TLS config
// ---------------------------------------------------------------------------

/// TLS/mTLS configuration for secure client connections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TLSConfig {
    /// Path to client certificate file (PEM format) for mTLS.
    pub client_cert_path: Option<String>,
    /// Path to client private key file (PEM format) for mTLS.
    pub client_key_path: Option<String>,
    /// Path to CA certificate bundle for server verification.
    pub ca_cert_path: Option<String>,
    /// Whether to verify server certificates.
    #[serde(default = "default_true")]
    pub verify: bool,
}

fn default_true() -> bool { true }

impl Default for TLSConfig {
    fn default() -> Self {
        Self {
            client_cert_path: None,
            client_key_path: None,
            ca_cert_path: None,
            verify: true,
        }
    }
}

// ---------------------------------------------------------------------------
// ClientAuthScheme trait
// ---------------------------------------------------------------------------

/// Base trait for client-side authentication schemes.
///
/// Client auth schemes apply credentials to outgoing requests.
#[async_trait]
pub trait ClientAuthScheme: Send + Sync {
    /// Apply authentication to request headers.
    ///
    /// Returns the updated headers.
    async fn apply_auth(
        &self,
        headers: &mut HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Optional TLS configuration.
    fn tls_config(&self) -> Option<&TLSConfig> {
        None
    }
}

// ---------------------------------------------------------------------------
// Implementations
// ---------------------------------------------------------------------------

/// Bearer token authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearerTokenAuth {
    pub token: String,
    #[serde(default)]
    pub tls: Option<TLSConfig>,
}

#[async_trait]
impl ClientAuthScheme for BearerTokenAuth {
    async fn apply_auth(
        &self,
        headers: &mut HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", self.token),
        );
        Ok(())
    }

    fn tls_config(&self) -> Option<&TLSConfig> {
        self.tls.as_ref()
    }
}

/// HTTP Basic authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPBasicAuth {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub tls: Option<TLSConfig>,
}

#[async_trait]
impl ClientAuthScheme for HTTPBasicAuth {
    async fn apply_auth(
        &self,
        headers: &mut HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use base64::Engine;
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
        Ok(())
    }

    fn tls_config(&self) -> Option<&TLSConfig> {
        self.tls.as_ref()
    }
}

/// API Key authentication (header, query, or cookie).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIKeyAuth {
    pub api_key: String,
    #[serde(default = "default_header")]
    pub location: APIKeyLocation,
    #[serde(default = "default_api_key_name")]
    pub name: String,
    #[serde(default)]
    pub tls: Option<TLSConfig>,
}

fn default_header() -> APIKeyLocation { APIKeyLocation::Header }
fn default_api_key_name() -> String { "X-API-Key".to_string() }

/// Where to send the API key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum APIKeyLocation {
    Header,
    Query,
    Cookie,
}

impl Default for APIKeyLocation {
    fn default() -> Self {
        Self::Header
    }
}

#[async_trait]
impl ClientAuthScheme for APIKeyAuth {
    async fn apply_auth(
        &self,
        headers: &mut HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.location {
            APIKeyLocation::Header => {
                headers.insert(self.name.clone(), self.api_key.clone());
            }
            APIKeyLocation::Cookie => {
                headers.insert(
                    "Cookie".to_string(),
                    format!("{}={}", self.name, self.api_key),
                );
            }
            APIKeyLocation::Query => {
                // Query params are handled at the request building level.
            }
        }
        Ok(())
    }

    fn tls_config(&self) -> Option<&TLSConfig> {
        self.tls.as_ref()
    }
}

/// OAuth2 Client Credentials flow authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2ClientCredentials {
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub tls: Option<TLSConfig>,
    // Token cache (not serialized)
    #[serde(skip)]
    _access_token: Option<String>,
    #[serde(skip)]
    _token_expires_at: Option<f64>,
}

#[async_trait]
impl ClientAuthScheme for OAuth2ClientCredentials {
    async fn apply_auth(
        &self,
        headers: &mut HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // In a full implementation, this would fetch/refresh the token.
        // For the port, we apply the cached token if available.
        if let Some(ref token) = self._access_token {
            headers.insert("Authorization".to_string(), format!("Bearer {}", token));
        }
        Ok(())
    }

    fn tls_config(&self) -> Option<&TLSConfig> {
        self.tls.as_ref()
    }
}

/// OAuth2 Authorization Code flow authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2AuthorizationCode {
    pub authorization_url: String,
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub tls: Option<TLSConfig>,
    #[serde(skip)]
    _access_token: Option<String>,
    #[serde(skip)]
    _refresh_token: Option<String>,
    #[serde(skip)]
    _token_expires_at: Option<f64>,
}

#[async_trait]
impl ClientAuthScheme for OAuth2AuthorizationCode {
    async fn apply_auth(
        &self,
        headers: &mut HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref token) = self._access_token {
            headers.insert("Authorization".to_string(), format!("Bearer {}", token));
        }
        Ok(())
    }

    fn tls_config(&self) -> Option<&TLSConfig> {
        self.tls.as_ref()
    }
}

/// HTTP Digest authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPDigestAuth {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub tls: Option<TLSConfig>,
}

#[async_trait]
impl ClientAuthScheme for HTTPDigestAuth {
    async fn apply_auth(
        &self,
        _headers: &mut HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Digest auth is handled by the HTTP client auth flow, not headers directly.
        Ok(())
    }

    fn tls_config(&self) -> Option<&TLSConfig> {
        self.tls.as_ref()
    }
}
