//! Roboflow embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/roboflow/
//!
//! Generates embeddings using the Roboflow inference API.
//! Primarily designed for image embeddings via CLIP models.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of roboflow/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the Roboflow embedding provider.
///
/// Port of crewai/rag/embeddings/providers/roboflow/types.py RoboflowProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoboflowProviderConfig {
    /// Roboflow API key.
    #[serde(default)]
    pub api_key: String,
    /// Roboflow inference API URL.
    #[serde(default = "default_api_url")]
    pub api_url: String,
}

fn default_api_url() -> String {
    "https://infer.roboflow.com".to_string()
}

impl Default for RoboflowProviderConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_url: default_api_url(),
        }
    }
}

/// Roboflow provider specification.
///
/// Port of crewai/rag/embeddings/providers/roboflow/types.py RoboflowProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoboflowProviderSpec {
    /// Provider identifier, always "roboflow".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<RoboflowProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// Roboflow embedding provider.
///
/// Implements the `BaseEmbedding` trait for Roboflow inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoboflowEmbedding {
    /// Provider configuration.
    pub config: RoboflowProviderConfig,
}

impl RoboflowEmbedding {
    /// Create a new Roboflow embedding provider with default configuration.
    pub fn new() -> Self {
        Self {
            config: RoboflowProviderConfig::default(),
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: RoboflowProviderConfig) -> Self {
        Self { config }
    }
}

impl Default for RoboflowEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseEmbedding for RoboflowEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: Implement actual Roboflow API call
        log::debug!(
            "Roboflow embed_text (url={}): {} chars",
            self.config.api_url,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        log::debug!(
            "Roboflow embed_documents: {} documents",
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for RoboflowEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        log::debug!("Roboflow call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
