//! Knowledge source implementations for ingesting data from various formats.
//!
//! Corresponds to `crewai/knowledge/source/`.
//!
//! Provides the `BaseKnowledgeSource` and `BaseFileKnowledgeSource` traits
//! along with concrete implementations for strings, text files, CSV, PDF,
//! JSON, and Excel sources.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::knowledge::storage::{BaseKnowledgeStorage, KnowledgeStorage};

// ---------------------------------------------------------------------------
// Base traits
// ---------------------------------------------------------------------------

/// Base trait for all knowledge sources.
///
/// Knowledge sources are responsible for loading content from their backing
/// store, chunking it into smaller pieces, and saving it to a
/// `KnowledgeStorage` instance.
///
/// Corresponds to `crewai.knowledge.source.base_knowledge_source.BaseKnowledgeSource`.
#[async_trait]
pub trait BaseKnowledgeSource: Send + Sync {
    /// Human-readable name of this source (for debugging / logging).
    fn source_name(&self) -> &str;

    /// Validate the content source (e.g., check file existence).
    ///
    /// Called before `add()` to ensure the source is valid.
    /// Default implementation always succeeds.
    fn validate_content(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }

    /// Load content from the source, returning a list of text chunks.
    fn load_content(&self) -> Result<Vec<String>, anyhow::Error>;

    /// Add loaded content to the knowledge storage (sync).
    ///
    /// Validates content, loads and chunks it, then saves to storage.
    fn add(&self, storage: &Arc<KnowledgeStorage>) -> Result<(), anyhow::Error>;

    /// Add loaded content to the knowledge storage asynchronously.
    ///
    /// Default implementation delegates to the synchronous `add()`.
    async fn aadd(&self, storage: &Arc<KnowledgeStorage>) -> Result<(), anyhow::Error> {
        self.add(storage)
    }

    /// Get optional metadata for this source.
    fn metadata(&self) -> HashMap<String, Value> {
        HashMap::new()
    }

    /// Get the list of embeddings for the chunks.
    ///
    /// Returns empty by default; sources that compute embeddings
    /// can override this.
    fn get_embeddings(&self) -> Vec<Vec<f32>> {
        Vec::new()
    }

    /// Chunk text content into smaller pieces.
    ///
    /// Uses a sliding window approach with configurable size and overlap.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to chunk.
    /// * `chunk_size` - Maximum size per chunk. Defaults to 4000.
    /// * `chunk_overlap` - Overlap between consecutive chunks. Defaults to 200.
    fn chunk_text(
        &self,
        text: &str,
        chunk_size: Option<usize>,
        chunk_overlap: Option<usize>,
    ) -> Vec<String> {
        let chunk_size = chunk_size.unwrap_or(4000);
        let chunk_overlap = chunk_overlap.unwrap_or(200);

        if text.len() <= chunk_size {
            return vec![text.to_string()];
        }

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < text.len() {
            let end = std::cmp::min(start + chunk_size, text.len());
            chunks.push(text[start..end].to_string());
            if end == text.len() {
                break;
            }
            start += chunk_size - chunk_overlap;
        }

        chunks
    }
}

/// Base trait for file-based knowledge sources.
///
/// Extends `BaseKnowledgeSource` with file path support and validation.
///
/// Corresponds to `crewai.knowledge.source.base_file_knowledge_source.BaseFileKnowledgeSource`.
#[async_trait]
pub trait BaseFileKnowledgeSource: BaseKnowledgeSource {
    /// Get the file paths for this source.
    fn file_paths(&self) -> &[PathBuf];

    /// Validate that all file paths exist and are accessible.
    fn validate_paths(&self) -> Result<(), anyhow::Error> {
        for path in self.file_paths() {
            if !path.exists() {
                return Err(anyhow::anyhow!(
                    "File not found: {}",
                    path.display()
                ));
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Concrete source implementations
// ---------------------------------------------------------------------------

/// Knowledge source for plain string content.
///
/// Directly ingests a string value into the knowledge base.
///
/// Corresponds to `crewai.knowledge.source.string_knowledge_source.StringKnowledgeSource`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringKnowledgeSource {
    /// The raw string content.
    pub content: String,
    /// Optional chunk size override.
    pub chunk_size: Option<usize>,
    /// Optional chunk overlap override.
    pub chunk_overlap: Option<usize>,
    /// Optional metadata to attach to chunks.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    /// Optional collection name override.
    pub collection_name: Option<String>,
}

impl StringKnowledgeSource {
    /// Create a new StringKnowledgeSource with the given content.
    pub fn new(content: String) -> Self {
        Self {
            content,
            chunk_size: None,
            chunk_overlap: None,
            metadata: HashMap::new(),
            collection_name: None,
        }
    }

    /// Builder: set metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Builder: set chunk parameters.
    pub fn with_chunking(mut self, chunk_size: usize, chunk_overlap: usize) -> Self {
        self.chunk_size = Some(chunk_size);
        self.chunk_overlap = Some(chunk_overlap);
        self
    }

    /// Builder: set collection name.
    pub fn with_collection_name(mut self, name: String) -> Self {
        self.collection_name = Some(name);
        self
    }
}

#[async_trait]
impl BaseKnowledgeSource for StringKnowledgeSource {
    fn source_name(&self) -> &str {
        "StringKnowledgeSource"
    }

    fn load_content(&self) -> Result<Vec<String>, anyhow::Error> {
        Ok(self.chunk_text(&self.content, self.chunk_size, self.chunk_overlap))
    }

    fn add(&self, storage: &Arc<KnowledgeStorage>) -> Result<(), anyhow::Error> {
        let chunks = self.load_content()?;
        storage.save_chunks(&chunks, &self.metadata)
    }

    fn metadata(&self) -> HashMap<String, Value> {
        self.metadata.clone()
    }
}

/// Knowledge source for plain text files.
///
/// Reads and ingests the contents of one or more text files.
///
/// Corresponds to `crewai.knowledge.source.text_file_knowledge_source.TextFileKnowledgeSource`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextFileKnowledgeSource {
    /// Paths to the text files.
    pub file_paths: Vec<PathBuf>,
    /// Optional chunk size override.
    pub chunk_size: Option<usize>,
    /// Optional chunk overlap override.
    pub chunk_overlap: Option<usize>,
    /// Optional metadata to attach to chunks.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    /// Optional collection name override.
    pub collection_name: Option<String>,
}

impl TextFileKnowledgeSource {
    /// Create a new TextFileKnowledgeSource with the given file paths.
    pub fn new(file_paths: Vec<PathBuf>) -> Self {
        Self {
            file_paths,
            chunk_size: None,
            chunk_overlap: None,
            metadata: HashMap::new(),
            collection_name: None,
        }
    }
}

#[async_trait]
impl BaseKnowledgeSource for TextFileKnowledgeSource {
    fn source_name(&self) -> &str {
        "TextFileKnowledgeSource"
    }

    fn load_content(&self) -> Result<Vec<String>, anyhow::Error> {
        let mut all_chunks = Vec::new();
        for path in &self.file_paths {
            let content = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;
            let chunks = self.chunk_text(&content, self.chunk_size, self.chunk_overlap);
            all_chunks.extend(chunks);
        }
        Ok(all_chunks)
    }

    fn add(&self, storage: &Arc<KnowledgeStorage>) -> Result<(), anyhow::Error> {
        let chunks = self.load_content()?;
        storage.save_chunks(&chunks, &self.metadata)
    }

    fn metadata(&self) -> HashMap<String, Value> {
        self.metadata.clone()
    }
}

#[async_trait]
impl BaseFileKnowledgeSource for TextFileKnowledgeSource {
    fn file_paths(&self) -> &[PathBuf] {
        &self.file_paths
    }
}

/// Knowledge source for CSV files.
///
/// Each row of the CSV becomes a separate chunk for ingestion.
///
/// Corresponds to `crewai.knowledge.source.csv_knowledge_source.CSVKnowledgeSource`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CSVKnowledgeSource {
    /// Paths to the CSV files.
    pub file_paths: Vec<PathBuf>,
    /// Optional chunk size override.
    pub chunk_size: Option<usize>,
    /// Optional chunk overlap override.
    pub chunk_overlap: Option<usize>,
    /// Optional metadata to attach to chunks.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    /// Optional collection name override.
    pub collection_name: Option<String>,
}

impl CSVKnowledgeSource {
    /// Create a new CSVKnowledgeSource with the given file paths.
    pub fn new(file_paths: Vec<PathBuf>) -> Self {
        Self {
            file_paths,
            chunk_size: None,
            chunk_overlap: None,
            metadata: HashMap::new(),
            collection_name: None,
        }
    }
}

#[async_trait]
impl BaseKnowledgeSource for CSVKnowledgeSource {
    fn source_name(&self) -> &str {
        "CSVKnowledgeSource"
    }

    fn load_content(&self) -> Result<Vec<String>, anyhow::Error> {
        let mut all_chunks = Vec::new();
        for path in &self.file_paths {
            let content = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;
            // Each row of a CSV becomes a chunk.
            for line in content.lines() {
                if !line.trim().is_empty() {
                    all_chunks.push(line.to_string());
                }
            }
        }
        Ok(all_chunks)
    }

    fn add(&self, storage: &Arc<KnowledgeStorage>) -> Result<(), anyhow::Error> {
        let chunks = self.load_content()?;
        storage.save_chunks(&chunks, &self.metadata)
    }

    fn metadata(&self) -> HashMap<String, Value> {
        self.metadata.clone()
    }
}

#[async_trait]
impl BaseFileKnowledgeSource for CSVKnowledgeSource {
    fn file_paths(&self) -> &[PathBuf] {
        &self.file_paths
    }
}

/// Knowledge source for PDF files.
///
/// Note: PDF parsing requires an external library such as `lopdf` or `pdf-extract`.
/// This implementation provides the structure and will return an error until
/// a PDF parsing crate is integrated.
///
/// Corresponds to `crewai.knowledge.source.pdf_knowledge_source.PDFKnowledgeSource`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PDFKnowledgeSource {
    /// Paths to the PDF files.
    pub file_paths: Vec<PathBuf>,
    /// Optional chunk size override.
    pub chunk_size: Option<usize>,
    /// Optional chunk overlap override.
    pub chunk_overlap: Option<usize>,
    /// Optional metadata to attach to chunks.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    /// Optional collection name override.
    pub collection_name: Option<String>,
}

impl PDFKnowledgeSource {
    /// Create a new PDFKnowledgeSource with the given file paths.
    pub fn new(file_paths: Vec<PathBuf>) -> Self {
        Self {
            file_paths,
            chunk_size: None,
            chunk_overlap: None,
            metadata: HashMap::new(),
            collection_name: None,
        }
    }
}

#[async_trait]
impl BaseKnowledgeSource for PDFKnowledgeSource {
    fn source_name(&self) -> &str {
        "PDFKnowledgeSource"
    }

    fn load_content(&self) -> Result<Vec<String>, anyhow::Error> {
        // PDF parsing requires an external crate (e.g., pdf-extract, lopdf).
        // This stub returns an error until integration is complete.
        Err(anyhow::anyhow!(
            "PDF knowledge source requires a PDF parsing library. \
             Please integrate `pdf-extract` or equivalent crate."
        ))
    }

    fn add(&self, storage: &Arc<KnowledgeStorage>) -> Result<(), anyhow::Error> {
        let chunks = self.load_content()?;
        storage.save_chunks(&chunks, &self.metadata)
    }

    fn metadata(&self) -> HashMap<String, Value> {
        self.metadata.clone()
    }
}

#[async_trait]
impl BaseFileKnowledgeSource for PDFKnowledgeSource {
    fn file_paths(&self) -> &[PathBuf] {
        &self.file_paths
    }
}

/// Knowledge source for JSON files.
///
/// Recursively flattens JSON structures into readable text, then chunks
/// and ingests them.
///
/// Corresponds to `crewai.knowledge.source.json_knowledge_source.JSONKnowledgeSource`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONKnowledgeSource {
    /// Paths to the JSON files.
    pub file_paths: Vec<PathBuf>,
    /// Optional chunk size override.
    pub chunk_size: Option<usize>,
    /// Optional chunk overlap override.
    pub chunk_overlap: Option<usize>,
    /// Optional metadata to attach to chunks.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    /// Optional collection name override.
    pub collection_name: Option<String>,
}

impl JSONKnowledgeSource {
    /// Create a new JSONKnowledgeSource with the given file paths.
    pub fn new(file_paths: Vec<PathBuf>) -> Self {
        Self {
            file_paths,
            chunk_size: None,
            chunk_overlap: None,
            metadata: HashMap::new(),
            collection_name: None,
        }
    }

    /// Recursively convert a JSON value to a readable text representation.
    fn json_to_text(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => arr
                .iter()
                .map(Self::json_to_text)
                .collect::<Vec<_>>()
                .join("\n"),
            Value::Object(map) => map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, Self::json_to_text(v)))
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

#[async_trait]
impl BaseKnowledgeSource for JSONKnowledgeSource {
    fn source_name(&self) -> &str {
        "JSONKnowledgeSource"
    }

    fn load_content(&self) -> Result<Vec<String>, anyhow::Error> {
        let mut all_chunks = Vec::new();
        for path in &self.file_paths {
            let content = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;
            let parsed: Value = serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse JSON {}: {}", path.display(), e))?;
            let text = Self::json_to_text(&parsed);
            let chunks = self.chunk_text(&text, self.chunk_size, self.chunk_overlap);
            all_chunks.extend(chunks);
        }
        Ok(all_chunks)
    }

    fn add(&self, storage: &Arc<KnowledgeStorage>) -> Result<(), anyhow::Error> {
        let chunks = self.load_content()?;
        storage.save_chunks(&chunks, &self.metadata)
    }

    fn metadata(&self) -> HashMap<String, Value> {
        self.metadata.clone()
    }
}

#[async_trait]
impl BaseFileKnowledgeSource for JSONKnowledgeSource {
    fn file_paths(&self) -> &[PathBuf] {
        &self.file_paths
    }
}

/// Knowledge source for Excel files.
///
/// Note: Excel parsing requires a library such as `calamine`.
/// This implementation provides the structure and will return an error until
/// an Excel parsing crate is integrated.
///
/// Corresponds to `crewai.knowledge.source.excel_knowledge_source.ExcelKnowledgeSource`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelKnowledgeSource {
    /// Paths to the Excel files.
    pub file_paths: Vec<PathBuf>,
    /// Optional chunk size override.
    pub chunk_size: Option<usize>,
    /// Optional chunk overlap override.
    pub chunk_overlap: Option<usize>,
    /// Optional metadata to attach to chunks.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    /// Optional collection name override.
    pub collection_name: Option<String>,
}

impl ExcelKnowledgeSource {
    /// Create a new ExcelKnowledgeSource with the given file paths.
    pub fn new(file_paths: Vec<PathBuf>) -> Self {
        Self {
            file_paths,
            chunk_size: None,
            chunk_overlap: None,
            metadata: HashMap::new(),
            collection_name: None,
        }
    }
}

#[async_trait]
impl BaseKnowledgeSource for ExcelKnowledgeSource {
    fn source_name(&self) -> &str {
        "ExcelKnowledgeSource"
    }

    fn load_content(&self) -> Result<Vec<String>, anyhow::Error> {
        // Excel parsing requires an external crate (e.g., calamine).
        // This stub returns an error until integration is complete.
        Err(anyhow::anyhow!(
            "Excel knowledge source requires an Excel parsing library. \
             Please integrate `calamine` or equivalent crate."
        ))
    }

    fn add(&self, storage: &Arc<KnowledgeStorage>) -> Result<(), anyhow::Error> {
        let chunks = self.load_content()?;
        storage.save_chunks(&chunks, &self.metadata)
    }

    fn metadata(&self) -> HashMap<String, Value> {
        self.metadata.clone()
    }
}

#[async_trait]
impl BaseFileKnowledgeSource for ExcelKnowledgeSource {
    fn file_paths(&self) -> &[PathBuf] {
        &self.file_paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_knowledge_source_new() {
        let source = StringKnowledgeSource::new("Hello world".to_string());
        assert_eq!(source.content, "Hello world");
        assert_eq!(source.source_name(), "StringKnowledgeSource");
    }

    #[test]
    fn test_string_knowledge_source_load_content() {
        let source = StringKnowledgeSource::new("Hello world".to_string());
        let chunks = source.load_content().unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Hello world");
    }

    #[test]
    fn test_string_knowledge_source_chunking() {
        let long_text = "a".repeat(5000);
        let source = StringKnowledgeSource::new(long_text);
        let chunks = source.load_content().unwrap();
        assert!(chunks.len() > 1);
    }

    #[test]
    fn test_chunk_text_basic() {
        let source = StringKnowledgeSource::new(String::new());
        let chunks = source.chunk_text("Hello world", None, None);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Hello world");
    }

    #[test]
    fn test_chunk_text_with_overlap() {
        let text = "a".repeat(100);
        let source = StringKnowledgeSource::new(String::new());
        let chunks = source.chunk_text(&text, Some(50), Some(10));
        assert!(chunks.len() > 1);
        // Each chunk should be at most 50 chars.
        for chunk in &chunks {
            assert!(chunk.len() <= 50);
        }
    }

    #[test]
    fn test_json_knowledge_source_json_to_text() {
        let json = serde_json::json!({"name": "Alice", "age": 30});
        let text = JSONKnowledgeSource::json_to_text(&json);
        assert!(text.contains("name: Alice"));
        assert!(text.contains("age: 30"));
    }

    #[test]
    fn test_string_knowledge_source_builders() {
        let mut meta = HashMap::new();
        meta.insert("key".to_string(), Value::String("value".to_string()));
        let source = StringKnowledgeSource::new("content".to_string())
            .with_metadata(meta.clone())
            .with_chunking(2000, 100)
            .with_collection_name("test_coll".to_string());

        assert_eq!(source.chunk_size, Some(2000));
        assert_eq!(source.chunk_overlap, Some(100));
        assert_eq!(source.collection_name.as_deref(), Some("test_coll"));
        assert_eq!(source.metadata().len(), 1);
    }
}
