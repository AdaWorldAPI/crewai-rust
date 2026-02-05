//! Embeddings factory and provider registry for the RAG system.
//!
//! Port of crewai/rag/embeddings/
//!
//! This module provides:
//! - Provider registry mapping provider names to implementations
//! - Factory functions for building embedding functions from specs
//! - Submodule `providers` with all supported embedding provider stubs

pub mod providers;

use std::collections::HashMap;

use serde_json::Value;

use crate::rag::core::{BaseEmbeddingsProvider, EmbeddingFunctionTrait};
use crate::rag::types::Embeddings;

// Provider types will be re-exported once implemented.
// Currently, provider stubs are placeholders for future native implementations.

/// Known provider module paths (corresponding to Python's PROVIDER_PATHS).
///
/// In the Rust port, these serve as identifiers rather than importable paths.
pub fn provider_registry() -> HashMap<&'static str, &'static str> {
    let mut registry = HashMap::new();
    registry.insert("azure", "microsoft.azure.AzureProvider");
    registry.insert("amazon-bedrock", "aws.bedrock.BedrockProvider");
    registry.insert("cohere", "cohere.CohereProvider");
    registry.insert("custom", "custom.CustomProvider");
    registry.insert("google-generativeai", "google.generative_ai.GenerativeAiProvider");
    registry.insert("google", "google.generative_ai.GenerativeAiProvider");
    registry.insert("google-vertex", "google.vertex.VertexAIProvider");
    registry.insert("huggingface", "huggingface.HuggingFaceProvider");
    registry.insert("instructor", "instructor.InstructorProvider");
    registry.insert("jina", "jina.JinaProvider");
    registry.insert("ollama", "ollama.OllamaProvider");
    registry.insert("onnx", "onnx.ONNXProvider");
    registry.insert("openai", "openai.OpenAIProvider");
    registry.insert("openclip", "openclip.OpenCLIPProvider");
    registry.insert("roboflow", "roboflow.RoboflowProvider");
    registry.insert("sentence-transformer", "sentence_transformer.SentenceTransformerProvider");
    registry.insert("text2vec", "text2vec.Text2VecProvider");
    registry.insert("voyageai", "voyageai.VoyageAIProvider");
    registry.insert("watsonx", "ibm.watsonx.WatsonXProvider");
    registry
}

/// Build an embedding function from a provider instance.
///
/// # Arguments
/// * `provider` - The embedding provider.
///
/// # Returns
/// A boxed embedding function trait object.
pub fn build_embedder_from_provider(
    provider: &dyn BaseEmbeddingsProvider,
) -> Result<Box<dyn EmbeddingFunctionTrait>, anyhow::Error> {
    provider.build_embedding_function()
}

/// Build an embedding function from a dictionary specification.
///
/// # Arguments
/// * `spec` - A JSON value with "provider" and optional "config" keys.
///
/// # Returns
/// A boxed embedding function trait object.
///
/// # Errors
/// Returns an error if the provider is not recognized or not available.
pub fn build_embedder_from_dict(
    spec: &Value,
) -> Result<Box<dyn EmbeddingFunctionTrait>, anyhow::Error> {
    let provider_name = spec
        .get("provider")
        .and_then(|p| p.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'provider' key in specification"))?;

    let registry = provider_registry();
    if !registry.contains_key(provider_name) {
        let available: Vec<&&str> = registry.keys().collect();
        return Err(anyhow::anyhow!(
            "Unknown provider: {}. Available providers: {:?}",
            provider_name,
            available
        ));
    }

    // TODO: Dynamically load and configure the provider based on provider_name
    // For now, return an error indicating the provider needs native integration
    Err(anyhow::anyhow!(
        "Provider '{}' is recognized but requires native Rust integration. \
         Provider path: {}",
        provider_name,
        registry[provider_name]
    ))
}

/// Build an embedding function from either a provider or a dictionary spec.
///
/// # Arguments
/// * `spec` - Either a JSON spec dict or a provider trait object reference.
///
/// This is the main entry point for building embedders.
pub fn build_embedder(spec: &Value) -> Result<Box<dyn EmbeddingFunctionTrait>, anyhow::Error> {
    build_embedder_from_dict(spec)
}

/// Backward compatibility alias.
pub fn get_embedding_function(
    spec: &Value,
) -> Result<Box<dyn EmbeddingFunctionTrait>, anyhow::Error> {
    build_embedder(spec)
}
