//! Audit page handlers and query helpers.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::console::{render_html, split_rows};
use crate::state::AppState;
use crate::templates::{AuditRow, AuditTableTemplate, AuditTemplate, ConsoleReceiptTemplate};

pub(crate) async fn console_receipt_page(Path(cid): Path<String>) -> Response {
    render_html(&ConsoleReceiptTemplate { cid })
}

pub(crate) async fn audit_page(
    Path(kind): Path<String>,
    Query(query): Query<std::collections::BTreeMap<String, String>>,
) -> Response {
    let kind = normalize_audit_kind(&kind);
    let world = query
        .get("world")
        .cloned()
        .unwrap_or_else(|| "*".to_string());
    render_html(&AuditTemplate { world, kind })
}

pub(crate) async fn audit_table_partial(
    State(state): State<AppState>,
    Query(query): Query<std::collections::BTreeMap<String, String>>,
) -> Response {
    let world = query
        .get("world")
        .map(|w| w.as_str())
        .filter(|w| !w.trim().is_empty() && *w != "*");
    let kind = normalize_audit_kind(query.get("kind").map(String::as_str).unwrap_or("reports"));
    let rows = match query_audit_rows(&state, &kind, world, 100).await {
        Ok(rows) => rows,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "@type":"ubl/error",
                    "code":"INTERNAL_ERROR",
                    "message": format!("audit table query failed: {}", e),
                })),
            )
                .into_response();
        }
    };
    let (visible_rows, hidden_rows) = split_rows(rows, 6);
    render_html(&AuditTableTemplate {
        kind,
        visible_rows,
        hidden_rows,
    })
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuditListQuery {
    pub(crate) world: Option<String>,
    pub(crate) limit: Option<usize>,
}

pub(crate) async fn list_audit_reports(
    State(state): State<AppState>,
    Query(query): Query<AuditListQuery>,
) -> (StatusCode, Json<Value>) {
    list_audit_kind_json(state, "reports", query).await
}

pub(crate) async fn list_audit_snapshots(
    State(state): State<AppState>,
    Query(query): Query<AuditListQuery>,
) -> (StatusCode, Json<Value>) {
    list_audit_kind_json(state, "snapshots", query).await
}

pub(crate) async fn list_audit_compactions(
    State(state): State<AppState>,
    Query(query): Query<AuditListQuery>,
) -> (StatusCode, Json<Value>) {
    list_audit_kind_json(state, "compactions", query).await
}

async fn list_audit_kind_json(
    state: AppState,
    kind: &str,
    query: AuditListQuery,
) -> (StatusCode, Json<Value>) {
    let world = query
        .world
        .as_deref()
        .filter(|w| !w.trim().is_empty() && *w != "*");
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    match query_audit_rows(&state, kind, world, limit).await {
        Ok(rows) => (
            StatusCode::OK,
            Json(json!({
                "@type": "ubl/audit.list",
                "kind": normalize_audit_kind(kind),
                "count": rows.len(),
                "rows": rows.iter().map(|r| json!({
                    "cid": r.cid,
                    "chip_type": r.chip_type,
                    "world": r.world,
                    "created_at": r.created_at,
                    "summary": r.summary,
                })).collect::<Vec<_>>()
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "@type": "ubl/error",
                "code": "INTERNAL_ERROR",
                "message": format!("audit list failed: {}", e),
            })),
        ),
    }
}

pub(crate) fn normalize_audit_kind(kind: &str) -> String {
    match kind {
        "reports" => "reports".to_string(),
        "snapshots" => "snapshots".to_string(),
        "compactions" => "compactions".to_string(),
        _ => "reports".to_string(),
    }
}

fn audit_chip_type_for_kind(kind: &str) -> &'static str {
    match kind {
        "reports" => "ubl/audit.dataset.v1",
        "snapshots" => "ubl/audit.snapshot.manifest.v1",
        "compactions" => "ubl/ledger.compaction.rollup.v1",
        _ => "ubl/audit.dataset.v1",
    }
}

pub(crate) async fn query_audit_rows(
    state: &AppState,
    kind: &str,
    world: Option<&str>,
    limit: usize,
) -> Result<Vec<AuditRow>, String> {
    let chip_type = audit_chip_type_for_kind(kind);
    let mut tags = Vec::new();
    if let Some(world) = world {
        tags.push(format!("world:{}", world));
    }
    let result = state
        .chip_store
        .query(&ubl_chipstore::ChipQuery {
            chip_type: Some(chip_type.to_string()),
            tags,
            created_after: None,
            created_before: None,
            executor_did: None,
            limit: Some(limit),
            offset: None,
        })
        .await
        .map_err(|e| e.to_string())?;

    let rows = result
        .chips
        .into_iter()
        .map(|chip| AuditRow {
            cid: chip.cid.as_str().to_string(),
            chip_type: chip.chip_type.clone(),
            world: chip
                .chip_data
                .get("@world")
                .and_then(|v| v.as_str())
                .unwrap_or("-")
                .to_string(),
            created_at: chip.created_at.clone(),
            summary: audit_summary(&chip.chip_data, kind),
        })
        .collect();
    Ok(rows)
}

pub(crate) fn audit_summary(chip_data: &Value, kind: &str) -> String {
    match kind {
        "reports" => format!(
            "lines={} format={}",
            chip_data
                .get("line_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            chip_data
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("ndjson")
        ),
        "snapshots" => format!(
            "segments={} dataset={}",
            chip_data
                .get("coverage")
                .and_then(|c| c.get("segments"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            chip_data
                .get("artifacts")
                .and_then(|a| a.get("dataset"))
                .and_then(|v| v.as_str())
                .unwrap_or("-")
        ),
        "compactions" => format!(
            "freed_bytes={} mode={}",
            chip_data
                .get("freed_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            chip_data
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("-")
        ),
        _ => "-".to_string(),
    }
}
