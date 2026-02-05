//! Contextual memory that aggregates and retrieves context from multiple memory sources.
//!
//! Port of crewai/memory/contextual/contextual_memory.py

use std::collections::HashMap;

use serde_json::Value;

use crate::memory::entity::EntityMemory;
use crate::memory::external::ExternalMemory;
use crate::memory::long_term::LongTermMemory;
use crate::memory::short_term::ShortTermMemory;

/// ContextualMemory aggregates and retrieves context from multiple memory sources
/// (short-term, long-term, entity, and external memory).
pub struct ContextualMemory {
    /// Short-term memory instance.
    pub stm: Option<ShortTermMemory>,
    /// Long-term memory instance.
    pub ltm: Option<LongTermMemory>,
    /// Entity memory instance.
    pub em: Option<EntityMemory>,
    /// External memory instance.
    pub exm: Option<ExternalMemory>,
}

impl ContextualMemory {
    /// Create a new ContextualMemory instance.
    ///
    /// # Arguments
    /// * `stm` - Optional ShortTermMemory instance.
    /// * `ltm` - Optional LongTermMemory instance.
    /// * `em` - Optional EntityMemory instance.
    /// * `exm` - Optional ExternalMemory instance.
    pub fn new(
        stm: Option<ShortTermMemory>,
        ltm: Option<LongTermMemory>,
        em: Option<EntityMemory>,
        exm: Option<ExternalMemory>,
    ) -> Self {
        Self { stm, ltm, em, exm }
    }

    /// Build contextual information for a task synchronously.
    ///
    /// # Arguments
    /// * `task_description` - The task description.
    /// * `context` - Additional context string.
    ///
    /// # Returns
    /// Formatted context string from all memory sources.
    pub fn build_context_for_task(
        &self,
        task_description: &str,
        context: &str,
    ) -> Result<String, anyhow::Error> {
        let query = format!("{} {}", task_description, context).trim().to_string();

        if query.is_empty() {
            return Ok(String::new());
        }

        let mut context_parts = Vec::new();

        if let Some(ltm_context) = self.fetch_ltm_context(task_description)? {
            if !ltm_context.is_empty() {
                context_parts.push(ltm_context);
            }
        }

        let stm_context = self.fetch_stm_context(&query)?;
        if !stm_context.is_empty() {
            context_parts.push(stm_context);
        }

        let entity_context = self.fetch_entity_context(&query)?;
        if !entity_context.is_empty() {
            context_parts.push(entity_context);
        }

        let external_context = self.fetch_external_context(&query)?;
        if !external_context.is_empty() {
            context_parts.push(external_context);
        }

        Ok(context_parts.join("\n"))
    }

    /// Build contextual information for a task asynchronously.
    pub async fn abuild_context_for_task(
        &self,
        task_description: &str,
        context: &str,
    ) -> Result<String, anyhow::Error> {
        let query = format!("{} {}", task_description, context).trim().to_string();

        if query.is_empty() {
            return Ok(String::new());
        }

        // Fetch all contexts concurrently
        let (ltm_result, stm_result, entity_result, external_result) = tokio::join!(
            self.afetch_ltm_context(task_description),
            self.afetch_stm_context(&query),
            self.afetch_entity_context(&query),
            self.afetch_external_context(&query),
        );

        let mut context_parts = Vec::new();

        if let Some(ltm_context) = ltm_result? {
            if !ltm_context.is_empty() {
                context_parts.push(ltm_context);
            }
        }

        let stm_context = stm_result?;
        if !stm_context.is_empty() {
            context_parts.push(stm_context);
        }

        let entity_context = entity_result?;
        if !entity_context.is_empty() {
            context_parts.push(entity_context);
        }

        let external_context = external_result?;
        if !external_context.is_empty() {
            context_parts.push(external_context);
        }

        Ok(context_parts.join("\n"))
    }

    /// Fetch recent relevant insights from STM.
    fn fetch_stm_context(&self, query: &str) -> Result<String, anyhow::Error> {
        let stm = match &self.stm {
            Some(s) => s,
            None => return Ok(String::new()),
        };

        let stm_results = stm.search(query, 5, 0.6)?;
        if stm_results.is_empty() {
            return Ok(String::new());
        }

        let formatted: Vec<String> = stm_results
            .iter()
            .filter_map(|r| r.get("content").and_then(|c| c.as_str()))
            .map(|c| format!("- {}", c))
            .collect();

        Ok(format!("Recent Insights:\n{}", formatted.join("\n")))
    }

    /// Fetch historical data from LTM.
    fn fetch_ltm_context(
        &self,
        task: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let ltm = match &self.ltm {
            Some(l) => l,
            None => return Ok(Some(String::new())),
        };

        let ltm_results = ltm.search(task, 2)?;
        if ltm_results.is_empty() {
            return Ok(None);
        }

        let mut suggestions: Vec<String> = Vec::new();
        for result in &ltm_results {
            if let Some(metadata) = result.get("metadata") {
                if let Some(suggs) = metadata.get("suggestions") {
                    if let Some(arr) = suggs.as_array() {
                        for s in arr {
                            if let Some(text) = s.as_str() {
                                if !suggestions.contains(&text.to_string()) {
                                    suggestions.push(text.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        let formatted: Vec<String> = suggestions.iter().map(|s| format!("- {}", s)).collect();

        if formatted.is_empty() {
            Ok(None)
        } else {
            Ok(Some(format!(
                "Historical Data:\n{}",
                formatted.join("\n")
            )))
        }
    }

    /// Fetch relevant entity information from Entity Memory.
    fn fetch_entity_context(&self, query: &str) -> Result<String, anyhow::Error> {
        let em = match &self.em {
            Some(e) => e,
            None => return Ok(String::new()),
        };

        let em_results = em.search(query, 5, 0.6)?;
        if em_results.is_empty() {
            return Ok(String::new());
        }

        let formatted: Vec<String> = em_results
            .iter()
            .filter_map(|r| r.get("content").and_then(|c| c.as_str()))
            .map(|c| format!("- {}", c))
            .collect();

        Ok(format!("Entities:\n{}", formatted.join("\n")))
    }

    /// Fetch relevant information from External Memory.
    fn fetch_external_context(&self, query: &str) -> Result<String, anyhow::Error> {
        let exm = match &self.exm {
            Some(e) => e,
            None => return Ok(String::new()),
        };

        let external_memories = exm.search(query, 5, 0.6)?;
        if external_memories.is_empty() {
            return Ok(String::new());
        }

        let formatted: Vec<String> = external_memories
            .iter()
            .filter_map(|r| r.get("content").and_then(|c| c.as_str()))
            .map(|c| format!("- {}", c))
            .collect();

        Ok(format!(
            "External memories:\n{}",
            formatted.join("\n")
        ))
    }

    /// Fetch STM context asynchronously.
    async fn afetch_stm_context(&self, query: &str) -> Result<String, anyhow::Error> {
        let stm = match &self.stm {
            Some(s) => s,
            None => return Ok(String::new()),
        };

        let stm_results = stm.asearch(query, 5, 0.6).await?;
        if stm_results.is_empty() {
            return Ok(String::new());
        }

        let formatted: Vec<String> = stm_results
            .iter()
            .filter_map(|r| r.get("content").and_then(|c| c.as_str()))
            .map(|c| format!("- {}", c))
            .collect();

        Ok(format!("Recent Insights:\n{}", formatted.join("\n")))
    }

    /// Fetch LTM context asynchronously.
    async fn afetch_ltm_context(
        &self,
        task: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let ltm = match &self.ltm {
            Some(l) => l,
            None => return Ok(Some(String::new())),
        };

        let ltm_results = ltm.asearch(task, 2).await?;
        if ltm_results.is_empty() {
            return Ok(None);
        }

        let mut suggestions: Vec<String> = Vec::new();
        for result in &ltm_results {
            if let Some(metadata) = result.get("metadata") {
                if let Some(suggs) = metadata.get("suggestions") {
                    if let Some(arr) = suggs.as_array() {
                        for s in arr {
                            if let Some(text) = s.as_str() {
                                if !suggestions.contains(&text.to_string()) {
                                    suggestions.push(text.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        let formatted: Vec<String> = suggestions.iter().map(|s| format!("- {}", s)).collect();

        if formatted.is_empty() {
            Ok(None)
        } else {
            Ok(Some(format!(
                "Historical Data:\n{}",
                formatted.join("\n")
            )))
        }
    }

    /// Fetch entity context asynchronously.
    async fn afetch_entity_context(&self, query: &str) -> Result<String, anyhow::Error> {
        let em = match &self.em {
            Some(e) => e,
            None => return Ok(String::new()),
        };

        let em_results = em.asearch(query, 5, 0.6).await?;
        if em_results.is_empty() {
            return Ok(String::new());
        }

        let formatted: Vec<String> = em_results
            .iter()
            .filter_map(|r| r.get("content").and_then(|c| c.as_str()))
            .map(|c| format!("- {}", c))
            .collect();

        Ok(format!("Entities:\n{}", formatted.join("\n")))
    }

    /// Fetch external context asynchronously.
    async fn afetch_external_context(&self, query: &str) -> Result<String, anyhow::Error> {
        let exm = match &self.exm {
            Some(e) => e,
            None => return Ok(String::new()),
        };

        let external_memories = exm.asearch(query, 5, 0.6).await?;
        if external_memories.is_empty() {
            return Ok(String::new());
        }

        let formatted: Vec<String> = external_memories
            .iter()
            .filter_map(|r| r.get("content").and_then(|c| c.as_str()))
            .map(|c| format!("- {}", c))
            .collect();

        Ok(format!(
            "External memories:\n{}",
            formatted.join("\n")
        ))
    }
}
