use std::sync::Arc;

use axum::{
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use tracing::warn;
use tracing_subscriber::EnvFilter;

use ubl_receipt::UnifiedReceipt;
use ubl_runtime::{
    error_response::{ErrorCode, UblError},
    rate_limit::{CanonRateLimiter, RateLimitConfig},
    rich_url::{build_public_receipt_link_v1, build_public_receipt_token_v1, PublicReceiptLink},
};

use crate::state::{AppState, McpWsAuth};

// ── Tracing ──────────────────────────────────────────────────────────────────

pub(crate) fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,ubl_runtime=debug,ubl_gate=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .try_init();
}

// ── Env helpers ───────────────────────────────────────────────────────────────

pub(crate) fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(default)
}

pub(crate) fn env_opt_trim(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub(crate) fn csv_env(name: &str) -> Vec<String> {
    env_opt_trim(name)
        .map(|s| {
            s.split(',')
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(crate) fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    if let Some(k) = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
    {
        return Some(k.to_string());
    }

    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?;
    let (scheme, token) = auth.split_once(' ')?;
    let scheme_ok = scheme.eq_ignore_ascii_case("apikey")
        || scheme.eq_ignore_ascii_case("api-key")
        || scheme.eq_ignore_ascii_case("bearer");
    if !scheme_ok {
        return None;
    }
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

pub(crate) fn world_scope_allows(scope_world: &str, target_world: &str) -> bool {
    let scope = scope_world.trim().trim_end_matches('/');
    let target = target_world.trim().trim_end_matches('/');
    if scope.is_empty() || target.is_empty() {
        return false;
    }
    if scope == "*" {
        return true;
    }
    if target == scope {
        return true;
    }
    target
        .strip_prefix(scope)
        .map(|rest| rest.starts_with('/'))
        .unwrap_or(false)
}

// ── URL / config helpers ──────────────────────────────────────────────────────

pub(crate) fn public_receipt_origin_from_env() -> String {
    if let Some(origin) = env_opt_trim("UBL_PUBLIC_RECEIPT_ORIGIN") {
        return origin;
    }
    if let Some(domain) = env_opt_trim("UBL_RICH_URL_DOMAIN") {
        let d = domain
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        return format!("https://{}", d);
    }
    "https://logline.world".to_string()
}

pub(crate) fn public_receipt_path_from_env() -> String {
    let path = env_opt_trim("UBL_PUBLIC_RECEIPT_PATH").unwrap_or_else(|| "/r".to_string());
    if path.starts_with('/') {
        path
    } else {
        format!("/{}", path)
    }
}

pub(crate) fn manifest_base_url_from_env() -> String {
    if let Some(origin) = env_opt_trim("UBL_MCP_BASE_URL") {
        return origin;
    }
    if let Some(origin) = env_opt_trim("UBL_API_BASE_URL") {
        return origin;
    }
    if let Some(domain) = env_opt_trim("UBL_API_DOMAIN") {
        let d = domain
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        return format!("https://{}", d);
    }
    "https://api.ubl.agency".to_string()
}

pub(crate) fn load_canon_rate_limiter() -> Option<Arc<CanonRateLimiter>> {
    if !env_bool("UBL_CANON_RATE_LIMIT_ENABLED", true) {
        return None;
    }
    let per_min = std::env::var("UBL_CANON_RATE_LIMIT_PER_MIN")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(120)
        .max(1);
    Some(Arc::new(CanonRateLimiter::new(
        RateLimitConfig::per_minute(per_min),
    )))
}

// ── Error builders ────────────────────────────────────────────────────────────

pub(crate) fn too_many_requests_error(message: String, details: Value) -> UblError {
    UblError {
        error_type: "ubl/error".to_string(),
        id: format!("err-rate-{}", chrono::Utc::now().timestamp_micros()),
        ver: "1.0".to_string(),
        world: "a/system/t/errors".to_string(),
        code: ErrorCode::TooManyRequests,
        message,
        link: "https://docs.ubl.agency/errors#TOO_MANY_REQUESTS".to_string(),
        details: Some(details),
    }
}

pub(crate) fn tamper_detected_error(message: String, details: Value) -> UblError {
    UblError {
        error_type: "ubl/error".to_string(),
        id: format!("err-tamper-{}", chrono::Utc::now().timestamp_micros()),
        ver: "1.0".to_string(),
        world: "a/system/t/errors".to_string(),
        code: ErrorCode::TamperDetected,
        message,
        link: "https://docs.ubl.agency/errors#TAMPER_DETECTED".to_string(),
        details: Some(details),
    }
}

pub(crate) fn write_access_error(code: ErrorCode, message: String, details: Value) -> UblError {
    UblError {
        error_type: "ubl/error".to_string(),
        id: format!("err-write-{}", chrono::Utc::now().timestamp_micros()),
        ver: "1.0".to_string(),
        world: "a/system/t/errors".to_string(),
        code,
        message,
        link: format!(
            "https://docs.ubl.agency/errors#{}",
            serde_json::to_value(code)
                .unwrap_or(Value::String("INTERNAL_ERROR".to_string()))
                .as_str()
                .unwrap_or("INTERNAL_ERROR")
        ),
        details: Some(details),
    }
}

pub(crate) async fn deny_write_with_receipt(
    state: &AppState,
    knock_cid: &str,
    reason_code: &str,
    reason_msg: &str,
    err_code: ErrorCode,
    value: &Value,
    subject_did: String,
) -> (StatusCode, HeaderMap, Value) {
    let details = json!({
        "@type": value.get("@type").and_then(|v| v.as_str()).unwrap_or("unknown"),
        "@world": value.get("@world").and_then(|v| v.as_str()).unwrap_or("unknown"),
        "knock_cid": knock_cid,
        "auth_required": state.write_access_policy.auth_required,
        "api_keys_configured": !state.write_access_policy.api_keys.is_empty(),
    });
    let ubl_err = write_access_error(err_code, reason_msg.to_string(), details);

    match state
        .pipeline
        .process_knock_rejection(knock_cid, reason_code, reason_msg, Some(subject_did))
        .await
    {
        Ok(result) => {
            let receipt_json = result.receipt.to_json().unwrap_or(json!({}));
            let public_receipt = build_public_receipt_link(state, &receipt_json);
            let receipt_url = public_receipt.as_ref().map(|p| p.url.clone());
            (
                StatusCode::from_u16(err_code.http_status()).unwrap_or(StatusCode::FORBIDDEN),
                HeaderMap::new(),
                json!({
                    "@type": "ubl/error",
                    "code": serde_json::to_value(err_code).unwrap_or(Value::String("POLICY_DENIED".to_string())),
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
                    "details": ubl_err.details,
                }),
            )
        }
        Err(process_err) => {
            let ubl_err = UblError::from_pipeline_error(&process_err);
            let status =
                StatusCode::from_u16(ubl_err.code.http_status()).unwrap_or(StatusCode::BAD_REQUEST);
            (status, HeaderMap::new(), ubl_err.to_json())
        }
    }
}

// ── Receipt / auth chain ──────────────────────────────────────────────────────

#[allow(clippy::result_large_err)]
pub(crate) fn verify_receipt_auth_chain(
    receipt_cid: &str,
    receipt_json: &Value,
) -> Result<(), UblError> {
    let receipt = UnifiedReceipt::from_json(receipt_json).map_err(|e| {
        tamper_detected_error(
            format!("receipt {} parse failed: {}", receipt_cid, e),
            json!({
                "receipt_cid": receipt_cid,
                "reason": "receipt_parse_failed"
            }),
        )
    })?;

    if !receipt.verify_auth_chain() {
        return Err(tamper_detected_error(
            format!("receipt {} auth chain broken", receipt_cid),
            json!({
                "receipt_cid": receipt_cid,
                "reason": "auth_chain_broken"
            }),
        ));
    }

    Ok(())
}

pub(crate) fn build_public_receipt_link(
    state: &AppState,
    receipt_json: &Value,
) -> Option<PublicReceiptLink> {
    let token = match build_public_receipt_token_v1(
        receipt_json,
        state.genesis_pubkey_sha256.as_deref(),
        state.release_commit.as_deref(),
        state.gate_binary_sha256.as_deref(),
    ) {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "failed to build public receipt token");
            return None;
        }
    };

    match build_public_receipt_link_v1(
        &state.public_receipt_origin,
        &state.public_receipt_path,
        &token,
    ) {
        Ok(link) => Some(link),
        Err(e) => {
            warn!(error = %e, "failed to build public receipt link");
            None
        }
    }
}

pub(crate) fn actor_hint_from_headers(
    headers: Option<&HeaderMap>,
) -> ubl_runtime::authorship::ActorHint {
    let mut hint = ubl_runtime::authorship::ActorHint::default();
    let Some(h) = headers else {
        return hint;
    };

    if let Some(forwarded_for) = h
        .get("CF-Connecting-IP")
        .or_else(|| h.get("X-Forwarded-For"))
        .and_then(|v| v.to_str().ok())
    {
        let ip = forwarded_for.split(',').next().unwrap_or_default().trim();
        if !ip.is_empty() {
            let parts: Vec<&str> = ip.split('.').collect();
            if parts.len() == 4 {
                hint.ip_prefix = Some(format!("{}.{}.{}.*", parts[0], parts[1], parts[2]));
            } else {
                hint.ip_prefix = Some(ip.to_string());
            }
        }
    }

    if let Some(ua) = h.get(header::USER_AGENT).and_then(|v| v.to_str().ok()) {
        let ua_hash = blake3::hash(ua.as_bytes());
        hint.user_agent_hash = Some(format!("b3:{}", hex::encode(ua_hash.as_bytes())));
    }

    hint
}

pub(crate) fn knock_reason_code(err: &ubl_runtime::knock::KnockError) -> String {
    let msg = err.to_string();
    msg.split(':').next().unwrap_or("KNOCK-000").to_string()
}

// ── Bearer / session auth ─────────────────────────────────────────────────────

pub(crate) fn parse_bearer_token(headers: &HeaderMap) -> Option<String> {
    let auth = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = auth.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") || token.trim().is_empty() {
        return None;
    }
    Some(token.trim().to_string())
}

pub(crate) fn scope_allows_any(scope: &[String], required: &[&str]) -> bool {
    scope.iter().any(|s| s == "*")
        || required
            .iter()
            .any(|needle| scope.iter().any(|s| s == needle))
}

pub(crate) async fn resolve_session_bearer(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<McpWsAuth>, String> {
    let Some(token_id) = parse_bearer_token(headers) else {
        return Ok(None);
    };

    let token_query = ubl_chipstore::ChipQuery {
        chip_type: Some("ubl/token".to_string()),
        tags: vec![format!("id:{}", token_id)],
        created_after: None,
        created_before: None,
        executor_did: None,
        limit: Some(10),
        offset: None,
    };
    let token_result = state
        .chip_store
        .query(&token_query)
        .await
        .map_err(|e| format!("token query failed: {}", e))?;

    let Some(token_chip) = token_result
        .chips
        .into_iter()
        .find(|chip| chip.chip_data.get("@id").and_then(|v| v.as_str()) == Some(token_id.as_str()))
    else {
        return Err("token not found".to_string());
    };

    let session = ubl_runtime::SessionToken::from_chip_body(&token_chip.chip_data)
        .map_err(|e| format!("invalid token chip: {}", e))?;

    let expires_at = chrono::DateTime::parse_from_rfc3339(&session.expires_at)
        .map(|t| t.with_timezone(&chrono::Utc))
        .map_err(|e| format!("invalid token expiry: {}", e))?;
    if expires_at <= chrono::Utc::now() {
        return Err("token expired".to_string());
    }

    let revoke_query = ubl_chipstore::ChipQuery {
        chip_type: Some("ubl/revoke".to_string()),
        tags: vec![format!("target_cid:{}", token_chip.cid.as_str())],
        created_after: None,
        created_before: None,
        executor_did: None,
        limit: Some(1),
        offset: None,
    };
    let revoked = state
        .chip_store
        .query(&revoke_query)
        .await
        .map(|r| r.total_count > 0)
        .unwrap_or(false);
    if revoked {
        return Err("token revoked".to_string());
    }

    let token_world = token_chip
        .chip_data
        .get("@world")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "token missing @world".to_string())?
        .to_string();

    let subject_did = match state.chip_store.get_chip(&session.user_cid).await {
        Ok(Some(user_chip)) => user_chip
            .chip_data
            .get("did")
            .and_then(|v| v.as_str())
            .filter(|d| d.starts_with("did:"))
            .map(|d| d.to_string()),
        _ => None,
    };

    let user_revoke_query = ubl_chipstore::ChipQuery {
        chip_type: Some("ubl/revoke".to_string()),
        tags: vec![format!("target_cid:{}", session.user_cid.as_str())],
        created_after: None,
        created_before: None,
        executor_did: None,
        limit: Some(1),
        offset: None,
    };
    let user_revoked = state
        .chip_store
        .query(&user_revoke_query)
        .await
        .map(|r| r.total_count > 0)
        .unwrap_or(false);
    if user_revoked {
        return Err("token user revoked".to_string());
    }

    Ok(Some(McpWsAuth {
        token_id,
        token_cid: token_chip.cid.as_str().to_string(),
        world: token_world,
        scope: session.scope,
        subject_did,
    }))
}

pub(crate) async fn validate_mcp_ws_bearer(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<McpWsAuth, Response> {
    let Some(token_id) = parse_bearer_token(headers) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "@type": "ubl/error",
                "code": "UNAUTHORIZED",
                "message": "missing Authorization: Bearer <token>"
            })),
        )
            .into_response());
    };

    let auth = match resolve_session_bearer(state, headers).await {
        Ok(Some(auth)) => auth,
        Ok(None) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "@type":"ubl/error",
                    "code":"UNAUTHORIZED",
                    "message":"token not found"
                })),
            )
                .into_response())
        }
        Err(msg) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "@type":"ubl/error",
                    "code":"UNAUTHORIZED",
                    "message": msg
                })),
            )
                .into_response())
        }
    };

    if !scope_allows_any(&auth.scope, &["mcp", "read", "write", "mcp:write"]) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "@type":"ubl/error",
                "code":"POLICY_DENIED",
                "message":"token scope does not allow MCP access"
            })),
        )
            .into_response());
    }
    if auth.token_id != token_id {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "@type":"ubl/error",
                "code":"UNAUTHORIZED",
                "message":"token mismatch"
            })),
        )
            .into_response());
    }

    Ok(auth)
}

pub(crate) fn parse_when_to_ms(input: &str) -> Option<i64> {
    if let Ok(ms) = input.parse::<i64>() {
        return Some(ms);
    }
    chrono::DateTime::parse_from_rfc3339(input)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

pub(crate) fn parse_window_duration(input: Option<&str>) -> Option<std::time::Duration> {
    let raw = input?.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(ms) = raw.parse::<u64>() {
        return Some(std::time::Duration::from_millis(ms));
    }
    let (num, unit) = raw.split_at(raw.len().saturating_sub(1));
    let value = num.parse::<u64>().ok()?;
    match unit {
        "s" | "S" => Some(std::time::Duration::from_secs(value)),
        "m" | "M" => Some(std::time::Duration::from_secs(value.saturating_mul(60))),
        "h" | "H" => Some(std::time::Duration::from_secs(value.saturating_mul(3600))),
        _ => None,
    }
}
