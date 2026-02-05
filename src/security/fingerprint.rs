//! Fingerprint identity tracking.
//!
//! Corresponds to `crewai/security/fingerprint.py`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::constants::crew_ai_namespace;

/// Maximum metadata size in bytes (10KB).
const MAX_METADATA_SIZE: usize = 10 * 1024;

/// Fingerprint for identity tracking of agents, tasks, and crews.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fingerprint {
    /// String representation of the UUID.
    uuid_str: String,
    /// Creation timestamp.
    created_at: DateTime<Utc>,
    /// Optional metadata (max 10KB, depth limit 1).
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Fingerprint {
    /// Generate a new fingerprint with optional seed and metadata.
    pub fn generate(seed: Option<&str>, metadata: Option<HashMap<String, serde_json::Value>>) -> Self {
        let uuid_str = match seed {
            Some(s) => Self::generate_uuid(s),
            None => Uuid::new_v4().to_string(),
        };
        let metadata = metadata.unwrap_or_default();
        Self::validate_metadata(&metadata).expect("Invalid metadata");
        Self {
            uuid_str,
            created_at: Utc::now(),
            metadata,
        }
    }

    /// Generate a deterministic UUID from seed using uuid5.
    fn generate_uuid(seed: &str) -> String {
        Uuid::new_v5(&crew_ai_namespace(), seed.as_bytes()).to_string()
    }

    /// Validate metadata constraints.
    fn validate_metadata(metadata: &HashMap<String, serde_json::Value>) -> Result<(), String> {
        let serialized = serde_json::to_string(metadata).map_err(|e| e.to_string())?;
        if serialized.len() > MAX_METADATA_SIZE {
            return Err(format!(
                "Metadata exceeds maximum size of {} bytes",
                MAX_METADATA_SIZE
            ));
        }
        // Depth limit 1: values must not be objects
        for (key, value) in metadata {
            if value.is_object() {
                return Err(format!(
                    "Metadata value for key '{}' exceeds depth limit of 1",
                    key
                ));
            }
        }
        Ok(())
    }

    /// Get the UUID string.
    pub fn uuid_str(&self) -> &str {
        &self.uuid_str
    }

    /// Get the creation timestamp.
    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    /// Get the UUID object.
    pub fn uuid(&self) -> Uuid {
        Uuid::parse_str(&self.uuid_str).unwrap()
    }

    /// Deserialize from a dictionary.
    pub fn from_dict(data: &HashMap<String, serde_json::Value>) -> Result<Self, String> {
        let uuid_str = data
            .get("uuid_str")
            .and_then(|v| v.as_str())
            .ok_or("Missing uuid_str")?
            .to_string();
        let created_at = data
            .get("created_at")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<DateTime<Utc>>().ok())
            .unwrap_or_else(Utc::now);
        let metadata = data
            .get("metadata")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        Ok(Self {
            uuid_str,
            created_at,
            metadata,
        })
    }

    /// Serialize to a dictionary.
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert(
            "uuid_str".to_string(),
            serde_json::Value::String(self.uuid_str.clone()),
        );
        map.insert(
            "created_at".to_string(),
            serde_json::Value::String(self.created_at.to_rfc3339()),
        );
        map.insert(
            "metadata".to_string(),
            serde_json::to_value(&self.metadata).unwrap_or_default(),
        );
        map
    }
}

impl Default for Fingerprint {
    fn default() -> Self {
        Self::generate(None, None)
    }
}

impl std::fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.uuid_str)
    }
}

impl PartialEq for Fingerprint {
    fn eq(&self, other: &Self) -> bool {
        self.uuid_str == other.uuid_str
    }
}

impl Eq for Fingerprint {}

impl std::hash::Hash for Fingerprint {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid_str.hash(state);
    }
}
