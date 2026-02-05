//! RAG (Retrieval-Augmented Generation) system for crewAI.
//!
//! This module provides the RAG subsystem including vector database clients
//! (ChromaDB, Qdrant), embedding providers, storage abstractions, and factory
//! functions for client creation.

pub mod types;
pub mod core;
pub mod config;
pub mod chromadb;
pub mod qdrant;
pub mod storage;
pub mod embeddings;
pub mod factory;

pub use types::{BaseRecord, SearchResult, Embeddings, EmbeddingFunction};
pub use factory::create_client;
