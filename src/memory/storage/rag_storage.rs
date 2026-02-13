//! RAG-based storage extending the base storage with embedding support.
//!
//! Port of crewai/memory/storage/rag_storage.py
//!
//! This MVP implements in-memory keyword-based search using TF-IDF-style
//! term frequency scoring. For production, swap the `entries` vec with a
//! proper vector DB (Qdrant, ChromaDB, LanceDB, etc.).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde_json::Value;

use crate::memory::storage::interface::Storage;

/// Maximum file name length for storage paths.
const MAX_FILE_NAME_LENGTH: usize = 255;

/// A single stored entry with its text and metadata.
#[derive(Debug, Clone)]
struct MemoryEntry {
    /// The raw text content.
    value: String,
    /// Lowercased words for search matching.
    tokens: Vec<String>,
    /// Associated metadata.
    metadata: HashMap<String, Value>,
}

/// RAGStorage extends Storage to handle embeddings for memory entries,
/// improving search efficiency through vector-based retrieval.
///
/// This MVP uses in-memory keyword search with term-frequency scoring.
/// When a real embedder/vector DB is configured, it should delegate to that.
pub struct RAGStorage {
    /// The type of memory (e.g., "short_term", "entities").
    pub storage_type: String,
    /// Whether reset is allowed.
    pub allow_reset: bool,
    /// Embedder configuration.
    pub embedder_config: Option<Value>,
    /// Concatenated sanitized agent roles.
    pub agents: String,
    /// The constructed storage file name.
    pub storage_file_name: String,
    /// Optional persist path.
    pub path: Option<String>,
    /// In-memory entries for keyword search MVP.
    entries: Arc<RwLock<Vec<MemoryEntry>>>,
}

impl RAGStorage {
    /// Create a new RAGStorage instance.
    ///
    /// # Arguments
    /// * `storage_type` - The type of memory storage.
    /// * `allow_reset` - Whether reset is allowed.
    /// * `embedder_config` - Optional embedder configuration.
    /// * `crew_agent_roles` - Optional list of agent role strings.
    /// * `path` - Optional persist directory path.
    pub fn new(
        storage_type: &str,
        allow_reset: bool,
        embedder_config: Option<Value>,
        crew_agent_roles: Option<Vec<String>>,
        path: Option<String>,
    ) -> Self {
        let sanitized_roles: Vec<String> = crew_agent_roles
            .unwrap_or_default()
            .iter()
            .map(|role| Self::sanitize_role(role))
            .collect();
        let agents_str = sanitized_roles.join("_");
        let storage_file_name =
            Self::build_storage_file_name(storage_type, &agents_str);

        Self {
            storage_type: storage_type.to_string(),
            allow_reset,
            embedder_config,
            agents: agents_str,
            storage_file_name,
            path,
            entries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Sanitize an agent role to ensure valid directory names.
    fn sanitize_role(role: &str) -> String {
        role.replace('\n', "")
            .replace(' ', "_")
            .replace('/', "_")
    }

    /// Build the storage file name, ensuring it does not exceed max allowed length.
    fn build_storage_file_name(storage_type: &str, file_name: &str) -> String {
        let base_path = format!(
            "{}/{}",
            crate::utilities::paths::db_storage_path(),
            storage_type
        );
        let trimmed = if file_name.len() > MAX_FILE_NAME_LENGTH {
            log::warn!(
                "Trimming file name from {} to {} characters.",
                file_name.len(),
                MAX_FILE_NAME_LENGTH
            );
            &file_name[..MAX_FILE_NAME_LENGTH]
        } else {
            file_name
        };
        format!("{}/{}", base_path, trimmed)
    }

    /// Get the collection name for this storage instance.
    fn collection_name(&self) -> String {
        if self.agents.is_empty() {
            format!("memory_{}", self.storage_type)
        } else {
            format!("memory_{}_{}", self.storage_type, self.agents)
        }
    }

    /// Tokenize text into lowercase words for keyword matching.
    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .filter(|w| w.len() >= 2)
            .map(String::from)
            .collect()
    }

    /// Compute a keyword overlap score between query tokens and entry tokens.
    /// Returns a value in [0.0, 1.0] representing the fraction of query terms found.
    fn keyword_score(query_tokens: &[String], entry_tokens: &[String]) -> f64 {
        if query_tokens.is_empty() {
            return 0.0;
        }
        let matches = query_tokens
            .iter()
            .filter(|qt| entry_tokens.contains(qt))
            .count();
        matches as f64 / query_tokens.len() as f64
    }
}

#[async_trait]
impl Storage for RAGStorage {
    fn save(
        &self,
        value: &str,
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error> {
        log::debug!(
            "RAGStorage save to '{}': value='{}'",
            self.collection_name(),
            &value[..std::cmp::min(value.len(), 100)]
        );

        let entry = MemoryEntry {
            value: value.to_string(),
            tokens: Self::tokenize(value),
            metadata: metadata.clone(),
        };

        let mut entries = self
            .entries
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        entries.push(entry);
        Ok(())
    }

    async fn asave(
        &self,
        value: &str,
        metadata: &HashMap<String, Value>,
    ) -> Result<(), anyhow::Error> {
        self.save(value, metadata)
    }

    fn search(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        log::debug!(
            "RAGStorage search in '{}': query='{}'",
            self.collection_name(),
            query
        );

        let query_tokens = Self::tokenize(query);
        let entries = self
            .entries
            .read()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

        // Score all entries and filter by threshold
        let mut scored: Vec<(f64, &MemoryEntry)> = entries
            .iter()
            .map(|entry| {
                let score = Self::keyword_score(&query_tokens, &entry.tokens);
                (score, entry)
            })
            .filter(|(score, _)| *score >= score_threshold)
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Take top-k results
        let results: Vec<Value> = scored
            .into_iter()
            .take(limit)
            .map(|(score, entry)| {
                serde_json::json!({
                    "content": entry.value,
                    "metadata": entry.metadata,
                    "score": score,
                })
            })
            .collect();

        Ok(results)
    }

    async fn asearch(
        &self,
        query: &str,
        limit: usize,
        score_threshold: f64,
    ) -> Result<Vec<Value>, anyhow::Error> {
        self.search(query, limit, score_threshold)
    }

    fn reset(&self) -> Result<(), anyhow::Error> {
        log::debug!("RAGStorage reset collection '{}'", self.collection_name());
        let mut entries = self
            .entries
            .write()
            .map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;
        entries.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_storage_save_and_search() {
        let storage = RAGStorage::new("short_term", true, None, None, None);

        let mut meta = HashMap::new();
        meta.insert("agent".to_string(), Value::String("researcher".to_string()));

        storage
            .save("Rust is a systems programming language", &meta)
            .unwrap();
        storage
            .save("Python is great for data science", &meta)
            .unwrap();
        storage
            .save("Rust and Python are both popular languages", &meta)
            .unwrap();

        let results = storage.search("Rust programming", 10, 0.1).unwrap();
        assert!(!results.is_empty());
        // First result should be about Rust programming
        let first = &results[0];
        assert!(first["content"].as_str().unwrap().contains("Rust"));
    }

    #[test]
    fn test_rag_storage_search_threshold() {
        let storage = RAGStorage::new("entities", true, None, None, None);

        let meta = HashMap::new();
        storage.save("machine learning algorithms", &meta).unwrap();
        storage.save("completely unrelated text", &meta).unwrap();

        // High threshold should filter out poor matches
        let results = storage.search("machine learning", 10, 0.9).unwrap();
        assert!(results.len() <= 1);
    }

    #[test]
    fn test_rag_storage_reset() {
        let storage = RAGStorage::new("short_term", true, None, None, None);
        let meta = HashMap::new();
        storage.save("test entry", &meta).unwrap();

        let results = storage.search("test", 10, 0.0).unwrap();
        assert!(!results.is_empty());

        storage.reset().unwrap();
        let results = storage.search("test", 10, 0.0).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_rag_storage_collection_name() {
        let s1 = RAGStorage::new("short_term", true, None, None, None);
        assert_eq!(s1.collection_name(), "memory_short_term");

        let s2 = RAGStorage::new(
            "short_term",
            true,
            None,
            Some(vec!["researcher".to_string(), "writer".to_string()]),
            None,
        );
        assert_eq!(s2.collection_name(), "memory_short_term_researcher_writer");
    }

    #[test]
    fn test_tokenize() {
        let tokens = RAGStorage::tokenize("Hello, World! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        // Single-char words should be filtered
        assert!(!tokens.contains(&"a".to_string()));
    }
}
