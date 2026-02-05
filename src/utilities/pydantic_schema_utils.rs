//! JSON schema utilities for model descriptions and ref resolution.
//!
//! Corresponds to `crewai/utilities/pydantic_schema_utils.py`.

use std::collections::HashMap;

use serde_json::{Map, Value};

/// Resolve `$ref` pointers in a JSON schema.
///
/// Recursively traverses the schema and replaces `$ref` pointers with
/// the referenced definitions.
///
/// # Arguments
/// * `schema` - The JSON schema value.
/// * `definitions` - The map of definitions (e.g., from `$defs`).
pub fn resolve_refs(schema: &Value, definitions: &Map<String, Value>) -> Value {
    match schema {
        Value::Object(obj) => {
            // Handle $ref
            if let Some(Value::String(ref_path)) = obj.get("$ref") {
                // Extract definition name from "#/$defs/Name" or "#/definitions/Name"
                let def_name = ref_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(ref_path);
                if let Some(def) = definitions.get(def_name) {
                    return resolve_refs(def, definitions);
                }
            }

            // Recursively resolve all object properties
            let mut resolved = Map::new();
            for (key, value) in obj {
                if key == "$defs" || key == "definitions" {
                    // Skip the definitions block in the output
                    continue;
                }
                resolved.insert(key.clone(), resolve_refs(value, definitions));
            }
            Value::Object(resolved)
        }
        Value::Array(arr) => {
            Value::Array(arr.iter().map(|v| resolve_refs(v, definitions)).collect())
        }
        other => other.clone(),
    }
}

/// Generate a human-readable model description from a JSON schema.
///
/// Returns a simplified schema map with field names, types, and descriptions.
pub fn generate_model_description(schema: &Value) -> Value {
    // First resolve any $ref pointers
    let definitions = schema
        .get("$defs")
        .or_else(|| schema.get("definitions"))
        .and_then(|d| d.as_object())
        .cloned()
        .unwrap_or_default();

    let resolved = resolve_refs(schema, &definitions);

    // Build a simplified description
    let mut description = Map::new();

    if let Some(title) = resolved.get("title").and_then(|v| v.as_str()) {
        description.insert("title".to_string(), Value::String(title.to_string()));
    }

    if let Some(desc) = resolved.get("description").and_then(|v| v.as_str()) {
        description.insert("description".to_string(), Value::String(desc.to_string()));
    }

    if let Some(properties) = resolved.get("properties").and_then(|v| v.as_object()) {
        let mut fields = Map::new();
        for (name, prop) in properties {
            let mut field_info = Map::new();
            if let Some(t) = prop.get("type").and_then(|v| v.as_str()) {
                field_info.insert("type".to_string(), Value::String(t.to_string()));
            }
            if let Some(d) = prop.get("description").and_then(|v| v.as_str()) {
                field_info.insert("description".to_string(), Value::String(d.to_string()));
            }
            fields.insert(name.clone(), Value::Object(field_info));
        }
        description.insert("fields".to_string(), Value::Object(fields));
    }

    Value::Object(description)
}

/// Create a schema-like description from a simple field map.
///
/// # Arguments
/// * `fields` - Map of field names to their types/descriptions.
pub fn create_model_from_schema(
    fields: &HashMap<String, (String, String)>,
) -> Value {
    let mut properties = Map::new();
    for (name, (type_name, description)) in fields {
        let mut prop = Map::new();
        prop.insert("type".to_string(), Value::String(type_name.clone()));
        prop.insert(
            "description".to_string(),
            Value::String(description.clone()),
        );
        properties.insert(name.clone(), Value::Object(prop));
    }

    let mut schema = Map::new();
    schema.insert("type".to_string(), Value::String("object".to_string()));
    schema.insert("properties".to_string(), Value::Object(properties));
    Value::Object(schema)
}
