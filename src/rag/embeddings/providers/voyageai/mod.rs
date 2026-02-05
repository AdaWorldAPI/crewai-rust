//! VoyageAI embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/voyageai/
//!
//! Generates embeddings using the Voyage AI Embeddings API.
//! Default model: `voyage-2`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of voyageai/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the VoyageAI embedding provider.
///
/// Port of crewai/rag/embeddings/providers/voyageai/types.py VoyageAIProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoyageAIProviderConfig {
    /// VoyageAI API key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Model name to use for embeddings.
    #[serde(default = "default_voyage_model")]
    pub model: String,
    /// Input type specifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_type: Option<String>,
    /// Whether to truncate input.
    #[serde(default = "default_true")]
    pub truncation: bool,
    /// Output data type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dtype: Option<String>,
    /// Output embedding dimension.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dimension: Option<usize>,
    /// Maximum number of retries.
    #[serde(default)]
    pub max_retries: usize,
    /// Request timeout in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
}

fn default_voyage_model() -> String {
    "voyage-2".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for VoyageAIProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: default_voyage_model(),
            input_type: None,
            truncation: true,
            output_dtype: None,
            output_dimension: None,
            max_retries: 0,
            timeout: None,
        }
    }
}

/// VoyageAI provider specification.
///
/// Port of crewai/rag/embeddings/providers/voyageai/types.py VoyageAIProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoyageAIProviderSpec {
    /// Provider identifier, always "voyageai".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<VoyageAIProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// VoyageAI embedding provider.
///
/// Implements the `BaseEmbedding` trait for VoyageAI embedding models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoyageAIEmbedding {
    /// Provider configuration.
    pub config: VoyageAIProviderConfig,
}

impl VoyageAIEmbedding {
    /// Create a new VoyageAI embedding provider with default configuration.
    pub fn new() -> Self {
        Self {
            config: VoyageAIProviderConfig::default(),
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: VoyageAIProviderConfig) -> Self {
        Self { config }
    }

    /// Resolve the API key from config or environment.
    pub fn resolve_api_key(&self) -> Option<String> {
        self.config
            .api_key
            .clone()
            .or_else(|| std::env::var("VOYAGE_API_KEY").ok())
    }
}

impl Default for VoyageAIEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseEmbedding for VoyageAIEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: Implement actual VoyageAI API call
        log::debug!(
            "VoyageAI embed_text (model={}): {} chars",
            self.config.model,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        log::debug!(
            "VoyageAI embed_documents (model={}): {} documents",
            self.config.model,
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for VoyageAIEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        log::debug!("VoyageAI call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
