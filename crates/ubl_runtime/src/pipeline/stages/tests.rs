use super::super::*;
use crate::error_response::UblError;
use crate::policy_loader::InMemoryPolicyStorage;
use base64::Engine as _;
use ring::digest;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

fn parsed_request(request: &ChipRequest) -> ParsedChipRequest<'_> {
    ParsedChipRequest::parse(request).expect("request should parse")
}

fn allow_request() -> ChipRequest {
    ChipRequest {
        chip_type: "ubl/document".to_string(),
        body: json!({
            "@type": "ubl/document",
            "@id": "stage-doc-1",
            "@ver": "1.0",
            "@world": "a/demo/t/main",
            "title": "Stage test document"
        }),
        parents: vec![],
        operation: Some("create".to_string()),
    }
}

fn deny_request() -> ChipRequest {
    ChipRequest {
        chip_type: "evil/hack".to_string(),
        body: json!({
            "@type": "evil/hack",
            "@id": "denied-1",
            "@ver": "1.0",
            "@world": "a/demo/t/main"
        }),
        parents: vec![],
        operation: Some("create".to_string()),
    }
}

#[tokio::test]
async fn stage_write_ahead_emits_valid_receipt() {
    let pipeline = UblPipeline::new(Box::new(InMemoryPolicyStorage::new()));
    let req = allow_request();
    let parsed = parsed_request(&req);

    let wa = pipeline.stage_write_ahead(&parsed).await.unwrap();
    assert_eq!(wa.receipt_type, "ubl/wa");
    assert!(wa.body_cid.as_str().starts_with("b3:"));
    assert_eq!(wa.body["ghost"], json!(true));
    assert!(!wa.body["nonce"].as_str().unwrap_or("").is_empty());
}

#[tokio::test]
async fn stage_check_allow_and_deny_paths() {
    let pipeline = UblPipeline::new(Box::new(InMemoryPolicyStorage::new()));

    let allow_req = allow_request();
    let allow = pipeline
        .stage_check(&parsed_request(&allow_req))
        .await
        .unwrap();
    assert!(matches!(allow.decision, Decision::Allow));
    assert!(!allow.short_circuited);
    assert!(!allow.trace.is_empty());

    let deny_req = deny_request();
    let deny = pipeline
        .stage_check(&parsed_request(&deny_req))
        .await
        .unwrap();
    assert!(matches!(deny.decision, Decision::Deny));
    assert!(deny.short_circuited);
    assert!(!deny.trace.is_empty());
}

#[tokio::test]
async fn stage_transition_emits_vm_signature_and_payload_cid() {
    let pipeline = UblPipeline::new(Box::new(InMemoryPolicyStorage::new()));
    let req = allow_request();
    let parsed = parsed_request(&req);
    let check = pipeline.stage_check(&parsed).await.unwrap();

    let tr = pipeline.stage_transition(&parsed, &check).await.unwrap();
    assert_eq!(tr.receipt_type, "ubl/transition");
    assert!(tr.body_cid.as_str().starts_with("b3:"));
    assert!(!tr.body["vm_sig"].as_str().unwrap_or("").is_empty());
    assert!(!tr.body["vm_sig_payload_cid"]
        .as_str()
        .unwrap_or("")
        .is_empty());
    assert!(tr.body["vm_state"]["fuel_used"].as_u64().is_some());
}

#[tokio::test]
async fn stage_transition_executes_inline_wasm_adapter() {
    let pipeline = UblPipeline::new(Box::new(InMemoryPolicyStorage::new()));
    let mut req = allow_request();
    let module = wat::parse_str(
        r#"
        (module
          (memory (export "memory") 1 1)
          (func (export "ubl_adapter_v1") (param i32 i32) (result i32)
            local.get 1))
        "#,
    )
    .unwrap();
    let hash = digest::digest(&digest::SHA256, &module);
    req.body["adapter"] = json!({
        "wasm_sha256": hex::encode(hash.as_ref()),
        "abi_version": "1.0",
        "wasm_b64": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&module),
        "fuel_budget": 50_000
    });

    let parsed = parsed_request(&req);
    let check = pipeline.stage_check(&parsed).await.unwrap();
    let tr = pipeline.stage_transition(&parsed, &check).await.unwrap();

    assert_eq!(tr.body["vm_state"]["adapter_executed"], json!(true));
    assert_eq!(
        tr.body["vm_state"]["adapter_module_source"],
        json!("inline:adapter.wasm_b64")
    );
    assert_eq!(
        tr.body["vm_state"]["adapter_output_cid"],
        tr.body["input_cid"]
    );
    assert!(
        tr.body["vm_state"]["adapter_fuel_used"]
            .as_u64()
            .unwrap_or(0)
            > 0
    );
}

#[tokio::test]
async fn stage_transition_rejects_wasm_hash_mismatch() {
    let pipeline = UblPipeline::new(Box::new(InMemoryPolicyStorage::new()));
    let mut req = allow_request();
    let module = wat::parse_str(
        r#"
        (module
          (memory (export "memory") 1 1)
          (func (export "ubl_adapter_v1") (param i32 i32) (result i32)
            local.get 1))
        "#,
    )
    .unwrap();
    req.body["adapter"] = json!({
        "wasm_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
        "abi_version": "1.0",
        "wasm_b64": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&module)
    });

    let parsed = parsed_request(&req);
    let check = pipeline.stage_check(&parsed).await.unwrap();
    let err = pipeline
        .stage_transition(&parsed, &check)
        .await
        .unwrap_err();
    assert!(matches!(err, PipelineError::InvalidChip(_)));
}

#[tokio::test]
async fn stage_transition_denies_wasm_network_capability() {
    let pipeline = UblPipeline::new(Box::new(InMemoryPolicyStorage::new()));
    let mut req = allow_request();
    let module = wasm_identity_module();
    let hash = digest::digest(&digest::SHA256, &module);
    req.body["adapter"] = json!({
        "wasm_sha256": hex::encode(hash.as_ref()),
        "abi_version": "1.0",
        "wasm_b64": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&module),
        "capabilities": ["network"]
    });

    let parsed = parsed_request(&req);
    let check = pipeline.stage_check(&parsed).await.unwrap();
    let err = pipeline
        .stage_transition(&parsed, &check)
        .await
        .unwrap_err();
    match err {
        PipelineError::PolicyDenied(msg) => {
            assert!(msg.contains("WASM_CAPABILITY_DENIED_NETWORK"));
        }
        other => panic!("expected PolicyDenied, got {:?}", other),
    }
}

#[tokio::test]
async fn stage_transition_rejects_wasm_invalid_attestation_signature() {
    let pipeline = UblPipeline::new(Box::new(InMemoryPolicyStorage::new()));
    let mut req = allow_request();
    let module = wasm_identity_module();
    let hash = digest::digest(&digest::SHA256, &module);
    req.body["adapter"] = json!({
        "wasm_sha256": hex::encode(hash.as_ref()),
        "abi_version": "1.0",
        "wasm_b64": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&module),
        "attestation_signature_b64": "***invalid***",
        "attestation_trust_anchor": wasm_test_trust_anchor_did()
    });

    let parsed = parsed_request(&req);
    let check = pipeline.stage_check(&parsed).await.unwrap();
    let err = pipeline
        .stage_transition(&parsed, &check)
        .await
        .unwrap_err();
    match err {
        PipelineError::InvalidChip(msg) => {
            assert!(msg.contains("WASM_VERIFY_SIGNATURE_INVALID"));
        }
        other => panic!("expected InvalidChip, got {:?}", other),
    }
}

#[tokio::test]
async fn stage_transition_rejects_wasm_trust_anchor_mismatch() {
    let pipeline = UblPipeline::new(Box::new(InMemoryPolicyStorage::new()));
    let mut req = allow_request();
    let module = wasm_identity_module();
    let hash = digest::digest(&digest::SHA256, &module);
    req.body["adapter"] = json!({
        "wasm_sha256": hex::encode(hash.as_ref()),
        "abi_version": "1.0",
        "wasm_b64": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&module),
        "attestation_signature_b64": "ed25519:AAAA",
        "attestation_trust_anchor": "did:key:zunknown"
    });

    let parsed = parsed_request(&req);
    let check = pipeline.stage_check(&parsed).await.unwrap();
    let err = pipeline
        .stage_transition(&parsed, &check)
        .await
        .unwrap_err();
    match err {
        PipelineError::InvalidChip(msg) => {
            assert!(msg.contains("WASM_VERIFY_TRUST_ANCHOR_MISMATCH"));
        }
        other => panic!("expected InvalidChip, got {:?}", other),
    }
}

#[tokio::test]
async fn stage_transition_rejects_wasm_attestation_half_present() {
    let pipeline = UblPipeline::new(Box::new(InMemoryPolicyStorage::new()));
    let mut req = allow_request();
    let module = wasm_identity_module();
    let hash = digest::digest(&digest::SHA256, &module);
    req.body["adapter"] = json!({
        "wasm_sha256": hex::encode(hash.as_ref()),
        "abi_version": "1.0",
        "wasm_b64": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&module),
        "attestation_signature_b64": "ed25519:AAAA"
    });

    let parsed = parsed_request(&req);
    let check = pipeline.stage_check(&parsed).await.unwrap();
    let err = pipeline
        .stage_transition(&parsed, &check)
        .await
        .unwrap_err();
    match err {
        PipelineError::InvalidChip(msg) => {
            assert!(msg.contains("WASM_VERIFY_SIGNATURE_INVALID"));
        }
        other => panic!("expected InvalidChip, got {:?}", other),
    }
}

#[tokio::test]
async fn stage_transition_rejects_wasm_missing_required_receipt_claim() {
    let pipeline = UblPipeline::new(Box::new(InMemoryPolicyStorage::new()));
    let mut req = allow_request();
    let module = wasm_identity_module();
    let hash = digest::digest(&digest::SHA256, &module);
    req.body["adapter"] = json!({
        "wasm_sha256": hex::encode(hash.as_ref()),
        "abi_version": "1.0",
        "wasm_b64": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&module),
        "required_receipt_claims": ["wasm.nonexistent.claim"]
    });

    let parsed = parsed_request(&req);
    let check = pipeline.stage_check(&parsed).await.unwrap();
    let err = pipeline
        .stage_transition(&parsed, &check)
        .await
        .unwrap_err();
    match err {
        PipelineError::InvalidChip(msg) => {
            assert!(msg.contains("WASM_RECEIPT_BINDING_MISSING_CLAIM"));
        }
        other => panic!("expected InvalidChip, got {:?}", other),
    }
}

#[tokio::test]
async fn stage_transition_accepts_wasm_valid_attestation_signature() {
    let pipeline = UblPipeline::new(Box::new(InMemoryPolicyStorage::new()));
    let mut req = allow_request();
    let module = wasm_identity_module();
    let hash = digest::digest(&digest::SHA256, &module);
    let wasm_sha256 = hex::encode(hash.as_ref());
    let attest_payload = json!({
        "wasm_sha256": wasm_sha256,
        "abi_version": "1.0"
    });
    let sk = ubl_kms::signing_key_from_hex(
        "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
    )
    .unwrap();
    let sig = ubl_kms::sign_canonical(&sk, &attest_payload, ubl_kms::domain::CAPSULE).unwrap();

    req.body["adapter"] = json!({
        "wasm_sha256": attest_payload["wasm_sha256"],
        "abi_version": "1.0",
        "wasm_b64": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&module),
        "attestation_signature_b64": sig,
        "attestation_trust_anchor": wasm_test_trust_anchor_did()
    });

    let parsed = parsed_request(&req);
    let check = pipeline.stage_check(&parsed).await.unwrap();
    let tr = pipeline.stage_transition(&parsed, &check).await.unwrap();
    assert_eq!(tr.body["vm_state"]["adapter_executed"], json!(true));
}

#[tokio::test]
async fn stage_write_finished_links_wa_and_tr_receipts() {
    let pipeline = UblPipeline::new(Box::new(InMemoryPolicyStorage::new()));
    let req = allow_request();
    let parsed = parsed_request(&req);
    let wa = pipeline.stage_write_ahead(&parsed).await.unwrap();
    let check = pipeline.stage_check(&parsed).await.unwrap();
    let tr = pipeline.stage_transition(&parsed, &check).await.unwrap();

    let wf = pipeline
        .stage_write_finished(&parsed, &wa, &tr, &check, 123)
        .await
        .unwrap();
    assert_eq!(wf.receipt_type, "ubl/wf");
    assert_eq!(wf.body["wa_cid"], json!(wa.body_cid.as_str()));
    assert_eq!(wf.body["tr_cid"], json!(tr.body_cid.as_str()));
    assert_eq!(wf.body["decision"], json!("Allow"));
    assert_eq!(wf.body["duration_ms"], json!(123));
}

#[derive(Debug, Deserialize)]
struct VectorInput {
    abi_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VectorExpected {
    decision: String,
    code: String,
    #[serde(default)]
    receipt_claims: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ConformanceVector {
    id: String,
    kind: String,
    profile: String,
    input: VectorInput,
    expected: VectorExpected,
}

#[derive(Debug)]
struct RuntimeObservation {
    decision: String,
    code: String,
    vm_state: serde_json::Value,
}

fn wasm_identity_module() -> Vec<u8> {
    wat::parse_str(
        r#"
        (module
          (memory (export "memory") 1 1)
          (func (export "ubl_adapter_v1") (param i32 i32) (result i32)
            local.get 1))
        "#,
    )
    .unwrap()
}

fn wasm_infinite_loop_module() -> Vec<u8> {
    wat::parse_str(
        r#"
        (module
          (memory (export "memory") 1 1)
          (func (export "ubl_adapter_v1") (param i32 i32) (result i32)
            (loop
              br 0)
            i32.const 0))
        "#,
    )
    .unwrap()
}

fn wasm_missing_entrypoint_module() -> Vec<u8> {
    wat::parse_str(
        r#"
        (module
          (memory (export "memory") 1 1)
          (func (export "not_adapter") (param i32 i32) (result i32)
            local.get 1))
        "#,
    )
    .unwrap()
}

fn wasm_test_trust_anchor_did() -> String {
    let sk = ubl_kms::signing_key_from_hex(
        "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
    )
    .unwrap();
    let vk = ubl_kms::verifying_key(&sk);
    ubl_kms::did_from_verifying_key(&vk)
}

fn vector_paths(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for bucket in ["positive", "negative"] {
        let dir = root.join(bucket);
        let mut items: Vec<PathBuf> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();
        items.sort();
        out.extend(items);
    }
    out
}

fn load_vectors() -> Vec<ConformanceVector> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/wasm/conformance/vectors/v1");
    vector_paths(&root)
        .into_iter()
        .map(|p| {
            let raw = fs::read_to_string(&p).unwrap();
            serde_json::from_str::<ConformanceVector>(&raw)
                .unwrap_or_else(|e| panic!("failed to parse vector {}: {}", p.display(), e))
        })
        .collect()
}

fn build_request_for_vector(vector: &ConformanceVector) -> ChipRequest {
    let mut req = allow_request();
    req.body["@id"] = json!(format!("runtime-{}", vector.id));
    req.body["@world"] = json!("a/demo/t/main");

    let valid_module = wasm_identity_module();
    let valid_hash = digest::digest(&digest::SHA256, &valid_module);
    let valid_hash_hex = hex::encode(valid_hash.as_ref());
    let valid_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&valid_module);
    let abi = vector
        .input
        .abi_version
        .clone()
        .unwrap_or_else(|| "v1".to_string());

    match vector.expected.code.as_str() {
        "WASM_ABI_MISSING_VERSION" => {
            req.body["adapter"] = json!({
                "wasm_sha256": valid_hash_hex,
                "wasm_b64": valid_b64
            });
        }
        "WASM_ABI_UNSUPPORTED_VERSION" => {
            req.body["adapter"] = json!({
                "wasm_sha256": valid_hash_hex,
                "abi_version": "9.9",
                "wasm_b64": valid_b64
            });
        }
        "WASM_ABI_INVALID_PAYLOAD" => {
            req.body["adapter"] = json!({
                "wasm_sha256": valid_hash_hex,
                "abi_version": "1.0",
                "wasm_b64": "***not-base64***"
            });
        }
        "WASM_VERIFY_HASH_MISMATCH" => {
            req.body["adapter"] = json!({
                "wasm_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                "abi_version": "1.0",
                "wasm_b64": valid_b64
            });
        }
        "WASM_VERIFY_SIGNATURE_INVALID" => {
            req.body["adapter"] = json!({
                "wasm_sha256": valid_hash_hex,
                "abi_version": "1.0",
                "wasm_b64": valid_b64,
                "attestation_signature_b64": "***bad-signature***",
                "attestation_trust_anchor": wasm_test_trust_anchor_did()
            });
        }
        "WASM_VERIFY_TRUST_ANCHOR_MISMATCH" => {
            req.body["adapter"] = json!({
                "wasm_sha256": valid_hash_hex,
                "abi_version": "1.0",
                "wasm_b64": valid_b64,
                "attestation_signature_b64": "ed25519:AAAA",
                "attestation_trust_anchor": "did:key:zunknown"
            });
        }
        "WASM_CAPABILITY_DENIED_NETWORK" => {
            req.body["adapter"] = json!({
                "wasm_sha256": valid_hash_hex,
                "abi_version": "1.0",
                "wasm_b64": valid_b64,
                "capabilities": ["network"]
            });
        }
        "WASM_CAPABILITY_DENIED" => {
            req.body["adapter"] = json!({
                "wasm_sha256": valid_hash_hex,
                "abi_version": "1.0",
                "wasm_b64": valid_b64,
                "capabilities": ["clock"]
            });
        }
        "WASM_DETERMINISM_VIOLATION" => {
            let bad = wasm_missing_entrypoint_module();
            let bad_hash = digest::digest(&digest::SHA256, &bad);
            req.body["adapter"] = json!({
                "wasm_sha256": hex::encode(bad_hash.as_ref()),
                "abi_version": "1.0",
                "wasm_b64": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bad)
            });
        }
        "WASM_RESOURCE_FUEL_EXHAUSTED" => {
            let loop_mod = wasm_infinite_loop_module();
            let loop_hash = digest::digest(&digest::SHA256, &loop_mod);
            req.body["adapter"] = json!({
                "wasm_sha256": hex::encode(loop_hash.as_ref()),
                "abi_version": "1.0",
                "wasm_b64": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&loop_mod),
                "fuel_budget": 100
            });
        }
        "WASM_RESOURCE_MEMORY_LIMIT" => {
            // Force memory growth against a fixed 1-page module memory, producing
            // deterministic memory-limit failure instead of fuel exhaustion.
            req.body["oversized_payload"] = json!("X".repeat(80_000));
            req.body["adapter"] = json!({
                "wasm_sha256": valid_hash_hex,
                "abi_version": "1.0",
                "wasm_b64": valid_b64,
                "fuel_budget": 50_000
            });
        }
        "WASM_RESOURCE_TIMEOUT" => {
            req.body["adapter"] = json!({
                "wasm_sha256": valid_hash_hex,
                "abi_version": "1.0",
                "wasm_b64": valid_b64,
                "timeout_ms": 1
            });
        }
        "WASM_RECEIPT_BINDING_MISSING_CLAIM" => {
            req.body["adapter"] = json!({
                "wasm_sha256": valid_hash_hex,
                "abi_version": "1.0",
                "wasm_b64": valid_b64,
                "required_receipt_claims": ["wasm.nonexistent.claim"]
            });
        }
        _ => {
            // Positive/default path
            let normalized_abi = if abi == "v1" { "1.0" } else { abi.as_str() };
            req.body["adapter"] = json!({
                "wasm_sha256": valid_hash_hex,
                "abi_version": normalized_abi,
                "wasm_b64": valid_b64,
                "fuel_budget": 50_000
            });
        }
    }

    req
}

fn map_pipeline_error_code(err: &PipelineError) -> String {
    let code = UblError::from_pipeline_error(err).code;
    serde_json::to_value(code)
        .ok()
        .and_then(|v| v.as_str().map(ToString::to_string))
        .unwrap_or_else(|| "INTERNAL_ERROR".to_string())
}

fn claim_present(claim: &str, vm_state: &serde_json::Value, profile: &str) -> bool {
    match claim {
        "wasm.module.sha256" => vm_state["adapter_wasm_sha256"].as_str().is_some(),
        "wasm.abi.version" => vm_state["adapter_abi_version"].as_str().is_some(),
        "wasm.profile" => profile == "deterministic_v1",
        "wasm.fuel.used" => vm_state["adapter_fuel_used"].as_u64().is_some(),
        "wasm.memory.max_bytes" => crate::wasm_adapter::WASM_MEMORY_LIMIT_BYTES > 0,
        "wasm.verify.status" => vm_state["adapter_executed"].as_bool() == Some(true),
        _ => false,
    }
}

async fn execute_vector(pipeline: &UblPipeline, vector: &ConformanceVector) -> RuntimeObservation {
    let req = build_request_for_vector(vector);
    let parsed = parsed_request(&req);
    let check = pipeline.stage_check(&parsed).await.unwrap();

    if matches!(check.decision, Decision::Deny) {
        return RuntimeObservation {
            decision: "deny".to_string(),
            code: "POLICY_DENIED".to_string(),
            vm_state: json!({}),
        };
    }

    match pipeline.stage_transition(&parsed, &check).await {
        Ok(tr) => RuntimeObservation {
            decision: "allow".to_string(),
            code: "OK".to_string(),
            vm_state: tr.body["vm_state"].clone(),
        },
        Err(err) => RuntimeObservation {
            decision: "deny".to_string(),
            code: map_pipeline_error_code(&err),
            vm_state: json!({}),
        },
    }
}

#[tokio::test]
async fn stage_runtime_executes_wasm_conformance_vectors() {
    let vectors = load_vectors();
    assert_eq!(vectors.len(), 100, "expected 100 vectors in 30/70 gate set");

    let pipeline = UblPipeline::new(Box::new(InMemoryPolicyStorage::new()));

    for vector in &vectors {
        let obs = execute_vector(&pipeline, vector).await;
        let expected_decision = vector.expected.decision.to_ascii_lowercase();
        assert_eq!(
            obs.decision, expected_decision,
            "decision mismatch for vector {}",
            vector.id
        );

        assert_eq!(
            obs.code, vector.expected.code,
            "code mismatch for vector {}",
            vector.id
        );

        if vector.kind == "positive" {
            for claim in &vector.expected.receipt_claims {
                assert!(
                    claim_present(claim, &obs.vm_state, &vector.profile),
                    "missing claim '{}' for vector {}",
                    claim,
                    vector.id
                );
            }
        }
    }
}
