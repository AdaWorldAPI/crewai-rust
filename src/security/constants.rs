//! Security constants.
//!
//! Corresponds to `crewai/security/constants.py`.

use uuid::Uuid;

/// Custom namespace UUID for deterministic UUID generation.
pub fn crew_ai_namespace() -> Uuid {
    // Deterministic namespace UUID for crewAI
    Uuid::parse_str("5f2b4f1a-8c3d-4e5f-9a1b-2c3d4e5f6a7b").unwrap()
}
