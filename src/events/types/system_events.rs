//! System signal event types for CrewAI.
//!
//! Corresponds to `crewai/events/types/system_events.py`.
//!
//! Contains event types for system-level signals like SIGTERM,
//! allowing listeners to perform cleanup operations before process
//! termination.

use serde::{Deserialize, Serialize};

use crate::events::base_event::BaseEventData;
use crate::impl_base_event;

// ---------------------------------------------------------------------------
// SignalType
// ---------------------------------------------------------------------------

/// Enumeration of supported system signals.
///
/// Corresponds to `SignalType` in Python.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalType {
    SIGTERM = 15,
    SIGINT = 2,
    SIGHUP = 1,
    SIGTSTP = 20,
    SIGCONT = 18,
}

// ---------------------------------------------------------------------------
// SigTermEvent
// ---------------------------------------------------------------------------

/// Event emitted when SIGTERM is received.
///
/// Corresponds to `SigTermEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigTermEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The signal number.
    pub signal_number: SignalType,
    /// Optional reason for the signal.
    pub reason: Option<String>,
}

impl SigTermEvent {
    pub fn new(reason: Option<String>) -> Self {
        Self {
            base: BaseEventData::new("SIGTERM"),
            signal_number: SignalType::SIGTERM,
            reason,
        }
    }
}

impl_base_event!(SigTermEvent);

// ---------------------------------------------------------------------------
// SigIntEvent
// ---------------------------------------------------------------------------

/// Event emitted when SIGINT is received.
///
/// Corresponds to `SigIntEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigIntEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The signal number.
    pub signal_number: SignalType,
    /// Optional reason for the signal.
    pub reason: Option<String>,
}

impl SigIntEvent {
    pub fn new(reason: Option<String>) -> Self {
        Self {
            base: BaseEventData::new("SIGINT"),
            signal_number: SignalType::SIGINT,
            reason,
        }
    }
}

impl_base_event!(SigIntEvent);

// ---------------------------------------------------------------------------
// SigHupEvent
// ---------------------------------------------------------------------------

/// Event emitted when SIGHUP is received.
///
/// Corresponds to `SigHupEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigHupEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The signal number.
    pub signal_number: SignalType,
    /// Optional reason for the signal.
    pub reason: Option<String>,
}

impl SigHupEvent {
    pub fn new(reason: Option<String>) -> Self {
        Self {
            base: BaseEventData::new("SIGHUP"),
            signal_number: SignalType::SIGHUP,
            reason,
        }
    }
}

impl_base_event!(SigHupEvent);

// ---------------------------------------------------------------------------
// SigTStpEvent
// ---------------------------------------------------------------------------

/// Event emitted when SIGTSTP is received.
///
/// Note: SIGSTOP cannot be caught -- it immediately suspends the process.
///
/// Corresponds to `SigTStpEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigTStpEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The signal number.
    pub signal_number: SignalType,
    /// Optional reason for the signal.
    pub reason: Option<String>,
}

impl SigTStpEvent {
    pub fn new(reason: Option<String>) -> Self {
        Self {
            base: BaseEventData::new("SIGTSTP"),
            signal_number: SignalType::SIGTSTP,
            reason,
        }
    }
}

impl_base_event!(SigTStpEvent);

// ---------------------------------------------------------------------------
// SigContEvent
// ---------------------------------------------------------------------------

/// Event emitted when SIGCONT is received.
///
/// Corresponds to `SigContEvent` in Python.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigContEvent {
    #[serde(flatten)]
    pub base: BaseEventData,
    /// The signal number.
    pub signal_number: SignalType,
    /// Optional reason for the signal.
    pub reason: Option<String>,
}

impl SigContEvent {
    pub fn new(reason: Option<String>) -> Self {
        Self {
            base: BaseEventData::new("SIGCONT"),
            signal_number: SignalType::SIGCONT,
            reason,
        }
    }
}

impl_base_event!(SigContEvent);

// ---------------------------------------------------------------------------
// SIGNAL_EVENT_TYPES – tuple of all signal event type names
// ---------------------------------------------------------------------------

/// All supported signal event type names.
pub const SIGNAL_EVENT_TYPE_NAMES: &[&str] = &[
    "SIGTERM", "SIGINT", "SIGHUP", "SIGTSTP", "SIGCONT",
];
