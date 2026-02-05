//! Base agent abstract definition.
//!
//! Corresponds to `crewai/agents/agent_builder/base_agent.py`.
//!
//! Provides the `PlatformApp` enum and the `BaseAgent` trait which is the
//! abstract base for all agents compatible with CrewAI.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;

use async_trait::async_trait;
use md5::{Md5, Digest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::utilities::base_token_process::TokenProcess;
use crate::agents::cache::CacheHandler;
use crate::security::security_config::SecurityConfig;

// ---------------------------------------------------------------------------
// PlatformApp enum
// ---------------------------------------------------------------------------

/// Supported platform applications for CrewAI AMP Tools.
///
/// Corresponds to Python's `PlatformApp` literal type with all 16 variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformApp {
    Asana,
    Box,
    Clickup,
    Github,
    Gmail,
    GoogleCalendar,
    GoogleSheets,
    Hubspot,
    Jira,
    Linear,
    Notion,
    Salesforce,
    Shopify,
    Slack,
    Stripe,
    Zendesk,
}

impl fmt::Display for PlatformApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            PlatformApp::Asana => "asana",
            PlatformApp::Box => "box",
            PlatformApp::Clickup => "clickup",
            PlatformApp::Github => "github",
            PlatformApp::Gmail => "gmail",
            PlatformApp::GoogleCalendar => "google_calendar",
            PlatformApp::GoogleSheets => "google_sheets",
            PlatformApp::Hubspot => "hubspot",
            PlatformApp::Jira => "jira",
            PlatformApp::Linear => "linear",
            PlatformApp::Notion => "notion",
            PlatformApp::Salesforce => "salesforce",
            PlatformApp::Shopify => "shopify",
            PlatformApp::Slack => "slack",
            PlatformApp::Stripe => "stripe",
            PlatformApp::Zendesk => "zendesk",
        };
        write!(f, "{}", name)
    }
}

/// A platform app reference that can be either a known `PlatformApp` enum
/// variant or a custom string (e.g., "gmail/send_email").
///
/// Corresponds to Python's `PlatformAppOrAction = PlatformApp | str`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlatformAppOrAction {
    /// A known platform application.
    Known(PlatformApp),
    /// A custom app or app/action string.
    Custom(String),
}

impl fmt::Display for PlatformAppOrAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlatformAppOrAction::Known(app) => write!(f, "{}", app),
            PlatformAppOrAction::Custom(s) => write!(f, "{}", s),
        }
    }
}

// ---------------------------------------------------------------------------
// BaseAgent trait
// ---------------------------------------------------------------------------

/// Abstract Base trait for all agents compatible with CrewAI.
///
/// Defines the core interface and shared behavior for agents including
/// task execution, tool management, delegation, caching, and input
/// interpolation.
#[async_trait]
pub trait BaseAgent: Send + Sync + fmt::Debug {
    /// Unique identifier for the agent.
    fn id(&self) -> Uuid;

    /// Role of the agent.
    fn role(&self) -> &str;

    /// Objective of the agent.
    fn goal(&self) -> &str;

    /// Backstory of the agent.
    fn backstory(&self) -> &str;

    /// Whether the agent should use a cache for tool usage.
    fn cache(&self) -> bool {
        true
    }

    /// Verbose mode for the Agent Execution.
    fn verbose(&self) -> bool {
        false
    }

    /// Maximum number of requests per minute.
    fn max_rpm(&self) -> Option<u32> {
        None
    }

    /// Enable agent to delegate and ask questions among each other.
    fn allow_delegation(&self) -> bool {
        false
    }

    /// Maximum iterations for an agent to execute a task.
    fn max_iter(&self) -> u32 {
        25
    }

    /// Maximum number of tokens for the agent to generate in a response.
    fn max_tokens(&self) -> Option<u32> {
        None
    }

    /// Whether the agent is adapted (e.g., via an adapter).
    fn adapted_agent(&self) -> bool {
        false
    }

    /// Get the agent's LLM as a dynamic reference.
    fn llm(&self) -> Option<&dyn Any>;

    /// Get the agent's crew as a dynamic reference.
    fn crew(&self) -> Option<&dyn Any>;

    /// Get the security configuration.
    fn security_config(&self) -> &SecurityConfig;

    /// Compute a deterministic key for the agent based on role, goal, and backstory.
    ///
    /// Returns an MD5 hex digest of "role|goal|backstory".
    fn key(&self) -> String {
        let source = format!(
            "{}|{}|{}",
            self.role(),
            self.goal(),
            self.backstory()
        );
        let mut hasher = Md5::new();
        hasher.update(source.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Execute a task synchronously.
    fn execute_task(
        &self,
        task: &dyn Any,
        context: Option<&str>,
        tools: Option<&[Box<dyn Any + Send + Sync>]>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Execute a task asynchronously.
    async fn aexecute_task(
        &self,
        task: &dyn Any,
        context: Option<&str>,
        tools: Option<&[Box<dyn Any + Send + Sync>]>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Create an agent executor with the given tools.
    fn create_agent_executor(
        &mut self,
        tools: Option<Vec<Box<dyn Any + Send + Sync>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get delegation tools for the given list of agents.
    fn get_delegation_tools(
        &self,
        agents: &[Box<dyn BaseAgent>],
    ) -> Vec<Box<dyn Any + Send + Sync>>;

    /// Get platform tools for the specified list of applications.
    fn get_platform_tools(
        &self,
        apps: &[PlatformAppOrAction],
    ) -> Vec<Box<dyn Any + Send + Sync>>;

    /// Get MCP tools for the specified list of MCP server references.
    fn get_mcp_tools(
        &self,
        mcps: &[String],
    ) -> Vec<Box<dyn Any + Send + Sync>>;

    /// Interpolate inputs into the agent role, goal, and backstory.
    fn interpolate_inputs(&mut self, inputs: &HashMap<String, String>);

    /// Set the cache handler for the agent.
    fn set_cache_handler(&mut self, cache_handler: CacheHandler);

    /// Set the RPM controller for the agent.
    fn set_rpm_controller(&mut self, max_rpm: u32);

    /// Create a copy of the agent.
    fn copy_agent(&self) -> Box<dyn BaseAgent>;
}

// ---------------------------------------------------------------------------
// BaseAgentData - shared data for BaseAgent implementations
// ---------------------------------------------------------------------------

/// Shared data fields for `BaseAgent` implementations.
///
/// Concrete agent structs can embed this to get the common fields
/// needed by the `BaseAgent` trait. This mirrors the Pydantic fields
/// on the Python `BaseAgent` class.
#[derive(Serialize, Deserialize)]
pub struct BaseAgentData {
    /// Unique identifier for the agent (auto-generated).
    pub id: Uuid,
    /// Role of the agent.
    pub role: String,
    /// Objective of the agent.
    pub goal: String,
    /// Backstory of the agent.
    pub backstory: String,
    /// Optional agent configuration.
    #[serde(default)]
    pub config: Option<HashMap<String, Value>>,
    /// Whether the agent should use a cache for tool usage.
    #[serde(default = "default_true")]
    pub cache: bool,
    /// Verbose mode for the Agent Execution.
    #[serde(default)]
    pub verbose: bool,
    /// Maximum number of requests per minute.
    #[serde(default)]
    pub max_rpm: Option<u32>,
    /// Enable delegation of tasks to agents.
    #[serde(default)]
    pub allow_delegation: bool,
    /// Maximum iterations for an agent to execute a task.
    #[serde(default = "default_max_iter")]
    pub max_iter: u32,
    /// Maximum number of tokens for the agent to generate.
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Whether the agent is adapted.
    #[serde(default)]
    pub adapted_agent: bool,
    /// Results of the tools used by the agent.
    #[serde(default)]
    pub tools_results: Vec<HashMap<String, Value>>,
    /// Security configuration including fingerprinting.
    #[serde(default)]
    pub security_config: SecurityConfig,
    /// Callbacks to be used for the agent.
    #[serde(skip)]
    pub callbacks: Vec<Box<dyn Fn(&dyn Any) + Send + Sync>>,
    /// Platform apps the agent can access.
    #[serde(default)]
    pub apps: Option<Vec<PlatformAppOrAction>>,
    /// MCP server references.
    #[serde(default)]
    pub mcps: Option<Vec<String>>,
    // --- Private-equivalent fields ---
    /// Original role (before interpolation).
    #[serde(skip)]
    pub original_role: Option<String>,
    /// Original goal (before interpolation).
    #[serde(skip)]
    pub original_goal: Option<String>,
    /// Original backstory (before interpolation).
    #[serde(skip)]
    pub original_backstory: Option<String>,
    /// Token process tracker.
    #[serde(skip)]
    pub token_process: TokenProcess,
}

impl fmt::Debug for BaseAgentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BaseAgentData")
            .field("id", &self.id)
            .field("role", &self.role)
            .field("goal", &self.goal)
            .field("backstory", &self.backstory)
            .field("config", &self.config)
            .field("cache", &self.cache)
            .field("verbose", &self.verbose)
            .field("max_rpm", &self.max_rpm)
            .field("allow_delegation", &self.allow_delegation)
            .field("max_iter", &self.max_iter)
            .field("max_tokens", &self.max_tokens)
            .field("adapted_agent", &self.adapted_agent)
            .field("security_config", &self.security_config)
            .field("callbacks", &format!("[{} callbacks]", self.callbacks.len()))
            .field("apps", &self.apps)
            .field("mcps", &self.mcps)
            .finish()
    }
}

impl Clone for BaseAgentData {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            role: self.role.clone(),
            goal: self.goal.clone(),
            backstory: self.backstory.clone(),
            config: self.config.clone(),
            cache: self.cache,
            verbose: self.verbose,
            max_rpm: self.max_rpm,
            allow_delegation: self.allow_delegation,
            max_iter: self.max_iter,
            max_tokens: self.max_tokens,
            adapted_agent: self.adapted_agent,
            tools_results: self.tools_results.clone(),
            security_config: self.security_config.clone(),
            callbacks: Vec::new(), // Callbacks are not cloneable; start empty
            apps: self.apps.clone(),
            mcps: self.mcps.clone(),
            original_role: self.original_role.clone(),
            original_goal: self.original_goal.clone(),
            original_backstory: self.original_backstory.clone(),
            token_process: self.token_process.clone(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_iter() -> u32 {
    25
}

impl Default for BaseAgentData {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            role: String::new(),
            goal: String::new(),
            backstory: String::new(),
            config: None,
            cache: true,
            verbose: false,
            max_rpm: None,
            allow_delegation: false,
            max_iter: 25,
            max_tokens: None,
            adapted_agent: false,
            tools_results: Vec::new(),
            security_config: SecurityConfig::default(),
            callbacks: Vec::new(),
            apps: None,
            mcps: None,
            original_role: None,
            original_goal: None,
            original_backstory: None,
            token_process: TokenProcess::new(),
        }
    }
}

impl BaseAgentData {
    /// Create a new `BaseAgentData` with the required fields.
    pub fn new(
        role: impl Into<String>,
        goal: impl Into<String>,
        backstory: impl Into<String>,
    ) -> Self {
        Self {
            role: role.into(),
            goal: goal.into(),
            backstory: backstory.into(),
            ..Default::default()
        }
    }

    /// Compute the agent key (MD5 hash of role|goal|backstory).
    pub fn key(&self) -> String {
        let role = self.original_role.as_deref().unwrap_or(&self.role);
        let goal = self.original_goal.as_deref().unwrap_or(&self.goal);
        let backstory = self.original_backstory.as_deref().unwrap_or(&self.backstory);

        let source = format!("{}|{}|{}", role, goal, backstory);
        let mut hasher = Md5::new();
        hasher.update(source.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Interpolate inputs into role, goal, and backstory.
    ///
    /// Saves the originals on first call, then replaces `{key}` placeholders
    /// with values from the inputs map.
    pub fn interpolate_inputs(&mut self, inputs: &HashMap<String, String>) {
        if self.original_role.is_none() {
            self.original_role = Some(self.role.clone());
        }
        if self.original_goal.is_none() {
            self.original_goal = Some(self.goal.clone());
        }
        if self.original_backstory.is_none() {
            self.original_backstory = Some(self.backstory.clone());
        }

        if !inputs.is_empty() {
            self.role = interpolate_string(
                self.original_role.as_deref().unwrap_or(&self.role),
                inputs,
            );
            self.goal = interpolate_string(
                self.original_goal.as_deref().unwrap_or(&self.goal),
                inputs,
            );
            self.backstory = interpolate_string(
                self.original_backstory.as_deref().unwrap_or(&self.backstory),
                inputs,
            );
        }
    }
}

/// Simple string interpolation: replaces `{key}` with value.
fn interpolate_string(template: &str, inputs: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in inputs {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Validate that an app string has at most one `/` separator.
pub fn validate_app_format(app: &str) -> Result<(), String> {
    if app.matches('/').count() > 1 {
        return Err(format!(
            "Invalid app format '{}'. Apps can only have one '/' for app/action format \
             (e.g., 'gmail/send_email')",
            app
        ));
    }
    Ok(())
}

/// Validate that an MCP reference starts with "https://" or "crewai-amp:".
pub fn validate_mcp_reference(mcp: &str) -> Result<(), String> {
    if mcp.starts_with("https://") || mcp.starts_with("crewai-amp:") {
        Ok(())
    } else {
        Err(format!(
            "Invalid MCP reference: {}. String references must start with 'https://' or 'crewai-amp:'",
            mcp
        ))
    }
}
