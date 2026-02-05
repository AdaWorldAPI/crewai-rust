//! LLM transport hooks and interceptors.
//!
//! Corresponds to `crewai/llms/hooks/` Python package.
//!
//! Provides abstract base traits for intercepting and modifying outbound and
//! inbound messages at the transport level. This enables request/response
//! modification (e.g., adding custom headers, logging, tracing) without
//! changing the core LLM provider implementation.
//!
//! # Architecture
//!
//! ```text
//! LLM Client
//!   |
//!   +-- BaseInterceptor::on_outbound(request) --> modified request
//!   |
//!   +-- HTTP Transport sends request
//!   |
//!   +-- HTTP Transport receives response
//!   |
//!   +-- BaseInterceptor::on_inbound(response) --> modified response
//!   |
//!   +-- LLM Client processes response
//! ```
//!
//! # Usage
//!
//! Implement the [`BaseInterceptor`] trait and pass the interceptor to an
//! LLM provider. The provider will call `on_outbound` before sending each
//! request and `on_inbound` after receiving each response.
//!
//! ```ignore
//! use crewai::llms::hooks::BaseInterceptor;
//!
//! #[derive(Debug)]
//! struct LoggingInterceptor;
//!
//! #[async_trait::async_trait]
//! impl BaseInterceptor<reqwest::Request, reqwest::Response> for LoggingInterceptor {
//!     fn on_outbound(&self, request: reqwest::Request) -> reqwest::Request {
//!         log::info!("Outbound request: {:?}", request.url());
//!         request
//!     }
//!
//!     fn on_inbound(&self, response: reqwest::Response) -> reqwest::Response {
//!         log::info!("Inbound response: status={}", response.status());
//!         response
//!     }
//! }
//! ```

use std::fmt;

use async_trait::async_trait;

// ---------------------------------------------------------------------------
// BaseInterceptor trait
// ---------------------------------------------------------------------------

/// Abstract base trait for intercepting transport-level messages.
///
/// Provides hooks to intercept and modify outbound and inbound messages
/// at the transport layer. This is the Rust equivalent of
/// `crewai.llms.hooks.base.BaseInterceptor[T, U]` in Python.
///
/// # Type Parameters
///
/// * `T` - Outbound message type (e.g., `reqwest::Request`).
/// * `U` - Inbound message type (e.g., `reqwest::Response`).
///
/// # Example
///
/// ```ignore
/// #[derive(Debug)]
/// struct HeaderInjector {
///     header_name: String,
///     header_value: String,
/// }
///
/// #[async_trait]
/// impl BaseInterceptor<HttpRequest, HttpResponse> for HeaderInjector {
///     fn on_outbound(&self, mut request: HttpRequest) -> HttpRequest {
///         request.headers_mut().insert(
///             &self.header_name,
///             self.header_value.parse().unwrap(),
///         );
///         request
///     }
///
///     fn on_inbound(&self, response: HttpResponse) -> HttpResponse {
///         response
///     }
/// }
/// ```
#[async_trait]
pub trait BaseInterceptor<T, U>: Send + Sync + fmt::Debug
where
    T: Send + 'static,
    U: Send + 'static,
{
    /// Intercept outbound message before sending.
    ///
    /// # Arguments
    ///
    /// * `message` - Outbound message object.
    ///
    /// # Returns
    ///
    /// Modified (or unchanged) message object.
    fn on_outbound(&self, message: T) -> T;

    /// Intercept inbound message after receiving.
    ///
    /// # Arguments
    ///
    /// * `message` - Inbound message object.
    ///
    /// # Returns
    ///
    /// Modified (or unchanged) message object.
    fn on_inbound(&self, message: U) -> U;

    /// Async version of `on_outbound`.
    ///
    /// Default implementation delegates to the synchronous `on_outbound`.
    async fn aon_outbound(&self, message: T) -> T {
        self.on_outbound(message)
    }

    /// Async version of `on_inbound`.
    ///
    /// Default implementation delegates to the synchronous `on_inbound`.
    async fn aon_inbound(&self, message: U) -> U {
        self.on_inbound(message)
    }
}

// ---------------------------------------------------------------------------
// HTTP Transport wrappers
// ---------------------------------------------------------------------------

/// Configuration for HTTP transport that wraps an interceptor.
///
/// Corresponds to `crewai.llms.hooks.transport.HTTPTransport` in Python.
/// In Rust, we store the interceptor and apply it via middleware rather than
/// subclassing an HTTP transport.
///
/// # Usage
///
/// This struct is used internally by provider implementations when a user
/// provides a `BaseInterceptor`. Users should not need to instantiate this
/// directly -- instead, pass an interceptor to the LLM provider.
#[derive(Debug)]
pub struct InterceptedTransport<T, U>
where
    T: Send + 'static,
    U: Send + 'static,
{
    /// The interceptor to apply to requests/responses.
    pub interceptor: Box<dyn BaseInterceptor<T, U>>,
}

impl<T, U> InterceptedTransport<T, U>
where
    T: Send + 'static,
    U: Send + 'static,
{
    /// Create a new intercepted transport with the given interceptor.
    pub fn new(interceptor: Box<dyn BaseInterceptor<T, U>>) -> Self {
        Self { interceptor }
    }

    /// Apply the outbound interceptor to a request.
    pub fn intercept_outbound(&self, request: T) -> T {
        self.interceptor.on_outbound(request)
    }

    /// Apply the inbound interceptor to a response.
    pub fn intercept_inbound(&self, response: U) -> U {
        self.interceptor.on_inbound(response)
    }

    /// Async: apply the outbound interceptor to a request.
    pub async fn intercept_outbound_async(&self, request: T) -> T {
        self.interceptor.aon_outbound(request).await
    }

    /// Async: apply the inbound interceptor to a response.
    pub async fn intercept_inbound_async(&self, response: U) -> U {
        self.interceptor.aon_inbound(response).await
    }
}

// ---------------------------------------------------------------------------
// No-op interceptor for testing
// ---------------------------------------------------------------------------

/// A no-op interceptor that passes messages through unchanged.
///
/// Useful for testing and as a default.
#[derive(Debug, Clone)]
pub struct NoOpInterceptor;

#[async_trait]
impl<T, U> BaseInterceptor<T, U> for NoOpInterceptor
where
    T: Send + 'static,
    U: Send + 'static,
{
    fn on_outbound(&self, message: T) -> T {
        message
    }

    fn on_inbound(&self, message: U) -> U {
        message
    }
}
