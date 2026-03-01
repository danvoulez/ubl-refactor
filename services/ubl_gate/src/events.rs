//! Event streaming and search handlers, plus hub event helpers.

use async_stream::stream;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{sse::{Event as SseEvent, KeepAlive, Sse}, IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::time::Duration;
use ubl_eventstore::EventQuery;
use ubl_runtime::event_bus::ReceiptEvent;

use crate::metrics;
use crate::state::AppState;
use crate::utils::parse_when_to_ms;

#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct EventStreamQuery {
    pub(crate) world: Option<String>,
    pub(crate) stage: Option<String>,
    pub(crate) decision: Option<String>,
    pub(crate) code: Option<String>,
    #[serde(rename = "type")]
    pub(crate) chip_type: Option<String>,
    pub(crate) actor: Option<String>,
    pub(crate) since: Option<String>,
    pub(crate) limit: Option<usize>,
}

pub(crate) async fn stream_events(
    State(state): State<AppState>,
    Query(query): Query<EventStreamQuery>,
) -> Response {
    let Some(store) = state.event_store.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "@type": "ubl/error",
                "code": "UNAVAILABLE",
                "message": "Event hub unavailable: enable EventStore",
            })),
        )
            .into_response();
    };

    let world_label = query.world.clone().unwrap_or_else(|| "*".to_string());
    let db_query = EventQuery {
        world: query.world.clone(),
        stage: query.stage.clone(),
        decision: query.decision.clone(),
        code: query.code.clone(),
        chip_type: query.chip_type.clone(),
        actor: query.actor.clone(),
        since: query.since.clone(),
        limit: query.limit,
    };

    let historical = match store.query(&db_query) {
        Ok(events) => events,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "@type": "ubl/error",
                    "code": "INTERNAL_ERROR",
                    "message": format!("event query failed: {}", e),
                })),
            )
                .into_response();
        }
    };

    struct StreamClientGuard {
        world: String,
    }
    impl Drop for StreamClientGuard {
        fn drop(&mut self) {
            metrics::dec_events_stream_clients(&self.world);
        }
    }

    metrics::inc_events_stream_clients(&world_label);
    let mut rx = state.pipeline.event_bus.subscribe();
    let stream_world = world_label.clone();
    let live_filters = query.clone();
    let sse_stream = stream! {
        let _guard = StreamClientGuard { world: stream_world };

        for event in historical {
            let payload = match serde_json::to_string(&event) {
                Ok(p) => p,
                Err(_) => {
                    metrics::inc_events_stream_dropped("serialize_error");
                    continue;
                }
            };
            let id = event.get("@id").and_then(|v| v.as_str()).unwrap_or("evt");
            yield Ok::<SseEvent, Infallible>(SseEvent::default().id(id).event("ubl.event").data(payload));
        }

        loop {
            match rx.recv().await {
                Ok(receipt_event) => {
                    let hub = to_hub_event(&receipt_event);
                    if !hub_matches_query(&hub, &live_filters) {
                        continue;
                    }
                    let payload = match serde_json::to_string(&hub) {
                        Ok(p) => p,
                        Err(_) => {
                            metrics::inc_events_stream_dropped("serialize_error");
                            continue;
                        }
                    };
                    let id = hub.get("@id").and_then(|v| v.as_str()).unwrap_or("evt");
                    yield Ok::<SseEvent, Infallible>(SseEvent::default().id(id).event("ubl.event").data(payload));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    metrics::inc_events_stream_dropped("client_lagged");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(sse_stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(10))
                .text("heartbeat"),
        )
        .into_response()
}

#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct EventSearchQuery {
    pub(crate) world: Option<String>,
    pub(crate) stage: Option<String>,
    pub(crate) decision: Option<String>,
    pub(crate) code: Option<String>,
    #[serde(rename = "type")]
    pub(crate) chip_type: Option<String>,
    pub(crate) actor: Option<String>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    pub(crate) page_key: Option<String>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct AdvisorQuery {
    pub(crate) world: Option<String>,
    pub(crate) window: Option<String>,
    pub(crate) interval_ms: Option<u64>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct Mock24hQuery {
    pub(crate) world: Option<String>,
    pub(crate) profile: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct LlmPanelQuery {
    pub(crate) page: Option<String>,
    pub(crate) tab: Option<String>,
    pub(crate) world: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) profile: Option<String>,
    pub(crate) cid: Option<String>,
    #[serde(rename = "type")]
    pub(crate) chip_type: Option<String>,
}

pub(crate) async fn search_events(
    State(state): State<AppState>,
    Query(query): Query<EventSearchQuery>,
) -> Response {
    let Some(store) = state.event_store.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "@type": "ubl/error",
                "code": "UNAVAILABLE",
                "message": "Event hub unavailable: enable EventStore",
            })),
        )
            .into_response();
    };

    let since = query
        .page_key
        .clone()
        .or_else(|| query.from.clone())
        .or_else(|| Some("0".to_string()));

    let db_query = EventQuery {
        world: query.world.clone(),
        stage: query.stage.clone(),
        decision: query.decision.clone(),
        code: query.code.clone(),
        chip_type: query.chip_type.clone(),
        actor: query.actor.clone(),
        since,
        limit: query.limit,
    };

    let mut events = match store.query(&db_query) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "@type": "ubl/error",
                    "code": "INTERNAL_ERROR",
                    "message": format!("event search failed: {}", e),
                })),
            )
                .into_response();
        }
    };

    if let Some(to) = query.to.as_deref().and_then(parse_when_to_ms) {
        events.retain(|e| {
            let when = e
                .get("when")
                .and_then(|v| v.as_str())
                .or_else(|| e.get("timestamp").and_then(|v| v.as_str()));
            when.and_then(parse_when_to_ms).is_some_and(|ms| ms <= to)
        });
    }

    let next_page_key = events
        .last()
        .and_then(|e| {
            e.get("when")
                .and_then(|v| v.as_str())
                .or_else(|| e.get("timestamp").and_then(|v| v.as_str()))
        })
        .map(ToString::to_string);

    (
        StatusCode::OK,
        Json(json!({
            "@type": "ubl/events.search.response",
            "count": events.len(),
            "next_page_key": next_page_key,
            "events": events,
        })),
    )
        .into_response()
}

// ── Hub event helpers ─────────────────────────────────────────────────────────

pub(crate) fn to_hub_event(event: &ReceiptEvent) -> Value {
    let stage = normalize_stage(&event.pipeline_stage);
    let event_id = deterministic_event_id(event, &stage);
    let world = event
        .world
        .clone()
        .unwrap_or_else(|| "a/system".to_string());
    let chip_id = event
        .metadata
        .get("@id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let chip_ver = event
        .metadata
        .get("@ver")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let code = event
        .metadata
        .get("code")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    let cap = event
        .metadata
        .get("cap")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let decision = event.decision.as_ref().map(|d| d.to_ascii_uppercase());
    let mut receipt = json!({
        "cid": event.receipt_cid.clone(),
        "decision": decision,
        "code": code,
        "knock_cid": event.knock_cid.clone(),
    });
    if let Some(obj) = receipt.as_object_mut() {
        if obj.get("code").is_some_and(Value::is_null) {
            obj.remove("code");
        }
        if obj.get("knock_cid").is_some_and(Value::is_null) {
            obj.remove("knock_cid");
        }
    }

    let mut actor = json!({
        "kid": event.actor.clone(),
        "did": event.subject_did.clone(),
        "cap": cap,
    });
    if let Some(obj) = actor.as_object_mut() {
        if obj.get("did").is_some_and(Value::is_null) {
            obj.remove("did");
        }
        if obj.get("cap").is_some_and(Value::is_null) {
            obj.remove("cap");
        }
    }

    json!({
        "@type": "ubl/event",
        "@ver": "1.0.0",
        "@id": event_id,
        "@world": world,
        "source": "pipeline",
        "stage": stage,
        "when": event.timestamp.clone(),
        "chip": {
            "type": event.receipt_type.clone(),
            "id": chip_id,
            "ver": chip_ver,
        },
        "receipt": receipt,
        "perf": {
            "latency_ms": event.latency_ms.or(event.duration_ms),
            "fuel": event.fuel_used,
            "mem_kb": Value::Null,
        },
        "actor": actor,
        "artifacts": event.artifact_cids.clone(),
        "runtime": {
            "binary_hash": event.binary_hash.clone(),
            "build": event.build_meta.clone(),
        },
        "labels": Value::Object(Default::default()),
    })
}

pub(crate) fn normalize_stage(stage: &str) -> String {
    match stage.to_ascii_lowercase().as_str() {
        "knock" => "KNOCK".to_string(),
        "wa" | "write_ahead" => "WA".to_string(),
        "check" => "CHECK".to_string(),
        "tr" | "transition" => "TR".to_string(),
        "wf" | "write_finished" => "WF".to_string(),
        "registry" => "REGISTRY".to_string(),
        other => other.to_ascii_uppercase(),
    }
}

pub(crate) fn deterministic_event_id(event: &ReceiptEvent, stage: &str) -> String {
    format!(
        "evt:{}:{}:{}:{}",
        event.receipt_cid,
        stage,
        event.input_cid.as_deref().unwrap_or(""),
        event.output_cid.as_deref().unwrap_or("")
    )
}

pub(crate) fn hub_matches_query(event: &Value, query: &EventStreamQuery) -> bool {
    if let Some(world) = &query.world {
        if event.get("@world").and_then(|v| v.as_str()) != Some(world.as_str()) {
            return false;
        }
    }
    if let Some(stage) = &query.stage {
        let actual = event.get("stage").and_then(|v| v.as_str()).unwrap_or("");
        if actual != stage && !actual.eq_ignore_ascii_case(stage) {
            return false;
        }
    }
    if let Some(decision) = &query.decision {
        let actual = event
            .get("receipt")
            .and_then(|v| v.get("decision"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if actual != decision && !actual.eq_ignore_ascii_case(decision) {
            return false;
        }
    }
    if let Some(code) = &query.code {
        if event
            .get("receipt")
            .and_then(|v| v.get("code"))
            .and_then(|v| v.as_str())
            != Some(code.as_str())
        {
            return false;
        }
    }
    if let Some(chip_type) = &query.chip_type {
        let actual = event
            .get("chip")
            .and_then(|v| v.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if chip_type != "*" && actual != chip_type {
            return false;
        }
    }
    if let Some(actor) = &query.actor {
        if event
            .get("actor")
            .and_then(|v| v.get("kid"))
            .and_then(|v| v.as_str())
            != Some(actor.as_str())
        {
            return false;
        }
    }
    true
}
