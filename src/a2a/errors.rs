//! A2A error codes and error response utilities.
//!
//! Corresponds to `crewai/a2a/errors.py`.
//!
//! Error codes follow JSON-RPC 2.0 conventions:
//! - -32700 to -32600: Standard JSON-RPC errors
//! - -32099 to -32000: Server errors (A2A-specific)
//! - -32768 to -32100: Reserved for implementation-defined errors

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// A2A protocol error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum A2AErrorCode {
    // JSON-RPC 2.0 Standard Errors
    /// Invalid JSON was received by the server.
    JsonParseError = -32700,
    /// The JSON sent is not a valid Request object.
    InvalidRequest = -32600,
    /// The method does not exist / is not available.
    MethodNotFound = -32601,
    /// Invalid method parameter(s).
    InvalidParams = -32602,
    /// Internal JSON-RPC error.
    InternalError = -32603,

    // A2A-Specific Errors
    /// The specified task was not found.
    TaskNotFound = -32001,
    /// The task cannot be canceled.
    TaskNotCancelable = -32002,
    /// Push notifications are not supported.
    PushNotificationNotSupported = -32003,
    /// The requested operation is not supported.
    UnsupportedOperation = -32004,
    /// Incompatible content types.
    ContentTypeNotSupported = -32005,
    /// The agent produced an invalid response.
    InvalidAgentResponse = -32006,

    // CrewAI Custom Extensions
    /// The requested A2A protocol version is not supported.
    UnsupportedVersion = -32009,
    /// Client does not support required protocol extensions.
    UnsupportedExtension = -32010,
    /// Authentication is required.
    AuthenticationRequired = -32011,
    /// Authorization check failed.
    AuthorizationFailed = -32012,
    /// Rate limit exceeded.
    RateLimitExceeded = -32013,
    /// Task execution timed out.
    TaskTimeout = -32014,
    /// Failed to negotiate a compatible transport protocol.
    TransportNegotiationFailed = -32015,
    /// The specified context was not found.
    ContextNotFound = -32016,
    /// The specified skill was not found.
    SkillNotFound = -32017,
    /// The specified artifact was not found.
    ArtifactNotFound = -32018,
}

impl A2AErrorCode {
    /// Get the default error message for this code.
    pub fn default_message(&self) -> &'static str {
        match self {
            Self::JsonParseError => "Parse error",
            Self::InvalidRequest => "Invalid Request",
            Self::MethodNotFound => "Method not found",
            Self::InvalidParams => "Invalid params",
            Self::InternalError => "Internal error",
            Self::TaskNotFound => "Task not found",
            Self::TaskNotCancelable => "Task not cancelable",
            Self::PushNotificationNotSupported => "Push Notification is not supported",
            Self::UnsupportedOperation => "This operation is not supported",
            Self::ContentTypeNotSupported => "Incompatible content types",
            Self::InvalidAgentResponse => "Invalid agent response",
            Self::UnsupportedVersion => "Unsupported A2A version",
            Self::UnsupportedExtension => "Client does not support required extensions",
            Self::AuthenticationRequired => "Authentication required",
            Self::AuthorizationFailed => "Authorization failed",
            Self::RateLimitExceeded => "Rate limit exceeded",
            Self::TaskTimeout => "Task execution timed out",
            Self::TransportNegotiationFailed => "Transport negotiation failed",
            Self::ContextNotFound => "Context not found",
            Self::SkillNotFound => "Skill not found",
            Self::ArtifactNotFound => "Artifact not found",
        }
    }
}

/// Base error for A2A protocol errors.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub struct A2AError {
    /// The A2A/JSON-RPC error code.
    pub code: i32,
    /// Human-readable error message.
    pub message: String,
    /// Optional additional error data.
    pub data: Option<Value>,
}

impl fmt::Display for A2AError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl A2AError {
    /// Create a new `A2AError` from an error code with default message.
    pub fn from_code(code: A2AErrorCode) -> Self {
        Self {
            code: code as i32,
            message: code.default_message().to_string(),
            data: None,
        }
    }

    /// Create a new `A2AError` with a custom message.
    pub fn new(code: A2AErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: None,
        }
    }

    /// Create with additional data.
    pub fn with_data(code: A2AErrorCode, message: impl Into<String>, data: Value) -> Self {
        Self {
            code: code as i32,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Convert to JSON-RPC error object format.
    pub fn to_dict(&self) -> Value {
        let mut error = serde_json::Map::new();
        error.insert("code".to_string(), Value::Number(self.code.into()));
        error.insert("message".to_string(), Value::String(self.message.clone()));
        if let Some(ref data) = self.data {
            error.insert("data".to_string(), data.clone());
        }
        Value::Object(error)
    }

    /// Convert to full JSON-RPC error response.
    pub fn to_response(&self, request_id: Option<Value>) -> Value {
        let mut resp = serde_json::Map::new();
        resp.insert("jsonrpc".to_string(), Value::String("2.0".to_string()));
        resp.insert("error".to_string(), self.to_dict());
        resp.insert(
            "id".to_string(),
            request_id.unwrap_or(Value::Null),
        );
        Value::Object(resp)
    }
}

/// Raised when polling exceeds the configured timeout.
#[derive(Debug, Error)]
#[error("A2A polling timeout: {message}")]
pub struct A2APollingTimeoutError {
    pub message: String,
}

/// Create a JSON-RPC error response.
pub fn create_error_response(
    code: A2AErrorCode,
    message: Option<&str>,
    data: Option<Value>,
    request_id: Option<Value>,
) -> Value {
    let msg = message
        .map(|s| s.to_string())
        .unwrap_or_else(|| code.default_message().to_string());
    let error = A2AError {
        code: code as i32,
        message: msg,
        data,
    };
    error.to_response(request_id)
}

/// Check if an error is potentially retryable.
pub fn is_retryable_error(code: i32) -> bool {
    matches!(
        code,
        -32603 | // InternalError
        -32013 | // RateLimitExceeded
        -32014   // TaskTimeout
    )
}

/// Check if an error is a client-side error.
pub fn is_client_error(code: i32) -> bool {
    matches!(
        code,
        -32700 | // JsonParseError
        -32600 | // InvalidRequest
        -32601 | // MethodNotFound
        -32602 | // InvalidParams
        -32001 | // TaskNotFound
        -32005 | // ContentTypeNotSupported
        -32009 | // UnsupportedVersion
        -32010 | // UnsupportedExtension
        -32016 | // ContextNotFound
        -32017 | // SkillNotFound
        -32018   // ArtifactNotFound
    )
}
