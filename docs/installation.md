# Installation

## Rust Version Requirements

crewAI-rust requires **Rust stable** with the **2021 edition**. Any recent stable toolchain (1.70+) will work. Verify your installation with:

```bash
rustc --version
cargo --version
```

If you need to install Rust, use [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Adding crewai to Your Project

### As a Path Dependency (local development)

If you have the crewai-rust source checked out locally:

```toml
[dependencies]
crewai = { path = "../path/to/crewai-rust" }
```

### As a Git Dependency

```toml
[dependencies]
crewai = { git = "https://github.com/crewAIInc/crewAI", path = "lib/crewai-rust" }
```

### From crates.io (future)

Once published to crates.io:

```toml
[dependencies]
crewai = "1.9.3"
```

## Dependencies

The crate pulls in the following key dependencies automatically:

| Dependency | Purpose |
|------------|---------|
| `serde`, `serde_json` | Serialization and JSON handling |
| `tokio` (full features) | Async runtime |
| `async-trait` | Async methods in traits |
| `uuid` | Unique identifiers (v4, v5) |
| `chrono` | Date/time handling |
| `reqwest` | HTTP client for LLM and MCP calls |
| `rusqlite` (bundled) | SQLite for flow persistence and long-term memory |
| `thiserror`, `anyhow` | Error handling |
| `regex` | Pattern matching in tool usage |
| `tera` | Template rendering |
| `dashmap`, `parking_lot` | Thread-safe collections and locks |
| `opentelemetry` | Telemetry and tracing |
| `log`, `env_logger` | Logging |
| `md-5` | MD5 hashing for keys |
| `base64` | Base64 encoding for file handling |

## Feature Flags

The crate currently does not define feature flags. All modules are compiled unconditionally. Future releases may introduce optional feature gates for:

- `llm-openai` -- OpenAI provider integration
- `llm-anthropic` -- Anthropic provider integration
- `llm-azure` -- Azure OpenAI provider integration
- `llm-bedrock` -- AWS Bedrock provider integration
- `llm-gemini` -- Google Gemini provider integration
- `mcp-transport` -- MCP transport layer implementations
- `telemetry` -- OpenTelemetry integration

## Building

### Check compilation

```bash
cargo check
```

### Build the library

```bash
cargo build
```

### Build in release mode

```bash
cargo build --release
```

### Run tests

```bash
cargo test
```

### Run a specific test module

```bash
cargo test flow::flow::tests
cargo test tools::tool_usage::tests
```

### Generate documentation

```bash
cargo doc --open
```

## crewai-tools Crate

The companion `crewai-tools` crate provides external tool implementations (web search, scraping, database connectors, etc.). It is a separate crate that depends on `crewai`:

```toml
[dependencies]
crewai = { path = "../crewai-rust" }
crewai-tools = { path = "../crewai-tools" }
```

See [Tools Overview](tools/overview.md) for the full list of available tools.

## Environment Variables

Some features require environment variables to be set:

| Variable | Purpose |
|----------|---------|
| `OPENAI_API_KEY` | OpenAI API authentication |
| `ANTHROPIC_API_KEY` | Anthropic API authentication |
| `AZURE_API_KEY` | Azure OpenAI authentication |
| `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` | AWS Bedrock authentication |
| `GOOGLE_API_KEY` | Google Gemini authentication |
| `CREWAI_TELEMETRY_ENABLED` | Enable/disable telemetry |
| `RUST_LOG` | Configure log level (e.g., `RUST_LOG=debug`) |

## Minimum Supported Platforms

- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)
- Windows (x86_64)

SQLite is bundled via `rusqlite` with the `bundled` feature, so no system SQLite installation is required.
