//! Receipt retrieval, public URLs, narration, trace, passport advisory handlers.

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{
        sse::{Event as SseEvent, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use ubl_runtime::advisory::{Advisory, AdvisoryHook};

use crate::llm::{call_real_llm, call_real_llm_stream_sse, llm_is_enabled};
use crate::state::AppState;
use crate::utils::{build_public_receipt_link, verify_receipt_auth_chain};

pub(crate) async fn get_receipt(
    State(state): State<AppState>,
    Path(cid): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !cid.starts_with("b3:") {
        return (
            StatusCode::BAD_REQUEST,
            HeaderMap::new(),
            Json(json!({"@type": "ubl/error", "code": "INVALID_CID", "message": "CID must start with b3:"})),
        );
    }

    if let Some(inm) = headers.get(header::IF_NONE_MATCH) {
        if let Ok(inm_str) = inm.to_str() {
            let etag = format!("\"{}\"", cid);
            if inm_str == etag || inm_str.trim_matches('"') == cid {
                let mut h = HeaderMap::new();
                h.insert(header::ETAG, etag.parse().unwrap());
                return (StatusCode::NOT_MODIFIED, h, Json(json!(null)));
            }
        }
    }

    let Some(store) = state.durable_store.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            HeaderMap::new(),
            Json(json!({
                "@type": "ubl/error",
                "code": "UNAVAILABLE",
                "message": "Receipt store unavailable: enable SQLite durable store",
            })),
        );
    };

    match store.get_receipt(&cid) {
        Ok(Some(receipt)) => {
            if let Err(ubl_err) = verify_receipt_auth_chain(&cid, &receipt) {
                return (
                    StatusCode::from_u16(ubl_err.code.http_status())
                        .unwrap_or(StatusCode::UNPROCESSABLE_ENTITY),
                    HeaderMap::new(),
                    Json(ubl_err.to_json()),
                );
            }
            let mut h = HeaderMap::new();
            let etag = format!("\"{}\"", cid);
            h.insert(header::ETAG, etag.parse().unwrap());
            h.insert(
                header::CACHE_CONTROL,
                "public, max-age=31536000, immutable".parse().unwrap(),
            );
            (StatusCode::OK, h, Json(receipt))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            HeaderMap::new(),
            Json(json!({"@type": "ubl/error", "code": "NOT_FOUND", "message": format!("Receipt {} not found", cid)})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            HeaderMap::new(),
            Json(json!({
                "@type": "ubl/error",
                "code": "INTERNAL_ERROR",
                "message": format!("Receipt fetch failed: {}", e),
            })),
        ),
    }
}

pub(crate) async fn get_receipt_public_url(
    State(state): State<AppState>,
    Path(cid): Path<String>,
) -> impl IntoResponse {
    if !cid.starts_with("b3:") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"@type": "ubl/error", "code": "INVALID_CID", "message": "CID must start with b3:"})),
        );
    }

    let Some(store) = state.durable_store.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "@type": "ubl/error",
                "code": "UNAVAILABLE",
                "message": "Receipt store unavailable: enable SQLite durable store",
            })),
        );
    };

    match store.get_receipt(&cid) {
        Ok(Some(receipt)) => {
            if let Err(ubl_err) = verify_receipt_auth_chain(&cid, &receipt) {
                return (
                    StatusCode::from_u16(ubl_err.code.http_status())
                        .unwrap_or(StatusCode::UNPROCESSABLE_ENTITY),
                    Json(ubl_err.to_json()),
                );
            }
            match build_public_receipt_link(&state, &receipt) {
                Some(link) => (
                    StatusCode::OK,
                    Json(json!({
                        "@type":"ubl/receipt.url",
                        "receipt_cid": cid,
                        "receipt_url": link.url,
                        "receipt_public": link,
                    })),
                ),
                None => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "@type":"ubl/error",
                        "code":"INTERNAL_ERROR",
                        "message":"failed to derive canonical public receipt URL",
                    })),
                ),
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"@type": "ubl/error", "code": "NOT_FOUND", "message": format!("Receipt {} not found", cid)})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "@type": "ubl/error",
                "code": "INTERNAL_ERROR",
                "message": format!("Receipt fetch failed: {}", e),
            })),
        ),
    }
}

pub(crate) async fn get_passport_advisories(
    State(state): State<AppState>,
    Path(passport_cid): Path<String>,
) -> (StatusCode, Json<Value>) {
    let query = ubl_chipstore::ChipQuery {
        chip_type: Some("ubl/advisory".to_string()),
        tags: vec![format!("passport_cid:{}", passport_cid)],
        created_after: None,
        created_before: None,
        executor_did: None,
        limit: Some(100),
        offset: None,
    };

    match state.chip_store.query(&query).await {
        Ok(result) => {
            let advisories: Vec<Value> = result
                .chips
                .iter()
                .map(|c| {
                    json!({
                        "cid": c.cid,
                        "action": c.chip_data.get("action").unwrap_or(&json!("unknown")),
                        "hook": c.chip_data.get("hook").unwrap_or(&json!("unknown")),
                        "confidence": c.chip_data.get("confidence").unwrap_or(&json!(0)),
                        "model": c.chip_data.get("model").unwrap_or(&json!("unknown")),
                        "input_cid": c.chip_data.get("input_cid").unwrap_or(&json!("")),
                        "created_at": c.created_at,
                    })
                })
                .collect();
            (
                StatusCode::OK,
                Json(json!({
                    "@type": "ubl/advisory.list",
                    "passport_cid": passport_cid,
                    "count": advisories.len(),
                    "advisories": advisories,
                })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"@type": "ubl/error", "code": "INTERNAL_ERROR", "message": e.to_string()})),
        ),
    }
}

pub(crate) async fn verify_advisory(
    State(state): State<AppState>,
    Path(cid): Path<String>,
) -> (StatusCode, Json<Value>) {
    let chip = match state.chip_store.get_chip(&cid).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"@type": "ubl/error", "code": "NOT_FOUND", "message": format!("Advisory {} not found", cid)})),
            )
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"@type": "ubl/error", "code": "INTERNAL_ERROR", "message": e.to_string()})),
            )
        }
    };

    if chip.chip_type != "ubl/advisory" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"@type": "ubl/error", "code": "INVALID_TYPE", "message": "Chip is not an advisory"})),
        );
    }

    let advisory = match ubl_runtime::advisory::Advisory::from_chip_body(&chip.chip_data) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({"@type": "ubl/error", "code": "INVALID_ADVISORY", "message": e.to_string()})),
            )
        }
    };

    let nrf_bytes = match ubl_ai_nrf1::to_nrf1_bytes(&chip.chip_data) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"@type": "ubl/error", "code": "ENCODING_ERROR", "message": e.to_string()})),
            )
        }
    };
    let computed_cid = match ubl_ai_nrf1::compute_cid(&nrf_bytes) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"@type": "ubl/error", "code": "CID_ERROR", "message": e.to_string()})),
            )
        }
    };

    let cid_valid = computed_cid == cid;

    let passport_exists = state
        .chip_store
        .get_chip(&advisory.passport_cid)
        .await
        .map(|r| r.is_some())
        .unwrap_or(false);

    let input_exists = state
        .chip_store
        .get_chip(&advisory.input_cid)
        .await
        .map(|r| r.is_some())
        .unwrap_or(false);

    (
        StatusCode::OK,
        Json(json!({
            "@type": "ubl/advisory.verification",
            "advisory_cid": cid,
            "verified": cid_valid,
            "cid_valid": cid_valid,
            "computed_cid": computed_cid,
            "passport_cid": advisory.passport_cid,
            "passport_exists": passport_exists,
            "input_cid": advisory.input_cid,
            "input_exists": input_exists,
            "action": advisory.action,
            "model": advisory.model,
            "hook": format!("{:?}", advisory.hook),
            "confidence": advisory.confidence,
        })),
    )
}

pub(crate) async fn get_receipt_trace(
    State(state): State<AppState>,
    Path(cid): Path<String>,
) -> (StatusCode, Json<Value>) {
    if let Some(store) = state.durable_store.as_ref() {
        match store.get_receipt(&cid) {
            Ok(Some(receipt_json)) => {
                if let Err(ubl_err) = verify_receipt_auth_chain(&cid, &receipt_json) {
                    return (
                        StatusCode::from_u16(ubl_err.code.http_status())
                            .unwrap_or(StatusCode::UNPROCESSABLE_ENTITY),
                        Json(ubl_err.to_json()),
                    );
                }
            }
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "@type":"ubl/error",
                        "code":"NOT_FOUND",
                        "message": format!("Receipt {} not found", cid)
                    })),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "@type":"ubl/error",
                        "code":"INTERNAL_ERROR",
                        "message": format!("Receipt fetch failed: {}", e)
                    })),
                );
            }
        }
    }

    match state.chip_store.get_chip_by_receipt_cid(&cid).await {
        Ok(Some(chip)) => (
            StatusCode::OK,
            Json(json!({
                "@type": "ubl/trace",
                "receipt_cid": cid,
                "chip_cid": chip.cid,
                "chip_type": chip.chip_type,
                "auth_chain_verified": state.durable_store.is_some(),
                "execution_metadata": {
                    "runtime_version": chip.execution_metadata.runtime_version,
                    "execution_time_ms": chip.execution_metadata.execution_time_ms,
                    "fuel_consumed": chip.execution_metadata.fuel_consumed,
                    "policies_applied": chip.execution_metadata.policies_applied,
                    "reproducible": chip.execution_metadata.reproducible,
                },
            })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"@type": "ubl/error", "code": "NOT_FOUND", "message": format!("Receipt {} not found", cid)})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"@type": "ubl/error", "code": "INTERNAL_ERROR", "message": e.to_string()})),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct NarrateQuery {
    pub persist: Option<bool>,
}

pub(crate) async fn narrate_receipt(
    State(state): State<AppState>,
    Path(cid): Path<String>,
    Query(query): Query<NarrateQuery>,
) -> (StatusCode, Json<Value>) {
    let chip = match state.chip_store.get_chip_by_receipt_cid(&cid).await {
        Ok(Some(chip)) => chip,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "@type": "ubl/error",
                    "code": "NOT_FOUND",
                    "message": format!("Receipt {} not found", cid)
                })),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "@type": "ubl/error",
                    "code": "INTERNAL_ERROR",
                    "message": e.to_string()
                })),
            );
        }
    };

    let world = chip
        .chip_data
        .get("@world")
        .and_then(|v| v.as_str())
        .unwrap_or("a/system/t/unknown");
    let policy_count = chip.execution_metadata.policies_applied.len();
    let latency_ms = chip.execution_metadata.execution_time_ms;
    let fuel = chip.execution_metadata.fuel_consumed;
    let decision = "allow";

    let base_summary = format!(
        "{} processed as {} in {}ms (fuel {}, policies {}).",
        chip.chip_type, decision, latency_ms, fuel, policy_count
    );

    let summary = if llm_is_enabled() {
        let llm_ctx = serde_json::json!({
            "chip_type": chip.chip_type,
            "chip_body": chip.chip_data,
            "decision": decision,
            "latency_ms": latency_ms,
            "fuel_consumed": fuel,
            "policy_count": policy_count,
            "world": world,
        });
        match call_real_llm(&state.http_client, "receipt", &llm_ctx).await {
            Ok(text) => text,
            Err(_) => base_summary.clone(),
        }
    } else {
        base_summary.clone()
    };

    let narration = json!({
        "@type": "ubl/advisory.narration",
        "receipt_cid": cid,
        "chip_cid": chip.cid,
        "chip_type": chip.chip_type,
        "decision": decision,
        "world": world,
        "policy_count": policy_count,
        "latency_ms": latency_ms,
        "fuel_consumed": fuel,
        "summary": summary,
        "generated_at": chrono::Utc::now().to_rfc3339(),
    });

    let mut persisted_advisory_cid: Option<String> = None;
    if query.persist.unwrap_or(false) {
        let adv = Advisory::new(
            state.advisory_engine.passport_cid.clone(),
            "narrate".to_string(),
            cid.clone(),
            narration.clone(),
            90,
            state.advisory_engine.model.clone(),
            AdvisoryHook::OnDemand,
        );
        let body = state.advisory_engine.advisory_to_chip_body(&adv);
        let metadata = ubl_chipstore::ExecutionMetadata {
            runtime_version: "advisory/on-demand".to_string(),
            execution_time_ms: 0,
            fuel_consumed: 0,
            policies_applied: vec![],
            executor_did: chip.execution_metadata.executor_did.clone(),
            reproducible: true,
        };
        match state
            .chip_store
            .store_executed_chip(body, cid.clone(), metadata)
            .await
        {
            Ok(adv_cid) => persisted_advisory_cid = Some(adv_cid),
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "@type":"ubl/error",
                        "code":"INTERNAL_ERROR",
                        "message": format!("narration persist failed: {}", e),
                    })),
                );
            }
        }
    }

    (
        StatusCode::OK,
        Json(json!({
            "@type":"ubl/advisory.narration.response",
            "receipt_cid": cid,
            "narration": narration,
            "persisted_advisory_cid": persisted_advisory_cid,
        })),
    )
}

pub(crate) async fn narrate_receipt_stream(
    State(state): State<AppState>,
    Path(cid): Path<String>,
) -> Response {
    let chip = match state.chip_store.get_chip_by_receipt_cid(&cid).await {
        Ok(Some(chip)) => chip,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"@type":"ubl/error","code":"NOT_FOUND","message":format!("Receipt {} not found",cid)})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"@type":"ubl/error","code":"INTERNAL_ERROR","message":e.to_string()})),
            )
                .into_response();
        }
    };

    let world = chip
        .chip_data
        .get("@world")
        .and_then(|v| v.as_str())
        .unwrap_or("a/system/t/unknown");
    let policy_count = chip.execution_metadata.policies_applied.len();
    let latency_ms = chip.execution_metadata.execution_time_ms;
    let fuel = chip.execution_metadata.fuel_consumed;
    let decision = "allow";

    if !llm_is_enabled() {
        let summary = format!(
            "{} processed as {} in {}ms (fuel {}, policies {}).",
            chip.chip_type, decision, latency_ms, fuel, policy_count
        );
        let sse_stream = async_stream::stream! {
            yield Ok::<SseEvent, Infallible>(SseEvent::default().event("token").data(summary));
            yield Ok::<SseEvent, Infallible>(SseEvent::default().event("done").data(""));
        };
        return Sse::new(sse_stream)
            .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(15)).text(":"))
            .into_response();
    }

    let context = serde_json::json!({
        "chip_type": chip.chip_type,
        "chip_body": chip.chip_data,
        "decision": decision,
        "latency_ms": latency_ms,
        "fuel_consumed": fuel,
        "policy_count": policy_count,
        "world": world,
    });

    call_real_llm_stream_sse(state.http_client.clone(), "receipt".to_string(), context).await
}
