//! Console UI handlers, mock-24h data generator, and template helpers.

use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::time::Duration;
use ubl_eventstore::EventQuery;

use crate::advisor::build_advisor_snapshot;
use crate::events::Mock24hQuery;
use crate::state::AppState;
use crate::templates::{
    ConsoleMock24hTemplate, ConsoleEventRow, ConsoleEventsTemplate, ConsoleKpisTemplate,
    ConsoleTemplate, MockHourRow, StageP95Row,
};

// ── Template helpers ──────────────────────────────────────────────────────────

pub(crate) fn render_html<T: Template>(template: &T) -> Response {
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "@type":"ubl/error",
                "code":"INTERNAL_ERROR",
                "message": format!("template render failed: {}", e),
            })),
        )
            .into_response(),
    }
}

pub(crate) fn split_rows<T: Clone>(rows: Vec<T>, keep: usize) -> (Vec<T>, Vec<T>) {
    let split = rows.len().min(keep);
    let (visible, hidden) = rows.split_at(split);
    (visible.to_vec(), hidden.to_vec())
}

pub(crate) fn normalize_console_tab(raw: &str) -> String {
    match raw.to_ascii_lowercase().as_str() {
        "stats" | "stat" | "estatisticas" => "stats".to_string(),
        _ => "live".to_string(),
    }
}

pub(crate) fn normalize_mock_profile(raw: &str) -> String {
    match raw.to_ascii_lowercase().as_str() {
        "spiky" => "spiky".to_string(),
        "degraded" => "degraded".to_string(),
        "chaos" => "chaos".to_string(),
        _ => "normal".to_string(),
    }
}

// ── Console page handlers ─────────────────────────────────────────────────────

pub(crate) async fn console_page(
    Query(query): Query<std::collections::BTreeMap<String, String>>,
) -> Response {
    let world = query
        .get("world")
        .cloned()
        .unwrap_or_else(|| "*".to_string());
    let tab = query
        .get("tab")
        .map(|v| normalize_console_tab(v))
        .unwrap_or_else(|| "live".to_string());
    let profile =
        normalize_mock_profile(query.get("profile").map(String::as_str).unwrap_or("normal"));
    let is_stats = tab == "stats";
    render_html(&ConsoleTemplate {
        world,
        tab,
        is_stats,
        profile,
    })
}

pub(crate) async fn console_kpis_partial(
    State(state): State<AppState>,
    Query(query): Query<std::collections::BTreeMap<String, String>>,
) -> Response {
    let world = query
        .get("world")
        .map(|w| w.as_str())
        .filter(|w| !w.trim().is_empty() && *w != "*");
    let Some(store) = state.event_store.as_ref() else {
        return render_html(&ConsoleKpisTemplate {
            available: false,
            message: "EventStore unavailable".to_string(),
            generated_at: "-".to_string(),
            total_events: 0,
            allow_count: 0,
            deny_count: 0,
            outbox_pending: "-".to_string(),
            visible_p95_rows: Vec::new(),
            hidden_p95_rows: Vec::new(),
        });
    };
    let snapshot =
        match build_advisor_snapshot(&state, store, world, Duration::from_secs(300), 5000) {
            Ok(s) => s,
            Err(e) => {
                return render_html(&ConsoleKpisTemplate {
                    available: false,
                    message: format!("Snapshot error: {}", e),
                    generated_at: "-".to_string(),
                    total_events: 0,
                    allow_count: 0,
                    deny_count: 0,
                    outbox_pending: "-".to_string(),
                    visible_p95_rows: Vec::new(),
                    hidden_p95_rows: Vec::new(),
                });
            }
        };

    let mut total_events = 0u64;
    if let Some(map) = snapshot
        .get("counts")
        .and_then(|c| c.get("stage"))
        .and_then(|v| v.as_object())
    {
        for value in map.values() {
            total_events = total_events.saturating_add(value.as_u64().unwrap_or(0));
        }
    }

    let allow_count = snapshot
        .get("counts")
        .and_then(|c| c.get("decision"))
        .and_then(|d| d.get("ALLOW"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let deny_count = snapshot
        .get("counts")
        .and_then(|c| c.get("decision"))
        .and_then(|d| d.get("DENY"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let outbox_pending = snapshot
        .get("outbox")
        .and_then(|o| o.get("pending"))
        .and_then(|v| v.as_i64())
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());

    let generated_at = snapshot
        .get("generated_at")
        .and_then(|v| v.as_str())
        .unwrap_or("-")
        .to_string();

    let mut p95_rows = Vec::new();
    if let Some(map) = snapshot
        .get("latency_ms_p95_by_stage")
        .and_then(|v| v.as_object())
    {
        for (stage, value) in map {
            let p95_ms = value
                .as_f64()
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "-".to_string());
            p95_rows.push(StageP95Row {
                stage: stage.clone(),
                p95_ms,
            });
        }
    }
    p95_rows.sort_by(|a, b| a.stage.cmp(&b.stage));
    let (visible_p95_rows, hidden_p95_rows) = split_rows(p95_rows, 6);

    render_html(&ConsoleKpisTemplate {
        available: true,
        message: String::new(),
        generated_at,
        total_events,
        allow_count,
        deny_count,
        outbox_pending,
        visible_p95_rows,
        hidden_p95_rows,
    })
}

pub(crate) async fn console_events_partial(
    State(state): State<AppState>,
    Query(query): Query<std::collections::BTreeMap<String, String>>,
) -> Response {
    let world = query
        .get("world")
        .map(|w| w.as_str())
        .filter(|w| !w.trim().is_empty() && *w != "*");
    let Some(store) = state.event_store.as_ref() else {
        return render_html(&crate::templates::ConsoleEventsTemplate {
            available: false,
            message: "EventStore unavailable".to_string(),
            visible_rows: Vec::new(),
            hidden_rows: Vec::new(),
        });
    };
    let events = match store.query(&EventQuery {
        world: world.map(ToString::to_string),
        limit: Some(20),
        ..Default::default()
    }) {
        Ok(v) => v,
        Err(e) => {
            return render_html(&ConsoleEventsTemplate {
                available: false,
                message: format!("Events query error: {}", e),
                visible_rows: Vec::new(),
                hidden_rows: Vec::new(),
            });
        }
    };

    let rows: Vec<ConsoleEventRow> = events
        .iter()
        .rev()
        .map(|event| ConsoleEventRow {
            when: event
                .get("when")
                .and_then(|v| v.as_str())
                .unwrap_or("-")
                .to_string(),
            stage: event
                .get("stage")
                .and_then(|v| v.as_str())
                .unwrap_or("-")
                .to_string(),
            decision: event
                .get("receipt")
                .and_then(|v| v.get("decision"))
                .and_then(|v| v.as_str())
                .unwrap_or("-")
                .to_string(),
            chip_type: event
                .get("chip")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("-")
                .to_string(),
            code: event
                .get("receipt")
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str())
                .unwrap_or("-")
                .to_string(),
            receipt_cid: event
                .get("receipt")
                .and_then(|v| v.get("cid"))
                .and_then(|v| v.as_str())
                .unwrap_or("-")
                .to_string(),
        })
        .collect();

    let (visible_rows, hidden_rows) = split_rows(rows, 6);

    render_html(&ConsoleEventsTemplate {
        available: true,
        message: String::new(),
        visible_rows,
        hidden_rows,
    })
}

pub(crate) async fn console_mock24h_partial(
    Query(query): Query<std::collections::BTreeMap<String, String>>,
) -> Response {
    let world = query
        .get("world")
        .cloned()
        .unwrap_or_else(|| "*".to_string());
    let profile =
        normalize_mock_profile(query.get("profile").map(String::as_str).unwrap_or("normal"));
    let rows = build_mock_24h_rows(&profile, &world);
    let (visible_rows, hidden_rows) = split_rows(rows, 6);
    render_html(&ConsoleMock24hTemplate {
        profile,
        generated_at: chrono::Utc::now().to_rfc3339(),
        visible_rows,
        hidden_rows,
    })
}

pub(crate) async fn mock24h_api(
    Query(query): Query<Mock24hQuery>,
) -> (StatusCode, Json<Value>) {
    let world = query.world.unwrap_or_else(|| "*".to_string());
    let profile = normalize_mock_profile(query.profile.as_deref().unwrap_or("normal"));
    let rows = build_mock_24h_rows(&profile, &world);
    (
        StatusCode::OK,
        Json(json!({
            "@type": "ubl/mock.system24h",
            "world": world,
            "profile": profile,
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "rows": rows,
        })),
    )
}

// ── Mock 24h data generator ───────────────────────────────────────────────────

pub(crate) fn build_mock_24h_rows(profile: &str, world: &str) -> Vec<MockHourRow> {
    let profile = normalize_mock_profile(profile);
    let seed = stable_seed(&format!("{}|{}", profile, world));
    let now = chrono::Utc::now();
    let mut rows = Vec::with_capacity(24);

    for hour_back in 0..24u64 {
        let ts = now - chrono::Duration::hours(hour_back as i64);
        let slot = 23 - hour_back;
        let n1 = mix64(seed ^ slot.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let n2 = mix64(seed ^ slot.wrapping_mul(0xBF58_476D_1CE4_E5B9));
        let wave = ((slot as f64 / 24.0) * std::f64::consts::TAU).sin();

        let base_events = match profile.as_str() {
            "spiky" => 820i64,
            "degraded" => 700i64,
            "chaos" => 620i64,
            _ => 900i64,
        };
        let mut events = base_events + (wave * 140.0) as i64 + ((n1 % 220) as i64 - 110);
        let mut deny_pct = 1.6 + ((n2 % 60) as f64 / 20.0);
        let mut p95_ms = 38.0 + ((n1 % 65) as f64);
        let mut outbox_pending = (n2 % 18) as i64;
        let mut error_pct = 0.20 + ((n1 % 30) as f64 / 120.0);

        if profile == "spiky" && slot % 7 == 0 {
            events += 820;
            p95_ms += 130.0;
            deny_pct += 3.4;
            outbox_pending += 45;
            error_pct += 1.2;
        }
        if profile == "degraded" {
            events -= 130;
            p95_ms += 95.0;
            deny_pct += 4.8;
            outbox_pending += 38;
            error_pct += 1.4;
        }
        if profile == "chaos" {
            let flip = (n2 % 3) as i64 - 1;
            events += flip * 350;
            p95_ms += ((n1 % 180) as f64) * 0.9;
            deny_pct += ((n2 % 90) as f64) / 12.0;
            outbox_pending += (n1 % 90) as i64;
            error_pct += ((n2 % 70) as f64) / 20.0;
        }

        events = events.max(80);
        outbox_pending = outbox_pending.max(0);
        deny_pct = deny_pct.clamp(0.2, 48.0);
        error_pct = error_pct.clamp(0.05, 22.0);
        let allow_pct = (100.0 - deny_pct - (error_pct * 0.25)).clamp(40.0, 99.5);

        rows.push(MockHourRow {
            hour_label: ts.format("%m-%d %H:00").to_string(),
            events: events as u64,
            allow_pct: format!("{:.2}", allow_pct),
            deny_pct: format!("{:.2}", deny_pct),
            p95_ms: format!("{:.1}", p95_ms),
            outbox_pending: outbox_pending as u64,
            error_pct: format!("{:.2}", error_pct),
        });
    }

    rows
}

fn stable_seed(input: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

fn mix64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}
