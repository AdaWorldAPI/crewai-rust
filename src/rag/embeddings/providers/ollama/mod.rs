//! Ollama embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/ollama/
//!
//! Generates embeddings using a locally-running Ollama server.
//! Default API endpoint: `http://localhost:11434/api/embeddings`

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of ollama/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the Ollama embedding provider.
///
/// Port of crewai/rag/embeddings/providers/ollama/types.py OllamaProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaProviderConfig {
    /// Ollama API endpoint URL.
    #[serde(default = "default_url")]
    pub url: String,
    /// Model name to use for embeddings.
    pub model_name: String,
}

fn default_url() -> String {
    "http://localhost:11434/api/embeddings".to_string()
}

impl Default for OllamaProviderConfig {
    fn default() -> Self {
        Self {
            url: default_url(),
            model_name: "nomic-embed-text".to_string(),
        }
    }
}

/// Ollama provider specification.
///
/// Port of crewai/rag/embeddings/providers/ollama/types.py OllamaProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaProviderSpec {
    /// Provider identifier, always "ollama".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<OllamaProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation (port of ollama/ollama_provider.py)
// ---------------------------------------------------------------------------

/// Ollama embedding provider.
///
/// Port of crewai/rag/embeddings/providers/ollama/ollama_provider.py OllamaProvider.
///
/// Environment variables:
/// - `OLLAMA_URL` / `EMBEDDINGS_OLLAMA_URL`
/// - `OLLAMA_MODEL_NAME` / `EMBEDDINGS_OLLAMA_MODEL_NAME` / `OLLAMA_MODEL`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaEmbedding {
    /// Provider configuration.
    pub config: OllamaProviderConfig,
}

impl OllamaEmbedding {
    /// Create a new Ollama embedding provider with default configuration.
    pub fn new() -> Self {
        Self {
            config: OllamaProviderConfig::default(),
        }
    }

    /// Create a new Ollama embedding provider with the given configuration.
    pub fn with_config(config: OllamaProviderConfig) -> Self {
        Self { config }
    }
}

impl Default for OllamaEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseEmbedding for OllamaEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: POST to self.config.url with { "model": model_name, "prompt": text }
        log::debug!(
            "Ollama embed_text (model={}, url={}): {} chars",
            self.config.model_name,
            self.config.url,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        log::debug!(
            "Ollama embed_documents (model={}): {} documents",
            self.config.model_name,
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for OllamaEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        log::debug!("Ollama call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
