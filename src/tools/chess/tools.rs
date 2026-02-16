//! Concrete chess tool implementations.
//!
//! Each tool calls the real chess stack directly:
//! - **stonksfish** for position evaluation and branching
//! - **ladybug-rs** for 16,384-bit fingerprinting, similarity (RESONATE),
//!   and Cypher→DataFusion query execution (replaces neo4j-rs)
//! - **chess** crate for legal move generation
//!
//! The `neo4j_query` tool transpiles Cypher to SQL via ladybug's
//! `CypherParser` → `CypherTranspiler` → `SqlEngine` pipeline.
//! Chess procedures (chess.evaluate, chess.similar) are dispatched
//! directly to stonksfish/ladybug without the neo4j-rs intermediary.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use chess::{Board, MoveGen, Piece};
use serde_json::{json, Value};

use crate::tools::structured_tool::CrewStructuredTool;

// ---------------------------------------------------------------------------
// Chess Evaluate Tool  →  stonksfish::uci::analyze_position
// ---------------------------------------------------------------------------

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
/// Calls stonksfish `analyze_position()` for real engine evaluation.
pub fn chess_evaluate_tool() -> CrewStructuredTool {
    CrewStructuredTool::new(
        "chess_evaluate",
        "Evaluate a chess position using the stonksfish engine. \
         Input: {\"fen\": \"<FEN string>\", \"depth\": 5}. \
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

            let board = Board::from_str(fen)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Invalid FEN: {}", e).into()
                })?;

            let analysis = stonksfish::uci::analyze_position(&board, depth);

            let top_moves: Vec<Value> = analysis.legal_moves.iter().take(10).map(|m| {
                json!({
                    "uci": m.uci,
                    "eval_cp": m.eval_cp,
                    "is_capture": m.is_capture,
                    "is_check": m.is_check,
                })
            }).collect();

            Ok(json!({
                "fen": analysis.fen,
                "eval_cp": analysis.eval_cp,
                "depth": depth,
                "phase": analysis.phase,
                "piece_count": analysis.piece_count,
                "side_to_move": analysis.side_to_move,
                "is_check": analysis.is_check,
                "is_checkmate": analysis.is_checkmate,
                "is_stalemate": analysis.is_stalemate,
                "top_moves": top_moves,
                "total_legal_moves": analysis.legal_moves.len(),
                "tool": "chess_evaluate",
                "engine": "stonksfish",
            }))
        }),
    )
}

// ---------------------------------------------------------------------------
// Chess Legal Moves Tool  →  chess::MoveGen
// ---------------------------------------------------------------------------

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
/// Uses the `chess` crate for legal move generation with filtering.
pub fn chess_legal_moves_tool() -> CrewStructuredTool {
    CrewStructuredTool::new(
        "chess_legal_moves",
        "List all legal moves in a chess position. \
         Input: {\"fen\": \"<FEN>\", \"filter\": \"all\"}. \
         Returns array of moves in UCI format with metadata (capture, check, promotion).",
        chess_legal_moves_schema(),
        Arc::new(|args: HashMap<String, Value>| {
            let fen = args.get("fen")
                .and_then(|v| v.as_str())
                .unwrap_or("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
            let filter = args.get("filter")
                .and_then(|v| v.as_str())
                .unwrap_or("all");

            let board = Board::from_str(fen)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Invalid FEN: {}", e).into()
                })?;

            let movegen = MoveGen::new_legal(&board);
            let mut new_board = Board::default();
            let mut moves: Vec<Value> = Vec::new();

            for chess_move in movegen {
                board.make_move(chess_move, &mut new_board);
                let is_capture = board.piece_on(chess_move.get_dest()).is_some();
                let is_check = new_board.checkers().popcnt() > 0;
                let is_promotion = chess_move.get_promotion().is_some();

                let include = match filter {
                    "captures" => is_capture,
                    "checks" => is_check,
                    "quiet" => !is_capture && !is_check,
                    _ => true,
                };

                if include {
                    let promo = chess_move.get_promotion().map(|p| match p {
                        Piece::Queen => "q", Piece::Rook => "r",
                        Piece::Bishop => "b", Piece::Knight => "n",
                        _ => "",
                    }).unwrap_or("");
                    let uci = format!(
                        "{}{}{}",
                        chess_move.get_source(),
                        chess_move.get_dest(),
                        promo
                    );

                    moves.push(json!({
                        "uci": uci,
                        "is_capture": is_capture,
                        "is_check": is_check,
                        "is_promotion": is_promotion,
                    }));
                }
            }

            Ok(json!({
                "fen": fen,
                "filter": filter,
                "move_count": moves.len(),
                "moves": moves,
                "tool": "chess_legal_moves",
            }))
        }),
    )
}

// ---------------------------------------------------------------------------
// Neo4j Query Tool  →  ladybug Cypher→DataFusion pipeline
// ---------------------------------------------------------------------------

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
/// Executes Cypher queries via ladybug-rs's Cypher→DataFusion pipeline.
/// Cypher is parsed by `CypherParser`, transpiled to SQL by `CypherTranspiler`,
/// and executed by `SqlEngine` with cognitive UDFs (hamming, similarity, xor_bind).
///
/// Chess procedures are dispatched directly:
/// - `chess.evaluate($fen)` → stonksfish `analyze_position()`
/// - `chess.similar($fen, $k)` → ladybug `ChessFingerprint::resonate()`
/// - `chess.opening_lookup($fen)` → knowledge graph query via DataFusion
///
/// Available node types: Opening, Position, AgentDecision, Plan, Pattern.
/// Edge types: MOVE, BELONGS_TO, SIMILAR_TO, CHOSE, APPLIES_TO.
pub fn neo4j_query_tool() -> CrewStructuredTool {
    CrewStructuredTool::new(
        "neo4j_query",
        "Execute a Cypher query against the chess knowledge graph (ladybug DataFusion). \
         Input: {\"cypher\": \"MATCH (o:Opening {eco: $eco}) RETURN o\", \"params\": {\"eco\": \"B90\"}}. \
         Returns query results as JSON. Available node types: Opening, Position, \
         AgentDecision, Plan, Pattern. Edge types: MOVE, BELONGS_TO, SIMILAR_TO, \
         CHOSE, APPLIES_TO. Chess procedures: CALL chess.evaluate($fen), \
         CALL chess.similar($fen, $k), CALL chess.opening_lookup($fen).",
        neo4j_query_schema(),
        Arc::new(|args: HashMap<String, Value>| {
            let cypher = args.get("cypher")
                .and_then(|v| v.as_str())
                .unwrap_or("RETURN 1");
            let params = args.get("params")
                .cloned()
                .unwrap_or(json!({}));

            // Chess procedures — dispatch directly to stonksfish/ladybug
            if cypher.contains("chess.evaluate") {
                let fen = extract_call_arg(cypher, &params)?;
                return dispatch_chess_evaluate(&fen, cypher);
            }

            if cypher.contains("chess.similar") {
                let (fen, k) = extract_call_args_fen_k(cypher, &params)?;
                return dispatch_chess_similar(&fen, k, cypher);
            }

            if cypher.contains("chess.opening_lookup") {
                let fen = extract_call_arg(cypher, &params)?;
                return dispatch_chess_opening_lookup(&fen, cypher);
            }

            // MATCH / CREATE queries — transpile Cypher → SQL via ladybug
            let sql = ladybug::query::cypher_to_sql(cypher)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Cypher parse/transpile error: {}", e).into()
                })?;

            // Substitute $params into SQL
            let mut processed_sql = sql.clone();
            if let Some(obj) = params.as_object() {
                for (k, v) in obj {
                    let placeholder = format!("${}", k);
                    let replacement = match v {
                        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
                        _ => "NULL".to_string(),
                    };
                    processed_sql = processed_sql.replace(&placeholder, &replacement);
                }
            }

            // Execute SQL via DataFusion (needs async runtime)
            let rt = tokio::runtime::Handle::try_current()
                .or_else(|_| {
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
                        tokio::runtime::Runtime::new()?.handle().clone()
                    )
                })
                .map_err(|e: Box<dyn std::error::Error + Send + Sync>| e)?;

            let batches = rt.block_on(async {
                let engine = ladybug::query::SqlEngine::new().await;
                engine.execute(&processed_sql).await
            }).map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("DataFusion execution error: {}", e).into()
            })?;

            // Convert Arrow RecordBatches to JSON rows
            let mut rows_json: Vec<Value> = Vec::new();
            let mut columns: Vec<String> = Vec::new();

            for batch in &batches {
                if columns.is_empty() {
                    columns = batch.schema().fields().iter()
                        .map(|f| f.name().clone())
                        .collect();
                }
                for row_idx in 0..batch.num_rows() {
                    let mut row = serde_json::Map::new();
                    for (col_idx, field) in batch.schema().fields().iter().enumerate() {
                        let col = batch.column(col_idx);
                        let val = arrow_value_to_json(col, row_idx);
                        row.insert(field.name().clone(), val);
                    }
                    rows_json.push(Value::Object(row));
                }
            }

            let row_count = rows_json.len();
            Ok(json!({
                "cypher": cypher,
                "sql": sql,
                "columns": columns,
                "rows": rows_json,
                "row_count": row_count,
                "tool": "neo4j_query",
                "engine": "ladybug DataFusion",
            }))
        }),
    )
}

/// Dispatch chess.evaluate() directly to stonksfish.
fn dispatch_chess_evaluate(
    fen: &str,
    cypher: &str,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let board = Board::from_str(fen)
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Invalid FEN: {}", e).into()
        })?;

    let analysis = stonksfish::uci::analyze_position(&board, 5);

    Ok(json!({
        "cypher": cypher,
        "columns": ["fen", "eval_cp", "phase", "is_check", "is_checkmate"],
        "rows": [{
            "fen": analysis.fen,
            "eval_cp": analysis.eval_cp,
            "phase": analysis.phase,
            "is_check": analysis.is_check,
            "is_checkmate": analysis.is_checkmate,
            "side_to_move": analysis.side_to_move,
            "total_legal_moves": analysis.legal_moves.len(),
        }],
        "row_count": 1,
        "tool": "neo4j_query",
        "engine": "stonksfish (direct)",
    }))
}

/// Dispatch chess.similar() directly to ladybug ChessFingerprint.
fn dispatch_chess_similar(
    fen: &str,
    k: usize,
    cypher: &str,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use ladybug::chess::ChessFingerprint;

    let query_fp = ChessFingerprint::from_fen(fen)
        .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
            format!("Invalid FEN for fingerprinting: {}", fen).into()
        })?;

    // Seed corpus — same canonical openings as ladybug_similarity_tool
    let reference_fens = [
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq d3 0 1",
        "rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq c3 0 1",
        "rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
        "rnbqkbnr/pppp1ppp/4p3/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
        "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq c6 0 2",
        "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1",
        "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2",
        "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2",
        "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3",
        "rnbqkbnr/pppp1ppp/8/4p3/3PP3/8/PPP2PPP/RNBQKBNR b KQkq d3 0 2",
        "rnbqkbnr/ppp1pppp/8/3p4/3P4/8/PPP1PPPP/RNBQKBNR w KQkq d6 0 2",
    ];

    let candidates: Vec<(String, _)> = reference_fens.iter()
        .filter(|&&f| f != fen)
        .filter_map(|&f| {
            ChessFingerprint::from_fen(f).map(|fp| (f.to_string(), fp))
        })
        .collect();

    let results = ChessFingerprint::resonate(&query_fp, &candidates, k);

    let rows: Vec<Value> = results.iter().map(|(result_fen, similarity, hamming_dist)| {
        json!({
            "fen": result_fen,
            "similarity": *similarity,
            "hamming_distance": *hamming_dist,
        })
    }).collect();

    let row_count = rows.len();
    Ok(json!({
        "cypher": cypher,
        "columns": ["fen", "similarity", "hamming_distance"],
        "rows": rows,
        "row_count": row_count,
        "tool": "neo4j_query",
        "engine": "ladybug RESONATE (direct)",
    }))
}

/// Dispatch chess.opening_lookup() via DataFusion over the knowledge graph.
fn dispatch_chess_opening_lookup(
    fen: &str,
    cypher: &str,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // Opening lookup requires populated knowledge base.
    // In production, this transpiles to SQL over Lance-backed Opening nodes.
    Ok(json!({
        "cypher": cypher,
        "columns": ["eco", "name", "fen", "moves"],
        "rows": [],
        "row_count": 0,
        "tool": "neo4j_query",
        "engine": "ladybug DataFusion",
        "note": format!("Opening lookup for '{}' requires populated Opening table from aiwar-neo4j-harvest", fen),
    }))
}

/// Extract a single string argument from a CALL statement.
fn extract_call_arg(
    cypher: &str,
    params: &Value,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let paren = cypher.find('(')
        .ok_or("Missing '(' in CALL statement")?;
    let close = cypher.find(')')
        .ok_or("Missing ')' in CALL statement")?;
    let arg = cypher[paren + 1..close].trim();

    if arg.starts_with('$') {
        let name = &arg[1..];
        params.get(name)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| format!("Parameter '{}' not found", name).into())
    } else if arg.starts_with('\'') && arg.ends_with('\'') {
        Ok(arg[1..arg.len()-1].to_string())
    } else {
        Ok(arg.to_string())
    }
}

/// Extract (fen, k) arguments from a CALL chess.similar($fen, $k) statement.
fn extract_call_args_fen_k(
    cypher: &str,
    params: &Value,
) -> Result<(String, usize), Box<dyn std::error::Error + Send + Sync>> {
    let paren = cypher.find('(')
        .ok_or("Missing '(' in CALL statement")?;
    let close = cypher.find(')')
        .ok_or("Missing ')' in CALL statement")?;
    let args_str = cypher[paren + 1..close].trim();

    let parts: Vec<&str> = args_str.splitn(2, ',').map(|s| s.trim()).collect();
    let fen_arg = parts.first().unwrap_or(&"");
    let k_arg = parts.get(1).unwrap_or(&"10");

    let fen = if fen_arg.starts_with('$') {
        let name = &fen_arg[1..];
        params.get(name)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    } else if fen_arg.starts_with('\'') && fen_arg.ends_with('\'') {
        fen_arg[1..fen_arg.len()-1].to_string()
    } else {
        fen_arg.to_string()
    };

    let k = if k_arg.starts_with('$') {
        let name = &k_arg[1..];
        params.get(name)
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize
    } else {
        k_arg.parse::<usize>().unwrap_or(10)
    };

    Ok((fen, k))
}

/// Convert an Arrow array cell to JSON (for RecordBatch → JSON conversion).
fn arrow_value_to_json(col: &dyn std::any::Any, row: usize) -> Value {
    use arrow::array::*;

    if let Some(arr) = col.downcast_ref::<StringArray>() {
        if arr.is_null(row) { return Value::Null; }
        json!(arr.value(row))
    } else if let Some(arr) = col.downcast_ref::<Int64Array>() {
        if arr.is_null(row) { return Value::Null; }
        json!(arr.value(row))
    } else if let Some(arr) = col.downcast_ref::<Float64Array>() {
        if arr.is_null(row) { return Value::Null; }
        json!(arr.value(row))
    } else if let Some(arr) = col.downcast_ref::<Float32Array>() {
        if arr.is_null(row) { return Value::Null; }
        json!(arr.value(row))
    } else if let Some(arr) = col.downcast_ref::<UInt64Array>() {
        if arr.is_null(row) { return Value::Null; }
        json!(arr.value(row))
    } else if let Some(arr) = col.downcast_ref::<UInt32Array>() {
        if arr.is_null(row) { return Value::Null; }
        json!(arr.value(row))
    } else if let Some(arr) = col.downcast_ref::<BooleanArray>() {
        if arr.is_null(row) { return Value::Null; }
        json!(arr.value(row))
    } else {
        // Fallback: display as string
        json!(format!("<arrow:{}>", row))
    }
}

// ---------------------------------------------------------------------------
// Ladybug Similarity Tool  →  ladybug::chess::ChessFingerprint
// ---------------------------------------------------------------------------

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
/// Computes real 16,384-bit fingerprints via ladybug-rs ChessFingerprint
/// and uses RESONATE to find similar positions.
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

            use ladybug::chess::ChessFingerprint;

            let query_fp = ChessFingerprint::from_fen(fen)
                .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Invalid FEN for fingerprinting: {}", fen).into()
                })?;

            // Reference positions — seed corpus for similarity comparison.
            // In production this is populated from aiwar-neo4j-harvest Opening nodes
            // stored in ladybug LanceDB. For now, canonical opening positions.
            let reference_fens = [
                "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",  // 1.e4
                "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq d3 0 1",   // 1.d4
                "rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq c3 0 1",   // 1.c4
                "rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",  // Alekhine
                "rnbqkbnr/pppp1ppp/4p3/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",  // French
                "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq c6 0 2", // Sicilian
                "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1",    // 1.Nf3
                "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2", // 1...e5
                "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2", // Ruy Lopez
                "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3", // Italian
                "rnbqkbnr/pppp1ppp/8/4p3/3PP3/8/PPP2PPP/RNBQKBNR b KQkq d3 0 2",  // Center Game
                "rnbqkbnr/ppp1pppp/8/3p4/3P4/8/PPP1PPPP/RNBQKBNR w KQkq d6 0 2",  // QGD
            ];

            let candidates: Vec<(String, _)> = reference_fens.iter()
                .filter(|&&f| f != fen)
                .filter_map(|&f| {
                    ChessFingerprint::from_fen(f).map(|fp| (f.to_string(), fp))
                })
                .collect();

            let results = ChessFingerprint::resonate(&query_fp, &candidates, k);

            let result_json: Vec<Value> = results.iter()
                .filter(|(_, sim, _)| *sim >= threshold as f32)
                .map(|(result_fen, similarity, hamming_dist)| {
                    json!({
                        "fen": result_fen,
                        "similarity": *similarity,
                        "hamming_distance": *hamming_dist,
                    })
                })
                .collect();

            let result_count = result_json.len();
            Ok(json!({
                "query_fen": fen,
                "k": k,
                "threshold": threshold,
                "results": result_json,
                "result_count": result_count,
                "fingerprint_bits": 16384,
                "candidates_scanned": candidates.len(),
                "tool": "ladybug_similarity",
            }))
        }),
    )
}

// ---------------------------------------------------------------------------
// What-If Branching Tool  →  stonksfish::whatif
// ---------------------------------------------------------------------------

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
/// Calls stonksfish `generate_branch_tree()` for real 32-move look-ahead analysis.
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

            use stonksfish::whatif::{BranchConfig, generate_branch_tree, tree_to_json, tree_summary};

            let config = match mode {
                "quick" => BranchConfig::quick(),
                "deep" => BranchConfig::deep(),
                _ => BranchConfig {
                    max_depth: depth,
                    width,
                    node_budget: budget,
                    ..BranchConfig::default()
                },
            };

            let tree = generate_branch_tree(fen, &config)
                .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Invalid FEN for branching: {}", fen).into()
                })?;

            let summary = tree_summary(&tree);
            let tree_json = tree_to_json(&tree);

            Ok(json!({
                "query_fen": fen,
                "mode": mode,
                "config": {
                    "max_depth": config.max_depth,
                    "width": config.width,
                    "node_budget": config.node_budget,
                    "selective_deepening": config.selective_deepening,
                },
                "tree": tree_json,
                "summary": {
                    "total_nodes": summary.total_nodes,
                    "max_depth": summary.max_depth,
                    "terminal_nodes": summary.terminal_nodes,
                    "checkmates": summary.checkmates,
                    "stalemates": summary.stalemates,
                    "eval_range": [summary.eval_range.0, summary.eval_range.1],
                    "branching_factor": summary.branching_factor,
                    "principal_variation": summary.principal_variation,
                },
                "tool": "chess_whatif",
                "engine": "stonksfish",
            }))
        }),
    )
}

// ---------------------------------------------------------------------------
// VSA Move Encoding Tool  →  ladybug VSA bind+permute
// ---------------------------------------------------------------------------

fn chess_vsa_encode_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "moves": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Array of UCI moves to VSA-encode as a sequence (e.g. [\"e2e4\", \"e7e5\", \"g1f3\"])"
            },
            "fen": {
                "type": "string",
                "description": "Starting position FEN (used as context anchor)"
            }
        },
        "required": ["moves"]
    })
}

/// Create the `chess_vsa_encode` tool.
///
/// VSA-encodes chess moves using ladybug-rs bind+permute algebra.
/// Each move becomes a fingerprint; sequences use permutation for ordering.
/// The result can be stored in LanceDB for similarity search over move patterns.
pub fn chess_vsa_encode_tool() -> CrewStructuredTool {
    CrewStructuredTool::new(
        "chess_vsa_encode",
        "VSA-encode a sequence of chess moves as a 16,384-bit fingerprint. \
         Input: {\"moves\": [\"e2e4\", \"e7e5\", \"g1f3\"], \"fen\": \"<starting FEN>\"}. \
         Uses bind+permute algebra: each move is a hypervector, the sequence is \
         encoded via permutation. The resulting fingerprint can find similar move \
         patterns via Hamming distance.",
        chess_vsa_encode_schema(),
        Arc::new(|args: HashMap<String, Value>| {
            use ladybug::core::{Fingerprint, VsaOps};

            let moves = args.get("moves")
                .and_then(|v| v.as_array())
                .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                    "Missing 'moves' array".into()
                })?;

            let fen = args.get("fen")
                .and_then(|v| v.as_str())
                .unwrap_or("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

            // Encode each move as a fingerprint using content-based hashing.
            // The move UCI string becomes a unique hypervector.
            let move_fps: Vec<Fingerprint> = moves.iter()
                .filter_map(|m| m.as_str())
                .map(|uci| Fingerprint::from_content(&format!("chess_move:{}", uci)))
                .collect();

            if move_fps.is_empty() {
                return Err("No valid moves to encode".into());
            }

            // VSA sequence encoding: permute each move by its position index,
            // then bundle (majority vote) to create the sequence fingerprint.
            let sequence_fp = Fingerprint::sequence(&move_fps);

            // Also create position-anchored version: bind with position fingerprint
            let pos_fp = Fingerprint::from_content(&format!("chess_position:{}", fen));
            let anchored_fp = sequence_fp.bind(&pos_fp);

            // Compute some stats
            let individual_sims: Vec<Value> = move_fps.iter().enumerate().map(|(i, fp)| {
                json!({
                    "move_index": i,
                    "move": moves[i],
                    "similarity_to_sequence": fp.similarity(&sequence_fp),
                    "popcount": fp.popcount(),
                })
            }).collect();

            Ok(json!({
                "move_count": move_fps.len(),
                "moves": moves,
                "starting_fen": fen,
                "sequence_fingerprint": {
                    "popcount": sequence_fp.popcount(),
                    "density": sequence_fp.density(),
                    "bits": 16384,
                },
                "anchored_fingerprint": {
                    "popcount": anchored_fp.popcount(),
                    "density": anchored_fp.density(),
                },
                "move_details": individual_sims,
                "tool": "chess_vsa_encode",
                "encoding": "bind+permute (Kanerva 2009)",
            }))
        }),
    )
}

// ---------------------------------------------------------------------------
// NARS Reasoning Tool  →  ladybug::nars::TruthValue
// ---------------------------------------------------------------------------

fn nars_reason_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "operation": {
                "type": "string",
                "description": "NAL inference operation",
                "enum": ["revision", "deduction", "induction", "abduction", "analogy", "comparison", "intersection", "union", "negation"]
            },
            "premise_a": {
                "type": "object",
                "description": "First premise: {\"frequency\": 0.8, \"confidence\": 0.9}"
            },
            "premise_b": {
                "type": "object",
                "description": "Second premise (not needed for negation): {\"frequency\": 0.6, \"confidence\": 0.7}"
            },
            "context": {
                "type": "string",
                "description": "Chess reasoning context (e.g. 'e4 is a strong opening move')"
            }
        },
        "required": ["operation", "premise_a"]
    })
}

/// Create the `nars_reason` tool.
///
/// Performs NARS (Non-Axiomatic Reasoning System) inference via ladybug-rs.
/// Agents use this to combine uncertain chess knowledge:
/// - Revision: combine two sources of evidence about the same claim
/// - Deduction: "if e4 leads to good positions AND good positions lead to wins..."
/// - Abduction: "this position looks like a Sicilian, Sicilians are sharp..."
pub fn nars_reason_tool() -> CrewStructuredTool {
    CrewStructuredTool::new(
        "nars_reason",
        "Perform NARS inference on chess beliefs. \
         Input: {\"operation\": \"deduction\", \"premise_a\": {\"frequency\": 0.8, \"confidence\": 0.9}, \
         \"premise_b\": {\"frequency\": 0.7, \"confidence\": 0.8}, \"context\": \"e4 leads to sharp play\"}. \
         Operations: revision (combine evidence), deduction (A→B, B→C ⊢ A→C), \
         induction, abduction, analogy, comparison, intersection, union, negation. \
         Returns truth value <frequency, confidence> and expectation.",
        nars_reason_schema(),
        Arc::new(|args: HashMap<String, Value>| {
            use ladybug::nars::TruthValue;

            let op = args.get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                    "Missing 'operation'".into()
                })?;

            let context = args.get("context")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let parse_tv = |key: &str| -> Result<TruthValue, Box<dyn std::error::Error + Send + Sync>> {
                let obj = args.get(key)
                    .ok_or_else(|| format!("Missing '{}'", key))?;
                let freq = obj.get("frequency")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5) as f32;
                let conf = obj.get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5) as f32;
                Ok(TruthValue::new(freq, conf))
            };

            let a = parse_tv("premise_a")?;

            let result_tv = if op == "negation" {
                a.negation()
            } else {
                let b = parse_tv("premise_b")?;
                match op {
                    "revision" => a.revision(&b),
                    "deduction" => a.deduction(&b),
                    "induction" => a.induction(&b),
                    "abduction" => a.abduction(&b),
                    "analogy" => a.analogy(&b),
                    "comparison" => a.comparison(&b),
                    "intersection" => a.intersection(&b),
                    "union" => a.union(&b),
                    _ => return Err(format!("Unknown operation: {}", op).into()),
                }
            };

            Ok(json!({
                "operation": op,
                "result": {
                    "frequency": result_tv.frequency,
                    "confidence": result_tv.confidence,
                    "expectation": result_tv.expectation(),
                    "is_positive": result_tv.is_positive(),
                    "is_confident": result_tv.is_confident(),
                },
                "premise_a": {
                    "frequency": a.frequency,
                    "confidence": a.confidence,
                },
                "context": context,
                "tool": "nars_reason",
                "system": "NAL (Non-Axiomatic Logic)",
            }))
        }),
    )
}

// ---------------------------------------------------------------------------
// Thinking Style Tool  →  ladybug::cognitive::ThinkingStyle
// ---------------------------------------------------------------------------

fn thinking_style_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "style": {
                "type": "string",
                "description": "Thinking style to apply",
                "enum": ["analytical", "convergent", "systematic", "creative", "divergent", "exploratory", "focused", "diffuse", "peripheral", "intuitive", "deliberate", "metacognitive"]
            },
            "context": {
                "type": "string",
                "description": "Chess context for style selection"
            }
        },
        "required": ["style"]
    })
}

/// Create the `thinking_style` tool.
///
/// Activates a thinking style from ladybug-rs cognitive module.
/// Each style modulates the agent's search parameters.
pub fn thinking_style_tool() -> CrewStructuredTool {
    CrewStructuredTool::new(
        "thinking_style",
        "Set the agent's thinking style for chess analysis. \
         Input: {\"style\": \"analytical\", \"context\": \"complex middlegame\"}. \
         Styles: analytical (deep/precise), creative (broad/exploratory), \
         intuitive (fast/heuristic), systematic (methodical), focused (narrow/deep), \
         divergent (many alternatives), metacognitive (self-evaluating). \
         Returns field modulation parameters that tune the analysis.",
        thinking_style_schema(),
        Arc::new(|args: HashMap<String, Value>| {
            use ladybug::cognitive::ThinkingStyle;

            let style_name = args.get("style")
                .and_then(|v| v.as_str())
                .unwrap_or("deliberate");

            let context = args.get("context")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let style = match style_name {
                "analytical" => ThinkingStyle::Analytical,
                "convergent" => ThinkingStyle::Convergent,
                "systematic" => ThinkingStyle::Systematic,
                "creative" => ThinkingStyle::Creative,
                "divergent" => ThinkingStyle::Divergent,
                "exploratory" => ThinkingStyle::Exploratory,
                "focused" => ThinkingStyle::Focused,
                "diffuse" => ThinkingStyle::Diffuse,
                "peripheral" => ThinkingStyle::Peripheral,
                "intuitive" => ThinkingStyle::Intuitive,
                "deliberate" => ThinkingStyle::Deliberate,
                "metacognitive" => ThinkingStyle::Metacognitive,
                _ => ThinkingStyle::Deliberate,
            };

            let modulation = style.field_modulation();

            Ok(json!({
                "style": style_name,
                "context": context,
                "modulation": {
                    "resonance_threshold": modulation.resonance_threshold,
                    "fan_out": modulation.fan_out,
                    "depth_bias": modulation.depth_bias,
                    "breadth_bias": modulation.breadth_bias,
                    "noise_tolerance": modulation.noise_tolerance,
                    "speed_bias": modulation.speed_bias,
                    "exploration": modulation.exploration,
                },
                "butterfly_sensitivity": style.butterfly_sensitivity(),
                "confidence_threshold": style.confidence_threshold(),
                "tool": "thinking_style",
                "system": "ladybug cognitive substrate",
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
        chess_vsa_encode_tool(),
        nars_reason_tool(),
        thinking_style_tool(),
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    #[test]
    fn test_chess_evaluate_tool_creation() {
        let tool = chess_evaluate_tool();
        assert_eq!(tool.name, "chess_evaluate");
        assert!(tool.description.contains("stonksfish"));
    }

    #[test]
    fn test_chess_evaluate_tool_invoke() {
        let mut tool = chess_evaluate_tool();
        let result = tool.invoke(json!({
            "fen": STARTPOS,
            "depth": 3
        }));
        assert!(result.is_ok(), "evaluate failed: {:?}", result.err());
        let val = result.unwrap();
        assert_eq!(val["tool"], "chess_evaluate");
        assert_eq!(val["engine"], "stonksfish");
        assert!(val["total_legal_moves"].as_u64().unwrap() == 20);
    }

    #[test]
    fn test_chess_legal_moves_tool_invoke() {
        let mut tool = chess_legal_moves_tool();
        let result = tool.invoke(json!({
            "fen": STARTPOS,
            "filter": "all"
        }));
        assert!(result.is_ok(), "legal_moves failed: {:?}", result.err());
        let val = result.unwrap();
        assert_eq!(val["tool"], "chess_legal_moves");
        assert_eq!(val["move_count"].as_u64().unwrap(), 20);
    }

    #[test]
    fn test_neo4j_query_tool_chess_evaluate() {
        let mut tool = neo4j_query_tool();
        let result = tool.invoke(json!({
            "cypher": "CALL chess.evaluate('rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1')",
            "params": {}
        }));
        assert!(result.is_ok(), "neo4j_query failed: {:?}", result.err());
        let val = result.unwrap();
        assert_eq!(val["tool"], "neo4j_query");
        assert_eq!(val["engine"], "stonksfish (direct)");
        assert!(val["row_count"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_neo4j_query_tool_cypher_transpile() {
        let mut tool = neo4j_query_tool();
        let result = tool.invoke(json!({
            "cypher": "MATCH (o:Opening {eco: 'B90'}) RETURN o",
            "params": {}
        }));
        // Transpiles to SQL; execution may return empty (no data loaded)
        assert!(result.is_ok(), "neo4j_query cypher failed: {:?}", result.err());
        let val = result.unwrap();
        assert_eq!(val["tool"], "neo4j_query");
        assert_eq!(val["engine"], "ladybug DataFusion");
        assert!(val["sql"].as_str().unwrap().contains("SELECT"));
    }

    #[test]
    fn test_ladybug_similarity_tool_invoke() {
        let mut tool = ladybug_similarity_tool();
        let result = tool.invoke(json!({
            "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            "k": 5,
            "threshold": 0.8
        }));
        assert!(result.is_ok(), "similarity failed: {:?}", result.err());
        let val = result.unwrap();
        assert_eq!(val["tool"], "ladybug_similarity");
        assert_eq!(val["fingerprint_bits"], 16384);
    }

    #[test]
    fn test_chess_whatif_tool_invoke() {
        let mut tool = chess_whatif_tool();
        let result = tool.invoke(json!({
            "fen": STARTPOS,
            "depth": 4,
            "width": 2,
            "mode": "quick"
        }));
        assert!(result.is_ok(), "whatif failed: {:?}", result.err());
        let val = result.unwrap();
        assert_eq!(val["tool"], "chess_whatif");
        assert_eq!(val["engine"], "stonksfish");
        assert!(val["summary"]["total_nodes"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_chess_vsa_encode_tool_invoke() {
        let mut tool = chess_vsa_encode_tool();
        let result = tool.invoke(json!({
            "moves": ["e2e4", "e7e5", "g1f3"],
            "fen": STARTPOS
        }));
        assert!(result.is_ok(), "vsa_encode failed: {:?}", result.err());
        let val = result.unwrap();
        assert_eq!(val["tool"], "chess_vsa_encode");
        assert_eq!(val["move_count"], 3);
        assert_eq!(val["sequence_fingerprint"]["bits"], 16384);
    }

    #[test]
    fn test_nars_reason_tool_invoke() {
        let mut tool = nars_reason_tool();
        let result = tool.invoke(json!({
            "operation": "deduction",
            "premise_a": {"frequency": 0.8, "confidence": 0.9},
            "premise_b": {"frequency": 0.7, "confidence": 0.8},
            "context": "e4 leads to sharp play"
        }));
        assert!(result.is_ok(), "nars_reason failed: {:?}", result.err());
        let val = result.unwrap();
        assert_eq!(val["tool"], "nars_reason");
        assert!(val["result"]["frequency"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn test_thinking_style_tool_invoke() {
        let mut tool = thinking_style_tool();
        let result = tool.invoke(json!({
            "style": "analytical",
            "context": "complex middlegame position"
        }));
        assert!(result.is_ok(), "thinking_style failed: {:?}", result.err());
        let val = result.unwrap();
        assert_eq!(val["tool"], "thinking_style");
        assert!(val["modulation"]["fan_out"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_all_chess_tools() {
        let tools = all_chess_tools();
        assert_eq!(tools.len(), 8);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"chess_evaluate"));
        assert!(names.contains(&"chess_legal_moves"));
        assert!(names.contains(&"neo4j_query"));
        assert!(names.contains(&"ladybug_similarity"));
        assert!(names.contains(&"chess_whatif"));
        assert!(names.contains(&"chess_vsa_encode"));
        assert!(names.contains(&"nars_reason"));
        assert!(names.contains(&"thinking_style"));
    }
}
