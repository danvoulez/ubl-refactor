//! Advisor snapshot and tap handlers.

use async_stream::stream;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{sse::{Event as SseEvent, KeepAlive, Sse}, IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::time::Duration;
use ubl_eventstore::{EventQuery, EventStore};

use crate::events::AdvisorQuery;
use crate::metrics;
use crate::state::AppState;
use crate::utils::parse_window_duration;

pub(crate) async fn advisor_snapshots(
    State(state): State<AppState>,
    Query(query): Query<AdvisorQuery>,
) -> Response {
    let Some(store) = state.event_store.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "@type": "ubl/error",
                "code": "UNAVAILABLE",
                "message": "Advisor snapshot unavailable: enable EventStore",
            })),
        )
            .into_response();
    };

    let window = parse_window_duration(query.window.as_deref()).unwrap_or(Duration::from_secs(300));
    let limit = query.limit.unwrap_or(10_000).clamp(100, 50_000);
    match build_advisor_snapshot(&state, store, query.world.as_deref(), window, limit) {
        Ok(frame) => (
            StatusCode::OK,
            Json(json!({
                "@type": "ubl/advisor.snapshot",
                "window_ms": window.as_millis() as u64,
                "snapshot": frame,
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "@type": "ubl/error",
                "code": "INTERNAL_ERROR",
                "message": format!("advisor snapshot failed: {}", e),
            })),
        )
            .into_response(),
    }
}

pub(crate) async fn advisor_tap(
    State(state): State<AppState>,
    Query(query): Query<AdvisorQuery>,
) -> Response {
    let Some(store) = state.event_store.clone() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "@type": "ubl/error",
                "code": "UNAVAILABLE",
                "message": "Advisor tap unavailable: enable EventStore",
            })),
        )
            .into_response();
    };

    let window = parse_window_duration(query.window.as_deref()).unwrap_or(Duration::from_secs(300));
    let interval = Duration::from_millis(query.interval_ms.unwrap_or(2_000).clamp(1_000, 5_000));
    let limit = query.limit.unwrap_or(10_000).clamp(100, 50_000);
    let world_filter = query.world.clone();
    let state_for_stream = state.clone();

    let sse_stream = stream! {
        loop {
            match build_advisor_snapshot(&state_for_stream, &store, world_filter.as_deref(), window, limit) {
                Ok(frame) => {
                    let payload = match serde_json::to_string(&frame) {
                        Ok(v) => v,
                        Err(_) => {
                            metrics::inc_events_stream_dropped("advisor_tap_serialize_error");
                            tokio::time::sleep(interval).await;
                            continue;
                        }
                    };
                    let id = frame.get("@id").and_then(|v| v.as_str()).unwrap_or("adv");
                    yield Ok::<SseEvent, Infallible>(SseEvent::default().id(id).event("ubl.advisor.frame").data(payload));
                }
                Err(_) => {
                    metrics::inc_events_stream_dropped("advisor_tap_query_error");
                }
            }
            tokio::time::sleep(interval).await;
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

pub(crate) fn build_advisor_snapshot(
    state: &AppState,
    store: &EventStore,
    world: Option<&str>,
    window: Duration,
    limit: usize,
) -> Result<Value, String> {
    let now = chrono::Utc::now();
    let since = now
        .checked_sub_signed(chrono::Duration::from_std(window).map_err(|e| e.to_string())?)
        .ok_or_else(|| "window underflow".to_string())?;

    let query = EventQuery {
        world: world.map(ToString::to_string),
        since: Some(since.timestamp_millis().to_string()),
        limit: Some(limit),
        ..Default::default()
    };
    let events = store.query(&query).map_err(|e| e.to_string())?;

    let mut by_stage = std::collections::BTreeMap::<String, u64>::new();
    let mut by_decision = std::collections::BTreeMap::<String, u64>::new();
    let mut by_code = std::collections::BTreeMap::<String, u64>::new();
    let mut lat_stage = std::collections::BTreeMap::<String, Vec<f64>>::new();
    let mut outliers: Vec<(f64, Value)> = Vec::new();

    for event in &events {
        if let Some(stage) = event.get("stage").and_then(|v| v.as_str()) {
            *by_stage.entry(stage.to_string()).or_default() += 1;
        }
        if let Some(decision) = event
            .get("receipt")
            .and_then(|v| v.get("decision"))
            .and_then(|v| v.as_str())
        {
            *by_decision.entry(decision.to_string()).or_default() += 1;
        }
        if let Some(code) = event
            .get("receipt")
            .and_then(|v| v.get("code"))
            .and_then(|v| v.as_str())
        {
            *by_code.entry(code.to_string()).or_default() += 1;
        }
        if let Some(lat) = event
            .get("perf")
            .and_then(|v| v.get("latency_ms"))
            .and_then(|v| v.as_f64())
        {
            let stage = event
                .get("stage")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN")
                .to_string();
            lat_stage.entry(stage).or_default().push(lat);
            outliers.push((
                lat,
                json!({
                    "receipt_cid": event.get("receipt").and_then(|v| v.get("cid")).cloned().unwrap_or(Value::Null),
                    "stage": event.get("stage").cloned().unwrap_or(Value::Null),
                    "chip_type": event.get("chip").and_then(|v| v.get("type")).cloned().unwrap_or(Value::Null),
                    "latency_ms": lat,
                }),
            ));
        }
    }

    let mut p95_by_stage = serde_json::Map::new();
    for (stage, mut vals) in lat_stage {
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((vals.len() - 1) as f64 * 0.95).round() as usize;
        p95_by_stage.insert(stage, json!(vals[idx]));
    }

    outliers.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let top_outliers: Vec<Value> = outliers.into_iter().take(5).map(|(_, v)| v).collect();

    let samples: Vec<Value> = events
        .iter()
        .rev()
        .take(5)
        .map(|event| {
            json!({
                "event_id": event.get("@id").cloned().unwrap_or(Value::Null),
                "when": event.get("when").cloned().unwrap_or(Value::Null),
                "stage": event.get("stage").cloned().unwrap_or(Value::Null),
                "chip_type": event.get("chip").and_then(|v| v.get("type")).cloned().unwrap_or(Value::Null),
                "receipt_cid": event.get("receipt").and_then(|v| v.get("cid")).cloned().unwrap_or(Value::Null),
                "decision": event.get("receipt").and_then(|v| v.get("decision")).cloned().unwrap_or(Value::Null),
                "code": event.get("receipt").and_then(|v| v.get("code")).cloned().unwrap_or(Value::Null),
            })
        })
        .collect();

    let outbox_pending = state
        .durable_store
        .as_ref()
        .and_then(|store| store.outbox_pending().ok());

    Ok(json!({
        "@type": "ubl/advisor.tap.frame",
        "@ver": "1.0.0",
        "@id": format!("adv-{}", now.timestamp_millis()),
        "@world": world.unwrap_or("*"),
        "generated_at": now.to_rfc3339(),
        "window_ms": window.as_millis() as u64,
        "counts": {
            "stage": by_stage,
            "decision": by_decision,
            "code": by_code,
        },
        "latency_ms_p95_by_stage": Value::Object(p95_by_stage),
        "top_outliers": top_outliers,
        "samples": samples,
        "outbox": {
            "pending": outbox_pending,
            "retries": Value::Null,
        },
    }))
}
