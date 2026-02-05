//! Utilities for creating and manipulating types.
//!
//! Corresponds to `crewai/types/utils.py`.

/// Create a set of valid string values from a tuple of strings.
///
/// In Python this creates a Literal type; in Rust we use a Vec<String>
/// for runtime validation (compile-time Literal types don't exist in Rust).
///
/// # Errors
///
/// Returns an error if values is empty.
pub fn create_literals_from_strings(values: &[&str]) -> Result<Vec<String>, String> {
    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    let unique: Vec<String> = values
        .iter()
        .filter(|v| seen.insert(**v))
        .map(|v| v.to_string())
        .collect();

    if unique.is_empty() {
        return Err("Cannot create Literal type from empty values".to_string());
    }

    Ok(unique)
}

/// Validate that a value is one of the allowed literal strings.
pub fn validate_literal(value: &str, allowed: &[String]) -> bool {
    allowed.iter().any(|a| a == value)
}
