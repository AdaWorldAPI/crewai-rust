//! OpenAI embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/openai/
//!
//! Generates embeddings using the OpenAI Embeddings API (e.g., text-embedding-ada-002,
//! text-embedding-3-small). Requires an `OPENAI_API_KEY` environment variable or
//! explicit API key configuration.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of openai/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the OpenAI embedding provider.
///
/// Port of crewai/rag/embeddings/providers/openai/types.py OpenAIProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIProviderConfig {
    /// OpenAI API key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Model name to use for embeddings.
    #[serde(default = "default_model_name")]
    pub model_name: String,
    /// Base URL for API requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base: Option<String>,
    /// API type (e.g., "azure").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_type: Option<String>,
    /// API version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    /// Default headers for API requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_headers: Option<HashMap<String, Value>>,
    /// Embedding dimensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
    /// Azure deployment ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_id: Option<String>,
    /// OpenAI organization ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
}

fn default_model_name() -> String {
    "text-embedding-ada-002".to_string()
}

impl Default for OpenAIProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model_name: default_model_name(),
            api_base: None,
            api_type: None,
            api_version: None,
            default_headers: None,
            dimensions: None,
            deployment_id: None,
            organization_id: None,
        }
    }
}

/// OpenAI provider specification.
///
/// Port of crewai/rag/embeddings/providers/openai/types.py OpenAIProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIProviderSpec {
    /// Provider identifier, always "openai".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<OpenAIProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation (port of openai/openai_provider.py)
// ---------------------------------------------------------------------------

/// OpenAI embedding provider.
///
/// Port of crewai/rag/embeddings/providers/openai/openai_provider.py OpenAIProvider.
///
/// Implements the `BaseEmbedding` trait to generate embeddings via the OpenAI API.
/// Supports configuration through environment variables:
/// - `OPENAI_API_KEY` / `EMBEDDINGS_OPENAI_API_KEY`
/// - `OPENAI_MODEL_NAME` / `EMBEDDINGS_OPENAI_MODEL_NAME`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIEmbedding {
    /// Provider configuration.
    pub config: OpenAIProviderConfig,
}

impl OpenAIEmbedding {
    /// Create a new OpenAI embedding provider with default configuration.
    pub fn new() -> Self {
        Self {
            config: OpenAIProviderConfig::default(),
        }
    }

    /// Create a new OpenAI embedding provider with the given configuration.
    pub fn with_config(config: OpenAIProviderConfig) -> Self {
        Self { config }
    }

    /// Resolve the API key from config or environment.
    pub fn resolve_api_key(&self) -> Option<String> {
        self.config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .or_else(|| std::env::var("EMBEDDINGS_OPENAI_API_KEY").ok())
    }
}

impl Default for OpenAIEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseEmbedding for OpenAIEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: Implement actual OpenAI API call using reqwest
        // POST https://api.openai.com/v1/embeddings
        // { "model": self.config.model_name, "input": text }
        log::debug!(
            "OpenAI embed_text (model={}): {} chars",
            self.config.model_name,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        // TODO: Implement batch OpenAI API call
        log::debug!(
            "OpenAI embed_documents (model={}): {} documents",
            self.config.model_name,
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for OpenAIEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        // TODO: Implement synchronous embedding via blocking runtime
        log::debug!("OpenAI call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
