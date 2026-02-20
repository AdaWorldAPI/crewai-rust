//! Blackboard — shared mutable state with phase-based `&mut` discipline.
//!
//! The blackboard is the central coordination point for multi-system
//! execution. Each subsystem (channel, agent, memory, hook) borrows the
//! blackboard mutably during its processing phase, then releases before
//! the next subsystem runs. Rust's borrow checker enforces that only one
//! subsystem has write access at any time.
//!
//! # Zero-Serde Design
//!
//! When all crates compile into one binary, subsystems share native Rust
//! values through typed slots (`TypedSlot`). No serialization occurs for
//! internal data flow. Bytes-based slots (`BlackboardSlot`) are reserved
//! for external boundaries (API, disk, wire).
//!
//! # A2A Awareness
//!
//! The blackboard guarantees shared Agent-to-Agent awareness via the
//! [`A2ARegistry`]. Every agent can discover peers, their capabilities,
//! and their current state — enabling coordination without message passing.
//!
//! # Phase Discipline
//!
//! The [`Phase`] type provides scoped `&mut Blackboard` access with
//! automatic trace logging. Rust's borrow checker prevents overlapping
//! phases at compile time.
//!
//! # Integration with Ladybug
//!
//! When the `ladybug` feature is enabled, slots can carry 8192-bit
//! Container fingerprints for cognitive addressing.

pub mod a2a;
pub mod phase;
pub mod slot;
pub mod typed_slot;
pub mod view;

pub use a2a::{A2ARegistry, AgentPresence, AgentState};
pub use phase::Phase;
pub use slot::{BlackboardSlot, SlotMeta};
pub use typed_slot::TypedSlot;
pub use view::Blackboard;
