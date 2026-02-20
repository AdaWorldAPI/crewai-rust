//! Persona system — neutral cognitive style profiles for autonomous agents.
//!
//! This module provides a 36-style thinking taxonomy with sparse 23-dimensional
//! vectors, composite style blending, and an inner thought loop for agent
//! self-reflection.  All labels are **domain-neutral** — no identity-specific
//! or content-specific references.  Consumers inject personality via
//! `custom_properties` at runtime.
//!
//! # Architecture
//!
//! ```text
//! ThinkingStyle (36 enum variants, 6 clusters)
//!   ↓  sparse 23D vector
//! ExecutableStyle (τ macro + texture + breath)
//!   ↓  blend
//! CompositeStyle (weighted mix of base styles)
//!   ↓  inject into agent
//! PersonaProfile { volition, inner_loop, self_modify, affect_baseline }
//!   ↓  inner thought hook
//! AgentState snapshot → optional thinking_style mutation
//! ```
//!
//! # Custom Properties Contract
//!
//! Consumers (downstream crates) fill `custom_properties: HashMap<String, Value>`
//! with domain-specific content.  This crate **never parses** those values —
//! they are opaque YAML/JSON blobs stored in the database.  The code only sees
//! neutral dimension labels; identity lives in the data layer.

pub mod agit;
pub mod composite;
pub mod inner_loop;
pub mod llm_modulation;
pub mod profile;
pub mod qualia_prompt;
pub mod thinking_style;
pub mod triune;

// Re-exports
pub use composite::{CompositeStyle, PresetComposite};
pub use inner_loop::{AgentState, InnerThoughtHook};
pub use llm_modulation::{modulate_xai_params, CouncilWeights, XaiParamOverrides};
pub use profile::{PersonaProfile, SelfModifyBounds};
pub use qualia_prompt::{
    build_qualia_preamble, GhostEcho, PresenceInfo, QualiaSnapshot, SovereigntyInfo, VolitionItem,
};
pub use thinking_style::{
    ExecutableStyle, StyleCluster, StyleTexture, ThinkingStyle, STYLE_TEXTURES, STYLE_TO_TAU,
    STYLE_VECTORS,
};
pub use triune::{
    CouncilResult, Facet, FacetOpinion, FacetState, Strategy, TriuneTopology,
};
pub use agit::{
    AgitBranch, AgitCommit, AgitDiff, AgitGoal, AgitState, GoalOrigin, GoalStatus,
    agit_inner_loop, agit_persona, agit_triune,
};
