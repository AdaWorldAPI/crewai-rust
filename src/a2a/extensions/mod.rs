//! Extensions module for A2A wrapper processing hooks.
//!
//! Corresponds to `crewai/a2a/extensions/base.py` and `crewai/a2a/extensions/registry.py`.
//!
//! These are CrewAI-specific processing hooks, NOT A2A protocol extensions.
//! A2A protocol extensions are capability declarations using AgentExtension
//! objects in AgentCard.capabilities.extensions.

use serde_json::Value;

// ---------------------------------------------------------------------------
// Conversation state
// ---------------------------------------------------------------------------

/// Trait for extension-specific conversation state.
///
/// Extensions can define their own state types that implement this trait
/// to track conversation-specific data.
pub trait ConversationState: Send + Sync {
    /// Check if the state indicates readiness for some action.
    fn is_ready(&self) -> bool;
}

// ---------------------------------------------------------------------------
// A2AExtension trait
// ---------------------------------------------------------------------------

/// Trait for A2A wrapper extensions.
///
/// Extensions can implement this trait to inject custom logic into
/// the A2A conversation flow at various integration points.
pub trait A2AExtension: Send + Sync {
    /// Inject extension-specific tools into an agent.
    ///
    /// Called when an agent is wrapped with A2A capabilities.
    fn inject_tools(&self, _agent_tools: &mut Vec<String>) {}

    /// Extract extension-specific state from conversation history.
    ///
    /// Called during prompt augmentation to allow extensions to analyze
    /// the conversation history and extract relevant state.
    fn extract_state_from_history(
        &self,
        _conversation_history: &[Value],
    ) -> Option<Box<dyn ConversationState>> {
        None
    }

    /// Augment the task prompt with extension-specific instructions.
    fn augment_prompt(
        &self,
        base_prompt: &str,
        _conversation_state: Option<&dyn ConversationState>,
    ) -> String {
        base_prompt.to_string()
    }

    /// Process and potentially modify the agent response.
    fn process_response(
        &self,
        agent_response: Value,
        _conversation_state: Option<&dyn ConversationState>,
    ) -> Value {
        agent_response
    }
}

// ---------------------------------------------------------------------------
// ExtensionRegistry
// ---------------------------------------------------------------------------

/// Registry for managing A2A extensions.
///
/// Maintains a collection of extensions and provides methods to invoke
/// their hooks at various integration points.
pub struct ExtensionRegistry {
    extensions: Vec<Box<dyn A2AExtension>>,
}

impl ExtensionRegistry {
    /// Create a new empty `ExtensionRegistry`.
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
        }
    }

    /// Register an extension.
    pub fn register(&mut self, extension: Box<dyn A2AExtension>) {
        self.extensions.push(extension);
    }

    /// Inject tools from all registered extensions.
    pub fn inject_all_tools(&self, agent_tools: &mut Vec<String>) {
        for ext in &self.extensions {
            ext.inject_tools(agent_tools);
        }
    }

    /// Extract conversation states from all registered extensions.
    pub fn extract_all_states(
        &self,
        conversation_history: &[Value],
    ) -> Vec<Option<Box<dyn ConversationState>>> {
        self.extensions
            .iter()
            .map(|ext| ext.extract_state_from_history(conversation_history))
            .collect()
    }

    /// Augment prompt with instructions from all registered extensions.
    pub fn augment_prompt_with_all(
        &self,
        base_prompt: &str,
        extension_states: &[Option<Box<dyn ConversationState>>],
    ) -> String {
        let mut augmented = base_prompt.to_string();
        for (i, ext) in self.extensions.iter().enumerate() {
            let state = extension_states
                .get(i)
                .and_then(|s| s.as_ref())
                .map(|s| s.as_ref());
            augmented = ext.augment_prompt(&augmented, state);
        }
        augmented
    }

    /// Process response through all registered extensions.
    pub fn process_response_with_all(
        &self,
        agent_response: Value,
        extension_states: &[Option<Box<dyn ConversationState>>],
    ) -> Value {
        let mut processed = agent_response;
        for (i, ext) in self.extensions.iter().enumerate() {
            let state = extension_states
                .get(i)
                .and_then(|s| s.as_ref())
                .map(|s| s.as_ref());
            processed = ext.process_response(processed, state);
        }
        processed
    }

    /// Get the number of registered extensions.
    pub fn len(&self) -> usize {
        self.extensions.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
