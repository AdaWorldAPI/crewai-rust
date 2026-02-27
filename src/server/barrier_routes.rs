//! Barrier Stack REST endpoints — HTTP interface for the 4-layer barrier.
//!
//! These endpoints expose the barrier stack for live testing from Railway,
//! MCP clients, or curl.
//!
//! # Endpoints
//!
//! - `POST /barrier/check-outbound` — Check if an action can cross the barrier
//! - `POST /barrier/check-inbound`  — Check if an inbound response is accepted
//! - `GET  /barrier/topology`       — Current triune facet intensities
//! - `POST /barrier/feedback`       — Apply success/failure feedback
//! - `GET  /barrier/stats`          — Markov barrier statistics

use std::sync::{Arc, RwLock};

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::drivers::barrier_stack::{BarrierStack, MulBlockReason, MulInput};
use crate::drivers::markov_barrier::GateDecision;
use crate::drivers::nars::NarsTruth;
use crate::persona::triune::Facet;

/// Shared barrier state.
pub type BarrierState = Arc<RwLock<BarrierStack>>;

/// Build the barrier router.
pub fn barrier_router(barrier: BarrierState) -> Router {
    Router::new()
        .route("/barrier/check-outbound", post(check_outbound_handler))
        .route("/barrier/check-inbound", post(check_inbound_handler))
        .route("/barrier/topology", get(topology_handler))
        .route("/barrier/feedback", post(feedback_handler))
        .route("/barrier/stats", get(stats_handler))
        .with_state(barrier)
}

// ============================================================================
// Request / Response types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CheckOutboundRequest {
    /// Action being attempted.
    pub action: String,
    /// NARS truth: frequency (0.0-1.0).
    pub nars_frequency: f32,
    /// NARS truth: confidence (0.0-1.0).
    pub nars_confidence: f32,
    /// MUL gate open.
    #[serde(default = "default_true")]
    pub mul_gate_open: bool,
    /// MUL free will modifier (0.0-1.0).
    #[serde(default = "default_one")]
    pub mul_free_will: f32,
    /// MUL DK position name.
    #[serde(default = "default_slope")]
    pub mul_dk_position: String,
    /// MUL trust level name.
    #[serde(default = "default_solid")]
    pub mul_trust_level: String,
    /// MUL allostatic load.
    #[serde(default)]
    pub mul_allostatic_load: f32,
    /// MUL block reason (if gate closed).
    #[serde(default)]
    pub mul_block_reason: Option<String>,
    /// Risk vector: epistemic (0.0-1.0).
    #[serde(default = "default_low_risk")]
    pub risk_epistemic: f32,
    /// Risk vector: moral (0.0-1.0).
    #[serde(default = "default_low_risk")]
    pub risk_moral: f32,
}

fn default_true() -> bool {
    true
}
fn default_one() -> f32 {
    1.0
}
fn default_slope() -> String {
    "slope_of_enlightenment".into()
}
fn default_solid() -> String {
    "solid".into()
}
fn default_low_risk() -> f32 {
    0.1
}

#[derive(Debug, Deserialize)]
pub struct CheckInboundRequest {
    /// Evidence frequency.
    pub evidence_frequency: f32,
    /// Evidence confidence.
    pub evidence_confidence: f32,
    /// Markov gate decision: "commit", "dampen", "reject".
    #[serde(default = "default_commit")]
    pub markov_gate: String,
    /// MUL gate open.
    #[serde(default = "default_true")]
    pub mul_gate_open: bool,
    /// MUL free will modifier.
    #[serde(default = "default_one")]
    pub mul_free_will: f32,
}

fn default_commit() -> String {
    "commit".into()
}

#[derive(Debug, Deserialize)]
pub struct FeedbackRequest {
    /// Which facet to give feedback to: "guardian", "driver", "catalyst".
    pub facet: String,
    /// Whether the action succeeded.
    pub success: bool,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /barrier/check-outbound
async fn check_outbound_handler(
    State(barrier): State<BarrierState>,
    Json(req): Json<CheckOutboundRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let stack = barrier.read().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "lock poisoned"})),
        )
    })?;

    let truth = NarsTruth::new(req.nars_frequency, req.nars_confidence);
    let block_reason = req.mul_block_reason.as_deref().and_then(parse_block_reason);

    let mul = MulInput::new(
        req.mul_gate_open,
        req.mul_free_will,
        &req.mul_dk_position,
        &req.mul_trust_level,
        req.mul_allostatic_load,
        block_reason,
        (req.risk_epistemic, req.risk_moral),
    );

    let decision = stack.check_outbound(&req.action, truth, None, &mul);

    // Build verdicts JSON outside the macro to avoid nested macro issues
    let v = &decision.verdicts;
    let mul_verdict = v.mul.as_ref().map(|m| {
        serde_json::json!({
            "gate_open": m.gate_open,
            "free_will_modifier": m.free_will_modifier,
            "dk_position": m.dk_position,
            "trust_level": m.trust_level,
            "allostatic_load": m.allostatic_load,
        })
    });
    let verdicts_json = serde_json::json!({
        "nars": v.nars,
        "markov": v.markov.map(|g| format!("{:?}", g)),
        "triune": v.triune.as_ref().map(|t| format!("{:?}", t)),
        "mul": mul_verdict,
    });

    Ok(Json(serde_json::json!({
        "proceed": decision.proceed,
        "effective_confidence": decision.effective_confidence,
        "is_clean": decision.is_clean(),
        "is_nudge": decision.is_nudge(),
        "is_blocked": decision.is_blocked(),
        "blocking_layers": decision.blocking.iter().map(|b| format!("{:?}", b)).collect::<Vec<_>>(),
        "nudge": decision.nudge.as_ref().map(|n| format!("{:?}", n)),
        "verdicts": verdicts_json,
    })))
}

/// POST /barrier/check-inbound
async fn check_inbound_handler(
    State(barrier): State<BarrierState>,
    Json(req): Json<CheckInboundRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let stack = barrier.read().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "lock poisoned"})),
        )
    })?;

    let evidence = NarsTruth::new(req.evidence_frequency, req.evidence_confidence);
    let markov_gate = match req.markov_gate.as_str() {
        "commit" => GateDecision::Commit,
        "dampen" => GateDecision::Dampen,
        "reject" => GateDecision::Reject,
        _ => GateDecision::Pending,
    };

    let mul = MulInput {
        gate_open: req.mul_gate_open,
        free_will_modifier: req.mul_free_will,
        dk_position: "slope_of_enlightenment".into(),
        trust_level: "solid".into(),
        allostatic_load: 0.0,
        block_reason: None,
        risk: (0.1, 0.1),
    };

    let decision = stack.check_inbound(evidence, markov_gate, &mul);

    Ok(Json(serde_json::json!({
        "proceed": decision.proceed,
        "effective_confidence": decision.effective_confidence,
        "blocking_layers": decision.blocking.iter().map(|b| format!("{:?}", b)).collect::<Vec<_>>(),
    })))
}

/// GET /barrier/topology
async fn topology_handler(
    State(barrier): State<BarrierState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let stack = barrier.read().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "lock poisoned"})),
        )
    })?;

    let t = &stack.triune.topology;
    Ok(Json(serde_json::json!({
        "guardian": { "intensity": t.guardian.intensity, "leading": t.guardian.leading },
        "driver": { "intensity": t.driver.intensity, "leading": t.driver.leading },
        "catalyst": { "intensity": t.catalyst.intensity, "leading": t.catalyst.leading },
        "is_fused": t.is_fused,
        "balance_score": t.balance_score(),
        "strategy": format!("{:?}", t.strategy()),
        "leader": format!("{:?}", t.leader()),
    })))
}

/// POST /barrier/feedback
async fn feedback_handler(
    State(barrier): State<BarrierState>,
    Json(req): Json<FeedbackRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let facet = match req.facet.as_str() {
        "guardian" => Facet::Guardian,
        "driver" => Facet::Driver,
        "catalyst" => Facet::Catalyst,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "facet must be guardian, driver, or catalyst"})),
            ))
        }
    };

    let mut stack = barrier.write().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "lock poisoned"})),
        )
    })?;

    stack.feedback(facet, req.success);

    let t = &stack.triune.topology;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "facet": req.facet,
        "success": req.success,
        "new_topology": {
            "guardian": t.guardian.intensity,
            "driver": t.driver.intensity,
            "catalyst": t.catalyst.intensity,
            "leader": format!("{:?}", t.leader()),
        },
    })))
}

/// GET /barrier/stats
async fn stats_handler(
    State(barrier): State<BarrierState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let stack = barrier.read().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "lock poisoned"})),
        )
    })?;

    let stats = stack.markov.stats();
    Ok(Json(serde_json::json!({
        "markov": {
            "total_transactions": stats.total_transactions,
            "commits": stats.commits,
            "dampens": stats.dampens,
            "rejects": stats.rejects,
            "cumulative_drift": stats.cumulative_drift,
            "drift_ceiling": stats.drift_ceiling,
            "needs_consolidation": stack.markov.needs_consolidation(),
        },
        "triune": {
            "leader": format!("{:?}", stack.triune.topology.leader()),
            "strategy": format!("{:?}", stack.triune.strategy()),
            "balance": stack.triune.topology.balance_score(),
        },
        "gates": {
            "guardian_min_conf": stack.triune.gate(Facet::Guardian).min_confidence,
            "driver_min_conf": stack.triune.gate(Facet::Driver).min_confidence,
            "catalyst_min_conf": stack.triune.gate(Facet::Catalyst).min_confidence,
        },
    })))
}

// ============================================================================
// Helpers
// ============================================================================

fn parse_block_reason(s: &str) -> Option<MulBlockReason> {
    match s {
        "mount_stupid" => Some(MulBlockReason::MountStupid),
        "complexity_unmapped" => Some(MulBlockReason::ComplexityUnmapped),
        "depleted" => Some(MulBlockReason::Depleted),
        "trust_insufficient" => Some(MulBlockReason::TrustInsufficient),
        "false_flow" => Some(MulBlockReason::FalseFlow),
        "low_free_will" => Some(MulBlockReason::LowFreeWill),
        _ => None,
    }
}
