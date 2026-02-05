//! Instructor embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/instructor/
//!
//! Generates embeddings using the Instructor embedding model (hkunlp/instructor-base).
//! Instruction-tuned embedding model that follows task-specific instructions.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of instructor/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the Instructor embedding provider.
///
/// Port of crewai/rag/embeddings/providers/instructor/types.py InstructorProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructorProviderConfig {
    /// Model name to use.
    #[serde(default = "default_instructor_model")]
    pub model_name: String,
    /// Device to run the model on.
    #[serde(default = "default_device")]
    pub device: String,
    /// Instruction prefix for the embeddings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
}

fn default_instructor_model() -> String {
    "hkunlp/instructor-base".to_string()
}

fn default_device() -> String {
    "cpu".to_string()
}

impl Default for InstructorProviderConfig {
    fn default() -> Self {
        Self {
            model_name: default_instructor_model(),
            device: default_device(),
            instruction: None,
        }
    }
}

/// Instructor provider specification.
///
/// Port of crewai/rag/embeddings/providers/instructor/types.py InstructorProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructorProviderSpec {
    /// Provider identifier, always "instructor".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<InstructorProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// Instructor embedding provider.
///
/// Implements the `BaseEmbedding` trait for Instructor embedding models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructorEmbedding {
    /// Provider configuration.
    pub config: InstructorProviderConfig,
}

impl InstructorEmbedding {
    /// Create a new Instructor embedding provider with default configuration.
    pub fn new() -> Self {
        Self {
            config: InstructorProviderConfig::default(),
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: InstructorProviderConfig) -> Self {
        Self { config }
    }
}

impl Default for InstructorEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseEmbedding for InstructorEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: Implement actual Instructor model inference
        log::debug!(
            "Instructor embed_text (model={}, device={}): {} chars",
            self.config.model_name,
            self.config.device,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        log::debug!(
            "Instructor embed_documents (model={}): {} documents",
            self.config.model_name,
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for InstructorEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        log::debug!("Instructor call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
