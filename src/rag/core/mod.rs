//! Core abstractions for the RAG system.
//!
//! Port of crewai/rag/core/

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

// Re-export core types so downstream modules (e.g., providers) can import
// `Embeddings` through `crate::rag::core::Embeddings`.
pub use crate::rag::types::{BaseRecord, Embeddings, SearchResult};

// ---------------------------------------------------------------------------
// EmbeddingResult type
// ---------------------------------------------------------------------------

/// Result type for individual embedding operations.
///
/// Represents a single embedding vector as a list of f32 values.
pub type EmbeddingResult = Vec<f32>;

// ---------------------------------------------------------------------------
// BaseClient trait
// ---------------------------------------------------------------------------

/// Parameters for collection operations.
pub struct CollectionParams {
    /// The name of the collection/index to operate on.
    pub collection_name: String,
}

/// Parameters for adding documents to a collection.
pub struct CollectionAddParams {
    /// The name of the collection to add documents to.
    pub collection_name: String,
    /// List of document records.
    pub documents: Vec<BaseRecord>,
    /// Optional batch size for processing documents.
    pub batch_size: Option<usize>,
}

/// Parameters for searching within a collection.
pub struct CollectionSearchParams {
    /// The name of the collection to search in.
    pub collection_name: String,
    /// The text query to search for.
    pub query: String,
    /// Maximum number of results to return.
    pub limit: Option<usize>,
    /// Filter results by metadata fields.
    pub metadata_filter: Option<HashMap<String, Value>>,
    /// Minimum similarity score for results (0-1).
    pub score_threshold: Option<f64>,
}

/// Trait for vector store client implementations.
///
/// Defines the interface that all vector store client implementations
/// must follow. Provides a consistent API for storing and retrieving
/// documents with their vector embeddings across different backends
/// (e.g., Qdrant, ChromaDB).
#[async_trait]
pub trait BaseClient: Send + Sync {
    /// Create a new collection/index in the vector database.
    fn create_collection(&self, params: &CollectionParams) -> Result<(), anyhow::Error>;

    /// Create a new collection asynchronously.
    async fn acreate_collection(&self, params: &CollectionParams) -> Result<(), anyhow::Error> {
        self.create_collection(params)
    }

    /// Get an existing collection or create it if it doesn't exist.
    fn get_or_create_collection(&self, params: &CollectionParams) -> Result<(), anyhow::Error>;

    /// Get or create a collection asynchronously.
    async fn aget_or_create_collection(
        &self,
        params: &CollectionParams,
    ) -> Result<(), anyhow::Error> {
        self.get_or_create_collection(params)
    }

    /// Add documents with their embeddings to a collection.
    fn add_documents(&self, params: &CollectionAddParams) -> Result<(), anyhow::Error>;

    /// Add documents asynchronously.
    async fn aadd_documents(&self, params: &CollectionAddParams) -> Result<(), anyhow::Error> {
        self.add_documents(params)
    }

    /// Search for similar documents using a query.
    fn search(&self, params: &CollectionSearchParams) -> Result<Vec<SearchResult>, anyhow::Error>;

    /// Search asynchronously.
    async fn asearch(
        &self,
        params: &CollectionSearchParams,
    ) -> Result<Vec<SearchResult>, anyhow::Error> {
        self.search(params)
    }

    /// Delete a collection and all its data.
    fn delete_collection(&self, params: &CollectionParams) -> Result<(), anyhow::Error>;

    /// Delete a collection asynchronously.
    async fn adelete_collection(&self, params: &CollectionParams) -> Result<(), anyhow::Error> {
        self.delete_collection(params)
    }

    /// Reset the vector database by deleting all collections and data.
    fn reset(&self) -> Result<(), anyhow::Error>;

    /// Reset asynchronously.
    async fn areset(&self) -> Result<(), anyhow::Error> {
        self.reset()
    }
}

// ---------------------------------------------------------------------------
// BaseEmbeddingsProvider trait
// ---------------------------------------------------------------------------

/// Trait for embedding providers.
///
/// Embedding providers configure and build embedding functions from various
/// backends (OpenAI, Cohere, HuggingFace, etc.).
pub trait BaseEmbeddingsProvider: Send + Sync {
    /// Get the provider name (e.g., "openai", "cohere").
    fn provider_name(&self) -> &str;

    /// Build an embedding function from this provider's configuration.
    ///
    /// # Returns
    /// A boxed embedding function.
    fn build_embedding_function(&self) -> Result<Box<dyn EmbeddingFunctionTrait>, anyhow::Error>;

    /// Get the provider's configuration as a JSON value.
    fn config(&self) -> Value;
}

// ---------------------------------------------------------------------------
// BaseEmbedding trait
// ---------------------------------------------------------------------------

/// Trait for embedding models.
///
/// Individual embedding providers implement this trait to provide
/// text-to-vector conversion capabilities. This mirrors the Python
/// embedding callable interface used by providers like OpenAI, Cohere,
/// HuggingFace, etc.
#[async_trait]
pub trait BaseEmbedding: Send + Sync {
    /// Embed a single text string into a vector.
    ///
    /// # Arguments
    /// * `text` - The text to embed.
    ///
    /// # Returns
    /// An embedding vector as a `Vec<f32>`.
    async fn embed_text(&self, text: &str) -> EmbeddingResult;

    /// Embed multiple documents into vectors.
    ///
    /// # Arguments
    /// * `documents` - Slice of document strings to embed.
    ///
    /// # Returns
    /// A vector of embedding results, one per document.
    async fn embed_documents(&self, documents: &[String]) -> Vec<EmbeddingResult>;
}

// ---------------------------------------------------------------------------
// EmbeddingFunction trait
// ---------------------------------------------------------------------------

/// Trait for embedding functions.
///
/// Embedding functions convert input text into vector embeddings.
pub trait EmbeddingFunctionTrait: Send + Sync {
    /// Convert input texts to embeddings.
    ///
    /// # Arguments
    /// * `input` - List of text strings to embed.
    ///
    /// # Returns
    /// A list of embedding vectors.
    fn call(&self, input: &[String]) -> Result<Embeddings, anyhow::Error>;

    /// Embed a single query (alias for `call` with a single input).
    fn embed_query(&self, input: &str) -> Result<Vec<f32>, anyhow::Error> {
        let results = self.call(&[input.to_string()])?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Embedding function returned no results"))
    }
}
