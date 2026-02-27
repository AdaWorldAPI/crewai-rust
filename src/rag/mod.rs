//! RAG (Retrieval-Augmented Generation) system for crewAI.
//!
//! This module provides the RAG subsystem including vector database clients
//! (ChromaDB, Qdrant), embedding providers, storage abstractions, and factory
//! functions for client creation.

pub mod chromadb;
pub mod config;
pub mod core;
pub mod embeddings;
pub mod factory;
pub mod qdrant;
pub mod storage;
pub mod types;

pub use factory::create_client;
pub use types::{BaseRecord, EmbeddingFunction, Embeddings, SearchResult};
