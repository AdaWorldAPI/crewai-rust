# Quickstart

This guide walks through creating a simple crew with two agents -- a researcher and a reporting analyst -- mirroring the standard Python crewAI quickstart.

## Prerequisites

- Rust stable toolchain installed (see [Installation](installation.md))
- An LLM API key (e.g., `OPENAI_API_KEY`)

## Step 1: Create a New Project

```bash
cargo new my_crew
cd my_crew
```

## Step 2: Add crewai Dependency

Edit `Cargo.toml`:

```toml
[package]
name = "my_crew"
version = "0.1.0"
edition = "2021"

[dependencies]
crewai = { path = "../path/to/crewai-rust" }
serde_json = "1"
```

## Step 3: Create Agents, Tasks, and a Crew

Edit `src/main.rs`:

```rust
use crewai::{Agent, Task, Crew, Process};
use std::collections::HashMap;

fn main() {
    // Create the researcher agent
    let mut researcher = Agent::new(
        "Senior Research Analyst".to_string(),
        "Uncover cutting-edge developments in AI and data science".to_string(),
        "You are a senior research analyst at a leading tech think tank. \
         Your expertise lies in identifying emerging trends and technologies \
         in AI and data science. You have a knack for dissecting complex data \
         and presenting actionable insights."
            .to_string(),
    );
    researcher.verbose = true;
    researcher.allow_delegation = false;
    // researcher.llm = Some("gpt-4o".to_string());

    // Create the reporting analyst agent
    let mut reporting_analyst = Agent::new(
        "Tech Content Strategist".to_string(),
        "Craft compelling content on tech advancements".to_string(),
        "You are a renowned content strategist known for your insightful \
         and engaging articles on technology and innovation. You transform \
         complex concepts into compelling narratives."
            .to_string(),
    );
    reporting_analyst.verbose = true;
    reporting_analyst.allow_delegation = false;

    // Create the research task
    let mut research_task = Task::new(
        "Conduct a comprehensive analysis of the latest advancements \
         in AI in 2024. Identify key trends, breakthrough technologies, \
         and potential industry impacts."
            .to_string(),
        "Full analysis report in bullet points".to_string(),
    );
    research_task.agent = Some("Senior Research Analyst".to_string());

    // Create the reporting task
    let mut reporting_task = Task::new(
        "Using the insights provided, develop an engaging blog post \
         that highlights the most significant AI advancements. Your post \
         should be informative yet accessible, catering to a tech-savvy \
         audience. Make it sound cool, avoid complex words."
            .to_string(),
        "Full blog post of at least 4 paragraphs".to_string(),
    );
    reporting_task.agent = Some("Tech Content Strategist".to_string());

    // Create the crew
    let mut crew = Crew::new(
        vec![research_task, reporting_task],
        vec![
            "Senior Research Analyst".to_string(),
            "Tech Content Strategist".to_string(),
        ],
    );
    crew.process = Process::Sequential;
    crew.verbose = true;

    // Run the crew
    match crew.kickoff(None) {
        Ok(output) => {
            println!("Crew finished successfully!");
            println!("Result: {}", output.raw);
        }
        Err(e) => {
            eprintln!("Crew execution failed: {}", e);
        }
    }
}
```

## Step 4: Run the Crew

```bash
cargo run
```

## Step 5: Pass Dynamic Inputs

You can pass inputs that get interpolated into task descriptions:

```rust
use std::collections::HashMap;

let mut inputs = HashMap::new();
inputs.insert("topic".to_string(), "AI Agents".to_string());
inputs.insert("year".to_string(), "2024".to_string());

// Use {topic} and {year} placeholders in task descriptions
let mut task = Task::new(
    "Research the latest developments in {topic} for {year}".to_string(),
    "A comprehensive report on {topic}".to_string(),
);
task.agent = Some("Senior Research Analyst".to_string());

let mut crew = Crew::new(
    vec![task],
    vec!["Senior Research Analyst".to_string()],
);

let result = crew.kickoff(Some(inputs));
```

## Step 6: Use Async Execution

For async workflows, use `kickoff_async`:

```rust
#[tokio::main]
async fn main() {
    let mut crew = Crew::new(
        vec![/* tasks */],
        vec![/* agents */],
    );

    match crew.kickoff_async(None).await {
        Ok(output) => println!("Result: {}", output.raw),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

## Important Notes

**LLM Provider Integration**: The current Rust port has complete data model and configuration support, but LLM provider calls (`LLM::call()`) are stub implementations pending provider SDK integration. Task execution returns placeholder outputs. See the [Technical Debt Report](../TECHNICAL_DEBT.md) for full details on implementation status.

**What works today**:
- All struct definitions, configuration, and validation
- Crew orchestration flow (sequential and hierarchical process routing)
- Task interpolation, callbacks, and guardrail types
- Flow state machine with method registration, listeners, and routers
- Event bus architecture with all event types
- MCP server configuration (Stdio, HTTP, SSE)
- Tool system (BaseTool trait, Tool struct, ToolUsage lifecycle)

**What requires provider integration**:
- Actual LLM API calls (OpenAI, Anthropic, etc.)
- Agent executor ReAct loop
- MCP transport layer I/O
- Memory storage backends (RAG, SQLite)
- Knowledge document ingestion

## Next Steps

- [Agents](concepts/agents.md) -- Learn about agent configuration
- [Tasks](concepts/tasks.md) -- Understand task structure and execution
- [Crews](concepts/crews.md) -- Deep dive into crew orchestration
- [Tools](concepts/tools.md) -- Add tools to your agents
- [Flows](concepts/flows.md) -- Build event-driven workflows
