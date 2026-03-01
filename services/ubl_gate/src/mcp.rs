//! MCP (Model Context Protocol) handlers: SSE bootstrap, JSON-RPC, WebSocket, dispatch.

use async_stream::stream;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event as SseEvent, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Duration;
use tracing::{info, warn};
use ubl_runtime::advisory::{Advisory, AdvisoryHook};
use ubl_runtime::error_response::{ErrorCode, UblError};

use crate::chip::submit_chip_bytes;
use crate::state::{AppState, McpWsAuth};
use crate::utils::{scope_allows_any, validate_mcp_ws_bearer, verify_receipt_auth_chain};

pub(crate) async fn openapi_spec(State(state): State<AppState>) -> Json<Value> {
    Json(state.manifest.to_openapi())
}

pub(crate) async fn mcp_manifest(State(state): State<AppState>) -> Json<Value> {
    Json(state.manifest.to_mcp_manifest())
}

pub(crate) async fn webmcp_manifest(State(state): State<AppState>) -> Json<Value> {
    Json(state.manifest.to_webmcp_manifest())
}

pub(crate) async fn mcp_rpc_sse(
    State(state): State<AppState>,
) -> Sse<impl futures_util::Stream<Item = Result<SseEvent, Infallible>>> {
    let tools = state
        .manifest
        .to_mcp_manifest()
        .get("tools")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let ready = json!({
        "jsonrpc":"2.0",
        "method":"mcp.ready",
        "params":{
            "server":"ubl-gate",
            "transport":"sse",
            "rpc_post_path":"/mcp/rpc",
            "tools": tools
        }
    })
    .to_string();

    let s = stream! {
        yield Ok::<SseEvent, Infallible>(SseEvent::default().event("mcp.ready").data(ready));
        let mut ticker = tokio::time::interval(Duration::from_secs(15));
        loop {
            ticker.tick().await;
            yield Ok::<SseEvent, Infallible>(SseEvent::default().event("ping").data("{}"));
        }
    };

    Sse::new(s).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keepalive"),
    )
}

pub(crate) async fn mcp_rpc(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(rpc): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let (status, payload) = handle_mcp_rpc_request(&state, rpc, Some(&headers), None).await;
    (status, Json(payload))
}

fn mcp_error_value(id: Value, code: i32, message: impl Into<String>, data: Option<Value>) -> Value {
    let mut err = json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into(),
        }
    });
    if let Some(data) = data {
        err["error"]["data"] = data;
    }
    err
}

fn canonical_tool_name(name: &str) -> &str {
    match name {
        "ubl.chip.submit" => "ubl.deliver",
        "ubl.chip.get" => "ubl.query",
        "ubl.chip.verify" => "ubl.verify",
        other => other,
    }
}

fn is_write_tool_call(tool_name: &str, arguments: &Value) -> bool {
    match canonical_tool_name(tool_name) {
        "ubl.deliver" => true,
        "ubl.narrate" => arguments
            .get("persist")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        _ => false,
    }
}

fn mcp_scope_allows_write(auth: &McpWsAuth) -> bool {
    scope_allows_any(&auth.scope, &["write", "mcp:write"])
}

pub(crate) async fn handle_mcp_rpc_request(
    state: &AppState,
    rpc: Value,
    mcp_headers: Option<&HeaderMap>,
    ws_auth: Option<&McpWsAuth>,
) -> (StatusCode, Value) {
    let id = rpc.get("id").cloned().unwrap_or(json!(null));
    let method = rpc.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let params = rpc.get("params").cloned().unwrap_or(json!({}));

    if rpc.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
        return (
            StatusCode::BAD_REQUEST,
            mcp_error_value(id, -32600, "Invalid Request: missing jsonrpc 2.0", None),
        );
    }

    match method {
        "tools/list" => {
            let manifest = state.manifest.to_mcp_manifest();
            (
                StatusCode::OK,
                json!({
                    "jsonrpc": "2.0", "id": id,
                    "result": { "tools": manifest["tools"] }
                }),
            )
        }

        "tools/call" => {
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

            if let Some(auth) = ws_auth {
                if let Some(retry_after) = state.mcp_token_rate_limiter.check(&auth.token_id).await
                {
                    return (
                        StatusCode::OK,
                        mcp_error_value(
                            id,
                            -32006,
                            format!("Rate limit exceeded for token; retry in {}s", retry_after),
                            Some(json!({ "retry_after_seconds": retry_after })),
                        ),
                    );
                }

                if is_write_tool_call(tool_name, &arguments) && !mcp_scope_allows_write(auth) {
                    return (
                        StatusCode::OK,
                        mcp_error_value(
                            id,
                            ErrorCode::PolicyDenied.mcp_code(),
                            "token scope does not allow write tools",
                            Some(json!({ "tool": tool_name, "required_scope": "write|*" })),
                        ),
                    );
                }
            }
            match tokio::time::timeout(
                Duration::from_secs(30),
                dispatch_tool_call(
                    state,
                    tool_name,
                    &arguments,
                    id.clone(),
                    mcp_headers,
                    ws_auth,
                ),
            )
            .await
            {
                Ok((status, Json(payload))) => (status, payload),
                Err(_) => (
                    StatusCode::OK,
                    mcp_error_value(
                        id,
                        -32000,
                        "Tool call timed out (30s limit)",
                        Some(json!({ "timeout_seconds": 30 })),
                    ),
                ),
            }
        }

        _ => (
            StatusCode::OK,
            mcp_error_value(id, -32601, format!("Method not found: {}", method), None),
        ),
    }
}

pub(crate) async fn mcp_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let auth = match validate_mcp_ws_bearer(&state, &headers).await {
        Ok(auth) => auth,
        Err(resp) => return resp,
    };

    ws.on_upgrade(move |socket| mcp_ws_session(socket, state, auth))
        .into_response()
}

pub(crate) async fn mcp_ws_session(mut socket: WebSocket, state: AppState, auth: McpWsAuth) {
    info!(
        token_id = %auth.token_id,
        token_cid = %auth.token_cid,
        world = %auth.world,
        scope_count = auth.scope.len(),
        "mcp/ws session started"
    );
    while let Some(next) = socket.recv().await {
        let msg = match next {
            Ok(m) => m,
            Err(e) => {
                warn!(error = %e, "mcp/ws receive failed");
                break;
            }
        };

        match msg {
            Message::Text(text) => {
                let rpc: Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(e) => {
                        let err = mcp_error_value(
                            json!(null),
                            -32700,
                            format!("Parse error: {}", e),
                            None,
                        );
                        let _ = socket.send(Message::Text(err.to_string())).await;
                        continue;
                    }
                };
                let (_status, payload) =
                    handle_mcp_rpc_request(&state, rpc, None, Some(&auth)).await;
                if socket
                    .send(Message::Text(payload.to_string()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Message::Binary(bytes) => {
                let text = match String::from_utf8(bytes.to_vec()) {
                    Ok(t) => t,
                    Err(_) => {
                        let err = mcp_error_value(
                            json!(null),
                            -32700,
                            "Parse error: binary payload must be UTF-8 JSON-RPC text",
                            None,
                        );
                        let _ = socket.send(Message::Text(err.to_string())).await;
                        continue;
                    }
                };
                let rpc: Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(e) => {
                        let err = mcp_error_value(
                            json!(null),
                            -32700,
                            format!("Parse error: {}", e),
                            None,
                        );
                        let _ = socket.send(Message::Text(err.to_string())).await;
                        continue;
                    }
                };
                let (_status, payload) =
                    handle_mcp_rpc_request(&state, rpc, None, Some(&auth)).await;
                if socket
                    .send(Message::Text(payload.to_string()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Message::Ping(payload) => {
                if socket.send(Message::Pong(payload)).await.is_err() {
                    break;
                }
            }
            Message::Pong(_) => {}
            Message::Close(_) => break,
        }
    }
    info!(token_id = %auth.token_id, "mcp/ws session ended");
}

#[derive(Default)]
struct McpRbCas {
    store: HashMap<String, Vec<u8>>,
}

impl rb_vm::CasProvider for McpRbCas {
    fn put(&mut self, bytes: &[u8]) -> rb_vm::Cid {
        let hash = blake3::hash(bytes);
        let cid = format!("b3:{}", hex::encode(hash.as_bytes()));
        self.store.insert(cid.clone(), bytes.to_vec());
        rb_vm::Cid(cid)
    }

    fn get(&self, cid: &rb_vm::Cid) -> Option<Vec<u8>> {
        self.store.get(&cid.0).cloned()
    }
}

struct McpRbSigner;

impl rb_vm::SignProvider for McpRbSigner {
    fn sign_jws(&self, _payload_nrf_bytes: &[u8]) -> Vec<u8> {
        vec![0_u8; 64]
    }

    fn kid(&self) -> String {
        "did:key:zMcpWs#rb".to_string()
    }
}

struct McpRbCanon;

impl rb_vm::canon::CanonProvider for McpRbCanon {
    fn canon(&self, v: serde_json::Value) -> serde_json::Value {
        rb_vm::RhoCanon.canon(v)
    }
}

pub(crate) async fn dispatch_tool_call(
    state: &AppState,
    tool_name: &str,
    arguments: &Value,
    id: Value,
    mcp_headers: Option<&HeaderMap>,
    ws_auth: Option<&McpWsAuth>,
) -> (StatusCode, Json<Value>) {
    let canonical_tool = canonical_tool_name(tool_name);

    match canonical_tool {
        "ubl.deliver" => {
            let chip = arguments.get("chip").cloned().unwrap_or(json!({}));
            let bytes = serde_json::to_vec(&chip).unwrap_or_default();
            let (status, _headers, payload) =
                submit_chip_bytes(state, mcp_headers, ws_auth.is_some(), &bytes).await;
            if status.is_success() {
                (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "result": { "content": [{ "type": "text", "text": serde_json::to_string(&payload).unwrap_or_default() }] }
                    })),
                )
            } else {
                let (mcp_code, message) = serde_json::from_value::<UblError>(payload.clone())
                    .map(|e| (e.code.mcp_code(), e.message))
                    .unwrap_or_else(|_| {
                        let code = if status == StatusCode::TOO_MANY_REQUESTS {
                            -32006
                        } else if status == StatusCode::BAD_REQUEST
                            || status == StatusCode::UNPROCESSABLE_ENTITY
                        {
                            -32602
                        } else if status == StatusCode::UNAUTHORIZED {
                            -32001
                        } else if status == StatusCode::FORBIDDEN {
                            -32003
                        } else if status == StatusCode::NOT_FOUND {
                            -32004
                        } else if status == StatusCode::CONFLICT {
                            -32005
                        } else if status == StatusCode::SERVICE_UNAVAILABLE {
                            -32000
                        } else {
                            -32603
                        };
                        (code, format!("HTTP {}", status.as_u16()))
                    });
                (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": { "code": mcp_code, "message": message, "data": payload }
                    })),
                )
            }
        }

        "ubl.query" => {
            let cid = arguments.get("cid").and_then(|v| v.as_str()).unwrap_or("");
            match state.chip_store.get_chip(cid).await {
                Ok(Some(chip)) => (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "result": { "content": [{ "type": "text", "text": serde_json::to_string(&json!({
                            "cid": chip.cid, "chip_type": chip.chip_type,
                            "chip_data": chip.chip_data, "receipt_cid": chip.receipt_cid,
                        })).unwrap_or_default() }] }
                    })),
                ),
                Ok(None) => (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": { "code": -32004, "message": format!("Chip {} not found", cid) }
                    })),
                ),
                Err(e) => (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": { "code": -32603, "message": e.to_string() }
                    })),
                ),
            }
        }

        "ubl.receipt" => {
            let cid = arguments.get("cid").and_then(|v| v.as_str()).unwrap_or("");
            if cid.is_empty() {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": { "code": -32602, "message": "missing required argument: cid" }
                    })),
                );
            }
            let Some(store) = state.durable_store.as_ref() else {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": { "code": -32000, "message": "receipt store unavailable" }
                    })),
                );
            };
            match store.get_receipt(cid) {
                Ok(Some(receipt)) => (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "result": { "content": [{ "type": "text", "text": serde_json::to_string(&receipt).unwrap_or_default() }] }
                    })),
                ),
                Ok(None) => (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": { "code": -32004, "message": format!("Receipt {} not found", cid) }
                    })),
                ),
                Err(e) => (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": { "code": -32603, "message": e.to_string() }
                    })),
                ),
            }
        }

        "ubl.verify" => {
            let cid = arguments.get("cid").and_then(|v| v.as_str()).unwrap_or("");
            match state.chip_store.get_chip(cid).await {
                Ok(Some(chip)) => {
                    let verified = match ubl_ai_nrf1::to_nrf1_bytes(&chip.chip_data) {
                        Ok(nrf) => ubl_ai_nrf1::compute_cid(&nrf)
                            .map(|c| c == cid)
                            .unwrap_or(false),
                        Err(_) => false,
                    };
                    (
                        StatusCode::OK,
                        Json(json!({
                            "jsonrpc": "2.0", "id": id,
                            "result": { "content": [{ "type": "text", "text": serde_json::to_string(&json!({
                                "cid": cid, "verified": verified
                            })).unwrap_or_default() }] }
                        })),
                    )
                }
                Ok(None) => (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": { "code": -32004, "message": format!("Chip {} not found", cid) }
                    })),
                ),
                Err(e) => (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": { "code": -32603, "message": e.to_string() }
                    })),
                ),
            }
        }

        "ubl.receipt.trace" => {
            let cid = arguments.get("cid").and_then(|v| v.as_str()).unwrap_or("");
            if cid.is_empty() {
                return (
                    StatusCode::OK,
                    Json(mcp_error_value(id, -32602, "missing required argument: cid", None)),
                );
            }

            if let Some(store) = state.durable_store.as_ref() {
                match store.get_receipt(cid) {
                    Ok(Some(receipt_json)) => {
                        if let Err(ubl_err) = verify_receipt_auth_chain(cid, &receipt_json) {
                            return (
                                StatusCode::OK,
                                Json(mcp_error_value(
                                    id,
                                    ubl_err.code.mcp_code(),
                                    ubl_err.message.clone(),
                                    Some(ubl_err.to_json()),
                                )),
                            );
                        }
                    }
                    Ok(None) => {
                        return (
                            StatusCode::OK,
                            Json(mcp_error_value(
                                id,
                                -32004,
                                format!("Receipt {} not found", cid),
                                None,
                            )),
                        );
                    }
                    Err(e) => {
                        return (
                            StatusCode::OK,
                            Json(mcp_error_value(
                                id,
                                -32603,
                                format!("Receipt fetch failed: {}", e),
                                None,
                            )),
                        );
                    }
                }
            }

            match state.chip_store.get_chip_by_receipt_cid(cid).await {
                Ok(Some(chip)) => (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "result": { "content": [{ "type": "text", "text": serde_json::to_string(&json!({
                            "receipt_cid": cid,
                            "chip_cid": chip.cid,
                            "chip_type": chip.chip_type,
                            "execution_metadata": chip.execution_metadata,
                        })).unwrap_or_default() }] }
                    })),
                ),
                Ok(None) => (
                    StatusCode::OK,
                    Json(mcp_error_value(id, -32004, format!("Receipt {} not found", cid), None)),
                ),
                Err(e) => (
                    StatusCode::OK,
                    Json(mcp_error_value(id, -32603, e.to_string(), None)),
                ),
            }
        }

        "ubl.cid" => {
            let payload = arguments
                .get("value")
                .or_else(|| arguments.get("json"))
                .or_else(|| arguments.get("chip"))
                .cloned()
                .unwrap_or(json!({}));
            match ubl_ai_nrf1::to_nrf1_bytes(&payload)
                .and_then(|bytes| ubl_ai_nrf1::compute_cid(&bytes))
            {
                Ok(cid) => (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc":"2.0", "id": id,
                        "result": { "content": [{ "type":"text", "text": serde_json::to_string(&json!({
                            "cid": cid
                        })).unwrap_or_default() }]}
                    })),
                ),
                Err(e) => (
                    StatusCode::OK,
                    Json(mcp_error_value(
                        id,
                        -32602,
                        format!("CID compute failed: {}", e),
                        None,
                    )),
                ),
            }
        }

        "ubl.rb.execute" => {
            let bytecode_hex = arguments
                .get("bytecode_hex")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if bytecode_hex.is_empty() {
                return (
                    StatusCode::OK,
                    Json(mcp_error_value(id, -32602, "missing required argument: bytecode_hex", None)),
                );
            }

            let fuel_limit = arguments
                .get("fuel_limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(1_000_000)
                .max(1);
            let bytecode = match hex::decode(bytecode_hex) {
                Ok(v) => v,
                Err(e) => {
                    return (
                        StatusCode::OK,
                        Json(mcp_error_value(id, -32602, format!("invalid bytecode_hex: {}", e), None)),
                    );
                }
            };
            let instructions = match rb_vm::tlv::decode_stream(&bytecode) {
                Ok(v) => v,
                Err(e) => {
                    return (
                        StatusCode::OK,
                        Json(mcp_error_value(id, -32602, format!("invalid bytecode stream: {}", e), None)),
                    );
                }
            };

            let signer = McpRbSigner;
            let mut vm = rb_vm::Vm::new(
                rb_vm::VmConfig {
                    fuel_limit,
                    ghost: false,
                    trace: true,
                },
                McpRbCas::default(),
                &signer,
                McpRbCanon,
                vec![],
            );

            match vm.run(&instructions) {
                Ok(outcome) => (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc":"2.0", "id": id,
                        "result": { "content": [{ "type":"text", "text": serde_json::to_string(&json!({
                            "rc_cid": outcome.rc_cid.map(|c| c.0),
                            "rc_sig": outcome.rc_sig,
                            "rc_payload_cid": outcome.rc_payload_cid.map(|c| c.0),
                            "steps": outcome.steps,
                            "fuel_used": outcome.fuel_used,
                            "trace_len": outcome.trace.len(),
                        })).unwrap_or_default() }]}
                    })),
                ),
                Err(e) => (
                    StatusCode::OK,
                    Json(mcp_error_value(id, -32602, format!("rb execute failed: {}", e), None)),
                ),
            }
        }

        "ubl.narrate" => {
            let receipt_cid = arguments.get("cid").and_then(|v| v.as_str()).unwrap_or("");
            let persist = arguments
                .get("persist")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if receipt_cid.is_empty() {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": { "code": -32602, "message": "missing required argument: cid" }
                    })),
                );
            }

            let chip = match state.chip_store.get_chip_by_receipt_cid(receipt_cid).await {
                Ok(Some(chip)) => chip,
                Ok(None) => {
                    return (
                        StatusCode::OK,
                        Json(json!({
                            "jsonrpc": "2.0", "id": id,
                            "error": { "code": -32004, "message": format!("Receipt {} not found", receipt_cid) }
                        })),
                    );
                }
                Err(e) => {
                    return (
                        StatusCode::OK,
                        Json(json!({
                            "jsonrpc": "2.0", "id": id,
                            "error": { "code": -32603, "message": e.to_string() }
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
            let summary = format!(
                "{} processed as allow in {}ms (fuel {}, policies {}).",
                chip.chip_type, latency_ms, fuel, policy_count
            );
            let narration = json!({
                "@type": "ubl/advisory.narration",
                "receipt_cid": receipt_cid,
                "chip_cid": chip.cid,
                "chip_type": chip.chip_type,
                "decision": "allow",
                "world": world,
                "policy_count": policy_count,
                "latency_ms": latency_ms,
                "fuel_consumed": fuel,
                "summary": summary,
                "generated_at": chrono::Utc::now().to_rfc3339(),
            });

            let mut persisted_advisory_cid: Option<String> = None;
            if persist {
                let adv = Advisory::new(
                    state.advisory_engine.passport_cid.clone(),
                    "narrate".to_string(),
                    receipt_cid.to_string(),
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
                    .store_executed_chip(body, receipt_cid.to_string(), metadata)
                    .await
                {
                    Ok(adv_cid) => persisted_advisory_cid = Some(adv_cid),
                    Err(e) => {
                        return (
                            StatusCode::OK,
                            Json(json!({
                                "jsonrpc": "2.0", "id": id,
                                "error": { "code": -32603, "message": format!("narration persist failed: {}", e) }
                            })),
                        );
                    }
                }
            }

            (
                StatusCode::OK,
                Json(json!({
                    "jsonrpc": "2.0", "id": id,
                    "result": { "content": [{ "type": "text", "text": serde_json::to_string(&json!({
                        "receipt_cid": receipt_cid,
                        "narration": narration,
                        "persisted_advisory_cid": persisted_advisory_cid,
                    })).unwrap_or_default() }] }
                })),
            )
        }

        "registry.listTypes" => {
            let types: Vec<Value> = state
                .manifest
                .chip_types
                .iter()
                .map(|ct| {
                    json!({
                        "type": ct.chip_type,
                        "description": ct.description,
                        "required_cap": ct.required_cap,
                    })
                })
                .collect();
            (
                StatusCode::OK,
                Json(json!({
                    "jsonrpc": "2.0", "id": id,
                    "result": { "content": [{ "type": "text", "text": serde_json::to_string(&types).unwrap_or_default() }] }
                })),
            )
        }

        _ => (
            StatusCode::OK,
            Json(json!({
                "jsonrpc": "2.0", "id": id,
                "error": { "code": -32601, "message": format!("Tool not found: {}", tool_name) }
            })),
        ),
    }
}
