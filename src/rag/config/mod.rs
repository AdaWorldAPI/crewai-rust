//! RAG configuration for provider selection and tuning.
//!
//! Port of crewai/rag/config/

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Supported RAG provider names.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SupportedProvider {
    /// ChromaDB vector database.
    #[serde(alias = "chromadb")]
    Chromadb,
    /// Qdrant vector database.
    #[serde(alias = "qdrant")]
    Qdrant,
}

impl std::fmt::Display for SupportedProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupportedProvider::Chromadb => write!(f, "chromadb"),
            SupportedProvider::Qdrant => write!(f, "qdrant"),
        }
    }
}

/// Base configuration for RAG providers.
///
/// Contains common settings shared by all RAG provider configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseRagConfig {
    /// The provider type.
    pub provider: SupportedProvider,
    /// Optional embedding function configuration.
    pub embedding_function: Option<Value>,
    /// Maximum number of results to return from searches.
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Minimum similarity score threshold for search results.
    #[serde(default = "default_score_threshold")]
    pub score_threshold: f64,
    /// Batch size for adding documents.
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

fn default_limit() -> usize {
    5
}

fn default_score_threshold() -> f64 {
    0.6
}

fn default_batch_size() -> usize {
    100
}

impl BaseRagConfig {
    /// Create a new BaseRagConfig for ChromaDB.
    pub fn chromadb() -> Self {
        Self {
            provider: SupportedProvider::Chromadb,
            embedding_function: None,
            limit: default_limit(),
            score_threshold: default_score_threshold(),
            batch_size: default_batch_size(),
        }
    }

    /// Create a new BaseRagConfig for Qdrant.
    pub fn qdrant() -> Self {
        Self {
            provider: SupportedProvider::Qdrant,
            embedding_function: None,
            limit: default_limit(),
            score_threshold: default_score_threshold(),
            batch_size: default_batch_size(),
        }
    }

    /// Set the embedding function configuration.
    pub fn with_embedding_function(mut self, config: Value) -> Self {
        self.embedding_function = Some(config);
        self
    }

    /// Set the search result limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set the score threshold.
    pub fn with_score_threshold(mut self, threshold: f64) -> Self {
        self.score_threshold = threshold;
        self
    }

    /// Set the batch size.
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }
}

/// Type alias for supported RAG configuration types.
///
/// This enum wraps provider-specific configurations and is
/// discriminated by the `provider` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider")]
pub enum RagConfigType {
    /// ChromaDB configuration.
    #[serde(rename = "chromadb")]
    Chromadb(BaseRagConfig),
    /// Qdrant configuration.
    #[serde(rename = "qdrant")]
    Qdrant(BaseRagConfig),
}

impl RagConfigType {
    /// Get the provider name.
    pub fn provider(&self) -> &SupportedProvider {
        match self {
            RagConfigType::Chromadb(c) => &c.provider,
            RagConfigType::Qdrant(c) => &c.provider,
        }
    }

    /// Get the base configuration.
    pub fn base_config(&self) -> &BaseRagConfig {
        match self {
            RagConfigType::Chromadb(c) => c,
            RagConfigType::Qdrant(c) => c,
        }
    }
}
