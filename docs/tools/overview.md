# Tools Overview

crewAI provides two categories of tools: **core tools** built into the main `crewai` crate, and **external tools** in the companion `crewai-tools` crate. Agents can also discover tools dynamically via MCP servers and create custom tools by implementing the `BaseTool` trait.

---

## Core Tools (in `crewai` crate)

These tools are built into the main crate and available without additional dependencies.

### Agent Collaboration Tools

| Tool | Module | Description |
|------|--------|-------------|
| `DelegateWorkTool` | `tools::agent_tools::delegate_work_tool` | Delegate a task to a co-worker agent |
| `AskQuestionTool` | `tools::agent_tools::ask_question_tool` | Ask a question to a co-worker agent |

### Utility Tools

| Tool | Module | Description |
|------|--------|-------------|
| `ReadFileTool` | `tools::agent_tools::read_file_tool` | Read a file from the filesystem |
| `AddImageTool` | `tools::agent_tools::add_image_tool` | Add an image to the agent's context |
| `CacheTools` | `tools::cache_tools` | Read from the agent's tool-result cache |

### MCP Tools

| Tool | Module | Description |
|------|--------|-------------|
| `MCPNativeTool` | `tools::mcp_native_tool` | Wraps an MCP client session for persistent connections |
| `MCPToolWrapper` | `tools::mcp_tool_wrapper` | On-demand MCP server connection per invocation |

## External Tools (in `crewai-tools` crate)

The companion `crewai-tools` crate provides a comprehensive set of tools organized by category. These mirror the Python `crewai_tools` package.

### Search Tools

| Tool | Description |
|------|-------------|
| `SerperDevTool` | Web search via Serper.dev API |
| `GoogleSerperSearchTool` | Google search via Serper |
| `SearchTool` | Generic search interface |
| `YoutubeVideoSearchTool` | Search YouTube videos |

### Web Scraping Tools

| Tool | Description |
|------|-------------|
| `ScrapeWebsiteTool` | Extract content from web pages |
| `SeleniumScrapingTool` | Browser-based scraping |
| `SpiderTool` | Web crawling |
| `FirecrawlScrapeWebsiteTool` | Firecrawl-based scraping |
| `FirecrawlCrawlWebsiteTool` | Firecrawl-based crawling |
| `FirecrawlSearchTool` | Firecrawl-based search |

### Database Tools

| Tool | Description |
|------|-------------|
| `PGSearchTool` | PostgreSQL search |
| `MySQLSearchTool` | MySQL search |
| `NL2SQLTool` | Natural language to SQL |

### File Operation Tools

| Tool | Description |
|------|-------------|
| `FileReadTool` | Read files |
| `FileWriterTool` | Write files |
| `DirectoryReadTool` | Read directory contents |
| `DirectorySearchTool` | Search directory trees |
| `PDFSearchTool` | Search within PDF files |
| `DOCXSearchTool` | Search within DOCX files |
| `CSVSearchTool` | Search within CSV files |
| `JSONSearchTool` | Search within JSON files |
| `TXTSearchTool` | Search within text files |
| `MDXSearchTool` | Search within MDX files |
| `XMLSearchTool` | Search within XML files |

### AI/ML Tools

| Tool | Description |
|------|-------------|
| `VisionTool` | Image analysis using vision models |
| `DallETool` | Image generation using DALL-E |
| `RagTool` | RAG-based search |

### Automation Tools

| Tool | Description |
|------|-------------|
| `CodeInterpreterTool` | Execute code |
| `CodeDocsSearchTool` | Search code documentation |
| `GithubSearchTool` | Search GitHub repositories |
| `ComposioTool` | Composio integration |

### Cloud Tools

| Tool | Description |
|------|-------------|
| `S3ReaderTool` | Read from S3 buckets |
| `S3WriterTool` | Write to S3 buckets |
| `EXASearchTool` | EXA search API |

### Browser Tools

| Tool | Description |
|------|-------------|
| `BrowserbaseLoadTool` | Load pages via Browserbase |
| `MultiOnTool` | Multi-page browser automation |

## How to Create Custom Tools

There are three ways to create custom tools:

### 1. Implement the `BaseTool` Trait

Full control over all tool behavior. See [Building Custom Tools](../guides/custom-tools.md).

### 2. Use the `Tool` Wrapper

Quick creation from a function:

```rust
use crewai::tools::Tool;
use std::sync::Arc;

let tool = Tool::new(
    "my_tool",
    "Does something useful",
    Arc::new(|args| {
        let input = args.get("input")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        Ok(serde_json::json!(format!("Processed: {}", input)))
    }),
);
```

### 3. Use `CrewStructuredTool::from_function`

Simple function wrapping with argument validation:

```rust
use crewai::tools::CrewStructuredTool;
use std::sync::Arc;

let tool = CrewStructuredTool::from_function(
    "calculator",
    "Perform calculations",
    Arc::new(|args| {
        let expr = args.get("expression")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        Ok(serde_json::json!({"result": expr}))
    }),
);
```

## Tool Lifecycle

When an agent uses a tool, it goes through this lifecycle:

1. **Parse**: LLM output is parsed to extract the tool name and arguments
2. **Select**: The tool is matched by name (exact or fuzzy with >0.85 similarity)
3. **Validate**: Arguments are checked against the tool's schema
4. **Execute**: The tool's `run()` method is called
5. **Cache**: The result is stored for potential reuse
6. **Emit**: Events are published for observability

See [Tools](../concepts/tools.md) for detailed documentation on the tool system.
