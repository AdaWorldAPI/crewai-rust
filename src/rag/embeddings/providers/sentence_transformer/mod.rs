//! Sentence Transformers embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/sentence_transformer/
//!
//! Generates embeddings using the sentence-transformers library.
//! Default model: `all-MiniLM-L6-v2`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of sentence_transformer/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the Sentence Transformer embedding provider.
///
/// Port of crewai/rag/embeddings/providers/sentence_transformer/types.py
/// SentenceTransformerProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceTransformerProviderConfig {
    /// Model name to use for embeddings.
    #[serde(default = "default_st_model")]
    pub model_name: String,
    /// Device to run the model on.
    #[serde(default = "default_device")]
    pub device: String,
    /// Whether to normalize output embeddings.
    #[serde(default)]
    pub normalize_embeddings: bool,
}

fn default_st_model() -> String {
    "all-MiniLM-L6-v2".to_string()
}

fn default_device() -> String {
    "cpu".to_string()
}

impl Default for SentenceTransformerProviderConfig {
    fn default() -> Self {
        Self {
            model_name: default_st_model(),
            device: default_device(),
            normalize_embeddings: false,
        }
    }
}

/// Sentence Transformer provider specification.
///
/// Port of crewai/rag/embeddings/providers/sentence_transformer/types.py
/// SentenceTransformerProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceTransformerProviderSpec {
    /// Provider identifier, always "sentence-transformer".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<SentenceTransformerProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// Sentence Transformer embedding provider.
///
/// Implements the `BaseEmbedding` trait for sentence-transformers models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceTransformerEmbedding {
    /// Provider configuration.
    pub config: SentenceTransformerProviderConfig,
}

impl SentenceTransformerEmbedding {
    /// Create a new Sentence Transformer embedding provider with default configuration.
    pub fn new() -> Self {
        Self {
            config: SentenceTransformerProviderConfig::default(),
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: SentenceTransformerProviderConfig) -> Self {
        Self { config }
    }
}

impl Default for SentenceTransformerEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseEmbedding for SentenceTransformerEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: Implement actual sentence-transformers inference
        log::debug!(
            "SentenceTransformer embed_text (model={}, device={}): {} chars",
            self.config.model_name,
            self.config.device,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        log::debug!(
            "SentenceTransformer embed_documents (model={}): {} documents",
            self.config.model_name,
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for SentenceTransformerEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        log::debug!("SentenceTransformer call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
