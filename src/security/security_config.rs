//! Security configuration.
//!
//! Corresponds to `crewai/security/security_config.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::fingerprint::Fingerprint;

/// Security configuration including fingerprinting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Security fingerprint for identity tracking.
    pub fingerprint: Fingerprint,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            fingerprint: Fingerprint::default(),
        }
    }
}

impl SecurityConfig {
    /// Create a new SecurityConfig with default fingerprint.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a SecurityConfig with a specific fingerprint.
    pub fn with_fingerprint(fingerprint: Fingerprint) -> Self {
        Self { fingerprint }
    }

    /// Convert to a dictionary.
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert(
            "fingerprint".to_string(),
            serde_json::to_value(self.fingerprint.to_dict()).unwrap_or_default(),
        );
        map
    }

    /// Create from a dictionary.
    pub fn from_dict(data: &HashMap<String, serde_json::Value>) -> Result<Self, String> {
        let fingerprint = if let Some(fp_val) = data.get("fingerprint") {
            let fp_map: HashMap<String, serde_json::Value> =
                serde_json::from_value(fp_val.clone()).map_err(|e| e.to_string())?;
            Fingerprint::from_dict(&fp_map)?
        } else {
            Fingerprint::default()
        };
        Ok(Self { fingerprint })
    }
}
