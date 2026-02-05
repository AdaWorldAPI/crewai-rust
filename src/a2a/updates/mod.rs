//! Update mechanism configurations for A2A protocol.
//!
//! Corresponds to `crewai/a2a/updates/`.

use serde::{Deserialize, Serialize};

/// Configuration for polling-based task updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingConfig {
    /// Seconds between poll attempts.
    #[serde(default = "default_interval")]
    pub interval: f64,
    /// Max seconds to poll before raising timeout error.
    pub timeout: Option<f64>,
    /// Max number of poll attempts.
    pub max_polls: Option<u32>,
    /// Number of messages to retrieve per poll.
    #[serde(default = "default_history_length")]
    pub history_length: u32,
}

fn default_interval() -> f64 { 2.0 }
fn default_history_length() -> u32 { 100 }

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            interval: default_interval(),
            timeout: None,
            max_polls: None,
            history_length: default_history_length(),
        }
    }
}

/// Configuration for SSE-based (streaming) task updates.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamingConfig {}

/// Configuration for webhook-based push notification task updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNotificationConfig {
    /// Callback URL where agent sends push notifications.
    pub url: String,
    /// Unique identifier for this config.
    pub id: Option<String>,
    /// Token to validate incoming notifications.
    pub token: Option<String>,
    /// Max seconds to wait for task completion.
    #[serde(default = "default_push_timeout")]
    pub timeout: Option<f64>,
    /// Seconds between result polling attempts.
    #[serde(default = "default_push_interval")]
    pub interval: f64,
    /// HMAC signature secret for webhook signing.
    pub signature_secret: Option<String>,
}

fn default_push_timeout() -> Option<f64> { Some(300.0) }
fn default_push_interval() -> f64 { 2.0 }

/// Enum representing the different update config types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UpdateConfig {
    Polling(PollingConfig),
    Streaming(StreamingConfig),
    PushNotification(PushNotificationConfig),
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self::Streaming(StreamingConfig::default())
    }
}
