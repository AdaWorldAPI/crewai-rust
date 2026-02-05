//! Text2Vec embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/text2vec/
//!
//! Generates embeddings using the text2vec library.
//! Default model: `shibing624/text2vec-base-chinese`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of text2vec/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the Text2Vec embedding provider.
///
/// Port of crewai/rag/embeddings/providers/text2vec/types.py Text2VecProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text2VecProviderConfig {
    /// Model name to use for embeddings.
    #[serde(default = "default_text2vec_model")]
    pub model_name: String,
}

fn default_text2vec_model() -> String {
    "shibing624/text2vec-base-chinese".to_string()
}

impl Default for Text2VecProviderConfig {
    fn default() -> Self {
        Self {
            model_name: default_text2vec_model(),
        }
    }
}

/// Text2Vec provider specification.
///
/// Port of crewai/rag/embeddings/providers/text2vec/types.py Text2VecProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text2VecProviderSpec {
    /// Provider identifier, always "text2vec".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<Text2VecProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// Text2Vec embedding provider.
///
/// Implements the `BaseEmbedding` trait for text2vec models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text2VecEmbedding {
    /// Provider configuration.
    pub config: Text2VecProviderConfig,
}

impl Text2VecEmbedding {
    /// Create a new Text2Vec embedding provider with default configuration.
    pub fn new() -> Self {
        Self {
            config: Text2VecProviderConfig::default(),
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: Text2VecProviderConfig) -> Self {
        Self { config }
    }
}

impl Default for Text2VecEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseEmbedding for Text2VecEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: Implement actual text2vec inference
        log::debug!(
            "Text2Vec embed_text (model={}): {} chars",
            self.config.model_name,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        log::debug!(
            "Text2Vec embed_documents (model={}): {} documents",
            self.config.model_name,
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for Text2VecEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        log::debug!("Text2Vec call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
