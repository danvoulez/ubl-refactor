//! Deterministic authorship identity resolution for incoming chips.
//!
//! Goal:
//! - Resolve a stable `subject_did` for "who is knocking".
//! - Prefer cryptographic identifiers when present.
//! - Fall back to deterministic anonymous DID derived from stable claims.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Optional transport-level hints from the gateway.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorHint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent_hash: Option<String>,
}

/// Resolve a subject DID from chip body + transport hints.
///
/// Priority:
/// 1. Explicit DID claims (`actor.did`, `did`, `owner_did`).
/// 2. Deterministic anonymous DID from stable claims.
pub fn resolve_subject_did(body: Option<&Value>, hint: Option<&ActorHint>) -> String {
    if let Some(did) = body.and_then(extract_explicit_did) {
        return did;
    }

    let mut claims = Map::new();
    if let Some(body) = body {
        if let Some(actor) = body.get("actor").and_then(|v| v.as_object()) {
            copy_if_str(actor, "installation_key", &mut claims);
            copy_if_str(actor, "client_pubkey", &mut claims);
            copy_if_str(actor, "device_id", &mut claims);
            copy_if_str(actor, "session_id", &mut claims);
            copy_if_str(actor, "kid", &mut claims);
        }
    }
    if let Some(h) = hint {
        if let Some(ip) = &h.ip_prefix {
            claims.insert("ip_prefix".to_string(), Value::String(ip.clone()));
        }
        if let Some(ua) = &h.user_agent_hash {
            claims.insert("user_agent_hash".to_string(), Value::String(ua.clone()));
        }
    }

    let claims_value = Value::Object(claims);
    let fp = claims_fingerprint(&claims_value);
    format!("did:ubl:anon:{}", fp)
}

/// Content-address a raw inbound envelope as a deterministic knock CID.
pub fn knock_cid_from_bytes(bytes: &[u8]) -> String {
    let hash = blake3::hash(bytes);
    format!("b3:{}", hex::encode(hash.as_bytes()))
}

/// Content-address a parsed JSON envelope using NRF-1 canonical bytes.
pub fn knock_cid_from_value(value: &Value) -> String {
    match ubl_ai_nrf1::to_nrf1_bytes(value).and_then(|nrf| ubl_ai_nrf1::compute_cid(&nrf)) {
        Ok(cid) => cid,
        Err(_) => {
            let fallback =
                serde_json::to_vec(value).unwrap_or_else(|_| b"{\"@type\":\"unknown\"}".to_vec());
            knock_cid_from_bytes(&fallback)
        }
    }
}

fn extract_explicit_did(body: &Value) -> Option<String> {
    let actor_did = body
        .get("actor")
        .and_then(|v| v.as_object())
        .and_then(|o| o.get("did"))
        .and_then(|v| v.as_str());
    if let Some(did) = actor_did.filter(|s| s.starts_with("did:")) {
        return Some(did.to_string());
    }

    let root_did = body.get("did").and_then(|v| v.as_str());
    if let Some(did) = root_did.filter(|s| s.starts_with("did:")) {
        return Some(did.to_string());
    }

    let owner_did = body.get("owner_did").and_then(|v| v.as_str());
    owner_did
        .filter(|s| s.starts_with("did:"))
        .map(|s| s.to_string())
}

fn copy_if_str(src: &Map<String, Value>, key: &str, dst: &mut Map<String, Value>) {
    if let Some(v) = src.get(key).and_then(|v| v.as_str()) {
        dst.insert(key.to_string(), Value::String(v.to_string()));
    }
}

fn claims_fingerprint(claims: &Value) -> String {
    match ubl_ai_nrf1::to_nrf1_bytes(claims).and_then(|nrf| ubl_ai_nrf1::compute_cid(&nrf)) {
        Ok(cid) => cid,
        Err(_) => {
            let raw = serde_json::to_vec(claims).unwrap_or_else(|_| b"{}".to_vec());
            knock_cid_from_bytes(&raw)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn explicit_actor_did_wins() {
        let body = json!({
            "@type":"ubl/document",
            "@world":"a/test/t/main",
            "actor": { "did": "did:key:zActor" }
        });
        assert_eq!(resolve_subject_did(Some(&body), None), "did:key:zActor");
    }

    #[test]
    fn anon_did_is_deterministic_for_same_claims() {
        let body = json!({
            "@type":"ubl/document",
            "@world":"a/test/t/main",
            "actor": { "installation_key": "inst-123", "device_id":"dev-1" }
        });
        let a = resolve_subject_did(Some(&body), None);
        let b = resolve_subject_did(Some(&body), None);
        assert_eq!(a, b);
        assert!(a.starts_with("did:ubl:anon:b3:"));
    }

    #[test]
    fn knock_cid_from_bytes_is_stable() {
        let bytes = br#"{"@type":"ubl/document","@world":"a/t","x":1}"#;
        let a = knock_cid_from_bytes(bytes);
        let b = knock_cid_from_bytes(bytes);
        assert_eq!(a, b);
        assert!(a.starts_with("b3:"));
    }
}
