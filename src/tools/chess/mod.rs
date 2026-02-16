//! Chess-specific tools for the ChessThinkTank multi-agent crew.
//!
//! These tools are registered with chess specialist agents (Strategist,
//! Tactician, Endgame Specialist, etc.) and provide the interface between
//! crewai-rust agents and the chess engine (stonksfish), knowledge graph
//! (ladybug Cypher→DataFusion), and similarity engine (ladybug-rs).
//!
//! The `neo4j_query` tool transpiles Cypher to SQL via ladybug-rs's
//! `CypherParser` → `CypherTranspiler` → `SqlEngine` pipeline, backed
//! by the 8+8/4096 CAM codebook (OpCategory::Cypher = 0x2). Chess
//! procedures are dispatched directly to stonksfish/ladybug.
//!
//! # Tool Inventory
//!
//! | Tool | Provider | Used By |
//! |------|----------|---------|
//! | `chess_evaluate` | stonksfish | Strategist, Tactician, Critic, Advocatus Diaboli |
//! | `chess_legal_moves` | chess crate | Tactician, Critic, Advocatus Diaboli |
//! | `neo4j_query` | ladybug DataFusion | Strategist, Endgame, Psychologist, Advocatus Diaboli |
//! | `ladybug_similarity` | ladybug-rs | Strategist, Endgame |
//! | `chess_whatif` | stonksfish | Strategist, Tactician, Advocatus Diaboli (32-move branching) |

pub mod tools;

pub use tools::{
    chess_evaluate_tool,
    chess_legal_moves_tool,
    chess_whatif_tool,
    chess_vsa_encode_tool,
    neo4j_query_tool,
    ladybug_similarity_tool,
    nars_reason_tool,
    thinking_style_tool,
    all_chess_tools,
};
