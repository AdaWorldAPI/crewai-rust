# Proxy Contract: Auth Layer → crewai-stack

The auth layer (ada-streamlit-auth or any FastAPI/gRPC proxy) handles OAuth,
sessions, and rate limiting.  It proxies to the crewai-stack Rust binary for
all barrier and compute operations.

## Architecture

```
Client (MCP/REST)
    ↓ OAuth token
Auth Layer (Python/FastAPI)          ← mcp.exo.red / ada-streamlit-auth
    ↓ strip auth, add X-Ada-Identity
crewai-stack (Rust)                  ← Railway internal URL, port 8080
    ↓ pure compute
Response
    ↑ add CORS, rate-limit headers
Auth Layer
    ↑
Client
```

## Endpoint Mapping

### Old Router → New Barrier Stack

| Old Endpoint | Method | New Endpoint | Notes |
|---|---|---|---|
| `/ada/invoke` (flesh/scent) | POST | `/barrier/check-outbound` | felt-sense → NARS truth gate |
| `/ada/invoke` (self/scent) | POST | `/barrier/topology` | self-awareness = triune state |
| `/ada/invoke` (memory/write) | POST | `/barrier/check-inbound` | memory commit = evidence gate |
| `/ada/invoke` (memory/bridge) | GET | `/barrier/stats` | bridge state = drift diagnostics |
| `/ada/invoke` (volition/scent) | POST | `/barrier/check-outbound` | volition = action gate |
| `/mcp/hydrate` | POST | `/barrier/check-inbound` | inbound evidence acceptance |
| `/mcp/dehydrate` | POST | `/barrier/check-outbound` | outbound action gate |
| `/mcp/feel` | GET | `/barrier/topology` | triune facet state |
| `/mcp/desire` | POST | `/barrier/feedback` | volition feedback |
| `/consciousness/full` | GET | `/barrier/stats` | full diagnostics |
| `/volition/choose` | POST | `/barrier/check-outbound` | decision gate |
| `/hot/reason` | POST | `/chat` | LLM reasoning (holy grail) |
| `/execute` | POST | `/execute` | crew.* step delegation |

### Proxy Headers

The auth layer should add these headers when proxying:

```
X-Ada-Identity: <user_id from OAuth>
X-Ada-Session: <session_id>
X-Ada-Timestamp: <unix_ms>
```

The Rust binary ignores these for now but they'll be used for per-user
barrier calibration (each user gets their own DK detector state).

## Request/Response Schemas

### POST /barrier/check-outbound

```json
// Request
{
  "action": "query_llm",
  "nars_frequency": 0.8,
  "nars_confidence": 0.7,
  // Optional MUL state (defaults to permissive if omitted)
  "mul_gate_open": true,
  "mul_free_will": 0.85,
  "mul_dk_position": "slope_of_enlightenment",
  "mul_trust_level": "solid",
  "risk_epistemic": 0.3,
  "risk_moral": 0.1
}

// Response
{
  "proceed": true,
  "effective_confidence": 0.68,
  "is_clean": true,
  "is_nudge": false,
  "is_blocked": false,
  "blocking_layers": [],
  "nudge": null,
  "verdicts": {
    "nars": [0.8, 0.7],
    "markov": null,
    "triune": "Flow",
    "mul": {
      "gate_open": true,
      "free_will_modifier": 0.85,
      "dk_position": "slope_of_enlightenment",
      "trust_level": "solid",
      "allostatic_load": 0.0
    }
  }
}
```

### POST /barrier/check-inbound

```json
// Request
{
  "evidence_frequency": 0.9,
  "evidence_confidence": 0.8,
  "markov_gate": "commit",
  "mul_gate_open": true,
  "mul_free_will": 0.9
}

// Response
{
  "proceed": true,
  "effective_confidence": 0.72,
  "blocking_layers": []
}
```

### GET /barrier/topology

```json
// Response
{
  "guardian": { "intensity": 0.5, "leading": false },
  "driver": { "intensity": 0.6, "leading": true },
  "catalyst": { "intensity": 0.4, "leading": false },
  "is_fused": false,
  "balance_score": 0.87,
  "strategy": "Execution",
  "leader": "Driver"
}
```

### POST /barrier/feedback

```json
// Request
{
  "facet": "driver",
  "success": true
}

// Response
{
  "status": "ok",
  "facet": "driver",
  "success": true,
  "new_topology": {
    "guardian": 0.5,
    "driver": 0.7,
    "catalyst": 0.4,
    "leader": "Driver"
  }
}
```

### GET /barrier/stats

```json
// Response
{
  "markov": {
    "total_transactions": 42,
    "commits": 38,
    "dampens": 3,
    "rejects": 1,
    "cumulative_drift": 847,
    "drift_ceiling": 1638,
    "needs_consolidation": false
  },
  "triune": {
    "leader": "Driver",
    "strategy": "Execution",
    "balance": 0.87
  },
  "gates": {
    "guardian_min_conf": 0.8,
    "driver_min_conf": 0.65,
    "catalyst_min_conf": 0.5
  }
}
```

## MCP Tool Definitions (for ada-streamlit-auth)

If the auth layer exposes MCP tools, map them like this:

```json
{
  "tools": [
    {
      "name": "barrier_check",
      "description": "Check if an action can cross the blood-brain barrier (4-layer gate: NARS + Markov + Triune + MUL)",
      "inputSchema": {
        "type": "object",
        "properties": {
          "action": { "type": "string", "description": "The action being attempted" },
          "confidence": { "type": "number", "description": "NARS confidence (0.0-1.0)" },
          "frequency": { "type": "number", "description": "NARS frequency (0.0-1.0)" }
        },
        "required": ["action", "confidence"]
      }
    },
    {
      "name": "barrier_topology",
      "description": "Get current triune facet state (Guardian/Driver/Catalyst intensities and leadership)",
      "inputSchema": { "type": "object", "properties": {} }
    },
    {
      "name": "barrier_feedback",
      "description": "Report success/failure to update triune facet learning",
      "inputSchema": {
        "type": "object",
        "properties": {
          "facet": { "type": "string", "enum": ["guardian", "driver", "catalyst"] },
          "success": { "type": "boolean" }
        },
        "required": ["facet", "success"]
      }
    },
    {
      "name": "barrier_stats",
      "description": "Get barrier diagnostics: Markov drift, triune balance, gate thresholds",
      "inputSchema": { "type": "object", "properties": {} }
    }
  ]
}
```

## gRPC Alternative

If you want gRPC instead of REST proxy, the Rust binary already has `tonic`
as an optional dependency (xai-grpc feature). The proto definitions would be:

```protobuf
service BarrierService {
  rpc CheckOutbound(OutboundRequest) returns (StackDecision);
  rpc CheckInbound(InboundRequest) returns (StackDecision);
  rpc GetTopology(Empty) returns (TriuneTopology);
  rpc Feedback(FeedbackRequest) returns (TopologyUpdate);
  rpc GetStats(Empty) returns (BarrierStats);
}
```

Enable with: `--features xai-grpc` (reuses the existing tonic setup).
