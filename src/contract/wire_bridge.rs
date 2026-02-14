//! Binary Wire Protocol Bridge for crewai-rust.
//!
//! Converts between crewai-rust's local types and CogPacket binary format.
//! This replaces JSON serialization for internal communication with ladybug-rs.
//!
//! External interfaces (REST API, webhooks) still use JSON — but internally,
//! all agent-to-agent and agent-to-substrate communication uses CogPackets.
//!
//! ```text
//! External (JSON)          Internal (Binary)
//! ═══════════════          ═════════════════
//! POST /execute  ──►  wire_bridge::ingest()  ──►  CogPacket
//!                                                     │
//!                     ┌───────────────────────────────┘
//!                     ▼
//!              CognitiveKernel.process_packet()
//!                     │
//!                     ▼
//!              CogPacket (response)
//!                     │
//!              wire_bridge::emit()  ──►  JSON response
//! ```

use ladybug_contract::container::Container;
use ladybug_contract::nars::TruthValue;
use ladybug_contract::wire::{self, CogPacket};

use crate::contract::types::{DataEnvelope, EnvelopeMetadata, StepDelegationRequest, StepDelegationResponse, StepStatus, UnifiedStep};

/// Convert a StepDelegationRequest to a CogPacket.
///
/// The step_type routes to the correct 8+8 prefix:
/// - "crew.*" → 0x0C (Agents)
/// - "lb.*"   → 0x05 (Causal) for resonate, 0x80+ (Node) for collapse
/// - "n8n.*"  → 0x0F (A2A)
pub fn ingest(request: &StepDelegationRequest) -> CogPacket {
    let step_type = &request.step.step_type;

    // Determine source/target addresses from step_type
    let (source_prefix, opcode) = route_step_type(step_type);
    let source_addr = (source_prefix as u16) << 8;
    let target_addr = source_addr | 0x01;

    // Hash the input data to a Container
    let content_hash = hash_json_to_u64(&request.input.data);
    let content = Container::random(content_hash);

    let mut pkt = CogPacket::request(opcode, source_addr, target_addr, content);

    // Pack metadata into header
    pkt.set_cycle(request.input.metadata.epoch as u64);

    // Pack confidence as NARS truth value
    let conf = request.input.metadata.confidence as f32;
    if conf > 0.0 {
        pkt.set_truth_value(&TruthValue::new(1.0, conf));
    }

    // Pack dominant layer
    if let Some(layer) = request.input.metadata.dominant_layer {
        pkt.set_layer(layer);
    }

    // Pack layer activations as satisfaction scores
    if let Some(ref activations) = request.input.metadata.layer_activations {
        for (i, &a) in activations.iter().enumerate().take(10) {
            pkt.set_satisfaction(i as u8, a);
        }
    }

    // Pack NARS frequency
    if let Some(freq) = request.input.metadata.nars_frequency {
        let tv = pkt.truth_value();
        pkt.set_truth_value(&TruthValue::new(freq as f32, tv.confidence));
    }

    pkt.set_flags(pkt.flags() | wire::FLAG_DELEGATION);
    pkt.update_checksum();
    pkt
}

/// Convert a CogPacket response back to a StepDelegationResponse.
///
/// This is the egress path — binary → JSON for external consumers.
pub fn emit(response: &CogPacket, original_step: &UnifiedStep) -> StepDelegationResponse {
    let tv = response.truth_value();
    let sat = response.satisfaction_array();

    let mut step = original_step.clone();
    step.status = if response.is_error() {
        StepStatus::Failed
    } else {
        StepStatus::Completed
    };
    step.confidence = Some(tv.confidence as f64);

    let metadata = EnvelopeMetadata {
        source_step: step.step_id.clone(),
        confidence: tv.confidence as f64,
        epoch: response.cycle() as i64,
        version: Some(format!("wire-v{}", wire::WIRE_VERSION)),
        dominant_layer: Some(response.layer()),
        layer_activations: Some(sat.to_vec()),
        nars_frequency: Some(tv.frequency as f64),
        calibration_error: None,
    };

    let output = DataEnvelope {
        data: serde_json::json!({
            "opcode": response.opcode(),
            "cycle": response.cycle(),
            "crystallized": response.flags() & wire::FLAG_CRYSTALLIZED != 0,
            "validated": response.flags() & wire::FLAG_VALIDATED != 0,
            "rung": response.rung(),
            "source_addr": format!("{:#06x}", response.source_addr()),
            "target_addr": format!("{:#06x}", response.target_addr()),
        }),
        metadata,
    };

    StepDelegationResponse {
        output,
        step: Some(step),
    }
}

/// Create a CogPacket for a crew agent execution result.
///
/// When a crew agent completes a task, its output is packed into a
/// CogPacket at L5 (Execution) with the agent's FieldModulation.
pub fn pack_agent_result(
    agent_id: &str,
    output: &str,
    confidence: f64,
    thinking_style: Option<&[f32; 10]>,
) -> CogPacket {
    let content_hash = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        agent_id.hash(&mut h);
        output.hash(&mut h);
        h.finish()
    };

    let content = Container::random(content_hash);

    // Agent slot derived from agent_id hash
    let agent_slot = (content_hash & 0xFF) as u8;
    let source_addr = (0x0Cu16 << 8) | agent_slot as u16;
    let target_addr = (0x10u16 << 8) | 0x00; // Fluid zone slot 0

    let mut pkt = CogPacket::response(
        wire::wire_ops::EXECUTE,
        source_addr,
        target_addr,
        content,
    );

    pkt.set_layer(4); // L5 Execution (0-indexed = 4)
    pkt.set_truth_value(&TruthValue::new(1.0, confidence as f32));

    // Pack thinking style as field modulation (10-layer cognitive stack)
    if let Some(style) = thinking_style {
        // style[0] = recognition → resonance_threshold
        pkt.set_resonance_threshold(style[0]);
        // style[1] = resonance → exploration
        pkt.set_exploration(style[1]);
        // style[2] = appraisal → depth_bias
        pkt.set_depth_bias(style[2]);
        // style[4] = execution → fan_out (scaled)
        pkt.set_fan_out((style[4] * 20.0) as u8);
        // style[5] = delegation → noise_tolerance (inverted)
        pkt.set_noise_tolerance(1.0 - style[5]);
        // style[6] = contingency → speed_bias
        pkt.set_speed_bias(style[6]);
        // Pack satisfaction scores for all 10 layers
        for (i, &s) in style.iter().enumerate().take(10) {
            pkt.set_satisfaction(i as u8, s);
        }
    }

    pkt.update_checksum();
    pkt
}

// =============================================================================
// HELPERS
// =============================================================================

/// Route step_type to (prefix, opcode).
fn route_step_type(step_type: &str) -> (u8, u16) {
    match step_type.split('.').next() {
        Some("crew") => (0x0C, wire::wire_ops::DELEGATE),
        Some("lb") => {
            if step_type.contains("resonate") {
                (0x05, wire::wire_ops::RESONATE)
            } else if step_type.contains("collapse") {
                (0x05, wire::wire_ops::COLLAPSE)
            } else {
                (0x05, wire::wire_ops::EXECUTE)
            }
        }
        Some("n8n") => (0x0F, wire::wire_ops::EXECUTE),
        _ => (0x0F, wire::wire_ops::EXECUTE),
    }
}

/// Hash JSON value to u64 for Container seeding.
fn hash_json_to_u64(value: &serde_json::Value) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let s = serde_json::to_string(value).unwrap_or_default();
    s.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_crew_step() {
        let request = StepDelegationRequest {
            step: UnifiedStep {
                step_id: "test-1".into(),
                execution_id: "exec-1".into(),
                step_type: "crew.agent".into(),
                name: "Research".into(),
                status: StepStatus::Pending,
                sequence: 0,
                input: serde_json::Value::Null,
                output: serde_json::Value::Null,
                error: None,
                started_at: None,
                finished_at: None,
                reasoning: None,
                confidence: Some(0.9),
                alternatives: None,
            },
            input: DataEnvelope {
                data: serde_json::json!({"query": "test"}),
                metadata: EnvelopeMetadata {
                    source_step: "trigger".into(),
                    confidence: 0.9,
                    epoch: 42,
                    version: None,
                    dominant_layer: Some(5),
                    layer_activations: None,
                    nars_frequency: None,
                    calibration_error: None,
                },
            },
        };

        let pkt = ingest(&request);
        assert!(pkt.verify_magic());
        assert_eq!(pkt.opcode(), wire::wire_ops::DELEGATE);
        assert_eq!(pkt.source_prefix(), 0x0C);
        assert!(pkt.is_delegation());
    }

    #[test]
    fn test_pack_agent_result() {
        let style = [0.9, 0.2, 0.8, 0.5, 0.7, 0.95, 0.6, 0.85, 0.9, 0.75];
        let pkt = pack_agent_result("analyst-01", "Research complete", 0.92, Some(&style));

        assert!(pkt.is_response());
        assert_eq!(pkt.layer(), 4); // L5 Execution
        assert!((pkt.resonance_threshold() - 0.9).abs() < 0.01);
        assert!((pkt.exploration() - 0.2).abs() < 0.01);
        assert_eq!(pkt.fan_out(), 14); // 0.7 * 20 = 14
    }

    #[test]
    fn test_emit_response() {
        let content = Container::random(42);
        let mut response = CogPacket::response(wire::wire_ops::EXECUTE, 0x8001, 0x0C00, content);
        response.set_layer(4);
        response.set_truth_value(&TruthValue::new(0.85, 0.92));
        response.set_flags(response.flags() | wire::FLAG_VALIDATED);
        response.update_checksum();

        let step = UnifiedStep {
            step_id: "test-1".into(),
            execution_id: "exec-1".into(),
            step_type: "crew.agent".into(),
            name: "Research".into(),
            status: StepStatus::Running,
            sequence: 0,
            input: serde_json::Value::Null,
            output: serde_json::Value::Null,
            error: None,
            started_at: None,
            finished_at: None,
            reasoning: None,
            confidence: None,
            alternatives: None,
        };

        let delegation_response = emit(&response, &step);
        assert_eq!(delegation_response.step.unwrap().status, StepStatus::Completed);
        assert!(delegation_response.output.metadata.confidence > 0.9);
        assert_eq!(delegation_response.output.metadata.dominant_layer, Some(4));
    }
}
