//! OpenCLIP embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/openclip/
//!
//! Generates embeddings using OpenCLIP models for text and/or image inputs.
//! Default model: `ViT-B-32` with checkpoint `laion2b_s34b_b79k`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of openclip/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the OpenCLIP embedding provider.
///
/// Port of crewai/rag/embeddings/providers/openclip/types.py OpenCLIPProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCLIPProviderConfig {
    /// Model name.
    #[serde(default = "default_openclip_model")]
    pub model_name: String,
    /// Model checkpoint.
    #[serde(default = "default_checkpoint")]
    pub checkpoint: String,
    /// Device to run the model on.
    #[serde(default = "default_device")]
    pub device: String,
}

fn default_openclip_model() -> String {
    "ViT-B-32".to_string()
}

fn default_checkpoint() -> String {
    "laion2b_s34b_b79k".to_string()
}

fn default_device() -> String {
    "cpu".to_string()
}

impl Default for OpenCLIPProviderConfig {
    fn default() -> Self {
        Self {
            model_name: default_openclip_model(),
            checkpoint: default_checkpoint(),
            device: default_device(),
        }
    }
}

/// OpenCLIP provider specification.
///
/// Port of crewai/rag/embeddings/providers/openclip/types.py OpenCLIPProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCLIPProviderSpec {
    /// Provider identifier, always "openclip".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<OpenCLIPProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// OpenCLIP embedding provider.
///
/// Implements the `BaseEmbedding` trait for OpenCLIP models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClipEmbedding {
    /// Provider configuration.
    pub config: OpenCLIPProviderConfig,
}

impl OpenClipEmbedding {
    /// Create a new OpenCLIP embedding provider with default configuration.
    pub fn new() -> Self {
        Self {
            config: OpenCLIPProviderConfig::default(),
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: OpenCLIPProviderConfig) -> Self {
        Self { config }
    }
}

impl Default for OpenClipEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseEmbedding for OpenClipEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: Implement actual OpenCLIP inference
        log::debug!(
            "OpenCLIP embed_text (model={}, checkpoint={}): {} chars",
            self.config.model_name,
            self.config.checkpoint,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        log::debug!(
            "OpenCLIP embed_documents (model={}): {} documents",
            self.config.model_name,
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for OpenClipEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        log::debug!("OpenCLIP call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
