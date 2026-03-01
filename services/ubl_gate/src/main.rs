//! UBL Gate — the single HTTP entry point for the UBL pipeline.
//!
//! Every mutation is a chip. Every chip goes through KNOCK→WA→CHECK→TR→WF.
//! Every output is a receipt. Nothing bypasses the gate.

use axum::{
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};
use ubl_chipstore::{ChipStore, SledBackend};
use ubl_eventstore::EventStore;
use ubl_runtime::advisory::AdvisoryEngine;
use ubl_runtime::durable_store::DurableStore;
use ubl_runtime::event_bus::EventBus;
use ubl_runtime::manifest::GateManifest;
use ubl_runtime::outbox_dispatcher::OutboxDispatcher;
use ubl_runtime::policy_loader::InMemoryPolicyStorage;
use ubl_runtime::UblPipeline;

mod metrics;
mod utils;
mod state;
mod outbox;
mod events;
mod advisor;
mod templates;
mod console;
mod audit;
mod registry;
mod chip;
mod llm;
mod receipt;
mod mcp;

use state::{AppState, McpTokenRateLimiter, WriteAccessPolicy};
use utils::{
    env_opt_trim, init_tracing,
    load_canon_rate_limiter, manifest_base_url_from_env,
    public_receipt_origin_from_env, public_receipt_path_from_env,
};
use outbox::{deliver_emit_receipt_event, outbox_endpoint_from_env};
use events::{
    search_events, stream_events,
    to_hub_event,
};
use advisor::{advisor_snapshots, advisor_tap};
use console::{
    console_events_partial, console_kpis_partial, console_mock24h_partial,
    console_page, mock24h_api,
};
use audit::{
    audit_page, audit_table_partial, list_audit_reports,
    list_audit_snapshots, list_audit_compactions, console_receipt_page,
};
use chip::{create_chip, verify_chip, get_chip, get_runtime_attestation, metrics_handler};
use receipt::{get_receipt, get_receipt_public_url, get_passport_advisories, verify_advisory,
    get_receipt_trace, narrate_receipt, narrate_receipt_stream};
use mcp::{
    openapi_spec, mcp_manifest, webmcp_manifest, mcp_rpc_sse, mcp_rpc,
    mcp_ws_upgrade,
};
use registry::{
    registry_page, registry_table_partial, registry_type_page, registry_kat_test,
    registry_types, registry_type_detail, registry_type_version,
};
use llm::{
    ui_llm_panel, ui_llm_panel_stream,
};



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    info!("starting UBL MASTER Gate");

    // Initialize shared components
    let _event_bus = Arc::new(EventBus::new());
    let backend = Arc::new(SledBackend::new("./data/chips")?);
    let chip_store = Arc::new(ChipStore::new_with_rebuild(backend).await?);

    let storage = InMemoryPolicyStorage::new();
    let mut pipeline = UblPipeline::with_chip_store(Box::new(storage), chip_store.clone());

    // Wire AdvisoryEngine for post-CHECK / post-WF advisory chips
    let advisory_engine = Arc::new(AdvisoryEngine::new(
        "b3:gate-passport".to_string(),
        "ubl-gate/0.1".to_string(),
        "a/system/t/gate".to_string(),
    ));
    pipeline.set_advisory_engine(advisory_engine.clone());

    // Wire NDJSON audit ledger — append-only log alongside Sled CAS
    let ledger = Arc::new(ubl_runtime::ledger::NdjsonLedger::new("./data/ledger"));
    pipeline.set_ledger(ledger);

    let pipeline = Arc::new(pipeline);

    // Bootstrap genesis chip — self-signed root of all policy
    match pipeline.bootstrap_genesis().await {
        Ok(cid) => info!(%cid, "genesis chip bootstrapped"),
        Err(e) => error!(error = %e, "FATAL: genesis bootstrap failed"),
    }

    // Start outbox dispatcher workers when SQLite durability is enabled.
    let durable_store = match DurableStore::from_env() {
        Ok(Some(store)) => {
            let store = Arc::new(store);
            let workers: usize = std::env::var("UBL_OUTBOX_WORKERS")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(1)
                .max(1);
            let outbox_endpoint = outbox_endpoint_from_env();
            if let Some(ref endpoint) = outbox_endpoint {
                info!(workers, endpoint = %endpoint, "outbox dispatcher started");
            } else {
                warn!(
                    workers,
                    "UBL_OUTBOX_ENDPOINT not set; emit_receipt outbox events will be dropped"
                );
            }
            let outbox_http_client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?;
            metrics::set_outbox_pending(store.outbox_pending().unwrap_or(0));

            for worker_id in 0..workers {
                let dispatcher = OutboxDispatcher::new((*store).clone()).with_backoff(2, 300);
                let store_for_metrics = store.clone();
                let outbox_endpoint_for_worker = outbox_endpoint.clone();
                let outbox_http_client_for_worker = outbox_http_client.clone();
                tokio::spawn(async move {
                    loop {
                        let processed = dispatcher
                            .run_once_async(64, |event| {
                                let outbox_endpoint = outbox_endpoint_for_worker.clone();
                                let outbox_http_client = outbox_http_client_for_worker.clone();
                                async move {
                                    if event.event_type == "emit_receipt" {
                                        return deliver_emit_receipt_event(
                                            &outbox_http_client,
                                            outbox_endpoint.as_deref(),
                                            event,
                                        )
                                        .await;
                                    }
                                    metrics::inc_outbox_retry();
                                    Err(format!("unknown outbox event type: {}", event.event_type))
                                }
                            })
                            .await;

                        match processed {
                            Ok(processed_count) => {
                                metrics::set_outbox_pending(
                                    store_for_metrics.outbox_pending().unwrap_or_default(),
                                );
                                if processed_count == 0 {
                                    tokio::time::sleep(Duration::from_millis(500)).await;
                                }
                            }
                            Err(e) => {
                                metrics::inc_outbox_retry();
                                warn!(worker_id, error = %e, "outbox worker error");
                                tokio::time::sleep(Duration::from_secs(1)).await;
                            }
                        }
                    }
                });
            }
            Some(store)
        }
        Ok(None) => None,
        Err(e) => {
            warn!(error = %e, "durable store init failed for gate");
            None
        }
    };

    let event_store = match EventStore::from_env() {
        Ok(Some(store)) => Some(Arc::new(store)),
        Ok(None) => None,
        Err(e) => {
            warn!(error = %e, "event store init failed for gate");
            None
        }
    };

    if let Some(store) = event_store.clone() {
        let mut rx = pipeline.event_bus.subscribe();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        let hub = to_hub_event(&event);
                        let stage = hub
                            .get("stage")
                            .and_then(|v| v.as_str())
                            .unwrap_or("UNKNOWN")
                            .to_string();
                        let world = hub
                            .get("@world")
                            .and_then(|v| v.as_str())
                            .unwrap_or("a/system")
                            .to_string();
                        metrics::inc_events_ingested(&stage, &world);
                        if let Err(e) = store.append_event_json(&hub) {
                            warn!(error = %e, "event store append failed");
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        metrics::inc_events_stream_dropped("hub_lagged");
                        warn!(skipped, "event hub ingestion lagged");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        info!("event hub ingestion task started");
    }

    let mut manifest_cfg = GateManifest::default();
    manifest_cfg.base_url = manifest_base_url_from_env();
    let manifest = Arc::new(manifest_cfg);
    let mcp_token_rate_limiter = Arc::new(McpTokenRateLimiter::from_env());
    let write_access_policy = Arc::new(WriteAccessPolicy::from_env());
    let public_receipt_origin = public_receipt_origin_from_env();
    let public_receipt_path = public_receipt_path_from_env();
    let genesis_pubkey_sha256 = env_opt_trim("UBL_GENESIS_PUBKEY_SHA256");
    let release_commit = env_opt_trim("UBL_RELEASE_COMMIT");
    let gate_binary_sha256 = env_opt_trim("UBL_GATE_BINARY_SHA256");

    let state = AppState {
        pipeline,
        chip_store,
        manifest,
        advisory_engine,
        http_client: reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?,
        canon_rate_limiter: load_canon_rate_limiter(),
        mcp_token_rate_limiter,
        durable_store,
        event_store,
        public_receipt_origin,
        public_receipt_path,
        genesis_pubkey_sha256,
        release_commit,
        gate_binary_sha256,
        write_access_policy,
    };

    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await?;
    info!("gate listening on http://0.0.0.0:4000");

    axum::serve(listener, app).await?;
    Ok(())
}
fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/console", get(console_page))
        .route("/console/_kpis", get(console_kpis_partial))
        .route("/console/_events", get(console_events_partial))
        .route("/console/_mock24h", get(console_mock24h_partial))
        .route("/console/receipt/:cid", get(console_receipt_page))
        .route("/ui/_llm", get(ui_llm_panel))
        .route("/audit/_table", get(audit_table_partial))
        .route("/audit/:kind", get(audit_page))
        .route("/registry", get(registry_page))
        .route("/registry/_table", get(registry_table_partial))
        .route("/registry/_kat_test", post(registry_kat_test))
        .route("/registry/*chip_type", get(registry_type_page))
        .route("/v1/audit/reports", get(list_audit_reports))
        .route("/v1/audit/snapshots", get(list_audit_snapshots))
        .route("/v1/audit/compactions", get(list_audit_compactions))
        .route("/v1/events", get(stream_events))
        .route("/v1/events/search", get(search_events))
        .route("/v1/mock/system24h", get(mock24h_api))
        .route("/v1/advisor/tap", get(advisor_tap))
        .route("/v1/advisor/snapshots", get(advisor_snapshots))
        .route("/v1/registry/types", get(registry_types))
        .route("/v1/registry/types/:chip_type", get(registry_type_detail))
        .route(
            "/v1/registry/types/:chip_type/versions/:ver",
            get(registry_type_version),
        )
        .route("/v1/runtime/attestation", get(get_runtime_attestation))
        .route("/v1/chips", post(create_chip))
        .route("/v1/chips/:cid", get(get_chip))
        .route("/v1/cas/:cid", get(get_chip))
        .route("/v1/receipts/:cid", get(get_receipt))
        .route("/v1/receipts/:cid/url", get(get_receipt_public_url))
        .route("/v1/receipts/:cid/trace", get(get_receipt_trace))
        .route("/v1/receipts/:cid/narrate", get(narrate_receipt))
        .route("/v1/receipts/:cid/narrate/stream", get(narrate_receipt_stream))
        .route("/ui/_llm/stream", get(ui_llm_panel_stream))
        .route(
            "/v1/passports/:cid/advisories",
            get(get_passport_advisories),
        )
        .route("/v1/advisories/:cid/verify", get(verify_advisory))
        .route("/v1/chips/:cid/verify", get(verify_chip))
        .route("/metrics", get(metrics_handler))
        .route("/openapi.json", get(openapi_spec))
        .route("/mcp/manifest", get(mcp_manifest))
        .route("/.well-known/webmcp.json", get(webmcp_manifest))
        .route("/mcp/rpc", get(mcp_rpc_sse).post(mcp_rpc))
        .route("/mcp/sse", get(mcp_rpc_sse))
        .route("/mcp/ws", get(mcp_ws_upgrade))
        .with_state(state)
}

async fn healthz() -> Json<Value> {
    Json(json!({"status": "ok", "system": "ubl-core", "pipeline": "KNOCK->WA->CHECK->TR->WF"}))
}

/// GET /v1/runtime/attestation — signed runtime self-attestation (PS3/F1).
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::{Method, Request, StatusCode};
    use crate::events::{hub_matches_query, EventStreamQuery};
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::ServiceExt;
    use ubl_chipstore::InMemoryBackend;
    use ubl_receipt::{PipelineStage, StageExecution, UnifiedReceipt};
    use ubl_runtime::durable_store::{CommitInput, NewOutboxEvent};
    use ubl_runtime::event_bus::ReceiptEvent;
    use ubl_runtime::rate_limit::{CanonRateLimiter, RateLimitConfig};

    const TEST_STAGE_SECRET_HEX: &str =
        "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

    fn test_state(canon_limiter: Option<Arc<CanonRateLimiter>>) -> AppState {
        let backend = Arc::new(InMemoryBackend::new());
        let chip_store = Arc::new(ChipStore::new(backend));
        let mut pipeline = UblPipeline::with_chip_store(
            Box::new(InMemoryPolicyStorage::new()),
            chip_store.clone(),
        );
        let advisory_engine = Arc::new(AdvisoryEngine::new(
            "b3:test-passport".to_string(),
            "ubl-gate/test".to_string(),
            "a/system/t/test".to_string(),
        ));
        pipeline.set_advisory_engine(advisory_engine.clone());
        AppState {
            pipeline: Arc::new(pipeline),
            chip_store,
            manifest: Arc::new(GateManifest::default()),
            advisory_engine,
            http_client: reqwest::Client::new(),
            canon_rate_limiter: canon_limiter,
            mcp_token_rate_limiter: Arc::new(McpTokenRateLimiter::from_env()),
            durable_store: None,
            event_store: None,
            public_receipt_origin: "https://logline.world".to_string(),
            public_receipt_path: "/r".to_string(),
            genesis_pubkey_sha256: Some("genesis-test-anchor".to_string()),
            release_commit: Some("test-commit".to_string()),
            gate_binary_sha256: Some("b3:test-runtime-hash".to_string()),
            write_access_policy: Arc::new(WriteAccessPolicy::open_for_tests()),
        }
    }

    fn test_state_with_receipt_store(receipt_cid: &str, receipt_json: Value) -> AppState {
        let mut state = test_state(None);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ubl_gate_receipts_{}.db", ts));
        let dsn = format!("file:{}?mode=rwc&_journal_mode=WAL", path.display());
        let store = DurableStore::new(dsn).unwrap();
        let input = CommitInput {
            receipt_cid: receipt_cid.to_string(),
            receipt_json,
            did: "did:key:ztest".to_string(),
            kid: "did:key:ztest#ed25519".to_string(),
            rt_hash: "b3:runtime-test".to_string(),
            decision: "allow".to_string(),
            idem_key: None,
            chain: vec![
                "b3:wa".to_string(),
                "b3:tr".to_string(),
                "b3:wf".to_string(),
            ],
            outbox_events: vec![NewOutboxEvent {
                event_type: "emit_receipt".to_string(),
                payload_json: json!({"receipt_cid": receipt_cid}),
            }],
            created_at: chrono::Utc::now().timestamp(),
            fail_after_receipt_write: false,
        };
        store.commit_wf_atomically(&input).unwrap();
        state.durable_store = Some(Arc::new(store));
        state
    }

    fn test_state_with_write_policy(policy: WriteAccessPolicy) -> AppState {
        let mut state = test_state(None);
        state.write_access_policy = Arc::new(policy);
        state
    }

    fn test_state_with_event_store(events: Vec<Value>) -> AppState {
        let mut state = test_state(None);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ubl_gate_events_{}", ts));
        let store = EventStore::open(path).unwrap();
        for event in events {
            store.append_event_json(&event).unwrap();
        }
        state.event_store = Some(Arc::new(store));
        state
    }

    fn make_unified_receipt_json(tampered: bool) -> (String, Value) {
        std::env::set_var("UBL_STAGE_SECRET", format!("hex:{}", TEST_STAGE_SECRET_HEX));

        let mut receipt = UnifiedReceipt::new(
            "a/test/t/main",
            "did:key:ztest",
            "did:key:ztest#ed25519",
            "0011223344556677",
        );
        receipt
            .append_stage(StageExecution {
                stage: PipelineStage::WriteAhead,
                timestamp: chrono::Utc::now().to_rfc3339(),
                input_cid: "b3:wa-input".to_string(),
                output_cid: Some("b3:wa-output".to_string()),
                fuel_used: None,
                policy_trace: vec![],
                vm_sig: None,
                vm_sig_payload_cid: None,
                auth_token: String::new(),
                duration_ms: 1,
            })
            .unwrap();
        let receipt_cid = receipt.receipt_cid.as_str().to_string();
        let mut receipt_json = receipt.to_json().unwrap();
        if tampered {
            if let Some(stage) = receipt_json
                .get_mut("stages")
                .and_then(|v| v.as_array_mut())
                .and_then(|arr| arr.first_mut())
            {
                stage["auth_token"] =
                    Value::String("hmac:00000000000000000000000000000000".to_string());
            }
        }

        (receipt_cid, receipt_json)
    }

    async fn seed_meta_chip(state: &AppState, body: Value, receipt_cid: &str) {
        let metadata: ubl_chipstore::ExecutionMetadata = serde_json::from_value(json!({
            "runtime_version": "test-runtime",
            "execution_time_ms": 1,
            "fuel_consumed": 0,
            "policies_applied": [],
            "executor_did": "did:key:ztest",
            "reproducible": true
        }))
        .unwrap();
        state
            .chip_store
            .store_executed_chip(body, receipt_cid.to_string(), metadata)
            .await
            .unwrap();
    }

    async fn seed_token_chip(state: &AppState, token_id: &str, world: &str, scope: &[&str]) {
        let metadata: ubl_chipstore::ExecutionMetadata = serde_json::from_value(json!({
            "runtime_version": "test-runtime",
            "execution_time_ms": 1,
            "fuel_consumed": 0,
            "policies_applied": [],
            "executor_did": "did:key:ztest",
            "reproducible": true
        }))
        .unwrap();

        let expires_at = (chrono::Utc::now() + chrono::Duration::hours(2)).to_rfc3339();
        let token = json!({
            "@type":"ubl/token",
            "@id": token_id,
            "@ver":"1.0",
            "@world": world,
            "user_cid":"b3:user-test",
            "scope": scope,
            "expires_at": expires_at,
            "kid":"did:key:ztest#ed25519"
        });

        state
            .chip_store
            .store_executed_chip(token, "b3:seed-token-receipt".to_string(), metadata)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn chips_endpoint_accepts_post_and_rejects_other_write_verbs() {
        let app = build_router(test_state(None));
        let req = Request::builder()
            .method(Method::PUT)
            .uri("/v1/chips")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn chips_endpoint_invalid_json_emits_knock_deny_receipt() {
        let app = build_router(test_state(None));
        let req = Request::builder()
            .method(Method::POST)
            .uri("/v1/chips")
            .header("content-type", "application/json")
            .body(Body::from("{invalid"))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["@type"], "ubl/error");
        assert_eq!(payload["code"], "KNOCK_REJECTED");
        assert!(payload["receipt_cid"]
            .as_str()
            .map(|s| s.starts_with("b3:"))
            .unwrap_or(false));
        assert_eq!(payload["receipt"]["@type"], "ubl/knock.deny.v1");
        assert_eq!(payload["receipt"]["decision"], "Deny");
        assert!(payload["receipt"]["knock_cid"]
            .as_str()
            .map(|s| s.starts_with("b3:"))
            .unwrap_or(false));
    }

    #[tokio::test]
    async fn cas_alias_route_is_read_only_and_reachable() {
        let app = build_router(test_state(None));
        let req = Request::builder()
            .method(Method::GET)
            .uri("/v1/cas/b3:missing")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn chips_endpoint_idempotent_replay_sets_header_and_same_receipt() {
        let app = build_router(test_state(None));
        let chip = json!({
            "@type": "ubl/document",
            "@id": "gate-idem-1",
            "@ver": "1.0",
            "@world": "a/test/t/main",
            "title": "hello"
        });

        let req1 = Request::builder()
            .method(Method::POST)
            .uri("/v1/chips")
            .header("content-type", "application/json")
            .body(Body::from(chip.to_string()))
            .unwrap();
        let res1 = app.clone().oneshot(req1).await.unwrap();
        assert_eq!(res1.status(), StatusCode::OK);
        assert!(res1.headers().get("X-UBL-Replay").is_none());
        let body1 = to_bytes(res1.into_body(), usize::MAX).await.unwrap();
        let v1: Value = serde_json::from_slice(&body1).unwrap();
        assert_eq!(v1["replayed"], Value::Bool(false));
        let cid1 = v1["receipt_cid"].as_str().unwrap().to_string();
        let receipt_url_1 = v1["receipt_url"].as_str().unwrap_or("");
        assert!(receipt_url_1.starts_with("https://logline.world/r#ubl:v1:"));

        let req2 = Request::builder()
            .method(Method::POST)
            .uri("/v1/chips")
            .header("content-type", "application/json")
            .body(Body::from(chip.to_string()))
            .unwrap();
        let res2 = app.clone().oneshot(req2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::OK);
        assert_eq!(
            res2.headers()
                .get("X-UBL-Replay")
                .and_then(|v| v.to_str().ok()),
            Some("true")
        );
        let body2 = to_bytes(res2.into_body(), usize::MAX).await.unwrap();
        let v2: Value = serde_json::from_slice(&body2).unwrap();
        assert_eq!(v2["replayed"], Value::Bool(true));
        let cid2 = v2["receipt_cid"].as_str().unwrap().to_string();
        let receipt_url_2 = v2["receipt_url"].as_str().unwrap_or("");
        assert_eq!(receipt_url_1, receipt_url_2);
        assert_eq!(cid1, cid2);
    }

    #[tokio::test]
    async fn chips_endpoint_requires_api_key_for_private_write_when_enabled() {
        let app = build_router(test_state_with_write_policy(WriteAccessPolicy {
            auth_required: true,
            api_keys: vec!["k-test".to_string()],
            public_worlds: vec!["a/chip-registry/t/public".to_string()],
            public_types: vec!["ubl/document".to_string()],
        }));
        let chip = json!({
            "@type": "ubl/document",
            "@id": "guard-private-1",
            "@ver": "1.0",
            "@world": "a/private/t/main",
            "title": "guard"
        });

        let req = Request::builder()
            .method(Method::POST)
            .uri("/v1/chips")
            .header("content-type", "application/json")
            .body(Body::from(chip.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["@type"], "ubl/error");
        assert_eq!(v["code"], "UNAUTHORIZED");
        assert_eq!(v["decision"], "Deny");
        assert!(v["receipt_cid"]
            .as_str()
            .map(|s| s.starts_with("b3:"))
            .unwrap_or(false));
    }

    #[tokio::test]
    async fn chips_endpoint_allows_public_lane_without_api_key() {
        let app = build_router(test_state_with_write_policy(WriteAccessPolicy {
            auth_required: true,
            api_keys: vec!["k-test".to_string()],
            public_worlds: vec!["a/chip-registry/t/public".to_string()],
            public_types: vec!["ubl/document".to_string()],
        }));
        let chip = json!({
            "@type": "ubl/document",
            "@id": "guard-public-1",
            "@ver": "1.0",
            "@world": "a/chip-registry/t/public",
            "title": "public lane"
        });

        let req = Request::builder()
            .method(Method::POST)
            .uri("/v1/chips")
            .header("content-type", "application/json")
            .body(Body::from(chip.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn chips_endpoint_allows_private_write_with_valid_api_key() {
        let app = build_router(test_state_with_write_policy(WriteAccessPolicy {
            auth_required: true,
            api_keys: vec!["k-test".to_string()],
            public_worlds: vec!["a/chip-registry/t/public".to_string()],
            public_types: vec!["ubl/document".to_string()],
        }));
        let chip = json!({
            "@type": "ubl/document",
            "@id": "guard-private-2",
            "@ver": "1.0",
            "@world": "a/private/t/main",
            "title": "private lane"
        });

        let req = Request::builder()
            .method(Method::POST)
            .uri("/v1/chips")
            .header("content-type", "application/json")
            .header("x-api-key", "k-test")
            .body(Body::from(chip.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn chips_endpoint_allows_private_write_with_valid_bearer_token() {
        let state = test_state_with_write_policy(WriteAccessPolicy {
            auth_required: true,
            api_keys: vec![],
            public_worlds: vec!["a/chip-registry/t/public".to_string()],
            public_types: vec!["ubl/document".to_string()],
        });
        seed_token_chip(&state, "tok-write-1", "a/private/t/main", &["write"]).await;
        let app = build_router(state);

        let chip = json!({
            "@type": "ubl/document",
            "@id": "guard-private-bearer-1",
            "@ver": "1.0",
            "@world": "a/private/t/main",
            "title": "private lane with bearer"
        });

        let req = Request::builder()
            .method(Method::POST)
            .uri("/v1/chips")
            .header("content-type", "application/json")
            .header("authorization", "Bearer tok-write-1")
            .body(Body::from(chip.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn chips_endpoint_denies_private_write_when_bearer_world_mismatch() {
        let state = test_state_with_write_policy(WriteAccessPolicy {
            auth_required: true,
            api_keys: vec![],
            public_worlds: vec!["a/chip-registry/t/public".to_string()],
            public_types: vec!["ubl/document".to_string()],
        });
        seed_token_chip(
            &state,
            "tok-write-wrong-world",
            "a/chip-registry/t/public",
            &["write"],
        )
        .await;
        let app = build_router(state);

        let chip = json!({
            "@type": "ubl/document",
            "@id": "guard-private-bearer-world-1",
            "@ver": "1.0",
            "@world": "a/private/t/main",
            "title": "private lane with bearer world mismatch"
        });

        let req = Request::builder()
            .method(Method::POST)
            .uri("/v1/chips")
            .header("content-type", "application/json")
            .header("authorization", "Bearer tok-write-wrong-world")
            .body(Body::from(chip.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["code"], "POLICY_DENIED");
        assert_eq!(v["decision"], "Deny");
        assert!(v["message"]
            .as_str()
            .unwrap_or("")
            .contains("does not authorize target world"));
    }

    #[tokio::test]
    async fn mcp_tools_call_requires_api_key_for_private_write_when_enabled() {
        let app = build_router(test_state_with_write_policy(WriteAccessPolicy {
            auth_required: true,
            api_keys: vec!["k-test".to_string()],
            public_worlds: vec!["a/chip-registry/t/public".to_string()],
            public_types: vec!["ubl/document".to_string()],
        }));

        let rpc = json!({
            "jsonrpc":"2.0",
            "id":"m1",
            "method":"tools/call",
            "params":{
                "name":"ubl.deliver",
                "arguments":{
                    "chip":{
                        "@type":"ubl/document",
                        "@id":"mcp-private-1",
                        "@ver":"1.0",
                        "@world":"a/private/t/main",
                        "title":"mcp guard"
                    }
                }
            }
        });

        let denied_req = Request::builder()
            .method(Method::POST)
            .uri("/mcp/rpc")
            .header("content-type", "application/json")
            .body(Body::from(rpc.to_string()))
            .unwrap();
        let denied_res = app.clone().oneshot(denied_req).await.unwrap();
        assert_eq!(denied_res.status(), StatusCode::OK);
        let denied_body = to_bytes(denied_res.into_body(), usize::MAX).await.unwrap();
        let denied_json: Value = serde_json::from_slice(&denied_body).unwrap();
        assert_eq!(denied_json["error"]["code"], -32001);

        let allowed_req = Request::builder()
            .method(Method::POST)
            .uri("/mcp/rpc")
            .header("content-type", "application/json")
            .header("x-api-key", "k-test")
            .body(Body::from(rpc.to_string()))
            .unwrap();
        let allowed_res = app.oneshot(allowed_req).await.unwrap();
        assert_eq!(allowed_res.status(), StatusCode::OK);
        let allowed_body = to_bytes(allowed_res.into_body(), usize::MAX).await.unwrap();
        let allowed_json: Value = serde_json::from_slice(&allowed_body).unwrap();
        assert!(allowed_json.get("result").is_some());
    }

    #[tokio::test]
    async fn mcp_tools_call_allows_private_write_with_valid_bearer_token() {
        let state = test_state_with_write_policy(WriteAccessPolicy {
            auth_required: true,
            api_keys: vec![],
            public_worlds: vec!["a/chip-registry/t/public".to_string()],
            public_types: vec!["ubl/document".to_string()],
        });
        seed_token_chip(
            &state,
            "tok-mcp-write-1",
            "a/private/t/main",
            &["mcp:write"],
        )
        .await;
        let app = build_router(state);

        let rpc = json!({
            "jsonrpc":"2.0",
            "id":"m2",
            "method":"tools/call",
            "params":{
                "name":"ubl.deliver",
                "arguments":{
                    "chip":{
                        "@type":"ubl/document",
                        "@id":"mcp-private-bearer-1",
                        "@ver":"1.0",
                        "@world":"a/private/t/main",
                        "title":"mcp bearer guard"
                    }
                }
            }
        });

        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp/rpc")
            .header("content-type", "application/json")
            .header("authorization", "Bearer tok-mcp-write-1")
            .body(Body::from(rpc.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert!(v.get("result").is_some());
    }

    #[tokio::test]
    async fn mcp_tools_call_denies_private_write_when_bearer_world_mismatch() {
        let state = test_state_with_write_policy(WriteAccessPolicy {
            auth_required: true,
            api_keys: vec![],
            public_worlds: vec!["a/chip-registry/t/public".to_string()],
            public_types: vec!["ubl/document".to_string()],
        });
        seed_token_chip(
            &state,
            "tok-mcp-write-wrong-world",
            "a/chip-registry/t/public",
            &["mcp:write"],
        )
        .await;
        let app = build_router(state);

        let rpc = json!({
            "jsonrpc":"2.0",
            "id":"m3",
            "method":"tools/call",
            "params":{
                "name":"ubl.deliver",
                "arguments":{
                    "chip":{
                        "@type":"ubl/document",
                        "@id":"mcp-private-bearer-world-1",
                        "@ver":"1.0",
                        "@world":"a/private/t/main",
                        "title":"mcp bearer world mismatch"
                    }
                }
            }
        });

        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp/rpc")
            .header("content-type", "application/json")
            .header("authorization", "Bearer tok-mcp-write-wrong-world")
            .body(Body::from(rpc.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["error"]["code"], -32003);
    }

    #[tokio::test]
    async fn chips_endpoint_canon_rate_limit_blocks_identical_payload_spam() {
        let limiter = Arc::new(CanonRateLimiter::new(RateLimitConfig::per_minute(1)));
        let app = build_router(test_state(Some(limiter)));
        let chip = json!({
            "@type": "ubl/document",
            "@id": "gate-rate-1",
            "@ver": "1.0",
            "@world": "a/test/t/main",
            "title": "same"
        });

        let req1 = Request::builder()
            .method(Method::POST)
            .uri("/v1/chips")
            .header("content-type", "application/json")
            .body(Body::from(chip.to_string()))
            .unwrap();
        let res1 = app.clone().oneshot(req1).await.unwrap();
        assert_eq!(res1.status(), StatusCode::OK);

        let req2 = Request::builder()
            .method(Method::POST)
            .uri("/v1/chips")
            .header("content-type", "application/json")
            .body(Body::from(chip.to_string()))
            .unwrap();
        let res2 = app.oneshot(req2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::TOO_MANY_REQUESTS);
        let body2 = to_bytes(res2.into_body(), usize::MAX).await.unwrap();
        let v2: Value = serde_json::from_slice(&body2).unwrap();
        assert_eq!(v2["code"], Value::String("TOO_MANY_REQUESTS".to_string()));
    }

    #[tokio::test]
    async fn receipts_endpoint_returns_raw_persisted_receipt() {
        let (receipt_cid, receipt_json) = make_unified_receipt_json(false);
        let app = build_router(test_state_with_receipt_store(&receipt_cid, receipt_json));

        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("/v1/receipts/{}", receipt_cid))
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["@type"], "ubl/receipt");
        assert_eq!(v["receipt_cid"], receipt_cid);
    }

    #[tokio::test]
    async fn receipt_public_url_endpoint_returns_canonical_link() {
        let (receipt_cid, receipt_json) = make_unified_receipt_json(false);
        let app = build_router(test_state_with_receipt_store(&receipt_cid, receipt_json));

        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("/v1/receipts/{}/url", receipt_cid))
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["@type"], "ubl/receipt.url");
        assert_eq!(v["receipt_cid"], receipt_cid);
        let receipt_url = v["receipt_url"].as_str().unwrap_or("");
        assert!(receipt_url.starts_with("https://logline.world/r#ubl:v1:"));
        assert_eq!(v["receipt_public"]["model"], "ubl:v1");
    }

    #[tokio::test]
    async fn chip_verify_returns_422_when_receipt_auth_chain_is_tampered() {
        let (receipt_cid, tampered_receipt_json) = make_unified_receipt_json(true);
        let state = test_state_with_receipt_store(&receipt_cid, tampered_receipt_json);

        let metadata: ubl_chipstore::ExecutionMetadata = serde_json::from_value(json!({
            "runtime_version": "test-runtime",
            "execution_time_ms": 1,
            "fuel_consumed": 0,
            "policies_applied": [],
            "executor_did": "did:key:ztest",
            "reproducible": true
        }))
        .unwrap();
        let chip_cid = state
            .chip_store
            .store_executed_chip(
                json!({
                    "@type": "ubl/document",
                    "@id": "tamper-test",
                    "@ver": "1.0",
                    "@world": "a/test/t/main",
                    "title": "tamper"
                }),
                receipt_cid.clone(),
                metadata,
            )
            .await
            .unwrap();

        let app = build_router(state);
        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("/v1/chips/{}/verify", chip_cid))
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["code"], "TAMPER_DETECTED");
    }

    #[tokio::test]
    async fn receipt_trace_returns_422_when_auth_chain_is_tampered() {
        let (receipt_cid, tampered_receipt_json) = make_unified_receipt_json(true);
        let state = test_state_with_receipt_store(&receipt_cid, tampered_receipt_json);

        let metadata: ubl_chipstore::ExecutionMetadata = serde_json::from_value(json!({
            "runtime_version": "test-runtime",
            "execution_time_ms": 1,
            "fuel_consumed": 0,
            "policies_applied": [],
            "executor_did": "did:key:ztest",
            "reproducible": true
        }))
        .unwrap();
        state
            .chip_store
            .store_executed_chip(
                json!({
                    "@type": "ubl/document",
                    "@id": "tamper-trace-test",
                    "@ver": "1.0",
                    "@world": "a/test/t/main",
                    "title": "tamper trace"
                }),
                receipt_cid.clone(),
                metadata,
            )
            .await
            .unwrap();

        let app = build_router(state);
        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("/v1/receipts/{}/trace", receipt_cid))
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["code"], "TAMPER_DETECTED");
    }

    #[tokio::test]
    async fn receipts_endpoint_unavailable_without_durable_store() {
        let app = build_router(test_state(None));
        let req = Request::builder()
            .method(Method::GET)
            .uri("/v1/receipts/b3:any")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn events_search_unavailable_without_event_store() {
        let app = build_router(test_state(None));
        let req = Request::builder()
            .method(Method::GET)
            .uri("/v1/events/search?world=a/acme")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn events_search_filters_world_and_decision() {
        let app = build_router(test_state_with_event_store(vec![
            json!({
                "@type": "ubl/event",
                "@ver": "1.0.0",
                "@id": "evt-allow-1",
                "@world": "a/acme/t/prod",
                "source": "pipeline",
                "stage": "WF",
                "when": "2026-02-18T12:00:00.000Z",
                "chip": {"type": "ubl/user", "id": "u1", "ver": "1.0"},
                "receipt": {"cid": "b3:r1", "decision": "ALLOW", "code": "ok"},
                "actor": {"kid": "did:key:z1#k1"},
            }),
            json!({
                "@type": "ubl/event",
                "@ver": "1.0.0",
                "@id": "evt-deny-1",
                "@world": "a/acme/t/prod",
                "source": "pipeline",
                "stage": "CHECK",
                "when": "2026-02-18T12:00:01.000Z",
                "chip": {"type": "ubl/user", "id": "u2", "ver": "1.0"},
                "receipt": {"cid": "b3:r2", "decision": "DENY", "code": "check.policy.deny"},
                "actor": {"kid": "did:key:z1#k1"},
            }),
            json!({
                "@type": "ubl/event",
                "@ver": "1.0.0",
                "@id": "evt-deny-2",
                "@world": "a/other/t/dev",
                "source": "pipeline",
                "stage": "CHECK",
                "when": "2026-02-18T12:00:02.000Z",
                "chip": {"type": "ubl/user", "id": "u3", "ver": "1.0"},
                "receipt": {"cid": "b3:r3", "decision": "DENY", "code": "check.policy.deny"},
                "actor": {"kid": "did:key:z1#k1"},
            }),
        ]));

        let req = Request::builder()
            .method(Method::GET)
            .uri("/v1/events/search?world=a/acme/t/prod&decision=deny")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["@type"], "ubl/events.search.response");
        assert_eq!(v["count"], 1);
        assert_eq!(v["events"][0]["@id"], "evt-deny-1");
    }

    #[tokio::test]
    async fn advisor_snapshots_unavailable_without_event_store() {
        let app = build_router(test_state(None));
        let req = Request::builder()
            .method(Method::GET)
            .uri("/v1/advisor/snapshots?window=5m")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn advisor_snapshots_returns_aggregates() {
        let now = chrono::Utc::now();
        let app = build_router(test_state_with_event_store(vec![
            json!({
                "@type": "ubl/event",
                "@ver": "1.0.0",
                "@id": "evt-adv-1",
                "@world": "a/acme/t/prod",
                "source": "pipeline",
                "stage": "CHECK",
                "when": now.to_rfc3339(),
                "chip": {"type": "ubl/user", "id": "u1", "ver": "1.0"},
                "receipt": {"cid": "b3:ra1", "decision": "DENY", "code": "check.policy.deny"},
                "perf": {"latency_ms": 10.0},
                "actor": {"kid": "did:key:z1#k1"},
            }),
            json!({
                "@type": "ubl/event",
                "@ver": "1.0.0",
                "@id": "evt-adv-2",
                "@world": "a/acme/t/prod",
                "source": "pipeline",
                "stage": "WF",
                "when": now.to_rfc3339(),
                "chip": {"type": "ubl/user", "id": "u2", "ver": "1.0"},
                "receipt": {"cid": "b3:ra2", "decision": "ALLOW", "code": "ok"},
                "perf": {"latency_ms": 20.0},
                "actor": {"kid": "did:key:z1#k1"},
            }),
        ]));

        let req = Request::builder()
            .method(Method::GET)
            .uri("/v1/advisor/snapshots?world=a/acme/t/prod&window=5m")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["@type"], "ubl/advisor.snapshot");
        assert_eq!(v["snapshot"]["counts"]["decision"]["ALLOW"], 1);
        assert_eq!(v["snapshot"]["counts"]["decision"]["DENY"], 1);
        assert_eq!(v["snapshot"]["counts"]["stage"]["CHECK"], 1);
        assert_eq!(v["snapshot"]["counts"]["stage"]["WF"], 1);
    }

    #[test]
    fn to_hub_event_maps_core_fields() {
        let event = ReceiptEvent {
            at_type: "ubl/event".to_string(),
            event_type: "ubl.receipt.wf".to_string(),
            schema_version: "1.0".to_string(),
            idempotency_key: "b3:receipt-1".to_string(),
            receipt_cid: "b3:receipt-1".to_string(),
            receipt_type: "ubl/user".to_string(),
            decision: Some("allow".to_string()),
            duration_ms: Some(12),
            timestamp: "2026-02-18T12:34:56.000Z".to_string(),
            pipeline_stage: "wf".to_string(),
            fuel_used: Some(7),
            rb_count: None,
            artifact_cids: vec!["b3:artifact-1".to_string()],
            metadata: json!({"@id":"chip-1","@ver":"1.0.0","code":"ok"}),
            input_cid: Some("b3:in".to_string()),
            output_cid: Some("b3:receipt-1".to_string()),
            binary_hash: Some("sha256:abc".to_string()),
            build_meta: Some(json!({"git":"abc123"})),
            world: Some("a/acme/t/prod".to_string()),
            actor: Some("did:key:z1#k1".to_string()),
            subject_did: Some("did:ubl:anon:b3:test".to_string()),
            knock_cid: Some("b3:knock".to_string()),
            latency_ms: Some(12),
        };

        let hub = to_hub_event(&event);
        assert_eq!(hub["@type"], "ubl/event");
        assert_eq!(hub["@ver"], "1.0.0");
        assert_eq!(hub["stage"], "WF");
        assert_eq!(hub["@world"], "a/acme/t/prod");
        assert_eq!(hub["chip"]["type"], "ubl/user");
        assert_eq!(hub["receipt"]["cid"], "b3:receipt-1");
        assert_eq!(hub["receipt"]["decision"], "ALLOW");
        assert_eq!(hub["perf"]["fuel"], 7);
    }

    #[test]
    fn hub_matches_query_applies_stage_and_world_filters() {
        let event = json!({
            "@type": "ubl/event",
            "@ver": "1.0.0",
            "@id": "evt-1",
            "@world": "a/acme/t/prod",
            "stage": "CHECK",
            "chip": {"type": "ubl/user"},
            "receipt": {"decision": "DENY", "code": "check.policy.deny"},
            "actor": {"kid": "did:key:z1#k1"}
        });

        let q_ok = EventStreamQuery {
            world: Some("a/acme/t/prod".to_string()),
            stage: Some("check".to_string()),
            decision: Some("deny".to_string()),
            code: Some("check.policy.deny".to_string()),
            chip_type: Some("ubl/user".to_string()),
            actor: Some("did:key:z1#k1".to_string()),
            since: None,
            limit: None,
        };
        assert!(hub_matches_query(&event, &q_ok));

        let q_bad_world = EventStreamQuery {
            world: Some("a/other".to_string()),
            ..q_ok
        };
        assert!(!hub_matches_query(&event, &q_bad_world));
    }

    #[tokio::test]
    async fn registry_types_materializes_meta_chips() {
        let state = test_state(None);
        seed_meta_chip(
            &state,
            json!({
                "@type":"ubl/meta.register",
                "@id":"reg-1",
                "@ver":"1.0",
                "@world":"a/acme/t/prod",
                "target_type":"acme/invoice",
                "description":"Invoice type",
                "type_version":"1.0",
                "schema":{
                    "required_fields":[{"name":"amount","field_type":"string","description":"Amount"}],
                    "optional_fields":[],
                    "required_cap":"invoice:create"
                },
                "kats":[{
                    "label":"allow invoice",
                    "input":{"@type":"acme/invoice","@id":"i1","@ver":"1.0","@world":"a/acme/t/prod","amount":"10.00"},
                    "expected_decision":"allow"
                }]
            }),
            "b3:r-meta-1",
        )
        .await;
        seed_meta_chip(
            &state,
            json!({
                "@type":"ubl/meta.describe",
                "@id":"desc-1",
                "@ver":"1.0",
                "@world":"a/acme/t/prod",
                "target_type":"acme/invoice",
                "description":"Invoice type updated",
                "docs_url":"https://example.com/acme-invoice"
            }),
            "b3:r-meta-2",
        )
        .await;
        seed_meta_chip(
            &state,
            json!({
                "@type":"ubl/meta.deprecate",
                "@id":"dep-1",
                "@ver":"1.0",
                "@world":"a/acme/t/prod",
                "target_type":"acme/invoice",
                "reason":"use acme/invoice.v2",
                "replacement_type":"acme/invoice.v2",
                "sunset_at":"2026-12-01T00:00:00Z"
            }),
            "b3:r-meta-3",
        )
        .await;
        let app = build_router(state);

        let req = Request::builder()
            .method(Method::GET)
            .uri("/v1/registry/types?world=a/acme/t/prod")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["@type"], "ubl/registry.types");
        assert_eq!(v["count"], 1);
        assert_eq!(v["types"][0]["type"], "acme/invoice");
        assert_eq!(v["types"][0]["deprecated"], true);
        assert_eq!(v["types"][0]["required_cap"], "invoice:create");
    }

    #[tokio::test]
    async fn registry_version_endpoint_returns_schema_and_kats() {
        let state = test_state(None);
        seed_meta_chip(
            &state,
            json!({
                "@type":"ubl/meta.register",
                "@id":"reg-v1",
                "@ver":"1.0",
                "@world":"a/acme/t/prod",
                "target_type":"acme/payment",
                "description":"Payment type",
                "type_version":"1.0",
                "schema":{
                    "required_fields":[{"name":"value","field_type":"string","description":"Value"}],
                    "optional_fields":[],
                    "required_cap":"payment:create"
                },
                "kats":[{
                    "label":"allow payment",
                    "input":{"@type":"acme/payment","@id":"p1","@ver":"1.0","@world":"a/acme/t/prod","value":"1"},
                    "expected_decision":"allow"
                }]
            }),
            "b3:r-meta-v1",
        )
        .await;
        let app = build_router(state);

        let req = Request::builder()
            .method(Method::GET)
            .uri("/v1/registry/types/acme%2Fpayment/versions/1.0")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["@type"], "ubl/registry.version");
        assert_eq!(v["type"], "acme/payment");
        assert_eq!(v["version"], "1.0");
        assert_eq!(v["required_cap"], "payment:create");
        assert_eq!(v["kats"][0]["label"], "allow payment");
    }

    #[tokio::test]
    async fn console_and_registry_pages_render_html() {
        let app = build_router(test_state(None));

        let console_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/console")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(console_res.status(), StatusCode::OK);
        let console_body = to_bytes(console_res.into_body(), usize::MAX).await.unwrap();
        let console_html = String::from_utf8(console_body.to_vec()).unwrap();
        assert!(console_html.contains("UBL Console"));

        let registry_res = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/registry")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(registry_res.status(), StatusCode::OK);
        let registry_body = to_bytes(registry_res.into_body(), usize::MAX)
            .await
            .unwrap();
        let registry_html = String::from_utf8(registry_body.to_vec()).unwrap();
        assert!(registry_html.contains("UBL Registry"));
    }

    #[tokio::test]
    async fn audit_pages_render_html() {
        let app = build_router(test_state(None));
        let reports_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/audit/reports")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reports_res.status(), StatusCode::OK);
        let reports_body = to_bytes(reports_res.into_body(), usize::MAX).await.unwrap();
        let reports_html = String::from_utf8(reports_body.to_vec()).unwrap();
        assert!(reports_html.contains("UBL Audit / reports"));

        let snapshots_res = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/audit/snapshots")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(snapshots_res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn audit_list_reports_returns_artifacts() {
        let state = test_state(None);
        seed_meta_chip(
            &state,
            json!({
                "@type":"ubl/audit.dataset.v1",
                "@id":"rpt-1",
                "@ver":"1.0.0",
                "@world":"a/acme/t/prod",
                "line_count": 3,
                "format": "ndjson"
            }),
            "b3:r-audit-1",
        )
        .await;
        let app = build_router(state);

        let res = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/v1/audit/reports?world=a/acme/t/prod")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["@type"], "ubl/audit.list");
        assert_eq!(v["kind"], "reports");
        assert_eq!(v["count"], 1);
        assert_eq!(v["rows"][0]["chip_type"], "ubl/audit.dataset.v1");
    }

    #[tokio::test]
    async fn registry_type_page_renders_for_wildcard_path() {
        let state = test_state(None);
        seed_meta_chip(
            &state,
            json!({
                "@type":"ubl/meta.register",
                "@id":"reg-html-1",
                "@ver":"1.0",
                "@world":"a/acme/t/prod",
                "target_type":"acme/invoice",
                "description":"Invoice type",
                "type_version":"1.0",
                "schema":{
                    "required_fields":[{"name":"amount","field_type":"string","description":"Amount"}],
                    "optional_fields":[],
                    "required_cap":"invoice:create"
                },
                "kats":[{
                    "label":"allow invoice",
                    "input":{"@type":"acme/invoice","@id":"i1","@ver":"1.0","@world":"a/acme/t/prod","amount":"10.00"},
                    "expected_decision":"allow"
                }]
            }),
            "b3:r-meta-html-1",
        )
        .await;
        let app = build_router(state);

        let res = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/registry/acme/invoice")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("Registry Type: acme/invoice"));
    }

    #[tokio::test]
    async fn registry_kat_test_endpoint_runs_and_renders_result() {
        let state = test_state(None);
        seed_meta_chip(
            &state,
            json!({
                "@type":"ubl/meta.register",
                "@id":"reg-kat-1",
                "@ver":"1.0",
                "@world":"a/acme/t/prod",
                "target_type":"acme/invoice",
                "description":"Invoice type",
                "type_version":"1.0",
                "schema":{
                    "required_fields":[{"name":"amount","field_type":"string","description":"Amount"}],
                    "optional_fields":[],
                    "required_cap":"invoice:create"
                },
                "kats":[{
                    "label":"allow invoice",
                    "input":{"@type":"acme/invoice","@id":"i-kat-1","@ver":"1.0","@world":"a/acme/t/prod","amount":"10.00"},
                    "expected_decision":"allow"
                }]
            }),
            "b3:r-meta-kat-1",
        )
        .await;
        let app = build_router(state);

        let res = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/registry/_kat_test")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(
                        "chip_type=acme%2Finvoice&version=1.0&kat_index=0",
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("KAT Result"));
        assert!(html.contains("allow invoice"));
    }
}
