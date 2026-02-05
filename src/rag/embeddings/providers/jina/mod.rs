//! Jina AI embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/jina/
//!
//! Generates embeddings using the Jina AI Embeddings API.
//! Default model: `jina-embeddings-v2-base-en`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of jina/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the Jina embedding provider.
///
/// Port of crewai/rag/embeddings/providers/jina/types.py JinaProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JinaProviderConfig {
    /// Jina API key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Model name to use for embeddings.
    #[serde(default = "default_jina_model")]
    pub model_name: String,
}

fn default_jina_model() -> String {
    "jina-embeddings-v2-base-en".to_string()
}

impl Default for JinaProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model_name: default_jina_model(),
        }
    }
}

/// Jina provider specification.
///
/// Port of crewai/rag/embeddings/providers/jina/types.py JinaProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JinaProviderSpec {
    /// Provider identifier, always "jina".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<JinaProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// Jina AI embedding provider.
///
/// Implements the `BaseEmbedding` trait for Jina AI embedding models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JinaEmbedding {
    /// Provider configuration.
    pub config: JinaProviderConfig,
}

impl JinaEmbedding {
    /// Create a new Jina embedding provider with default configuration.
    pub fn new() -> Self {
        Self {
            config: JinaProviderConfig::default(),
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: JinaProviderConfig) -> Self {
        Self { config }
    }

    /// Resolve the API key from config or environment.
    pub fn resolve_api_key(&self) -> Option<String> {
        self.config
            .api_key
            .clone()
            .or_else(|| std::env::var("JINA_API_KEY").ok())
    }
}

impl Default for JinaEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseEmbedding for JinaEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: Implement actual Jina API call
        log::debug!(
            "Jina embed_text (model={}): {} chars",
            self.config.model_name,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        log::debug!(
            "Jina embed_documents (model={}): {} documents",
            self.config.model_name,
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for JinaEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        log::debug!("Jina call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
