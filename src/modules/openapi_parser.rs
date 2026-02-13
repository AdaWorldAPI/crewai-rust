//! Minimal OpenAPI 3.0 parser — extracts paths into `Vec<CapabilityTool>`.
//!
//! This is *intentionally* minimal.  We parse:
//! - `paths.<path>.<method>` → tool name  (`GET /users/{id}` → `get_user`)
//! - `parameters` → `args_schema`
//! - `summary` / `description` → tool description
//! - `requestBody` schema properties → additional args
//!
//! We do **not** attempt full OpenAPI compliance — just enough to bootstrap
//! tools from real-world specs.

use std::collections::HashMap;

use serde_json::Value;

use crate::capabilities::capability::{CapabilityTool, ToolArgSchema};

use super::error::ModuleError;

/// Parse an OpenAPI 3.0 JSON/YAML string into a list of capability tools.
pub fn parse_openapi_spec(spec_content: &str) -> Result<Vec<CapabilityTool>, ModuleError> {
    // Try YAML first (superset of JSON)
    let doc: Value = serde_yaml::from_str(spec_content)
        .map_err(|e| ModuleError::OpenApi(format!("Failed to parse spec: {}", e)))?;

    let paths = doc
        .get("paths")
        .and_then(|p| p.as_object())
        .ok_or_else(|| ModuleError::OpenApi("No 'paths' key in OpenAPI spec".into()))?;

    let mut tools = Vec::new();

    for (path, path_item) in paths {
        let path_obj = match path_item.as_object() {
            Some(o) => o,
            None => continue,
        };

        for method in &["get", "post", "put", "patch", "delete"] {
            let op = match path_obj.get(*method) {
                Some(o) => o,
                None => continue,
            };

            let tool_name = derive_tool_name(method, path);
            let description = op
                .get("summary")
                .or_else(|| op.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or(&tool_name)
                .to_string();

            let mut args_schema = HashMap::new();

            // Extract path parameters
            if let Some(params) = op.get("parameters").and_then(|p| p.as_array()) {
                for param in params {
                    if let Some(name) = param.get("name").and_then(|n| n.as_str()) {
                        let required = param
                            .get("required")
                            .and_then(|r| r.as_bool())
                            .unwrap_or(false);
                        let param_type = param
                            .get("schema")
                            .and_then(|s| s.get("type"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("string")
                            .to_string();
                        let param_desc = param
                            .get("description")
                            .and_then(|d| d.as_str())
                            .map(String::from);

                        args_schema.insert(
                            name.to_string(),
                            ToolArgSchema {
                                arg_type: param_type,
                                required,
                                default: None,
                                description: param_desc,
                                enum_values: None,
                                items: None,
                                pattern: None,
                            },
                        );
                    }
                }
            }

            // Also check path-level parameters
            if let Some(params) = path_obj.get("parameters").and_then(|p| p.as_array()) {
                for param in params {
                    if let Some(name) = param.get("name").and_then(|n| n.as_str()) {
                        if args_schema.contains_key(name) {
                            continue; // operation-level takes precedence
                        }
                        let required = param
                            .get("required")
                            .and_then(|r| r.as_bool())
                            .unwrap_or(false);
                        let param_type = param
                            .get("schema")
                            .and_then(|s| s.get("type"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("string")
                            .to_string();
                        let param_desc = param
                            .get("description")
                            .and_then(|d| d.as_str())
                            .map(String::from);

                        args_schema.insert(
                            name.to_string(),
                            ToolArgSchema {
                                arg_type: param_type,
                                required,
                                default: None,
                                description: param_desc,
                                enum_values: None,
                                items: None,
                                pattern: None,
                            },
                        );
                    }
                }
            }

            // Extract requestBody schema properties (for POST/PUT/PATCH)
            if let Some(body) = op.get("requestBody") {
                if let Some(props) = body
                    .pointer("/content/application~1json/schema/properties")
                    .and_then(|p| p.as_object())
                {
                    let required_fields: Vec<String> = body
                        .pointer("/content/application~1json/schema/required")
                        .and_then(|r| r.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();

                    for (prop_name, prop_schema) in props {
                        let prop_type = prop_schema
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("string")
                            .to_string();
                        let prop_desc = prop_schema
                            .get("description")
                            .and_then(|d| d.as_str())
                            .map(String::from);

                        args_schema.insert(
                            prop_name.clone(),
                            ToolArgSchema {
                                arg_type: prop_type,
                                required: required_fields.contains(prop_name),
                                default: None,
                                description: prop_desc,
                                enum_values: None,
                                items: None,
                                pattern: None,
                            },
                        );
                    }
                }
            }

            let read_only = *method == "get";

            tools.push(CapabilityTool {
                name: tool_name,
                description,
                args_schema,
                result_as_answer: false,
                cam_opcode: None,
                fingerprint_hint: None,
                requires_roles: Vec::new(),
                requires_approval: false,
                idempotent: read_only || *method == "put",
                read_only,
                max_rpm: None,
            });
        }
    }

    Ok(tools)
}

/// Parse an OpenAPI 3.0 spec from a file path.
pub fn parse_openapi_file(path: &str) -> Result<Vec<CapabilityTool>, ModuleError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ModuleError::OpenApi(format!("Failed to read spec file '{}': {}", path, e)))?;
    parse_openapi_spec(&content)
}

/// Derive a tool name from an HTTP method and path.
///
/// Examples:
/// - `GET /users` → `list_users`
/// - `GET /users/{id}` → `get_user`
/// - `POST /users` → `create_user`
/// - `PUT /users/{id}` → `update_user`
/// - `DELETE /users/{id}` → `delete_user`
/// - `GET /users/{userId}/posts` → `list_user_posts`
fn derive_tool_name(method: &str, path: &str) -> String {
    let segments: Vec<&str> = path
        .split('/')
        .filter(|s| !s.is_empty() && !s.starts_with('{'))
        .collect();

    // Check if the *last* segment is a path param (e.g. /users/{id})
    let last_raw_segment = path.split('/').filter(|s| !s.is_empty()).last();
    let ends_with_param = last_raw_segment.map_or(false, |s| s.starts_with('{'));

    if segments.is_empty() {
        return format!("{}_root", method);
    }

    let last = segments.last().unwrap();
    let last_singular = singularize(last);

    let prefix = match method {
        "get" if ends_with_param => "get",
        "get" => "list",
        "post" => "create",
        "put" | "patch" => "update",
        "delete" => "delete",
        other => other,
    };

    let resource: &str = if ends_with_param && method == "get" {
        &last_singular
    } else {
        last
    };

    if segments.len() > 1 {
        // Nested resource: include parent
        let parent = singularize(segments[segments.len() - 2]);
        format!("{}_{}_{}",
            prefix,
            parent.replace('-', "_"),
            resource.replace('-', "_"),
        )
    } else {
        format!("{}_{}", prefix, resource.replace('-', "_"))
    }
}

/// Naive singularization: strip trailing 's' if present.
fn singularize(word: &str) -> String {
    if word.ends_with("ies") {
        format!("{}y", &word[..word.len() - 3])
    } else if word.ends_with('s') && !word.ends_with("ss") {
        word[..word.len() - 1].to_string()
    } else {
        word.to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_tool_name_basic() {
        assert_eq!(derive_tool_name("get", "/users"), "list_users");
        assert_eq!(derive_tool_name("get", "/users/{id}"), "get_user");
        assert_eq!(derive_tool_name("post", "/users"), "create_users");
        assert_eq!(derive_tool_name("put", "/users/{id}"), "update_users");
        assert_eq!(derive_tool_name("delete", "/users/{id}"), "delete_users");
    }

    #[test]
    fn test_derive_tool_name_nested() {
        assert_eq!(
            derive_tool_name("get", "/users/{userId}/posts"),
            "list_user_posts"
        );
        assert_eq!(
            derive_tool_name("get", "/users/{userId}/posts/{postId}"),
            "get_user_post"
        );
    }

    #[test]
    fn test_singularize() {
        assert_eq!(singularize("users"), "user");
        assert_eq!(singularize("entries"), "entry");
        assert_eq!(singularize("class"), "class");
        assert_eq!(singularize("process"), "process");
    }

    #[test]
    fn test_parse_minimal_openapi_spec() {
        let spec = r#"
openapi: "3.0.0"
info:
  title: "Test API"
  version: "1.0.0"
paths:
  /users:
    get:
      summary: "List all users"
      parameters:
        - name: "limit"
          in: query
          schema:
            type: integer
          description: "Max results"
    post:
      summary: "Create a user"
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required:
                - name
              properties:
                name:
                  type: string
                  description: "User name"
                email:
                  type: string
                  description: "User email"
  /users/{id}:
    get:
      summary: "Get a user by ID"
      parameters:
        - name: "id"
          in: path
          required: true
          schema:
            type: string
    delete:
      summary: "Delete a user"
      parameters:
        - name: "id"
          in: path
          required: true
          schema:
            type: string
"#;
        let tools = parse_openapi_spec(spec).unwrap();
        assert_eq!(tools.len(), 4);

        // list_users
        let list = tools.iter().find(|t| t.name == "list_users").unwrap();
        assert_eq!(list.description, "List all users");
        assert!(list.read_only);
        assert!(list.args_schema.contains_key("limit"));

        // create_users
        let create = tools.iter().find(|t| t.name == "create_users").unwrap();
        assert_eq!(create.description, "Create a user");
        assert!(!create.read_only);
        assert!(create.args_schema["name"].required);
        assert!(!create.args_schema["email"].required);

        // get_user
        let get = tools.iter().find(|t| t.name == "get_user").unwrap();
        assert!(get.args_schema["id"].required);

        // delete_users
        let del = tools.iter().find(|t| t.name == "delete_users").unwrap();
        assert!(!del.read_only);
    }

    #[test]
    fn test_parse_empty_paths() {
        let spec = r#"
openapi: "3.0.0"
info:
  title: "Empty"
  version: "1.0.0"
paths: {}
"#;
        let tools = parse_openapi_spec(spec).unwrap();
        assert!(tools.is_empty());
    }

    #[test]
    fn test_parse_missing_paths() {
        let spec = r#"
openapi: "3.0.0"
info:
  title: "No paths"
  version: "1.0.0"
"#;
        assert!(parse_openapi_spec(spec).is_err());
    }

    #[test]
    fn test_parse_path_level_parameters() {
        let spec = r#"
openapi: "3.0.0"
info:
  title: "Test"
  version: "1.0.0"
paths:
  /items/{itemId}:
    parameters:
      - name: "itemId"
        in: path
        required: true
        schema:
          type: string
    get:
      summary: "Get item"
    delete:
      summary: "Delete item"
"#;
        let tools = parse_openapi_spec(spec).unwrap();
        assert_eq!(tools.len(), 2);
        // Both should inherit itemId parameter
        for tool in &tools {
            assert!(tool.args_schema.contains_key("itemId"));
            assert!(tool.args_schema["itemId"].required);
        }
    }
}
