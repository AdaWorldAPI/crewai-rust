//! Concrete chess tool implementations.
//!
//! Each tool wraps a call to an external service (stonksfish, neo4j-rs,
//! ladybug-rs) and returns structured JSON for the ReAct agent loop.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::tools::structured_tool::CrewStructuredTool;

// ---------------------------------------------------------------------------
// Chess Evaluate Tool
// ---------------------------------------------------------------------------

/// Schema for the chess_evaluate tool input.
fn chess_evaluate_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "fen": {
                "type": "string",
                "description": "FEN string of the chess position to evaluate"
            },
            "depth": {
                "type": "integer",
                "description": "Search depth (1-20, default 5)",
                "default": 5,
                "minimum": 1,
                "maximum": 20
            }
        },
        "required": ["fen"]
    })
}

/// Create the `chess_evaluate` tool.
///
/// Evaluates a chess position using Stonksfish's alpha-beta search,
/// returning the evaluation in centipawns, game phase, piece count,
/// and the top N moves with their evaluations.
///
/// In production, this calls stonksfish via UCI or in-process. The
/// current implementation returns a structured mock response that
/// matches the real stonksfish `analyze_position` output format.
pub fn chess_evaluate_tool() -> CrewStructuredTool {
    CrewStructuredTool::new(
        "chess_evaluate",
        "Evaluate a chess position. Input: {\"fen\": \"<FEN string>\", \"depth\": 5}. \
         Returns evaluation in centipawns, game phase, legal moves with scores, \
         and position flags (check, checkmate, stalemate).",
        chess_evaluate_schema(),
        Arc::new(|args: HashMap<String, Value>| {
            let fen = args.get("fen")
                .and_then(|v| v.as_str())
                .unwrap_or("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
            let depth = args.get("depth")
                .and_then(|v| v.as_u64())
                .unwrap_or(5) as u8;

            // In production: call stonksfish UCI or in-process analyze_position()
            // For now: return the expected output schema so agents know the format
            Ok(json!({
                "fen": fen,
                "eval_cp": 0,
                "depth": depth,
                "phase": "opening",
                "piece_count": 32,
                "side_to_move": "White",
                "is_check": false,
                "is_checkmate": false,
                "is_stalemate": false,
                "top_moves": [
                    {"uci": "e2e4", "eval_cp": 30, "is_capture": false, "is_check": false},
                    {"uci": "d2d4", "eval_cp": 25, "is_capture": false, "is_check": false},
                    {"uci": "g1f3", "eval_cp": 20, "is_capture": false, "is_check": false},
                ],
                "tool": "chess_evaluate",
                "engine": "stonksfish",
                "note": "Connect stonksfish UCI for live evaluation"
            }))
        }),
    )
}

// ---------------------------------------------------------------------------
// Chess Legal Moves Tool
// ---------------------------------------------------------------------------

/// Schema for the chess_legal_moves tool input.
fn chess_legal_moves_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "fen": {
                "type": "string",
                "description": "FEN string of the chess position"
            },
            "filter": {
                "type": "string",
                "description": "Filter: 'all', 'captures', 'checks', 'quiet'",
                "default": "all",
                "enum": ["all", "captures", "checks", "quiet"]
            }
        },
        "required": ["fen"]
    })
}

/// Create the `chess_legal_moves` tool.
///
/// Lists all legal moves in a position, optionally filtered by type.
pub fn chess_legal_moves_tool() -> CrewStructuredTool {
    CrewStructuredTool::new(
        "chess_legal_moves",
        "List all legal moves in a chess position. Input: {\"fen\": \"<FEN>\", \"filter\": \"all\"}. \
         Returns array of moves in UCI format with metadata (capture, check, promotion).",
        chess_legal_moves_schema(),
        Arc::new(|args: HashMap<String, Value>| {
            let fen = args.get("fen")
                .and_then(|v| v.as_str())
                .unwrap_or("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
            let filter = args.get("filter")
                .and_then(|v| v.as_str())
                .unwrap_or("all");

            // In production: call stonksfish to generate legal moves
            Ok(json!({
                "fen": fen,
                "filter": filter,
                "move_count": 20,
                "moves": [
                    {"uci": "e2e4", "is_capture": false, "is_check": false, "is_promotion": false},
                    {"uci": "d2d4", "is_capture": false, "is_check": false, "is_promotion": false},
                ],
                "tool": "chess_legal_moves",
                "note": "Connect stonksfish for live move generation"
            }))
        }),
    )
}

// ---------------------------------------------------------------------------
// Neo4j Query Tool
// ---------------------------------------------------------------------------

/// Schema for the neo4j_query tool input.
fn neo4j_query_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "cypher": {
                "type": "string",
                "description": "Cypher query to execute against the chess knowledge graph"
            },
            "params": {
                "type": "object",
                "description": "Named parameters for the Cypher query",
                "default": {}
            }
        },
        "required": ["cypher"]
    })
}

/// Create the `neo4j_query` tool.
///
/// Executes Cypher queries against the neo4j-rs chess knowledge graph.
/// Used by Strategist (opening book), Endgame Specialist (endgame
/// patterns), and Psychologist (opponent history).
///
/// Example queries:
/// - `MATCH (o:Opening {eco: $eco}) RETURN o` — look up an opening
/// - `MATCH (p:Position {fen: $fen})-[:MOVE]->(next) RETURN next` — next moves
/// - `MATCH (p:Position)-[:SIMILAR_TO]->(q) WHERE p.fen = $fen RETURN q LIMIT 5`
pub fn neo4j_query_tool() -> CrewStructuredTool {
    CrewStructuredTool::new(
        "neo4j_query",
        "Execute a Cypher query against the chess knowledge graph (neo4j-rs). \
         Input: {\"cypher\": \"MATCH (o:Opening {eco: $eco}) RETURN o\", \"params\": {\"eco\": \"B90\"}}. \
         Returns query results as JSON. Available node types: Opening, Position, \
         AgentDecision, Plan, Pattern. Edge types: MOVE, BELONGS_TO, SIMILAR_TO, \
         CHOSE, APPLIES_TO.",
        neo4j_query_schema(),
        Arc::new(|args: HashMap<String, Value>| {
            let cypher = args.get("cypher")
                .and_then(|v| v.as_str())
                .unwrap_or("RETURN 1");
            let params = args.get("params")
                .cloned()
                .unwrap_or(json!({}));

            // In production: execute via Graph<MemoryBackend>::execute()
            Ok(json!({
                "cypher": cypher,
                "params": params,
                "columns": [],
                "rows": [],
                "execution_time_ms": 0,
                "tool": "neo4j_query",
                "note": "Connect neo4j-rs Graph for live queries"
            }))
        }),
    )
}

// ---------------------------------------------------------------------------
// Ladybug Similarity Tool
// ---------------------------------------------------------------------------

/// Schema for the ladybug_similarity tool input.
fn ladybug_similarity_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "fen": {
                "type": "string",
                "description": "FEN string of the chess position to find similar positions for"
            },
            "k": {
                "type": "integer",
                "description": "Number of similar positions to return (default 10)",
                "default": 10,
                "minimum": 1,
                "maximum": 100
            },
            "threshold": {
                "type": "number",
                "description": "Minimum similarity score (0.0-1.0, default 0.7)",
                "default": 0.7
            }
        },
        "required": ["fen"]
    })
}

/// Create the `ladybug_similarity` tool.
///
/// Finds the K most similar chess positions using ladybug-rs 16,384-bit
/// fingerprints and SIMD Hamming distance. Uses the HDR cascade for
/// efficient filtering.
///
/// This is the RESONATE operation: given a position fingerprint, find
/// the K nearest neighbors in the knowledge graph.
pub fn ladybug_similarity_tool() -> CrewStructuredTool {
    CrewStructuredTool::new(
        "ladybug_similarity",
        "Find similar chess positions using 16,384-bit fingerprints (RESONATE operation). \
         Input: {\"fen\": \"<FEN>\", \"k\": 10, \"threshold\": 0.7}. \
         Returns K most similar positions with similarity scores, hamming distances, \
         and position metadata (eval, phase, opening).",
        ladybug_similarity_schema(),
        Arc::new(|args: HashMap<String, Value>| {
            let fen = args.get("fen")
                .and_then(|v| v.as_str())
                .unwrap_or("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
            let k = args.get("k")
                .and_then(|v| v.as_u64())
                .unwrap_or(10) as usize;
            let threshold = args.get("threshold")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7);

            // In production: compute fingerprint, RESONATE via HDR cascade
            Ok(json!({
                "query_fen": fen,
                "k": k,
                "threshold": threshold,
                "results": [],
                "fingerprint_bits": 16384,
                "cascade_levels_searched": 4,
                "candidates_scanned": 0,
                "tool": "ladybug_similarity",
                "note": "Connect ladybug-rs for live RESONATE queries"
            }))
        }),
    )
}

// ---------------------------------------------------------------------------
// What-If Branching Tool
// ---------------------------------------------------------------------------

/// Schema for the chess_whatif tool input.
fn chess_whatif_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "fen": {
                "type": "string",
                "description": "FEN string of the position to branch from"
            },
            "depth": {
                "type": "integer",
                "description": "Maximum look-ahead depth in half-moves (1-32, default 32)",
                "default": 32,
                "minimum": 1,
                "maximum": 32
            },
            "width": {
                "type": "integer",
                "description": "Number of candidate moves to explore at each depth (1-5, default 3)",
                "default": 3,
                "minimum": 1,
                "maximum": 5
            },
            "budget": {
                "type": "integer",
                "description": "Maximum total nodes to generate (default 10000)",
                "default": 10000
            },
            "mode": {
                "type": "string",
                "description": "Branching mode: 'quick' (8-ply/2-wide), 'normal' (32-ply/3-wide), 'deep' (32-ply/3-wide/50K budget)",
                "default": "normal",
                "enum": ["quick", "normal", "deep"]
            }
        },
        "required": ["fen"]
    })
}

/// Create the `chess_whatif` tool.
///
/// Generates a tree of 32-move look-ahead branches from a position for
/// what-if testing. Each branch represents a speculative future that
/// agents can evaluate. Uses selective deepening to focus computation
/// on the most promising lines.
///
/// The branching tree is stored in the neo4j-rs knowledge graph with
/// fork_id tracking via UnifiedExecution.fork(). Each branch node
/// maps to a (:Position) -[:MOVE]-> (:Position) chain with
/// (:AgentDecision) nodes recording which agent evaluated each fork.
pub fn chess_whatif_tool() -> CrewStructuredTool {
    CrewStructuredTool::new(
        "chess_whatif",
        "Generate a 32-move look-ahead branching tree for what-if testing. \
         Input: {\"fen\": \"<FEN>\", \"depth\": 32, \"width\": 3, \"mode\": \"normal\"}. \
         Returns a tree of candidate move sequences with evaluations at each node, \
         principal variation, and fork IDs for tracking in the knowledge graph. \
         Each branch can be independently evaluated by specialist agents.",
        chess_whatif_schema(),
        Arc::new(|args: HashMap<String, Value>| {
            let fen = args.get("fen")
                .and_then(|v| v.as_str())
                .unwrap_or("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
            let mode = args.get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("normal");
            let depth = args.get("depth")
                .and_then(|v| v.as_u64())
                .unwrap_or(32) as u8;
            let width = args.get("width")
                .and_then(|v| v.as_u64())
                .unwrap_or(3) as usize;
            let budget = args.get("budget")
                .and_then(|v| v.as_u64())
                .unwrap_or(10_000) as usize;

            // In production: call stonksfish::whatif::generate_branch_tree()
            // For now: return the expected output schema
            Ok(json!({
                "query_fen": fen,
                "mode": mode,
                "config": {
                    "max_depth": depth,
                    "width": width,
                    "node_budget": budget,
                    "selective_deepening": true,
                },
                "tree": {
                    "total_nodes": 0,
                    "max_depth_reached": 0,
                    "principal_variation": [],
                    "branches": [],
                },
                "summary": {
                    "total_nodes": 0,
                    "max_depth": 0,
                    "terminal_nodes": 0,
                    "checkmates": 0,
                    "stalemates": 0,
                    "eval_range": [0, 0],
                    "branching_factor": 0.0,
                },
                "tool": "chess_whatif",
                "engine": "stonksfish",
                "note": "Connect stonksfish whatif module for live branching"
            }))
        }),
    )
}

// ---------------------------------------------------------------------------
// Convenience: all chess tools
// ---------------------------------------------------------------------------

/// Get all chess tools for registering with the ChessThinkTank agents.
pub fn all_chess_tools() -> Vec<CrewStructuredTool> {
    vec![
        chess_evaluate_tool(),
        chess_legal_moves_tool(),
        neo4j_query_tool(),
        ladybug_similarity_tool(),
        chess_whatif_tool(),
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chess_evaluate_tool_creation() {
        let tool = chess_evaluate_tool();
        assert_eq!(tool.name, "chess_evaluate");
        assert!(tool.description.contains("Evaluate"));
    }

    #[test]
    fn test_chess_evaluate_tool_invoke() {
        let mut tool = chess_evaluate_tool();
        let result = tool.invoke(json!({
            "fen": "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "depth": 5
        }));
        assert!(result.is_ok());
        let val = result.unwrap();
        assert_eq!(val["tool"], "chess_evaluate");
        assert_eq!(val["engine"], "stonksfish");
    }

    #[test]
    fn test_chess_legal_moves_tool_invoke() {
        let mut tool = chess_legal_moves_tool();
        let result = tool.invoke(json!({
            "fen": "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "filter": "all"
        }));
        assert!(result.is_ok());
        let val = result.unwrap();
        assert_eq!(val["tool"], "chess_legal_moves");
    }

    #[test]
    fn test_neo4j_query_tool_invoke() {
        let mut tool = neo4j_query_tool();
        let result = tool.invoke(json!({
            "cypher": "MATCH (o:Opening {eco: 'B90'}) RETURN o",
            "params": {"eco": "B90"}
        }));
        assert!(result.is_ok());
        let val = result.unwrap();
        assert_eq!(val["tool"], "neo4j_query");
    }

    #[test]
    fn test_ladybug_similarity_tool_invoke() {
        let mut tool = ladybug_similarity_tool();
        let result = tool.invoke(json!({
            "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            "k": 5,
            "threshold": 0.8
        }));
        assert!(result.is_ok());
        let val = result.unwrap();
        assert_eq!(val["tool"], "ladybug_similarity");
        assert_eq!(val["fingerprint_bits"], 16384);
    }

    #[test]
    fn test_chess_whatif_tool_invoke() {
        let mut tool = chess_whatif_tool();
        let result = tool.invoke(json!({
            "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            "depth": 16,
            "width": 2,
            "mode": "quick"
        }));
        assert!(result.is_ok());
        let val = result.unwrap();
        assert_eq!(val["tool"], "chess_whatif");
        assert_eq!(val["engine"], "stonksfish");
        assert_eq!(val["config"]["max_depth"], 16);
        assert_eq!(val["config"]["width"], 2);
    }

    #[test]
    fn test_all_chess_tools() {
        let tools = all_chess_tools();
        assert_eq!(tools.len(), 5);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"chess_evaluate"));
        assert!(names.contains(&"chess_legal_moves"));
        assert!(names.contains(&"neo4j_query"));
        assert!(names.contains(&"ladybug_similarity"));
        assert!(names.contains(&"chess_whatif"));
    }
}
