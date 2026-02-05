//! Embedding provider implementations.
//!
//! Port of crewai/rag/embeddings/providers/
//!
//! Each submodule provides a stub implementation of a specific embedding provider
//! that implements the `BaseEmbedding` trait. These providers generate vector
//! embeddings from text using various external APIs and local models.
//!
//! # Supported Providers
//!
//! | Provider | Module | Provider Literal |
//! |---|---|---|
//! | AWS Bedrock | [`aws`] | `"amazon-bedrock"` |
//! | Cohere | [`cohere`] | `"cohere"` |
//! | Custom | [`custom`] | `"custom"` |
//! | Google Generative AI / Vertex AI | [`google`] | `"google-generativeai"` / `"google-vertex"` |
//! | HuggingFace | [`huggingface`] | `"huggingface"` |
//! | IBM WatsonX | [`ibm`] | `"watsonx"` |
//! | Instructor | [`instructor`] | `"instructor"` |
//! | Jina | [`jina`] | `"jina"` |
//! | Microsoft Azure | [`microsoft`] | `"azure"` |
//! | Ollama | [`ollama`] | `"ollama"` |
//! | ONNX Runtime | [`onnx`] | `"onnx"` |
//! | OpenAI | [`openai`] | `"openai"` |
//! | OpenCLIP | [`openclip`] | `"openclip"` |
//! | Roboflow | [`roboflow`] | `"roboflow"` |
//! | Sentence Transformers | [`sentence_transformer`] | `"sentence-transformer"` |
//! | Text2Vec | [`text2vec`] | `"text2vec"` |
//! | VoyageAI | [`voyageai`] | `"voyageai"` |

pub mod aws;
pub mod cohere;
pub mod custom;
pub mod google;
pub mod huggingface;
pub mod ibm;
pub mod instructor;
pub mod jina;
pub mod microsoft;
pub mod ollama;
pub mod onnx;
pub mod openai;
pub mod openclip;
pub mod roboflow;
pub mod sentence_transformer;
pub mod text2vec;
pub mod voyageai;

// Re-export all provider embedding types for convenience.
pub use aws::AwsBedrockEmbedding;
pub use cohere::CohereEmbedding;
pub use custom::CustomEmbedding;
pub use google::GoogleEmbedding;
pub use huggingface::HuggingFaceEmbedding;
pub use ibm::IbmWatsonXEmbedding;
pub use instructor::InstructorEmbedding;
pub use jina::JinaEmbedding;
pub use microsoft::AzureEmbedding;
pub use ollama::OllamaEmbedding;
pub use onnx::OnnxEmbedding;
pub use openai::OpenAIEmbedding;
pub use openclip::OpenClipEmbedding;
pub use roboflow::RoboflowEmbedding;
pub use sentence_transformer::SentenceTransformerEmbedding;
pub use text2vec::Text2VecEmbedding;
pub use voyageai::VoyageAIEmbedding;

/// Allowed embedding provider name literals.
///
/// Port of crewai/rag/embeddings/types.py AllowedEmbeddingProviders.
pub const ALLOWED_EMBEDDING_PROVIDERS: &[&str] = &[
    "azure",
    "amazon-bedrock",
    "cohere",
    "custom",
    "google-generativeai",
    "google-vertex",
    "huggingface",
    "instructor",
    "jina",
    "ollama",
    "onnx",
    "openai",
    "openclip",
    "roboflow",
    "sentence-transformer",
    "text2vec",
    "voyageai",
    "watsonx",
];
