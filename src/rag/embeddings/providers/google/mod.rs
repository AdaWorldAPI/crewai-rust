//! Google embedding provider (Generative AI and Vertex AI).
//!
//! Port of crewai/rag/embeddings/providers/google/
//!
//! Supports two backends:
//! - Google Generative AI (`google-generativeai`): Uses the Google AI Studio API.
//! - Google Vertex AI (`google-vertex`): Uses the Vertex AI SDK, supporting both
//!   legacy `textembedding-gecko` models and new `gemini-embedding-001` models.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of google/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the Google Generative AI embedding provider.
///
/// Port of crewai/rag/embeddings/providers/google/types.py GenerativeAiProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerativeAiProviderConfig {
    /// Google API key for authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Embedding model name.
    #[serde(default = "default_genai_model")]
    pub model_name: String,
    /// Task type for embeddings (default: "RETRIEVAL_DOCUMENT").
    #[serde(default = "default_task_type")]
    pub task_type: String,
}

fn default_genai_model() -> String {
    "gemini-embedding-001".to_string()
}

fn default_task_type() -> String {
    "RETRIEVAL_DOCUMENT".to_string()
}

impl Default for GenerativeAiProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model_name: default_genai_model(),
            task_type: default_task_type(),
        }
    }
}

/// Google Generative AI provider specification.
///
/// Port of crewai/rag/embeddings/providers/google/types.py GenerativeAiProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerativeAiProviderSpec {
    /// Provider identifier, always "google-generativeai".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<GenerativeAiProviderConfig>,
}

/// Configuration for the Google Vertex AI embedding provider.
///
/// Port of crewai/rag/embeddings/providers/google/types.py VertexAIProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexAIProviderConfig {
    /// Google API key (optional if using project_id with ADC).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Embedding model name.
    #[serde(default = "default_vertex_model")]
    pub model_name: String,
    /// GCP project ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// GCP region/location.
    #[serde(default = "default_location")]
    pub location: String,
    /// Task type for embeddings.
    #[serde(default = "default_task_type")]
    pub task_type: String,
    /// Output embedding dimension (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dimensionality: Option<usize>,
}

fn default_vertex_model() -> String {
    "textembedding-gecko".to_string()
}

fn default_location() -> String {
    "us-central1".to_string()
}

impl Default for VertexAIProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model_name: default_vertex_model(),
            project_id: None,
            location: default_location(),
            task_type: default_task_type(),
            output_dimensionality: None,
        }
    }
}

/// Google Vertex AI provider specification.
///
/// Port of crewai/rag/embeddings/providers/google/types.py VertexAIProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexAIProviderSpec {
    /// Provider identifier, always "google-vertex".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<VertexAIProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// Which Google embedding backend to use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoogleVariant {
    /// Google Generative AI (AI Studio).
    GenerativeAi,
    /// Google Vertex AI.
    VertexAi,
}

/// Google embedding provider (covers both Generative AI and Vertex AI).
///
/// The provider variant is determined by the `variant` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleEmbedding {
    /// Which Google backend to use.
    pub variant: GoogleVariant,
    /// Generative AI config (used when variant is GenerativeAi).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genai_config: Option<GenerativeAiProviderConfig>,
    /// Vertex AI config (used when variant is VertexAi).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertex_config: Option<VertexAIProviderConfig>,
}

impl GoogleEmbedding {
    /// Create a Google Generative AI embedding provider.
    pub fn generative_ai(config: GenerativeAiProviderConfig) -> Self {
        Self {
            variant: GoogleVariant::GenerativeAi,
            genai_config: Some(config),
            vertex_config: None,
        }
    }

    /// Create a Google Vertex AI embedding provider.
    pub fn vertex_ai(config: VertexAIProviderConfig) -> Self {
        Self {
            variant: GoogleVariant::VertexAi,
            genai_config: None,
            vertex_config: Some(config),
        }
    }
}

impl Default for GoogleEmbedding {
    fn default() -> Self {
        Self::generative_ai(GenerativeAiProviderConfig::default())
    }
}

#[async_trait]
impl BaseEmbedding for GoogleEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: Implement actual Google API call
        log::debug!(
            "Google embed_text (variant={:?}): {} chars",
            self.variant,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        log::debug!(
            "Google embed_documents (variant={:?}): {} documents",
            self.variant,
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for GoogleEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        log::debug!("Google call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
