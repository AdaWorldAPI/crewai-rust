//! Cache handler for tool usage results.
//!
//! Corresponds to `crewai/agents/cache/cache_handler.py`.
//!
//! Provides thread-safe in-memory caching for tool outputs based on
//! tool name and input. The cache key is built from "{tool}-{input}".

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde_json::Value;

/// Handles caching of tool execution results.
///
/// Provides thread-safe in-memory caching for tool outputs based on tool
/// name and input. Uses a `RwLock` to allow concurrent reads while ensuring
/// exclusive write access.
///
/// Corresponds to `crewai.agents.cache.cache_handler.CacheHandler`.
#[derive(Debug, Clone)]
pub struct CacheHandler {
    /// Internal cache storage, keyed by "{tool}-{input}".
    cache: Arc<RwLock<HashMap<String, Value>>>,
}

impl Default for CacheHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheHandler {
    /// Create a new empty `CacheHandler`.
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a tool result to the cache.
    ///
    /// # Arguments
    ///
    /// * `tool` - Name of the tool.
    /// * `input` - Input string used for the tool.
    /// * `output` - Output result from tool execution.
    pub fn add(&self, tool: &str, input: &str, output: Value) {
        let key = format!("{}-{}", tool, input);
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(key, output);
        }
    }

    /// Retrieve a cached tool result.
    ///
    /// # Arguments
    ///
    /// * `tool` - Name of the tool.
    /// * `input` - Input string used for the tool.
    ///
    /// # Returns
    ///
    /// The cached result if found, `None` otherwise.
    pub fn read(&self, tool: &str, input: &str) -> Option<Value> {
        let key = format!("{}-{}", tool, input);
        if let Ok(cache) = self.cache.read() {
            cache.get(&key).cloned()
        } else {
            None
        }
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// Get the number of cached entries.
    pub fn len(&self) -> usize {
        self.cache.read().map(|c| c.len()).unwrap_or(0)
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_add_and_read() {
        let cache = CacheHandler::new();
        cache.add(
            "search",
            "query=rust",
            Value::String("Found results".to_string()),
        );

        let result = cache.read("search", "query=rust");
        assert_eq!(result, Some(Value::String("Found results".to_string())));
    }

    #[test]
    fn test_cache_miss() {
        let cache = CacheHandler::new();
        let result = cache.read("search", "missing");
        assert_eq!(result, None);
    }

    #[test]
    fn test_cache_clear() {
        let cache = CacheHandler::new();
        cache.add("tool", "input", Value::Bool(true));
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_thread_safety() {
        use std::thread;

        let cache = CacheHandler::new();
        let cache_clone = cache.clone();

        let writer = thread::spawn(move || {
            for i in 0..100 {
                cache_clone.add("tool", &format!("input_{}", i), Value::Number(i.into()));
            }
        });

        let cache_clone2 = cache.clone();
        let reader = thread::spawn(move || {
            for i in 0..100 {
                let _ = cache_clone2.read("tool", &format!("input_{}", i));
            }
        });

        writer.join().unwrap();
        reader.join().unwrap();

        assert_eq!(cache.len(), 100);
    }
}
