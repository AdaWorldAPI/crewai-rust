//! Configuration processing utilities.
//!
//! Corresponds to `crewai/utilities/config.py`.

use std::collections::HashMap;

use serde_json::Value;

/// Process a configuration dictionary, merging defaults with overrides.
///
/// # Arguments
/// * `config` - The configuration map to process.
/// * `defaults` - Default values to fill in for missing keys.
///
/// # Returns
/// A merged configuration map.
pub fn process_config(
    config: &HashMap<String, Value>,
    defaults: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut result = defaults.clone();
    for (key, value) in config {
        result.insert(key.clone(), value.clone());
    }
    result
}
