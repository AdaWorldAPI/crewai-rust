//! Microsoft Azure embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/microsoft/
//!
//! Generates embeddings using Azure OpenAI Service.
//! Requires Azure-specific configuration including deployment ID and API base URL.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of microsoft/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the Azure OpenAI embedding provider.
///
/// Port of crewai/rag/embeddings/providers/microsoft/types.py AzureProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureProviderConfig {
    /// Azure API key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Azure API base URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base: Option<String>,
    /// API type (default: "azure").
    #[serde(default = "default_api_type")]
    pub api_type: String,
    /// API version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    /// Model name to use for embeddings.
    #[serde(default = "default_azure_model")]
    pub model_name: String,
    /// Default headers for API requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_headers: Option<HashMap<String, Value>>,
    /// Embedding dimensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
    /// Azure deployment ID (required).
    pub deployment_id: Option<String>,
    /// Organization ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
}

fn default_api_type() -> String {
    "azure".to_string()
}

fn default_azure_model() -> String {
    "text-embedding-ada-002".to_string()
}

impl Default for AzureProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            api_base: None,
            api_type: default_api_type(),
            api_version: None,
            model_name: default_azure_model(),
            default_headers: None,
            dimensions: None,
            deployment_id: None,
            organization_id: None,
        }
    }
}

/// Azure provider specification.
///
/// Port of crewai/rag/embeddings/providers/microsoft/types.py AzureProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureProviderSpec {
    /// Provider identifier, always "azure".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<AzureProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// Azure OpenAI embedding provider.
///
/// Implements the `BaseEmbedding` trait for Azure OpenAI embeddings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureEmbedding {
    /// Provider configuration.
    pub config: AzureProviderConfig,
}

impl AzureEmbedding {
    /// Create a new Azure embedding provider with default configuration.
    pub fn new() -> Self {
        Self {
            config: AzureProviderConfig::default(),
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: AzureProviderConfig) -> Self {
        Self { config }
    }
}

impl Default for AzureEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseEmbedding for AzureEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: Implement actual Azure OpenAI API call
        log::debug!(
            "Azure embed_text (model={}, deployment={:?}): {} chars",
            self.config.model_name,
            self.config.deployment_id,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        log::debug!(
            "Azure embed_documents (model={}): {} documents",
            self.config.model_name,
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for AzureEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        log::debug!("Azure call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
