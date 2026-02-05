//! LLM provider implementations.
//!
//! Corresponds to `crewai/llms/providers/` Python package.
//!
//! This module contains provider-specific LLM implementations that use
//! native SDKs for each cloud provider. Each provider implements the
//! [`BaseLLM`](crate::llms::base_llm::BaseLLM) trait and handles
//! authentication, request formatting, streaming, tool calling, and
//! error handling specific to that provider.
//!
//! # Available Providers
//!
//! | Provider | Module | Python Equivalent |
//! |----------|--------|-------------------|
//! | OpenAI | [`openai`] | `crewai.llms.providers.openai.completion` |
//! | Anthropic | [`anthropic`] | `crewai.llms.providers.anthropic.completion` |
//! | Azure | [`azure`] | `crewai.llms.providers.azure.completion` |
//! | Bedrock | [`bedrock`] | `crewai.llms.providers.bedrock.completion` |
//! | Gemini | [`gemini`] | `crewai.llms.providers.gemini.completion` |
//!
//! # Shared Utilities
//!
//! The [`utils`] module provides common helpers shared across providers,
//! such as tool name validation, tool info extraction, and function name
//! sanitization.

pub mod anthropic;
pub mod azure;
pub mod bedrock;
pub mod gemini;
pub mod openai;
pub mod utils;
