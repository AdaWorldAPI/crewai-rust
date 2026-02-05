# LLMs

The LLM system in crewAI-rust provides a unified interface for configuring and calling language models across multiple providers. It is structured as two layers: the high-level `LLM` configuration struct (in `src/llm/mod.rs`) and the low-level `BaseLLM` trait for provider implementations (in `src/llms/base_llm.rs`).

**Source**: `src/llm/mod.rs` and `src/llms/base_llm.rs`
**Python counterpart**: `crewai/llm.py` and `crewai/llms/base_llm.py`

---

## Architecture Overview

```
LLM (src/llm/mod.rs)
  |-- Configuration struct with builder pattern
  |-- Provider inference from model name
  |-- Context window size lookup
  |-- Implements BaseLLMTrait for trait-object usage
  |
BaseLLM (src/llms/base_llm.rs)
  |-- #[async_trait] abstract trait for provider implementations
  |-- Defines call()/acall() signatures
  |-- Capability queries (function calling, multimodal, stop words)
  |-- Token usage tracking interface
  |
BaseLLMState (src/llms/base_llm.rs)
  |-- Shared state struct embedded in provider implementations
  |-- Stop word application
  |-- Message formatting
  |-- Token usage accumulation
  |-- Structured output validation
  |
Providers (src/llms/providers/)
  |-- Native SDK wrappers (OpenAI, Anthropic, etc.)
  |
Hooks (src/llms/hooks.rs)
  |-- BaseInterceptor trait for request/response modification
```

### Key Difference from Python

In Python, `LLM` extends `BaseLLM` through class inheritance. In Rust, the relationship is split:

- **`LLM`** is a concrete configuration struct that holds all model parameters and provides convenience methods.
- **`BaseLLM`** is an `#[async_trait]` trait that provider implementations (OpenAI, Anthropic, etc.) implement.
- **`BaseLLMState`** is a concrete struct that providers embed to share common functionality (stop words, token tracking, message formatting) without needing trait inheritance.
- **`BaseLLMTrait`** is a simplified bridge trait that `LLM` implements, enabling it to be used as a `dyn BaseLLMTrait` trait object.

---

## LLM Configuration Struct

### Creating an LLM

```rust
use crewai::llm::{LLM, ReasoningEffort};

// Simple creation with model name
let llm = LLM::new("gpt-4o");

// With explicit provider
let llm = LLM::with_provider("my-model", "anthropic");

// Builder pattern for full configuration
let llm = LLM::new("gpt-4o")
    .temperature(0.7)
    .max_tokens(2000)
    .api_key("sk-...")
    .base_url("https://api.example.com/v1")
    .timeout(30.0)
    .stream(true)
    .stop(vec!["STOP".to_string()])
    .reasoning_effort(ReasoningEffort::High);
```

The builder methods consume `self` and return `Self`, enabling chained construction. The `LLM::new()` constructor automatically detects Anthropic models by checking the model name against known prefixes.

### Complete Field Reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `String` | Required | Model identifier (e.g., `"gpt-4o"`, `"claude-3-5-sonnet-20241022"`) |
| `temperature` | `Option<f64>` | `None` | Sampling temperature |
| `top_p` | `Option<f64>` | `None` | Nucleus sampling parameter |
| `n` | `Option<i32>` | `None` | Number of completions to generate |
| `stop` | `Vec<String>` | `[]` | Stop sequences |
| `max_tokens` | `Option<i64>` | `None` | Max tokens to generate |
| `max_completion_tokens` | `Option<i64>` | `None` | Max completion tokens (newer OpenAI parameter) |
| `presence_penalty` | `Option<f64>` | `None` | Presence penalty (-2.0 to 2.0) |
| `frequency_penalty` | `Option<f64>` | `None` | Frequency penalty (-2.0 to 2.0) |
| `logit_bias` | `Option<HashMap<i64, f64>>` | `None` | Token logit biases |
| `response_format` | `Option<Value>` | `None` | Structured output format specification |
| `seed` | `Option<i64>` | `None` | Random seed for reproducibility |
| `logprobs` | `Option<i32>` | `None` | Whether to return log probabilities |
| `top_logprobs` | `Option<i32>` | `None` | Number of top log probabilities |
| `timeout` | `Option<f64>` | `None` | API call timeout in seconds |
| `base_url` | `Option<String>` | `None` | Custom API base URL |
| `api_base` | `Option<String>` | `None` | Alias for `base_url` (OpenAI compatibility) |
| `api_key` | `Option<String>` | `None` | API key (marked `#[serde(skip_serializing)]`) |
| `api_version` | `Option<String>` | `None` | API version (for Azure and versioned APIs) |
| `callbacks` | `Vec<Box<dyn Any + Send + Sync>>` | `[]` | Callbacks (marked `#[serde(skip)]`, not cloneable) |
| `reasoning_effort` | `Option<ReasoningEffort>` | `None` | Reasoning effort level |
| `stream` | `bool` | `false` | Enable streaming responses |
| `prefer_upload` | `bool` | `false` | Prefer file upload over inline base64 |
| `context_window_size` | `i64` | `0` | Override context window size (0 = auto-detect) |
| `additional_params` | `HashMap<String, Value>` | `{}` | Extra provider-specific parameters |
| `is_anthropic` | `bool` | auto | Auto-detected from model name |
| `is_litellm` | `bool` | `false` | Whether this LLM uses LiteLLM backend |
| `provider` | `Option<String>` | `None` | Explicit provider override |
| `completion_cost` | `Option<f64>` | `None` | Completion cost from last call |

### Serialization Notes

- `api_key` is annotated with `#[serde(skip_serializing)]` -- it will never appear in serialized output, preventing accidental credential exposure.
- `callbacks` is annotated with `#[serde(skip)]` -- function pointers cannot be serialized.
- `Clone` is manually implemented because `callbacks` (containing trait objects) cannot be derived. The clone leaves `callbacks` empty.

---

## Reasoning Effort

The `ReasoningEffort` enum controls how much computation a reasoning model should use:

```rust
use crewai::llm::ReasoningEffort;

let llm = LLM::new("o3-mini")
    .reasoning_effort(ReasoningEffort::High);

// Enum variants
// ReasoningEffort::None    => "none"
// ReasoningEffort::Low     => "low"
// ReasoningEffort::Medium  => "medium"
// ReasoningEffort::High    => "high"
```

The enum derives `Serialize` and `Deserialize` with `#[serde(rename_all = "lowercase")]`, and implements `Display` for string conversion. This corresponds to the Python `Literal["none", "low", "medium", "high"]` type.

---

## Provider Inference

The LLM automatically infers its provider from the model name using a multi-step resolution process:

```rust
let llm = LLM::new("gpt-4o");
assert_eq!(llm.infer_provider(), "openai");

let llm = LLM::new("claude-3-5-sonnet-20241022");
assert_eq!(llm.infer_provider(), "anthropic");

let llm = LLM::new("gemini-2.0-flash");
assert_eq!(llm.infer_provider(), "gemini");

let llm = LLM::new("openai/gpt-4o");       // prefix-based
assert_eq!(llm.infer_provider(), "openai");

let llm = LLM::new("bedrock/anthropic.claude-3");
assert_eq!(llm.infer_provider(), "bedrock");
```

### Resolution Priority

1. **Explicit `provider` field** -- If set via `LLM::with_provider()`, used directly.
2. **Model string prefix** -- Checks for `provider/model` format (e.g., `"openai/gpt-4"`).
3. **Model name pattern matching** -- Checks against known model name prefixes.
4. **Default** -- Falls back to `"openai"`.

### Supported Native Providers

```rust
pub const SUPPORTED_NATIVE_PROVIDERS: &[&str] = &[
    "openai", "anthropic", "claude", "azure", "azure_openai",
    "google", "gemini", "bedrock", "aws",
];
```

### Pattern Matching Rules

| Provider | Matched Prefixes/Patterns |
|----------|---------------------------|
| `openai` | `gpt-`, `o1`, `o3`, `o4`, `whisper-` |
| `anthropic` / `claude` | `claude-`, `anthropic.` |
| `gemini` / `google` | `gemini-`, `gemma-`, `learnlm-` |
| `azure` | `gpt-`, `gpt-35-`, `o1`, `o3`, `o4`, `azure-` |
| `bedrock` | Model names containing `.` (e.g., `anthropic.claude-3`) |
| `mistral` | `mistral` |

### Anthropic Model Detection

The `_is_anthropic_model()` method performs a separate check used to set the `is_anthropic` flag at construction time:

```rust
// Checks (case-insensitive):
// - Starts with "anthropic/", "claude-", or "claude/"
// - Contains "claude" anywhere in the name
```

---

## Context Window Management

### Automatic Lookup

The LLM automatically looks up context window sizes from a comprehensive built-in table:

```rust
let llm = LLM::new("gpt-4o");
assert_eq!(llm.get_context_window_size(), 128_000);

let llm = LLM::new("gemini-1.5-pro");
assert_eq!(llm.get_context_window_size(), 2_097_152);  // 2M tokens

// Strip provider prefix for lookup
let llm = LLM::new("openai/gpt-4o");
assert_eq!(llm.get_context_window_size(), 128_000);
```

### Lookup Priority

1. **Explicit override** -- If `context_window_size > 0`, use it (clamped to `[MIN_CONTEXT, MAX_CONTEXT]`).
2. **Exact model name match** -- Look up in `llm_context_window_sizes()`.
3. **Prefix-stripped match** -- Try `model.split_once('/')` and look up the suffix.
4. **Default** -- `DEFAULT_CONTEXT_WINDOW_SIZE` (8,192).

### Usable Context Window

The system reserves 15% of the context window for overhead:

```rust
let llm = LLM::new("gpt-4o");
let usable = llm.get_usable_context_window_size();
// 128_000 * 0.85 = 108_800
```

### Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `MIN_CONTEXT` | 1,024 | Minimum context window (clamping floor) |
| `MAX_CONTEXT` | 2,097,152 | Maximum context window (clamping ceiling) |
| `DEFAULT_CONTEXT_WINDOW_SIZE` | 8,192 | Fallback for unknown models |
| `CONTEXT_WINDOW_USAGE_RATIO` | 0.85 | Usable fraction of context window |

### Known Model Sizes

The `llm_context_window_sizes()` function returns a comprehensive `HashMap<&'static str, i64>` covering:

- **OpenAI**: GPT-4, GPT-4o, GPT-4o-mini, GPT-4.1, o1/o3/o4 variants
- **Gemini**: gemini-1.5-pro (2M), gemini-2.0-flash, gemma-3 variants
- **DeepSeek**: deepseek-chat (128K)
- **Groq**: llama-3.x variants, mixtral, gemma
- **SambaNova**: Meta-Llama, Qwen variants
- **Bedrock**: us/eu/apac regional prefixes, Amazon Nova, Anthropic Claude, Meta Llama, Cohere, AI21, Mistral
- **Mistral**: mistral-tiny through mistral-large

---

## Calling an LLM

```rust
use std::collections::HashMap;

let llm = LLM::new("gpt-4o").api_key("sk-...");

let mut messages = vec![];
let mut msg = HashMap::new();
msg.insert("role".to_string(), "user".to_string());
msg.insert("content".to_string(), "What is Rust?".to_string());
messages.push(msg);

// Synchronous call
let response = llm.call(&messages, None)?;

// Async call
let response = llm.acall(&messages, None).await?;

// With tools
let tools = vec![serde_json::json!({
    "type": "function",
    "function": {
        "name": "search",
        "description": "Search the web",
        "parameters": {"type": "object", "properties": {"query": {"type": "string"}}}
    }
})];
let response = llm.call(&messages, Some(&tools))?;
```

**Note**: `LLM::call()` is currently a stub that returns an error. Full provider integration is tracked as technical debt. The method signature is stable and matches the Python interface.

### Capability Checks

```rust
let llm = LLM::new("gpt-4o");
assert!(llm.supports_function_calling());

let llm = LLM::new("claude-3-5-sonnet-20241022");
assert!(llm.supports_function_calling());

// Checks against known function-calling model families:
// gpt-4, gpt-3.5-turbo, claude-3, claude-2, gemini,
// mistral, llama-3, command, o1, o3, o4
```

---

## Completion Parameters

Gather all configured fields into a single `HashMap<String, Value>` for passing to a provider SDK:

```rust
let llm = LLM::new("gpt-4o")
    .temperature(0.5)
    .max_tokens(500)
    .stream(true);

let params = llm.prepare_completion_params();
// {"model": "gpt-4o", "temperature": 0.5, "max_tokens": 500, "stream": true}
```

The method includes all non-`None` optional fields, the `stop` list (if non-empty), `stream` (if `true`), `reasoning_effort`, and all entries from `additional_params`. The `api_key`, `base_url`/`api_base`, and `api_version` are also included when present.

---

## BaseLLM Trait (Provider Interface)

The `BaseLLM` trait in `src/llms/base_llm.rs` defines the full interface that provider implementations must satisfy. It is decorated with `#[async_trait]` to support async methods.

```rust
use async_trait::async_trait;

#[async_trait]
pub trait BaseLLM: Send + Sync + fmt::Debug {
    // --- Required methods ---
    fn model(&self) -> &str;
    fn temperature(&self) -> Option<f64>;
    fn stop(&self) -> &[String];
    fn set_stop(&mut self, stop: Vec<String>);
    fn call(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;
    fn get_token_usage_summary(&self) -> UsageMetrics;
    fn track_token_usage(&mut self, usage_data: &HashMap<String, Value>);

    // --- Optional methods with defaults ---
    fn provider(&self) -> &str { "openai" }
    fn is_litellm(&self) -> bool { false }
    fn supports_function_calling(&self) -> bool { false }
    fn supports_stop_words(&self) -> bool { true }
    fn get_context_window_size(&self) -> usize { 4096 }
    fn supports_multimodal(&self) -> bool { false }
    fn format_text_content(&self, text: &str) -> Value { /* ... */ }
    fn convert_tools_for_inference(&self, tools: Vec<Value>) -> Vec<Value> { tools }
    async fn acall(
        &self,
        messages: Vec<LLMMessage>,
        tools: Option<Vec<Value>>,
        available_functions: Option<HashMap<String, Box<dyn Any + Send + Sync>>>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err("Async call not implemented for this LLM".into())
    }
}
```

### LLMMessage Type

```rust
/// A single message in an LLM conversation.
/// Corresponds to Python's LLMMessage TypedDict with "role" and "content"
/// keys, plus optional "files", "tool_calls", "tool_call_id", etc.
pub type LLMMessage = HashMap<String, Value>;
```

Using `HashMap<String, Value>` rather than a fixed struct allows maximum flexibility across providers with varying message schemas.

---

## BaseLLMState (Shared Provider State)

Provider implementations embed `BaseLLMState` to reuse common functionality:

```rust
use crewai::llms::base_llm::BaseLLMState;

let mut state = BaseLLMState::new("gpt-4o");

// Fields
state.model;             // String
state.temperature;       // Option<f64>
state.api_key;           // Option<String>
state.base_url;          // Option<String>
state.stop;              // Vec<String>
state.provider;          // String (default: "openai")
state.prefer_upload;     // bool
state.additional_params; // HashMap<String, Value>
state.token_usage;       // TokenUsage
```

### Stop Word Application

```rust
let mut state = BaseLLMState::new("gpt-4o");
state.stop = vec!["Observation:".to_string()];

let content = "I need to search.\n\nAction: search\nObservation: Found results";
let truncated = state.apply_stop_words(content);
assert_eq!(truncated, "I need to search.\n\nAction: search");
```

The method finds the earliest occurrence of any stop word and truncates the content at that position.

### Token Usage Tracking

```rust
let mut state = BaseLLMState::new("test");
let mut usage = HashMap::new();
usage.insert("prompt_tokens".to_string(), serde_json::json!(100));
usage.insert("completion_tokens".to_string(), serde_json::json!(50));
usage.insert("cached_tokens".to_string(), serde_json::json!(10));

state.track_token_usage_internal(&usage);

let summary = state.get_token_usage_summary();
assert_eq!(summary.total_tokens, 150);
assert_eq!(summary.prompt_tokens, 100);
assert_eq!(summary.completion_tokens, 50);
assert_eq!(summary.cached_prompt_tokens, 10);
assert_eq!(summary.successful_requests, 1);
```

Token tracking supports multiple field name conventions across providers:

| Field | OpenAI | Anthropic | Gemini |
|-------|--------|-----------|--------|
| Prompt tokens | `prompt_tokens` | `input_tokens` | `prompt_token_count` |
| Completion tokens | `completion_tokens` | `output_tokens` | `candidates_token_count` |
| Cached tokens | `cached_tokens` / `cached_prompt_tokens` | -- | -- |

### Structured Output Validation

```rust
// Direct JSON parsing
let result = BaseLLMState::validate_structured_output(r#"{"key": "value"}"#);
assert!(result.is_ok());

// Extracts JSON from mixed text using regex fallback
let result = BaseLLMState::validate_structured_output(
    "Here is the JSON: {\"key\": \"value\"}"
);
assert!(result.is_ok());

// Returns error if no JSON found
let result = BaseLLMState::validate_structured_output("No JSON here");
assert!(result.is_err());
```

### Message Formatting

```rust
// Convert a plain string to a user message
let messages = BaseLLMState::string_to_messages("Hello!");
// [{"role": "user", "content": "Hello!"}]

// Validate existing messages have required keys
let state = BaseLLMState::new("test");
let result = state.format_messages(messages);
assert!(result.is_ok());  // Messages have both "role" and "content"
```

### Provider Extraction

```rust
assert_eq!(BaseLLMState::extract_provider("openai/gpt-4"), "openai");
assert_eq!(BaseLLMState::extract_provider("anthropic/claude-3"), "anthropic");
assert_eq!(BaseLLMState::extract_provider("gpt-4"), "openai");  // default
```

---

## Token Usage Metrics

```rust
use crewai::types::usage_metrics::UsageMetrics;

// UsageMetrics fields:
// total_tokens: i64         -- prompt + completion
// prompt_tokens: i64        -- input token count
// cached_prompt_tokens: i64 -- cached input tokens
// completion_tokens: i64    -- output token count
// successful_requests: i64  -- number of successful API calls
```

The `TokenUsage` struct inside `BaseLLMState` accumulates counts across multiple calls. Calling `get_token_usage_summary()` converts it to `UsageMetrics`.

---

## Event Emission and Hooks

The module provides stub functions for LLM lifecycle events:

```rust
use crewai::llms::base_llm::{
    emit_call_started_event,
    emit_call_completed_event,
    emit_call_failed_event,
    emit_stream_chunk_event,
    invoke_before_llm_call_hooks,
    invoke_after_llm_call_hooks,
    LLMCallType,
};

// Event types
// LLMCallType::LlmCall  -- regular completion call
// LLMCallType::ToolCall  -- tool/function call

// Call ID generation
let call_id = generate_call_id();  // UUID v4
let seq = next_call_sequence();     // monotonically increasing counter
```

These are currently log-only stubs. Full event bus integration will wire them to `CrewAIEventsBus::emit()`.

---

## BaseLLMTrait Bridge

The `BaseLLMTrait` is a simplified trait that `LLM` implements, allowing it to be used as a trait object:

```rust
#[async_trait]
pub trait BaseLLMTrait: Send + Sync {
    fn call(
        &self,
        messages: &[HashMap<String, String>],
        tools: Option<&[Value]>,
    ) -> Result<String, String>;

    async fn acall(
        &self,
        messages: &[HashMap<String, String>],
        tools: Option<&[Value]>,
    ) -> Result<String, String>;

    fn supports_function_calling(&self) -> bool;
    fn model_name(&self) -> &str;
    fn get_context_window_size(&self) -> i64;
}

// LLM implements BaseLLMTrait, delegating to its own methods
```

This enables passing `&dyn BaseLLMTrait` to functions that need an LLM without depending on the concrete `LLM` struct.

---

## Hooks Module

The `src/llms/hooks.rs` module provides the `BaseInterceptor` trait for request/response modification:

```rust
pub use hooks::BaseInterceptor;
```

Interceptors can be registered to modify LLM requests before they are sent and responses after they are received.

---

## Python vs. Rust Comparison

| Aspect | Python | Rust |
|--------|--------|------|
| LLM class | `class LLM(BaseLLM)` | `struct LLM` + `impl BaseLLMTrait for LLM` |
| Provider base | `class BaseLLM(ABC)` | `#[async_trait] trait BaseLLM: Send + Sync + Debug` |
| Shared state | `BaseLLM.__init__` instance vars | `BaseLLMState` embedded struct |
| Configuration | `__init__` kwargs | Builder pattern (`.temperature()`, `.max_tokens()`, etc.) |
| Secret protection | Not serialized by convention | `#[serde(skip_serializing)]` on `api_key` |
| Context window lookup | `LLM_CONTEXT_WINDOW_SIZES` dict | `llm_context_window_sizes() -> HashMap` |
| Call ID management | `contextvars.ContextVar` | `Uuid::new_v4()` + `AtomicUsize` counter |
| Token usage | Instance variables | `TokenUsage` struct with typed fields |
| Async support | `async def acall()` | `#[async_trait] async fn acall()` |
| Error handling | Raises exceptions | Returns `Result<T, E>` |

---

## Implementation Status

- LLM configuration, builder pattern, and serialization: **complete**
- Provider inference and context window lookup: **complete**
- BaseLLM trait and BaseLLMState: **complete**
- Token usage tracking: **complete**
- Provider SDK integration (actual API calls): **stub** -- see [Technical Debt Report](../../TECHNICAL_DEBT.md)
- Event emission: **stub** -- log-level only
- Hook invocation: **stub** -- always allows and passes through
