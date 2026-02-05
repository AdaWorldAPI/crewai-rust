//! Knowledge module for CrewAI's knowledge management system.
//!
//! Corresponds to `crewai/knowledge/`.
//!
//! This module provides the knowledge management infrastructure including:
//! - The main `Knowledge` struct for managing knowledge sources and queries
//! - Knowledge source abstractions (trait-based, with concrete implementations)
//! - Knowledge storage backend (trait-based, with RAG integration)
//! - Knowledge configuration for query behavior

pub mod knowledge;
pub mod knowledge_config;
pub mod source;
pub mod storage;

// Re-export main types.
pub use self::knowledge::Knowledge;
pub use self::knowledge_config::KnowledgeConfig;
pub use self::source::{BaseFileKnowledgeSource, BaseKnowledgeSource, StringKnowledgeSource};
pub use self::storage::{BaseKnowledgeStorage, KnowledgeStorage};
