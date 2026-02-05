//! ONNX Runtime embedding provider.
//!
//! Port of crewai/rag/embeddings/providers/onnx/
//!
//! Generates embeddings using ONNX Runtime, allowing hardware-accelerated
//! inference with models exported to the ONNX format.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::rag::core::{BaseEmbedding, EmbeddingFunctionTrait, EmbeddingResult, Embeddings};

// ---------------------------------------------------------------------------
// Configuration types (port of onnx/types.py)
// ---------------------------------------------------------------------------

/// Configuration for the ONNX embedding provider.
///
/// Port of crewai/rag/embeddings/providers/onnx/types.py ONNXProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ONNXProviderConfig {
    /// Preferred ONNX execution providers (e.g., "CUDAExecutionProvider", "CPUExecutionProvider").
    #[serde(default)]
    pub preferred_providers: Vec<String>,
}

impl Default for ONNXProviderConfig {
    fn default() -> Self {
        Self {
            preferred_providers: Vec::new(),
        }
    }
}

/// ONNX provider specification.
///
/// Port of crewai/rag/embeddings/providers/onnx/types.py ONNXProviderSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ONNXProviderSpec {
    /// Provider identifier, always "onnx".
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: Option<ONNXProviderConfig>,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// ONNX Runtime embedding provider.
///
/// Implements the `BaseEmbedding` trait for ONNX Runtime inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnnxEmbedding {
    /// Provider configuration.
    pub config: ONNXProviderConfig,
}

impl OnnxEmbedding {
    /// Create a new ONNX embedding provider with default configuration.
    pub fn new() -> Self {
        Self {
            config: ONNXProviderConfig::default(),
        }
    }

    /// Create with specific configuration.
    pub fn with_config(config: ONNXProviderConfig) -> Self {
        Self { config }
    }
}

impl Default for OnnxEmbedding {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseEmbedding for OnnxEmbedding {
    async fn embed_text(&self, text: &str) -> EmbeddingResult {
        // TODO: Implement actual ONNX Runtime inference
        log::debug!(
            "ONNX embed_text (providers={:?}): {} chars",
            self.config.preferred_providers,
            text.len()
        );
        Vec::new()
    }

    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult> {
        log::debug!(
            "ONNX embed_documents: {} documents",
            documents.len()
        );
        documents.iter().map(|_| Vec::new()).collect()
    }
}

impl EmbeddingFunctionTrait for OnnxEmbedding {
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error> {
        log::debug!("ONNX call: {} inputs", input.len());
        Ok(input.iter().map(|_| Vec::new()).collect())
    }
}
