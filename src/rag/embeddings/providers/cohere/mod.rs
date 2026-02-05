//! Cohere embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/cohere/
//!
//! Generates embeddings using the Cohere Embed API.
//! Requires a `COHERE_API_KEY` environment variable or explicit API key.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of cohere/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the Cohere embedding provider.
///
/// Port of crewai/rag/embeddings/providers/cohere/types.py CohereProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereProviderConfig {
    /// Cohere API key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Model name to use for embeddings.
    #[serde(default = "default_model")]
    pub model_name: String,
}

fn default_model() -> String {
    "large".to_string()
}

impl Default for CohereProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model_name: default_model(),
        }
    }
}

/// Cohere provider specification.
///
/// Port of crewai/rag/embeddings/providers/cohere/types.py CohereProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereProviderSpec {
    /// Provider identifier, always "cohere".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<CohereProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation (port of cohere/cohere_provider.py)
// ---------------------------------------------------------------------------

/// Cohere embedding provider.
///
/// Port of crewai/rag/embeddings/providers/cohere/cohere_provider.py CohereProvider.
///
/// Environment variables:
/// - `COHERE_API_KEY` / `EMBEDDINGS_COHERE_API_KEY`
/// - `EMBEDDINGS_COHERE_MODEL_NAME`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereEmbedding {
    /// Provider configuration.
    pub config: CohereProviderConfig,
}

impl CohereEmbedding {
    /// Create a new Cohere embedding provider with default configuration.
    pub fn new() -> Self {
        Self {
            config: CohereProviderConfig::default(),
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: CohereProviderConfig) -> Self {
        Self { config }
    }

    /// Resolve the API key from config or environment.
    pub fn resolve_api_key(&self) -> Option<String> {
        self.config
            .api_key
            .clone()
            .or_else(|| std::env::var("COHERE_API_KEY").ok())
            .or_else(|| std::env::var("EMBEDDINGS_COHERE_API_KEY").ok())
    }
}

impl Default for CohereEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseEmbedding for CohereEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: Implement actual Cohere API call
        log::debug!(
            "Cohere embed_text (model={}): {} chars",
            self.config.model_name,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        log::debug!(
            "Cohere embed_documents (model={}): {} documents",
            self.config.model_name,
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for CohereEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        log::debug!("Cohere call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
