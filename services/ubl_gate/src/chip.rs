//! Chip submission, creation, retrieval and verification handlers.

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};

use crate::metrics;
use crate::state::AppState;
use crate::utils::{
    actor_hint_from_headers, build_public_receipt_link, deny_write_with_receipt,
    knock_reason_code, parse_bearer_token, resolve_session_bearer, scope_allows_any, too_many_requests_error, verify_receipt_auth_chain,
    world_scope_allows,
};
use ubl_runtime::error_response::{ErrorCode, UblError};
use ubl_runtime::rate_limit::RateLimitResult;

pub(crate) async fn submit_chip_bytes(
    state: &AppState,
    headers: Option<&HeaderMap>,
    trusted_write: bool,
    body: &[u8],
) -> (StatusCode, HeaderMap, Value) {
    metrics::inc_chips_total();
    let t0 = std::time::Instant::now();
    let knock_cid = ubl_runtime::authorship::knock_cid_from_bytes(body);
    let actor_hint = actor_hint_from_headers(headers);

    let value = match ubl_runtime::knock::knock(body) {
        Ok(v) => v,
        Err(e) => {
            metrics::observe_pipeline_seconds(t0.elapsed().as_secs_f64());
            let reason_code = knock_reason_code(&e);
            let reason_msg = e.to_string();
            let subject_did = ubl_runtime::authorship::resolve_subject_did(None, Some(&actor_hint));
            metrics::inc_knock_reject();
            metrics::inc_error("KNOCK_REJECTED");

            match state
                .pipeline
                .process_knock_rejection(&knock_cid, &reason_code, &reason_msg, Some(subject_did))
                .await
            {
                Ok(result) => {
                    let receipt_json = result.receipt.to_json().unwrap_or(json!({}));
                    let public_receipt = build_public_receipt_link(state, &receipt_json);
                    let receipt_url = public_receipt.as_ref().map(|p| p.url.clone());
                    let status = StatusCode::UNPROCESSABLE_ENTITY;
                    return (
                        status,
                        HeaderMap::new(),
                        json!({
                            "@type": "ubl/error",
                            "code": "KNOCK_REJECTED",
                            "message": reason_msg,
                            "receipt_cid": result.receipt.receipt_cid.as_str(),
                            "receipt_url": receipt_url,
                            "receipt_public": public_receipt,
                            "chain": result.chain,
                            "receipt": receipt_json,
                            "subject_did": result.receipt.subject_did,
                            "knock_cid": result.receipt.knock_cid,
                            "decision": "Deny",
                            "status": "denied",
                        }),
                    );
                }
                Err(process_err) => {
                    let ubl_err = UblError::from_pipeline_error(&process_err);
                    let status = StatusCode::from_u16(ubl_err.code.http_status())
                        .unwrap_or(StatusCode::BAD_REQUEST);
                    return (status, HeaderMap::new(), ubl_err.to_json());
                }
            }
        }
    };

    let mut subject_did_from_token_hint: Option<String> = None;

    if !trusted_write {
        let chip_type = value.get("@type").and_then(|v| v.as_str()).unwrap_or("");
        let world = value.get("@world").and_then(|v| v.as_str()).unwrap_or("");
        let mut authorized_via_token = false;
        if let Some(h) = headers {
            if parse_bearer_token(h).is_some() {
                match resolve_session_bearer(state, h).await {
                    Ok(Some(auth)) => {
                        if scope_allows_any(&auth.scope, &["write", "chip:write", "mcp:write"]) {
                            if !world_scope_allows(&auth.world, world) {
                                let err_code = ErrorCode::PolicyDenied;
                                let reason_msg = format!(
                                    "token world '{}' does not authorize target world '{}'",
                                    auth.world, world
                                );
                                let subject_did = auth.subject_did.clone().unwrap_or_else(|| {
                                    ubl_runtime::authorship::resolve_subject_did(
                                        Some(&value),
                                        Some(&actor_hint),
                                    )
                                });
                                let reason_code = serde_json::to_value(err_code)
                                    .ok()
                                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                                    .unwrap_or_else(|| "POLICY_DENIED".to_string());
                                return deny_write_with_receipt(
                                    state,
                                    &knock_cid,
                                    &reason_code,
                                    &reason_msg,
                                    err_code,
                                    &value,
                                    subject_did,
                                )
                                .await;
                            }
                            authorized_via_token = true;
                            subject_did_from_token_hint = auth.subject_did.clone();
                        } else {
                            let err_code = ErrorCode::PolicyDenied;
                            let reason_msg = "token scope does not allow write".to_string();
                            let subject_did = auth.subject_did.clone().unwrap_or_else(|| {
                                ubl_runtime::authorship::resolve_subject_did(
                                    Some(&value),
                                    Some(&actor_hint),
                                )
                            });
                            let reason_code = serde_json::to_value(err_code)
                                .ok()
                                .and_then(|v| v.as_str().map(|s| s.to_string()))
                                .unwrap_or_else(|| "POLICY_DENIED".to_string());
                            return deny_write_with_receipt(
                                state,
                                &knock_cid,
                                &reason_code,
                                &reason_msg,
                                err_code,
                                &value,
                                subject_did,
                            )
                            .await;
                        }
                    }
                    Ok(None) => {}
                    Err(msg) => {
                        let err_code = ErrorCode::Unauthorized;
                        let subject_did = ubl_runtime::authorship::resolve_subject_did(
                            Some(&value),
                            Some(&actor_hint),
                        );
                        let reason_code = serde_json::to_value(err_code)
                            .ok()
                            .and_then(|v| v.as_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| "UNAUTHORIZED".to_string());
                        return deny_write_with_receipt(
                            state,
                            &knock_cid,
                            &reason_code,
                            &msg,
                            err_code,
                            &value,
                            subject_did,
                        )
                        .await;
                    }
                }
            }
        }

        if !authorized_via_token {
            if let Err((err_code, reason_msg)) = state
                .write_access_policy
                .authorize_write(headers, chip_type, world)
            {
                let subject_did = subject_did_from_token_hint.clone().unwrap_or_else(|| {
                    ubl_runtime::authorship::resolve_subject_did(Some(&value), Some(&actor_hint))
                });
                let reason_code = serde_json::to_value(err_code)
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| "UNAUTHORIZED".to_string());
                return deny_write_with_receipt(
                    state,
                    &knock_cid,
                    &reason_code,
                    &reason_msg,
                    err_code,
                    &value,
                    subject_did,
                )
                .await;
            }
        }
    }

    if let Some(ref limiter) = state.canon_rate_limiter {
        if let Some((fp, RateLimitResult::Limited { retry_after, .. })) =
            limiter.check_body(&value).await
        {
            metrics::observe_pipeline_seconds(t0.elapsed().as_secs_f64());
            metrics::inc_error("TooManyRequests");
            let mut headers = HeaderMap::new();
            let retry_secs = retry_after.as_secs().saturating_add(1);
            if let Ok(v) = retry_secs.to_string().parse() {
                headers.insert(header::RETRY_AFTER, v);
            }
            let err = too_many_requests_error(
                format!(
                    "Rate limit exceeded for canonical payload {}",
                    fp.rate_key()
                ),
                json!({
                    "limited_by": "canon_fingerprint",
                    "fingerprint": fp.hash,
                    "at_type": fp.at_type,
                    "at_ver": fp.at_ver,
                    "at_world": fp.at_world,
                    "retry_after_seconds": retry_secs,
                }),
            );
            return (StatusCode::TOO_MANY_REQUESTS, headers, err.to_json());
        }
    }

    let chip_type = value["@type"].as_str().unwrap_or("").to_string();
    let request = ubl_runtime::pipeline::ChipRequest {
        chip_type,
        body: value,
        parents: vec![],
        operation: Some("create".to_string()),
    };
    let subject_did = if !trusted_write {
        subject_did_from_token_hint.unwrap_or_else(|| {
            ubl_runtime::authorship::resolve_subject_did(Some(&request.body), Some(&actor_hint))
        })
    } else {
        ubl_runtime::authorship::resolve_subject_did(Some(&request.body), Some(&actor_hint))
    };
    let ctx = ubl_runtime::pipeline::AuthorshipContext {
        subject_did_hint: Some(subject_did),
        knock_cid: Some(knock_cid.clone()),
    };

    match state.pipeline.process_chip_with_context(request, ctx).await {
        Ok(result) => {
            metrics::observe_pipeline_seconds(t0.elapsed().as_secs_f64());
            let decision_str = format!("{:?}", result.decision);
            if decision_str.contains("Allow") {
                metrics::inc_allow();
            } else {
                metrics::inc_deny();
            }
            let receipt_json = result.receipt.to_json().unwrap_or(json!({}));
            let public_receipt = build_public_receipt_link(state, &receipt_json);
            let mut headers = HeaderMap::new();
            if result.replayed {
                metrics::inc_idempotency_hit();
                metrics::inc_idempotency_replay_block();
                headers.insert("X-UBL-Replay", "true".parse().unwrap());
            }
            let receipt_url = public_receipt.as_ref().map(|p| p.url.clone());
            (
                StatusCode::OK,
                headers,
                json!({
                    "@type": "ubl/response",
                    "status": "success",
                    "decision": decision_str,
                    "receipt_cid": result.receipt.receipt_cid,
                    "receipt_url": receipt_url,
                    "receipt_public": public_receipt,
                    "chain": result.chain,
                    "subject_did": result.receipt.subject_did,
                    "knock_cid": result.receipt.knock_cid,
                    "receipt": receipt_json,
                    "replayed": result.replayed,
                }),
            )
        }
        Err(e) => {
            metrics::observe_pipeline_seconds(t0.elapsed().as_secs_f64());
            let ubl_err = UblError::from_pipeline_error(&e);
            match ubl_err.code {
                ErrorCode::SignError | ErrorCode::InvalidSignature => {
                    let mode = std::env::var("UBL_CRYPTO_MODE")
                        .unwrap_or_else(|_| "compat_v1".to_string());
                    metrics::inc_crypto_verify_fail("pipeline", &mode);
                }
                ErrorCode::CanonError => metrics::inc_canon_divergence("pipeline"),
                _ => {}
            }
            let code_str = format!("{:?}", ubl_err.code);
            if code_str.contains("Knock") {
                metrics::inc_knock_reject();
            }
            metrics::inc_error(&code_str);
            let status = StatusCode::from_u16(ubl_err.code.http_status())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (status, HeaderMap::new(), ubl_err.to_json())
        }
    }
}

pub(crate) async fn get_runtime_attestation(
    State(state): State<AppState>,
) -> (StatusCode, Json<Value>) {
    match state.pipeline.runtime_self_attestation() {
        Ok(attestation) => {
            let verified = attestation.verify().unwrap_or(false);
            (
                StatusCode::OK,
                Json(json!({
                    "@type": "ubl/runtime.attestation.response",
                    "verified": verified,
                    "attestation": attestation,
                })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "@type": "ubl/error",
                "code": "INTERNAL_ERROR",
                "message": e.to_string(),
            })),
        ),
    }
}

pub(crate) async fn create_chip(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let (status, headers, payload) = submit_chip_bytes(&state, Some(&headers), false, &body).await;
    (status, headers, Json(payload))
}

pub(crate) async fn metrics_handler() -> String {
    metrics::encode_metrics()
}

pub(crate) async fn verify_chip(
    State(state): State<AppState>,
    Path(cid): Path<String>,
) -> (StatusCode, Json<Value>) {
    if !cid.starts_with("b3:") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"@type": "ubl/error", "code": "INVALID_CID", "message": "CID must start with b3:"})),
        );
    }

    let chip = match state.chip_store.get_chip(&cid).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"@type": "ubl/error", "code": "NOT_FOUND", "message": format!("Chip {} not found", cid)})),
            )
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"@type": "ubl/error", "code": "INTERNAL_ERROR", "message": e.to_string()})),
            )
        }
    };

    let (computed_cid, encoding_ok) = match ubl_ai_nrf1::to_nrf1_bytes(&chip.chip_data) {
        Ok(nrf_bytes) => match ubl_ai_nrf1::compute_cid(&nrf_bytes) {
            Ok(c) => (c, true),
            Err(_) => (String::new(), false),
        },
        Err(_) => (String::new(), false),
    };

    let cid_matches = encoding_ok && computed_cid == cid;

    let receipt_cid = &chip.receipt_cid;
    let mut auth_chain_verified: Option<bool> = None;
    let receipt_exists = if receipt_cid.as_str().is_empty() {
        false
    } else {
        match state.durable_store.as_ref() {
            Some(store) => match store.get_receipt(receipt_cid.as_str()) {
                Ok(Some(receipt_json)) => {
                    if let Err(ubl_err) =
                        verify_receipt_auth_chain(receipt_cid.as_str(), &receipt_json)
                    {
                        return (
                            StatusCode::from_u16(ubl_err.code.http_status())
                                .unwrap_or(StatusCode::UNPROCESSABLE_ENTITY),
                            Json(ubl_err.to_json()),
                        );
                    }
                    auth_chain_verified = Some(true);
                    true
                }
                Ok(None) => false,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "@type": "ubl/error",
                            "code": "INTERNAL_ERROR",
                            "message": format!("Receipt fetch failed: {}", e)
                        })),
                    );
                }
            },
            None => state
                .chip_store
                .get_chip_by_receipt_cid(receipt_cid.as_str())
                .await
                .map(|c| c.is_some())
                .unwrap_or(false),
        }
    };

    (
        StatusCode::OK,
        Json(json!({
            "@type": "ubl/chip.verification",
            "cid": cid,
            "verified": cid_matches,
            "cid_matches": cid_matches,
            "computed_cid": computed_cid,
            "encoding_ok": encoding_ok,
            "chip_type": chip.chip_type,
            "receipt_cid": chip.receipt_cid,
            "receipt_exists": receipt_exists,
            "auth_chain_verified": auth_chain_verified,
            "created_at": chip.created_at,
        })),
    )
}

pub(crate) async fn get_chip(
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

    match state.chip_store.get_chip(&cid).await {
        Ok(Some(chip)) => {
            let mut h = HeaderMap::new();
            let etag = format!("\"{}\"", chip.cid);
            h.insert(header::ETAG, etag.parse().unwrap());
            h.insert(
                header::CACHE_CONTROL,
                "public, max-age=31536000, immutable".parse().unwrap(),
            );
            (
                StatusCode::OK,
                h,
                Json(json!({
                    "@type": "ubl/chip",
                    "cid": chip.cid,
                    "chip_type": chip.chip_type,
                    "chip_data": chip.chip_data,
                    "receipt_cid": chip.receipt_cid,
                    "created_at": chip.created_at,
                    "tags": chip.tags,
                })),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            HeaderMap::new(),
            Json(json!({"@type": "ubl/error", "code": "NOT_FOUND", "message": format!("Chip {} not found", cid)})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            HeaderMap::new(),
            Json(json!({"@type": "ubl/error", "code": "INTERNAL_ERROR", "message": e.to_string()})),
        ),
    }
}
