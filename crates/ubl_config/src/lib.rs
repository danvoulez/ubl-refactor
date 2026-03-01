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
            dsn: std::env::var("UBL_STORE_DSN").ok(),
            idempotency_dsn: std::env::var("UBL_IDEMPOTENCY_DSN").ok(),
            outbox_dsn: std::env::var("UBL_OUTBOX_DSN").ok(),
            outbox_workers: 1,
            outbox_endpoint: None,
            eventstore_enabled: false,
            eventstore_path: "./data/events".to_string(),
        }
    }
}

impl StorageConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        cfg.backend = std::env::var("UBL_STORE_BACKEND").unwrap_or_else(|_| cfg.backend.clone());
        cfg.outbox_workers = std::env::var("UBL_OUTBOX_WORKERS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(cfg.outbox_workers)
            .max(1);
        cfg.outbox_endpoint = std::env::var("UBL_OUTBOX_ENDPOINT").ok();
        cfg.eventstore_enabled = std::env::var("UBL_EVENTSTORE_ENABLED")
            .map(|v| {
                let n = v.trim().to_ascii_lowercase();
                n == "1" || n == "true" || n == "yes" || n == "on"
            })
            .unwrap_or(false);
        cfg.eventstore_path = std::env::var("UBL_EVENTSTORE_PATH").unwrap_or(cfg.eventstore_path);
        cfg
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservabilityConfig {
    pub rust_log: String,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            rust_log: "info,ubl_gate=info".to_string(),
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
