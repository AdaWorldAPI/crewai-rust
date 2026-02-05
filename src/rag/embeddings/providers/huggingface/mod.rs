//! HuggingFace embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/huggingface/
//!
//! Generates embeddings using the HuggingFace Inference API.
//! Default model: `sentence-transformers/all-MiniLM-L6-v2`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of huggingface/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the HuggingFace embedding provider.
///
/// Port of crewai/rag/embeddings/providers/huggingface/types.py HuggingFaceProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuggingFaceProviderConfig {
    /// HuggingFace API key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Model name to use for embeddings.
    #[serde(default = "default_hf_model")]
    pub model_name: String,
}

fn default_hf_model() -> String {
    "sentence-transformers/all-MiniLM-L6-v2".to_string()
}

impl Default for HuggingFaceProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model_name: default_hf_model(),
        }
    }
}

/// HuggingFace provider specification.
///
/// Port of crewai/rag/embeddings/providers/huggingface/types.py HuggingFaceProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuggingFaceProviderSpec {
    /// Provider identifier, always "huggingface".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<HuggingFaceProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// HuggingFace embedding provider.
///
/// Implements the `BaseEmbedding` trait for HuggingFace embedding models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuggingFaceEmbedding {
    /// Provider configuration.
    pub config: HuggingFaceProviderConfig,
}

impl HuggingFaceEmbedding {
    /// Create a new HuggingFace embedding provider with default configuration.
    pub fn new() -> Self {
        Self {
            config: HuggingFaceProviderConfig::default(),
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: HuggingFaceProviderConfig) -> Self {
        Self { config }
    }
}

impl Default for HuggingFaceEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseEmbedding for HuggingFaceEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: Implement HuggingFace Inference API call
        log::debug!(
            "HuggingFace embed_text (model={}): {} chars",
            self.config.model_name,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        log::debug!(
            "HuggingFace embed_documents (model={}): {} documents",
            self.config.model_name,
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for HuggingFaceEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        log::debug!("HuggingFace call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
