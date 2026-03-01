use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::http::HeaderMap;
use ubl_chipstore::ChipStore;
use ubl_eventstore::EventStore;
use ubl_runtime::advisory::AdvisoryEngine;
use ubl_runtime::durable_store::DurableStore;
use ubl_runtime::manifest::GateManifest;
use ubl_runtime::rate_limit::CanonRateLimiter;
use ubl_runtime::UblPipeline;
use ubl_runtime::error_response::ErrorCode;

use crate::utils::{env_bool, csv_env, extract_api_key};

#[derive(Clone)]
pub(crate) struct AppState {
    pub pipeline: Arc<UblPipeline>,
    pub chip_store: Arc<ChipStore>,
    pub manifest: Arc<GateManifest>,
    pub advisory_engine: Arc<AdvisoryEngine>,
    pub http_client: reqwest::Client,
    pub canon_rate_limiter: Option<Arc<CanonRateLimiter>>,
    pub mcp_token_rate_limiter: Arc<McpTokenRateLimiter>,
    pub durable_store: Option<Arc<DurableStore>>,
    pub event_store: Option<Arc<EventStore>>,
    pub public_receipt_origin: String,
    pub public_receipt_path: String,
    pub genesis_pubkey_sha256: Option<String>,
    pub release_commit: Option<String>,
    pub gate_binary_sha256: Option<String>,
    pub write_access_policy: Arc<WriteAccessPolicy>,
}

#[derive(Clone)]
pub(crate) struct McpTokenRateLimiter {
    pub per_minute: usize,
    pub buckets: Arc<tokio::sync::RwLock<HashMap<String, VecDeque<Instant>>>>,
}

impl McpTokenRateLimiter {
    pub fn from_env() -> Self {
        let per_minute = std::env::var("UBL_MCP_TOKEN_RPM")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(120)
            .max(1);
        Self {
            per_minute,
            buckets: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Returns retry-after seconds when limited.
    pub async fn check(&self, token_id: &str) -> Option<u64> {
        let now = Instant::now();
        let window = Duration::from_secs(60);
        let mut buckets = self.buckets.write().await;
        let bucket = buckets
            .entry(token_id.to_string())
            .or_insert_with(VecDeque::new);

        while bucket
            .front()
            .is_some_and(|ts| now.duration_since(*ts) >= window)
        {
            bucket.pop_front();
        }

        if bucket.len() >= self.per_minute {
            let retry_after = bucket
                .front()
                .map(|oldest| {
                    let elapsed = now.duration_since(*oldest);
                    window.saturating_sub(elapsed).as_secs().saturating_add(1)
                })
                .unwrap_or(1);
            return Some(retry_after);
        }

        bucket.push_back(now);
        None
    }
}

#[derive(Clone)]
pub(crate) struct McpWsAuth {
    pub token_id: String,
    pub token_cid: String,
    pub world: String,
    pub scope: Vec<String>,
    pub subject_did: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct WriteAccessPolicy {
    pub auth_required: bool,
    pub api_keys: Vec<String>,
    pub public_worlds: Vec<String>,
    pub public_types: Vec<String>,
}

impl WriteAccessPolicy {
    pub fn from_env() -> Self {
        let auth_required = env_bool("UBL_WRITE_AUTH_REQUIRED", false);
        let api_keys = csv_env("UBL_WRITE_API_KEYS");
        let public_worlds = {
            let worlds = csv_env("UBL_PUBLIC_WRITE_WORLDS");
            if worlds.is_empty() {
                vec![
                    "a/chip-registry/t/public".to_string(),
                    "a/demo/t/dev".to_string(),
                ]
            } else {
                worlds
            }
        };
        let public_types = {
            let types = csv_env("UBL_PUBLIC_WRITE_TYPES");
            if types.is_empty() {
                vec![
                    "ubl/document".to_string(),
                    "audit/advisory.request.v1".to_string(),
                ]
            } else {
                types
            }
        };

        Self {
            auth_required,
            api_keys,
            public_worlds,
            public_types,
        }
    }

    #[cfg(test)]
    pub fn open_for_tests() -> Self {
        Self {
            auth_required: false,
            api_keys: vec![],
            public_worlds: vec![],
            public_types: vec![],
        }
    }

    pub fn authorize_write(
        &self,
        headers: Option<&HeaderMap>,
        chip_type: &str,
        world: &str,
    ) -> Result<(), (ErrorCode, String)> {
        if !self.auth_required && self.api_keys.is_empty() {
            return Ok(());
        }

        if self.matches_api_key(headers) {
            return Ok(());
        }

        if self.allows_public_unauthenticated(chip_type, world) {
            return Ok(());
        }

        Err((
            ErrorCode::Unauthorized,
            format!(
                "write auth required for @type='{}' @world='{}'; provide X-API-Key or use allowed public onboarding lane",
                chip_type, world
            ),
        ))
    }

    pub fn allows_public_unauthenticated(&self, chip_type: &str, world: &str) -> bool {
        if ubl_runtime::auth::is_onboarding_type(chip_type) {
            return true;
        }
        self.public_worlds.iter().any(|w| w == world)
            && self.public_types.iter().any(|t| t == chip_type)
    }

    pub fn matches_api_key(&self, headers: Option<&HeaderMap>) -> bool {
        let Some(headers) = headers else {
            return false;
        };
        let Some(presented) = extract_api_key(headers) else {
            return false;
        };
        self.api_keys.iter().any(|k| k == &presented)
    }
}
