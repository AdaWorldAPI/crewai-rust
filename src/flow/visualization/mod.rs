//! Flow structure visualization types and builder.
//!
//! Corresponds to `crewai/flow/visualization/`.
//!
//! Provides types and functions for building a structural representation
//! of a Flow's method graph and rendering it as an interactive HTML
//! visualization using inline JavaScript/CSS.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::collections::VecDeque;

use super::flow::{FlowMethodRegistration, FlowMethodType};
use super::flow_wrappers::{FlowConditionType, FlowMethodName};

/// Simple BFS-based node level calculation for visualization.
fn calculate_node_levels(
    start_nodes: &[String],
    adjacency: &HashMap<String, Vec<String>>,
) -> HashMap<String, usize> {
    let mut levels = HashMap::new();
    let mut queue = VecDeque::new();

    for start in start_nodes {
        levels.insert(start.clone(), 0_usize);
        queue.push_back(start.clone());
    }

    while let Some(current) = queue.pop_front() {
        let current_level = levels[&current];
        if let Some(neighbors) = adjacency.get(&current) {
            for neighbor in neighbors {
                if !levels.contains_key(neighbor) {
                    levels.insert(neighbor.clone(), current_level + 1);
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    levels
}

/// Metadata for a single node in the flow structure.
///
/// Corresponds to `crewai.flow.visualization.types.NodeMetadata`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeMetadata {
    /// Unique identifier for the node (the method name).
    pub id: String,
    /// Display label for the node.
    pub label: String,
    /// Type of the node (e.g., "start", "listen", "router").
    #[serde(rename = "type")]
    pub node_type: Option<String>,
    /// Whether this node is a start method.
    #[serde(default)]
    pub is_start: bool,
    /// Whether this node is a router.
    pub is_router: Option<bool>,
    /// Possible router paths.
    pub router_paths: Option<Vec<String>>,
    /// Condition type (OR/AND).
    pub condition_type: Option<String>,
    /// Trigger condition type.
    pub trigger_condition_type: Option<String>,
    /// Methods that trigger this node.
    pub trigger_methods: Option<Vec<String>>,
    /// Full trigger condition specification.
    pub trigger_condition: Option<HashMap<String, serde_json::Value>>,
    /// Method signature information.
    pub method_signature: Option<HashMap<String, serde_json::Value>>,
    /// Source code of the method.
    pub source_code: Option<String>,
    /// Source code lines.
    pub source_lines: Option<Vec<String>>,
    /// Start line number in source file.
    pub source_start_line: Option<i32>,
    /// Source file path.
    pub source_file: Option<String>,
    /// Class signature.
    pub class_signature: Option<String>,
    /// Class name.
    pub class_name: Option<String>,
    /// Class line number.
    pub class_line_number: Option<i32>,
    /// Level in the graph (BFS depth from root).
    #[serde(default)]
    pub level: usize,
}

/// Represents a connection (edge) in the flow structure.
///
/// Corresponds to `crewai.flow.visualization.types.StructureEdge`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructureEdge {
    /// Source node name.
    pub source: String,
    /// Target node name.
    pub target: String,
    /// Edge label (e.g., "listens_to", router path name).
    pub label: Option<String>,
    /// Condition type on this edge (if any).
    pub condition_type: Option<String>,
    /// Whether this edge is a router path.
    pub is_router_path: Option<bool>,
    /// Label for the router path.
    pub router_path_label: Option<String>,
}

/// Complete structure representation of a Flow.
///
/// Corresponds to `crewai.flow.visualization.types.FlowStructure`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlowStructure {
    /// Nodes in the flow, keyed by method name.
    pub nodes: HashMap<String, NodeMetadata>,
    /// Edges connecting nodes.
    pub edges: Vec<StructureEdge>,
    /// Names of start methods.
    pub start_methods: Vec<String>,
    /// Names of router methods.
    pub router_methods: Vec<String>,
    /// Name of the flow.
    #[serde(default)]
    pub flow_name: String,
}

impl FlowStructure {
    /// Create a new empty FlowStructure.
    pub fn new(flow_name: &str) -> Self {
        Self {
            flow_name: flow_name.to_string(),
            ..Default::default()
        }
    }
}

/// Build the flow structure from method registrations.
///
/// Analyzes the flow's registered methods and their relationships to
/// produce a complete structure for visualization.
///
/// Corresponds to `crewai.flow.visualization.builder.build_flow_structure()`.
///
/// # Arguments
///
/// * `methods` - Registered flow method metadata.
///
/// # Returns
///
/// The complete FlowStructure.
pub fn build_flow_structure(
    methods: &[FlowMethodRegistration],
) -> FlowStructure {
    let mut structure = FlowStructure::default();

    // Build adjacency list for level calculation.
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    let mut start_nodes: Vec<String> = Vec::new();

    for method in methods {
        let name = method.name.0.clone();

        if method.is_start_method {
            start_nodes.push(name.clone());
        }

        // Build adjacency from trigger methods.
        if let Some(ref triggers) = method.trigger_methods {
            for trigger in triggers {
                adjacency
                    .entry(trigger.0.clone())
                    .or_default()
                    .push(name.clone());
            }
        }

        // Build adjacency from router paths.
        if method.is_router {
            if let Some(ref paths) = method.router_paths {
                for path in paths {
                    adjacency
                        .entry(name.clone())
                        .or_default()
                        .push(path.clone());
                }
            }
        }
    }

    // Calculate node levels using BFS.
    let node_levels = calculate_node_levels(&start_nodes, &adjacency);

    for method in methods {
        let name = method.name.0.clone();
        let level = node_levels.get(&name).copied().unwrap_or(0);

        let mut metadata = NodeMetadata::default();
        metadata.id = name.clone();
        metadata.label = name.clone();
        metadata.node_type = Some(format!("{}", method.method_type));
        metadata.is_start = method.is_start_method;
        metadata.is_router = Some(method.is_router);
        metadata.router_paths = method.router_paths.clone();
        metadata.level = level;

        if let Some(ref triggers) = method.trigger_methods {
            metadata.trigger_methods =
                Some(triggers.iter().map(|t| t.0.clone()).collect());
        }

        if let Some(ref ct) = method.condition_type {
            metadata.condition_type = Some(format!("{}", ct));
        }

        structure.nodes.insert(name.clone(), metadata);

        if method.is_start_method {
            structure.start_methods.push(name.clone());
        }

        if method.is_router {
            structure.router_methods.push(name.clone());
        }

        // Build edges from trigger methods.
        if let Some(ref triggers) = method.trigger_methods {
            for trigger in triggers {
                structure.edges.push(StructureEdge {
                    source: trigger.0.clone(),
                    target: name.clone(),
                    label: Some("listens_to".to_string()),
                    condition_type: method
                        .condition_type
                        .as_ref()
                        .map(|c| format!("{}", c)),
                    is_router_path: Some(false),
                    router_path_label: None,
                });
            }
        }

        // Build edges for router paths.
        if method.is_router {
            if let Some(ref paths) = method.router_paths {
                for path in paths {
                    structure.edges.push(StructureEdge {
                        source: name.clone(),
                        target: path.clone(),
                        label: Some(path.clone()),
                        condition_type: None,
                        is_router_path: Some(true),
                        router_path_label: Some(path.clone()),
                    });
                }
            }
        }
    }

    structure
}

/// Calculate execution paths through the flow.
///
/// Returns a list of paths (each path is a list of node IDs).
///
/// Corresponds to `crewai.flow.visualization.builder.calculate_execution_paths()`.
pub fn calculate_execution_paths(structure: &FlowStructure) -> Vec<Vec<String>> {
    let mut paths: Vec<Vec<String>> = Vec::new();

    // Build adjacency from edges.
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &structure.edges {
        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());
    }

    // DFS from each start node.
    for start in &structure.start_methods {
        let mut stack: Vec<(String, Vec<String>)> =
            vec![(start.clone(), vec![start.clone()])];

        while let Some((current, path)) = stack.pop() {
            let has_next = if let Some(neighbors) = adjacency.get(&current) {
                let mut found = false;
                for neighbor in neighbors {
                    if !path.contains(neighbor) {
                        let mut new_path = path.clone();
                        new_path.push(neighbor.clone());
                        stack.push((neighbor.clone(), new_path));
                        found = true;
                    }
                }
                found
            } else {
                false
            };

            // If this is a leaf (no unvisited neighbors), record the path.
            if !has_next {
                paths.push(path);
            }
        }
    }

    paths
}

/// Render an interactive HTML visualization of the flow structure.
///
/// Produces a standalone HTML file with embedded JavaScript and CSS
/// that renders the flow graph. The visualization includes node types
/// (color-coded), edges with labels, and tooltips with metadata.
///
/// Corresponds to `crewai.flow.visualization.renderers.render_interactive()`.
///
/// # Arguments
///
/// * `structure` - The flow structure to render.
/// * `filename` - Output filename (without `.html` extension).
///
/// # Returns
///
/// The path to the generated HTML file, or an error.
pub fn render_interactive(
    structure: &FlowStructure,
    filename: &str,
) -> Result<String, anyhow::Error> {
    let output_path = format!("{}.html", filename);

    // Sort nodes by level for layered display.
    let mut sorted_nodes: Vec<&NodeMetadata> = structure.nodes.values().collect();
    sorted_nodes.sort_by_key(|n| n.level);

    let nodes_json = serde_json::to_string_pretty(&sorted_nodes)?;
    let edges_json = serde_json::to_string_pretty(&structure.edges)?;
    let flow_name = if structure.flow_name.is_empty() {
        "Flow"
    } else {
        &structure.flow_name
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Flow: {flow_name}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background: #f5f5f5;
        }}
        h1 {{ color: #333; }}
        .flow-container {{
            background: white;
            border-radius: 8px;
            padding: 20px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .level {{
            margin-bottom: 15px;
            text-align: center;
        }}
        .node {{
            display: inline-block;
            padding: 10px 20px;
            margin: 5px;
            border-radius: 6px;
            font-weight: 500;
            font-size: 14px;
            cursor: pointer;
        }}
        .node-start {{
            background: #4CAF50;
            color: white;
        }}
        .node-listen {{
            background: #2196F3;
            color: white;
        }}
        .node-router {{
            background: #FF9800;
            color: white;
        }}
        .edges-section {{
            margin-top: 20px;
            border-top: 1px solid #eee;
            padding-top: 10px;
        }}
        .edge {{
            color: #666;
            font-size: 12px;
            margin: 2px 0;
            padding: 4px 8px;
        }}
        .edge.router-path {{
            color: #FF9800;
            font-style: italic;
        }}
        .legend {{
            margin-top: 20px;
            padding: 10px;
            background: #fafafa;
            border-radius: 4px;
        }}
        .legend-item {{
            display: inline-block;
            margin-right: 20px;
        }}
        .legend-color {{
            display: inline-block;
            width: 12px;
            height: 12px;
            border-radius: 50%;
            margin-right: 4px;
            vertical-align: middle;
        }}
        .tooltip {{
            display: none;
            position: absolute;
            background: #333;
            color: white;
            padding: 8px 12px;
            border-radius: 4px;
            font-size: 12px;
            max-width: 400px;
            z-index: 10;
        }}
    </style>
</head>
<body>
    <h1>Flow: {flow_name}</h1>
    <div class="flow-container">
        <div id="graph"></div>
    </div>
    <div class="legend">
        <span class="legend-item"><span class="legend-color" style="background:#4CAF50"></span> Start</span>
        <span class="legend-item"><span class="legend-color" style="background:#2196F3"></span> Listen</span>
        <span class="legend-item"><span class="legend-color" style="background:#FF9800"></span> Router</span>
    </div>
    <div class="tooltip" id="tooltip"></div>
    <script>
        const nodes = {nodes_json};
        const edges = {edges_json};

        const container = document.getElementById('graph');
        const tooltip = document.getElementById('tooltip');

        // Group nodes by level.
        const levels = {{}};
        nodes.forEach(n => {{
            const level = n.level || 0;
            if (!levels[level]) levels[level] = [];
            levels[level].push(n);
        }});

        // Render nodes grouped by level.
        Object.keys(levels).sort((a, b) => a - b).forEach(level => {{
            const levelDiv = document.createElement('div');
            levelDiv.className = 'level';

            levels[level].forEach(node => {{
                const el = document.createElement('span');
                const nodeType = (node.type || 'listen').toLowerCase();
                el.className = 'node node-' + (node.is_start ? 'start' : nodeType);
                el.textContent = node.label || node.id;

                // Tooltip on hover.
                el.addEventListener('mouseenter', (e) => {{
                    const info = [];
                    info.push('Type: ' + (node.type || 'unknown'));
                    if (node.condition_type) info.push('Condition: ' + node.condition_type);
                    if (node.trigger_methods) info.push('Triggers: ' + node.trigger_methods.join(', '));
                    if (node.router_paths && node.router_paths.length) info.push('Paths: ' + node.router_paths.join(', '));
                    tooltip.textContent = info.join(' | ');
                    tooltip.style.display = 'block';
                    tooltip.style.left = e.pageX + 10 + 'px';
                    tooltip.style.top = e.pageY + 10 + 'px';
                }});
                el.addEventListener('mouseleave', () => {{
                    tooltip.style.display = 'none';
                }});

                levelDiv.appendChild(el);
            }});

            container.appendChild(levelDiv);
        }});

        // Render edges.
        if (edges.length > 0) {{
            const edgeDiv = document.createElement('div');
            edgeDiv.className = 'edges-section';
            edgeDiv.innerHTML = '<h3>Connections</h3>';
            edges.forEach(edge => {{
                const el = document.createElement('div');
                el.className = 'edge' + (edge.is_router_path ? ' router-path' : '');
                const label = edge.router_path_label || edge.label || '';
                el.textContent = edge.source + ' \u2192 ' + edge.target +
                    (label ? ' [' + label + ']' : '') +
                    (edge.condition_type ? ' (' + edge.condition_type + ')' : '');
                edgeDiv.appendChild(el);
            }});
            container.appendChild(edgeDiv);
        }}
    </script>
</body>
</html>"#,
        flow_name = flow_name,
        nodes_json = nodes_json,
        edges_json = edges_json,
    );

    std::fs::write(&output_path, &html)?;

    log::info!("Flow visualization written to {}", output_path);

    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::flow::{FlowMethodRegistration, FlowMethodType};
    use crate::flow::flow_wrappers::{FlowConditionType, FlowMethodName};

    #[test]
    fn test_build_flow_structure_empty() {
        let structure = build_flow_structure(&[]);
        assert!(structure.nodes.is_empty());
        assert!(structure.edges.is_empty());
    }

    #[test]
    fn test_build_flow_structure_simple() {
        let methods = vec![
            FlowMethodRegistration {
                name: FlowMethodName::new("start"),
                method_type: FlowMethodType::Start,
                is_start_method: true,
                trigger_methods: None,
                condition_type: None,
                trigger_condition: None,
                is_router: false,
                router_paths: None,
            },
            FlowMethodRegistration {
                name: FlowMethodName::new("process"),
                method_type: FlowMethodType::Listen,
                is_start_method: false,
                trigger_methods: Some(vec![FlowMethodName::new("start")]),
                condition_type: Some(FlowConditionType::OR),
                trigger_condition: None,
                is_router: false,
                router_paths: None,
            },
        ];

        let structure = build_flow_structure(&methods);
        assert_eq!(structure.nodes.len(), 2);
        assert_eq!(structure.edges.len(), 1);
        assert_eq!(structure.edges[0].source, "start");
        assert_eq!(structure.edges[0].target, "process");
    }

    #[test]
    fn test_build_flow_structure_with_router() {
        let methods = vec![
            FlowMethodRegistration {
                name: FlowMethodName::new("begin"),
                method_type: FlowMethodType::Start,
                is_start_method: true,
                trigger_methods: None,
                condition_type: None,
                trigger_condition: None,
                is_router: false,
                router_paths: None,
            },
            FlowMethodRegistration {
                name: FlowMethodName::new("decide"),
                method_type: FlowMethodType::Router,
                is_start_method: false,
                trigger_methods: Some(vec![FlowMethodName::new("begin")]),
                condition_type: Some(FlowConditionType::OR),
                trigger_condition: None,
                is_router: true,
                router_paths: Some(vec!["path_a".to_string(), "path_b".to_string()]),
            },
        ];

        let structure = build_flow_structure(&methods);
        assert_eq!(structure.nodes.len(), 2);
        // 1 listen edge + 2 router path edges.
        assert_eq!(structure.edges.len(), 3);

        let router_edges: Vec<_> = structure
            .edges
            .iter()
            .filter(|e| e.is_router_path == Some(true))
            .collect();
        assert_eq!(router_edges.len(), 2);
    }

    #[test]
    fn test_calculate_execution_paths() {
        let mut structure = FlowStructure::default();
        structure.start_methods = vec!["a".to_string()];
        structure
            .nodes
            .insert("a".to_string(), NodeMetadata { id: "a".to_string(), is_start: true, ..Default::default() });
        structure
            .nodes
            .insert("b".to_string(), NodeMetadata { id: "b".to_string(), ..Default::default() });
        structure.edges.push(StructureEdge {
            source: "a".to_string(),
            target: "b".to_string(),
            label: Some("listens_to".to_string()),
            is_router_path: Some(false),
            ..Default::default()
        });

        let paths = calculate_execution_paths(&structure);
        assert!(!paths.is_empty());
        assert!(paths
            .iter()
            .any(|p| p == &vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn test_node_metadata_serialization() {
        let node = NodeMetadata {
            id: "test".to_string(),
            label: "Test Node".to_string(),
            node_type: Some("start".to_string()),
            is_start: true,
            is_router: Some(false),
            ..Default::default()
        };

        let json = serde_json::to_string(&node).unwrap();
        let deserialized: NodeMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test");
        assert!(deserialized.is_start);
    }
}
