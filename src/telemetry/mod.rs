//! Telemetry module for CrewAI.
//!
//! Corresponds to `crewai/telemetry/telemetry.py`.
//!
//! Provides anonymous telemetry collection for development purposes.
//! No prompts, task descriptions, agent backstories/goals, responses, or
//! sensitive data is collected. Users can opt-in to share more complete data
//! using the `share_crew` attribute.

use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex, OnceLock};

// opentelemetry trace types available for future use when full OTEL SDK
// initialization is wired up.

// ---------------------------------------------------------------------------
// Singleton
// ---------------------------------------------------------------------------

static INSTANCE: OnceLock<Arc<Mutex<Telemetry>>> = OnceLock::new();

/// Get the global `Telemetry` singleton.
pub fn telemetry() -> Arc<Mutex<Telemetry>> {
    INSTANCE
        .get_or_init(|| Arc::new(Mutex::new(Telemetry::new())))
        .clone()
}

// ---------------------------------------------------------------------------
// Telemetry
// ---------------------------------------------------------------------------

/// Handle anonymous telemetry for the CrewAI package.
#[derive(Debug)]
pub struct Telemetry {
    /// Whether telemetry is initialized and ready.
    pub ready: bool,
    /// Whether the tracer provider has been set.
    pub trace_set: bool,
}

impl Telemetry {
    /// Create a new `Telemetry` instance.
    fn new() -> Self {
        let mut t = Self {
            ready: false,
            trace_set: false,
        };

        if t.is_telemetry_disabled() {
            return t;
        }

        t.set_tracer();
        t
    }

    /// Check whether telemetry is disabled via environment variables.
    ///
    /// Checks `CREWAI_TELEMETRY_OPT_OUT` and `OTEL_SDK_DISABLED`.
    pub fn is_telemetry_disabled(&self) -> bool {
        let opt_out = env::var("CREWAI_TELEMETRY_OPT_OUT")
            .unwrap_or_default()
            .to_lowercase();
        let otel_disabled = env::var("OTEL_SDK_DISABLED")
            .unwrap_or_default()
            .to_lowercase();

        opt_out == "true" || opt_out == "1" || otel_disabled == "true" || otel_disabled == "1"
    }

    /// Set up the OpenTelemetry tracer provider.
    pub fn set_tracer(&mut self) {
        if self.trace_set {
            return;
        }

        if self.is_telemetry_disabled() {
            return;
        }

        // In the Rust port we mark the tracer as set but defer actual
        // OpenTelemetry SDK initialization to runtime configuration.
        // The `opentelemetry` crate handles TracerProvider setup externally.
        self.trace_set = true;
        self.ready = true;
    }

    /// Create a span with the given name and attributes.
    ///
    /// Returns a `SpanHandle` that can be used to add attributes or end the span.
    pub fn create_span(&self, name: &str, attributes: HashMap<String, String>) -> SpanHandle {
        SpanHandle {
            name: name.to_string(),
            attributes,
            ended: false,
        }
    }

    /// Record crew creation telemetry.
    pub fn crew_creation(&self, crew_id: &str, _attributes: HashMap<String, String>) -> SpanHandle {
        self.create_span(
            &format!("crew_creation_{}", crew_id),
            HashMap::new(),
        )
    }

    /// Record crew execution telemetry.
    pub fn crew_execution(
        &self,
        crew_id: &str,
        _attributes: HashMap<String, String>,
    ) -> SpanHandle {
        self.create_span(
            &format!("crew_execution_{}", crew_id),
            HashMap::new(),
        )
    }

    /// Record tool usage telemetry.
    pub fn tool_usage(&self, tool_name: &str, agent_id: &str) -> SpanHandle {
        let mut attrs = HashMap::new();
        attrs.insert("tool_name".to_string(), tool_name.to_string());
        attrs.insert("agent_id".to_string(), agent_id.to_string());
        self.create_span("tool_usage", attrs)
    }
}

/// Handle to a telemetry span.
#[derive(Debug)]
pub struct SpanHandle {
    /// Span name.
    pub name: String,
    /// Span attributes.
    pub attributes: HashMap<String, String>,
    /// Whether the span has been ended.
    pub ended: bool,
}

impl SpanHandle {
    /// Add an attribute to the span.
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        if !self.ended {
            self.attributes.insert(key.into(), value.into());
        }
    }

    /// End (close) the span.
    pub fn end(&mut self) {
        self.ended = true;
    }
}
