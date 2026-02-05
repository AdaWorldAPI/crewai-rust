//! Custom embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/custom/
//!
//! Allows users to provide their own embedding function implementation.
//! The custom provider wraps a user-supplied callable that converts text to vectors.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of custom/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the custom embedding provider.
///
/// Port of crewai/rag/embeddings/providers/custom/types.py CustomProviderConfig.
///
/// In the Python version, this holds a reference to a custom `EmbeddingFunction` class.
/// In Rust, the user provides a trait object or closure at construction time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProviderConfig {
    /// Description of the custom embedding function (for logging/debugging).
    #[serde(default)]
    pub description: Option<String>,
}

impl Default for CustomProviderConfig {
    fn default() -> Self {
        Self { description: None }
    }
}

/// Custom provider specification.
///
/// Port of crewai/rag/embeddings/providers/custom/types.py CustomProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProviderSpec {
    /// Provider identifier, always "custom".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<CustomProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// Type alias for a custom embedding closure.
pub type CustomEmbedFn = Arc<dyn Fn(&[String]) -> Result<Embeddings, anyhow::Error> + Send + Sync>;

/// Custom embedding provider.
///
/// Wraps a user-supplied embedding function. Since closures are not
/// serializable, this struct holds a type-erased function reference.
pub struct CustomEmbedding {
    /// Provider configuration.
    pub config: CustomProviderConfig,
    /// The user-supplied embedding function.
    pub embed_fn: CustomEmbedFn,
}

impl CustomEmbedding {
    /// Create a custom embedding provider with the given function.
    pub fn new<F>(embed_fn: F) -> Self
    where
        F: Fn(&[String]) -> Result<Embeddings, anyhow::Error> + Send + Sync + 'static,
    {
        Self {
            config: CustomProviderConfig::default(),
            embed_fn: Arc::new(embed_fn),
        }
    }

    /// Create with a description and function.
    pub fn with_description<F>(description: &str, embed_fn: F) -> Self
    where
        F: Fn(&[String]) -> Result<Embeddings, anyhow::Error> + Send + Sync + 'static,
    {
        Self {
            config: CustomProviderConfig {
                description: Some(description.to_string()),
            },
            embed_fn: Arc::new(embed_fn),
        }
    }
}

impl std::fmt::Debug for CustomEmbedding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomEmbedding")
            .field("config", &self.config)
            .field("embed_fn", &"<custom function>")
            .finish()
    }
}

#[async_trait]
impl BaseEmbedding for CustomEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        match (self.embed_fn)(&[text.to_string()]) {
            Ok(mut results) => results.pop().unwrap_or_default(),
            Err(e) => {
                log::error!("Custom embed_text error: {}", e);
                Vec::new()
            }
        }
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        match (self.embed_fn)(documents) {
            Ok(results) => results,
            Err(e) => {
                log::error!("Custom embed_documents error: {}", e);
                documents.iter().map(|_| Vec::new()).collect()
            }
        }
    }
}

impl EmbeddingFunctionTrait for CustomEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        (self.embed_fn)(input)
    }
}
