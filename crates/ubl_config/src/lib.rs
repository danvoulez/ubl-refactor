const DEFAULT_SQLITE_DSN: &str = "file:./data/ubl.db?mode=rwc&_journal_mode=WAL";

fn env_opt_trim(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(default)
}

fn csv_env(name: &str) -> Vec<String> {
    env_opt_trim(name)
        .map(|s| {
            s.split(',')
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateConfig {
    pub bind: String,
    pub data_dir: String,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:4000".to_string(),
            data_dir: "./data".to_string(),
        }
    }
}

impl GateConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(bind) = std::env::var("UBL_GATE_BIND") {
            let bind = bind.trim();
            if !bind.is_empty() {
                cfg.bind = bind.to_string();
            }
        }
        if let Ok(data_dir) = std::env::var("UBL_DATA_DIR") {
            let data_dir = data_dir.trim();
            if !data_dir.is_empty() {
                cfg.data_dir = data_dir.to_string();
            }
        }
        cfg
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.bind.trim().is_empty() {
            return Err("GateConfig.bind must not be empty".to_string());
        }
        if self.data_dir.trim().is_empty() {
            return Err("GateConfig.data_dir must not be empty".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageConfig {
    pub backend: String,
    pub dsn: Option<String>,
    pub idempotency_dsn: Option<String>,
    pub outbox_dsn: Option<String>,
    pub outbox_workers: usize,
    pub outbox_endpoint: Option<String>,
    pub eventstore_enabled: bool,
    pub eventstore_path: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: "memory".to_string(),
            dsn: None,
            idempotency_dsn: None,
            outbox_dsn: None,
            outbox_workers: 1,
            outbox_endpoint: None,
            eventstore_enabled: true,
            eventstore_path: "./data/events".to_string(),
        }
    }
}

impl StorageConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        cfg.backend = env_opt_trim("UBL_STORE_BACKEND").unwrap_or(cfg.backend);
        cfg.dsn = env_opt_trim("UBL_STORE_DSN");
        cfg.idempotency_dsn = env_opt_trim("UBL_IDEMPOTENCY_DSN");
        cfg.outbox_dsn = env_opt_trim("UBL_OUTBOX_DSN");
        cfg.outbox_workers = std::env::var("UBL_OUTBOX_WORKERS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(cfg.outbox_workers)
            .max(1);
        cfg.outbox_endpoint = env_opt_trim("UBL_OUTBOX_ENDPOINT");
        cfg.eventstore_enabled = std::env::var("UBL_EVENTSTORE_ENABLED")
            .map(|v| {
                let n = v.trim();
                matches!(n, "1" | "true" | "TRUE" | "yes" | "on")
            })
            .unwrap_or(true);
        cfg.eventstore_path = env_opt_trim("UBL_EVENTSTORE_PATH").unwrap_or(cfg.eventstore_path);
        cfg
    }

    pub fn effective_sqlite_dsn(&self) -> String {
        self.dsn
            .clone()
            .or_else(|| self.idempotency_dsn.clone())
            .or_else(|| self.outbox_dsn.clone())
            .unwrap_or_else(|| DEFAULT_SQLITE_DSN.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservabilityConfig {
    pub rust_log: String,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            rust_log: "info,ubl_runtime=debug,ubl_gate=debug".to_string(),
        }
    }
}

impl ObservabilityConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = std::env::var("RUST_LOG") {
            let v = v.trim();
            if !v.is_empty() {
                cfg.rust_log = v.to_string();
            }
        }
        cfg
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateUrlsConfig {
    pub public_receipt_origin: String,
    pub public_receipt_path: String,
    pub manifest_base_url: String,
}

impl Default for GateUrlsConfig {
    fn default() -> Self {
        Self {
            public_receipt_origin: "https://logline.world".to_string(),
            public_receipt_path: "/r".to_string(),
            manifest_base_url: "https://api.ubl.agency".to_string(),
        }
    }
}

impl GateUrlsConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        cfg.public_receipt_origin = env_opt_trim("UBL_PUBLIC_RECEIPT_ORIGIN")
            .or_else(|| {
                env_opt_trim("UBL_RICH_URL_DOMAIN").map(|domain| {
                    let d = domain
                        .trim_start_matches("https://")
                        .trim_start_matches("http://")
                        .to_string();
                    format!("https://{}", d)
                })
            })
            .unwrap_or(cfg.public_receipt_origin);

        cfg.public_receipt_path = env_opt_trim("UBL_PUBLIC_RECEIPT_PATH")
            .map(|path| {
                if path.starts_with('/') {
                    path
                } else {
                    format!("/{}", path)
                }
            })
            .unwrap_or(cfg.public_receipt_path);

        cfg.manifest_base_url = env_opt_trim("UBL_MCP_BASE_URL")
            .or_else(|| env_opt_trim("UBL_API_BASE_URL"))
            .or_else(|| {
                env_opt_trim("UBL_API_DOMAIN").map(|domain| {
                    let d = domain
                        .trim_start_matches("https://")
                        .trim_start_matches("http://")
                        .to_string();
                    format!("https://{}", d)
                })
            })
            .unwrap_or(cfg.manifest_base_url);

        cfg
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateLimitsConfig {
    pub canon_rate_limit_enabled: bool,
    pub canon_rate_limit_per_min: u32,
    pub mcp_token_rpm: usize,
}

impl Default for GateLimitsConfig {
    fn default() -> Self {
        Self {
            canon_rate_limit_enabled: true,
            canon_rate_limit_per_min: 120,
            mcp_token_rpm: 120,
        }
    }
}

impl GateLimitsConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        cfg.canon_rate_limit_enabled = env_bool("UBL_CANON_RATE_LIMIT_ENABLED", true);
        cfg.canon_rate_limit_per_min = std::env::var("UBL_CANON_RATE_LIMIT_PER_MIN")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(cfg.canon_rate_limit_per_min)
            .max(1);
        cfg.mcp_token_rpm = std::env::var("UBL_MCP_TOKEN_RPM")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(cfg.mcp_token_rpm)
            .max(1);
        cfg
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateWritePolicyConfig {
    pub write_auth_required: bool,
    pub write_api_keys: Vec<String>,
    pub public_write_worlds: Vec<String>,
    pub public_write_types: Vec<String>,
}

impl Default for GateWritePolicyConfig {
    fn default() -> Self {
        Self {
            write_auth_required: false,
            write_api_keys: Vec::new(),
            public_write_worlds: vec![
                "a/chip-registry/t/public".to_string(),
                "a/demo/t/dev".to_string(),
            ],
            public_write_types: vec![
                "ubl/document".to_string(),
                "audit/advisory.request.v1".to_string(),
            ],
        }
    }
}

impl GateWritePolicyConfig {
    pub fn from_env() -> Self {
        let worlds = csv_env("UBL_PUBLIC_WRITE_WORLDS");
        let types = csv_env("UBL_PUBLIC_WRITE_TYPES");

        Self {
            write_auth_required: env_bool("UBL_WRITE_AUTH_REQUIRED", false),
            write_api_keys: csv_env("UBL_WRITE_API_KEYS"),
            public_write_worlds: if worlds.is_empty() {
                Self::default().public_write_worlds
            } else {
                worlds
            },
            public_write_types: if types.is_empty() {
                Self::default().public_write_types
            } else {
                types
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BuildInfoConfig {
    pub genesis_pubkey_sha256: Option<String>,
    pub release_commit: Option<String>,
    pub gate_binary_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmConfig {
    pub enabled: bool,
    pub base_url: Option<String>,
    pub model: String,
    pub openai_api_key: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: None,
            model: "gpt-4o-mini".to_string(),
            openai_api_key: None,
        }
    }
}

impl LlmConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        cfg.enabled = env_bool("UBL_ENABLE_REAL_LLM", false);
        cfg.base_url = env_opt_trim("UBL_LLM_BASE_URL");
        cfg.model = env_opt_trim("UBL_LLM_MODEL").unwrap_or_else(|| {
            if cfg.base_url.is_some() {
                "qwen3:4b".to_string()
            } else {
                cfg.model.clone()
            }
        });
        cfg.openai_api_key = env_opt_trim("OPENAI_API_KEY");
        cfg
    }

    pub fn openai_api_key_redacted(&self) -> &'static str {
        if self.openai_api_key.is_some() {
            "<redacted>"
        } else {
            "<unset>"
        }
    }

    pub fn to_redacted_log(&self) -> String {
        format!(
            "llm(enabled={},base_url_set={},model={},openai_api_key={})",
            self.enabled,
            self.base_url.is_some(),
            self.model,
            self.openai_api_key_redacted(),
        )
    }
}

impl BuildInfoConfig {
    pub fn from_env() -> Self {
        Self {
            genesis_pubkey_sha256: env_opt_trim("UBL_GENESIS_PUBKEY_SHA256"),
            release_commit: env_opt_trim("UBL_RELEASE_COMMIT"),
            gate_binary_sha256: env_opt_trim("UBL_GATE_BINARY_SHA256"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoConfig {
    pub crypto_mode: String,
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {
            crypto_mode: "compat_v1".to_string(),
        }
    }
}

impl CryptoConfig {
    pub fn from_env() -> Self {
        Self {
            crypto_mode: env_opt_trim("UBL_CRYPTO_MODE").unwrap_or_else(|| "compat_v1".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub gate: GateConfig,
    pub storage: StorageConfig,
    pub obs: ObservabilityConfig,
    pub urls: GateUrlsConfig,
    pub limits: GateLimitsConfig,
    pub write: GateWritePolicyConfig,
    pub build: BuildInfoConfig,
    pub llm: LlmConfig,
    pub crypto: CryptoConfig,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            gate: GateConfig::from_env(),
            storage: StorageConfig::from_env(),
            obs: ObservabilityConfig::from_env(),
            urls: GateUrlsConfig::from_env(),
            limits: GateLimitsConfig::from_env(),
            write: GateWritePolicyConfig::from_env(),
            build: BuildInfoConfig::from_env(),
            llm: LlmConfig::from_env(),
            crypto: CryptoConfig::from_env(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.gate.validate()?;
        if self.storage.backend.eq_ignore_ascii_case("sqlite")
            && self.storage.effective_sqlite_dsn().trim().is_empty()
        {
            return Err("StorageConfig sqlite dsn must not be empty".to_string());
        }
        Ok(())
    }

    pub fn to_redacted_log(&self) -> String {
        format!(
            "gate(bind={},data_dir={}) storage(backend={},dsn_set={},idempotency_dsn_set={},outbox_dsn_set={},eventstore_enabled={},eventstore_path={}) {}",
            self.gate.bind,
            self.gate.data_dir,
            self.storage.backend,
            self.storage.dsn.is_some(),
            self.storage.idempotency_dsn.is_some(),
            self.storage.outbox_dsn.is_some(),
            self.storage.eventstore_enabled,
            self.storage.eventstore_path,
            self.llm.to_redacted_log(),
        )
    }
}
