//! Type definitions for RAG (Retrieval-Augmented Generation) systems.
//!
//! Port of crewai/rag/types.py

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A document record for storage in vector databases.
///
/// Represents a single document with its content, optional ID, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseRecord {
    /// Optional unique identifier for the document.
    /// If not provided, a content-based ID will be generated using SHA256 hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_id: Option<String>,
    /// The text content of the document (required).
    pub content: String,
    /// Optional metadata associated with the document.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl BaseRecord {
    /// Create a new BaseRecord with only content.
    pub fn new(content: String) -> Self {
        Self {
            doc_id: None,
            content,
            metadata: HashMap::new(),
        }
    }

    /// Create a new BaseRecord with content and a doc_id.
    pub fn with_id(doc_id: String, content: String) -> Self {
        Self {
            doc_id: Some(doc_id),
            content,
            metadata: HashMap::new(),
        }
    }

    /// Set metadata for this record.
    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Get or generate the document ID.
    ///
    /// If no doc_id was explicitly set, generates one from the content hash.
    pub fn get_or_generate_id(&self) -> String {
        match &self.doc_id {
            Some(id) => id.clone(),
            None => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                self.content.hash(&mut hasher);
                format!("{:x}", hasher.finish())
            }
        }
    }
}

/// Type alias for embedding vectors.
/// Each embedding is a vector of f32 values.
pub type Embeddings = Vec<Vec<f32>>;

/// Type alias for an embedding function.
///
/// An embedding function takes a list of text strings and returns embeddings.
pub type EmbeddingFunction = Box<dyn Fn(&[String]) -> Embeddings + Send + Sync>;

/// Standard search result format for vector store queries.
///
/// Provides a consistent interface for search results across different
/// vector store implementations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Unique identifier of the document.
    pub id: String,
    /// The text content of the document.
    pub content: String,
    /// Metadata associated with the document.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    /// Similarity score (higher is better, typically between 0 and 1).
    pub score: f64,
}

impl SearchResult {
    /// Create a new SearchResult.
    pub fn new(
        id: String,
        content: String,
        metadata: HashMap<String, Value>,
        score: f64,
    ) -> Self {
        Self {
            id,
            content,
            metadata,
            score,
        }
    }
}
