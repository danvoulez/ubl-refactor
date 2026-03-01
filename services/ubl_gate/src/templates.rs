//! Askama template structs and shared data-view types for the console UI.

use askama::Template;
use serde::Deserialize;
use serde_json::Value;

// ── Registry data views ───────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RegistryView {
    pub(crate) types: std::collections::BTreeMap<String, RegistryTypeView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RegistryTypeView {
    pub(crate) chip_type: String,
    pub(crate) latest_version: Option<String>,
    pub(crate) deprecated: bool,
    pub(crate) has_kats: bool,
    pub(crate) required_cap: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) docs_url: Option<String>,
    pub(crate) deprecation: Option<Value>,
    pub(crate) last_cid: Option<String>,
    pub(crate) last_updated_at: Option<String>,
    pub(crate) versions: std::collections::BTreeMap<String, RegistryVersionView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RegistryVersionView {
    pub(crate) version: String,
    pub(crate) schema: Option<Value>,
    pub(crate) kats: Vec<Value>,
    pub(crate) required_cap: Option<String>,
    pub(crate) register_cid: Option<String>,
    pub(crate) updated_at: Option<String>,
}

// ── Console templates ─────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "console.html")]
pub(crate) struct ConsoleTemplate {
    pub(crate) world: String,
    pub(crate) tab: String,
    pub(crate) is_stats: bool,
    pub(crate) profile: String,
}

#[derive(Template)]
#[template(path = "console_kpis.html")]
pub(crate) struct ConsoleKpisTemplate {
    pub(crate) available: bool,
    pub(crate) message: String,
    pub(crate) generated_at: String,
    pub(crate) total_events: u64,
    pub(crate) allow_count: u64,
    pub(crate) deny_count: u64,
    pub(crate) outbox_pending: String,
    pub(crate) visible_p95_rows: Vec<StageP95Row>,
    pub(crate) hidden_p95_rows: Vec<StageP95Row>,
}

#[derive(Clone)]
pub(crate) struct StageP95Row {
    pub(crate) stage: String,
    pub(crate) p95_ms: String,
}

#[derive(Template)]
#[template(path = "console_events.html")]
pub(crate) struct ConsoleEventsTemplate {
    pub(crate) available: bool,
    pub(crate) message: String,
    pub(crate) visible_rows: Vec<ConsoleEventRow>,
    pub(crate) hidden_rows: Vec<ConsoleEventRow>,
}

#[derive(Clone)]
pub(crate) struct ConsoleEventRow {
    pub(crate) when: String,
    pub(crate) stage: String,
    pub(crate) decision: String,
    pub(crate) chip_type: String,
    pub(crate) code: String,
    pub(crate) receipt_cid: String,
}

// ── Registry templates ────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "registry.html")]
pub(crate) struct RegistryTemplate {
    pub(crate) world: String,
}

#[derive(Template)]
#[template(path = "registry_table.html")]
pub(crate) struct RegistryTableTemplate {
    pub(crate) visible_rows: Vec<RegistryRow>,
    pub(crate) hidden_rows: Vec<RegistryRow>,
}

#[derive(Clone)]
pub(crate) struct RegistryRow {
    pub(crate) chip_type: String,
    pub(crate) latest_version: String,
    pub(crate) deprecated: bool,
    pub(crate) has_kats: bool,
    pub(crate) required_cap: String,
    pub(crate) last_updated_at: String,
}

#[derive(Template)]
#[template(path = "registry_type.html")]
pub(crate) struct RegistryTypeTemplate {
    pub(crate) chip_type: String,
    pub(crate) latest_version: String,
    pub(crate) deprecated: bool,
    pub(crate) description: String,
    pub(crate) docs_url: Option<String>,
    pub(crate) deprecation_json: String,
    pub(crate) versions: Vec<RegistryTypeVersionRow>,
}

#[derive(Clone)]
pub(crate) struct RegistryTypeVersionRow {
    pub(crate) version: String,
    pub(crate) required_cap: String,
    pub(crate) kats_count: usize,
    pub(crate) register_cid: String,
    pub(crate) updated_at: String,
    pub(crate) kats: Vec<RegistryKatRow>,
}

#[derive(Clone)]
pub(crate) struct RegistryKatRow {
    pub(crate) index: usize,
    pub(crate) label: String,
    pub(crate) expected_decision: String,
    pub(crate) expected_error: String,
    pub(crate) input_json_preview: String,
}

#[derive(Template)]
#[template(path = "registry_kat_result.html")]
pub(crate) struct RegistryKatResultTemplate {
    pub(crate) status_code: u16,
    pub(crate) kat_label: String,
    pub(crate) expected_decision: String,
    pub(crate) expected_error: String,
    pub(crate) actual_decision: String,
    pub(crate) actual_error: String,
    pub(crate) receipt_cid: String,
    pub(crate) pass: bool,
    pub(crate) response_json: String,
    pub(crate) message: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegistryKatTestForm {
    pub(crate) chip_type: String,
    pub(crate) version: String,
    pub(crate) kat_index: usize,
}

// ── Shared console templates ──────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "console_receipt.html")]
pub(crate) struct ConsoleReceiptTemplate {
    pub(crate) cid: String,
}

#[derive(Template)]
#[template(path = "audit.html")]
pub(crate) struct AuditTemplate {
    pub(crate) world: String,
    pub(crate) kind: String,
}

#[derive(Template)]
#[template(path = "audit_table.html")]
pub(crate) struct AuditTableTemplate {
    pub(crate) kind: String,
    pub(crate) visible_rows: Vec<AuditRow>,
    pub(crate) hidden_rows: Vec<AuditRow>,
}

#[derive(Clone)]
pub(crate) struct AuditRow {
    pub(crate) cid: String,
    pub(crate) chip_type: String,
    pub(crate) world: String,
    pub(crate) created_at: String,
    pub(crate) summary: String,
}

#[derive(Template)]
#[template(path = "console_mock24h.html")]
pub(crate) struct ConsoleMock24hTemplate {
    pub(crate) profile: String,
    pub(crate) generated_at: String,
    pub(crate) visible_rows: Vec<MockHourRow>,
    pub(crate) hidden_rows: Vec<MockHourRow>,
}

#[derive(Clone, serde::Serialize)]
pub(crate) struct MockHourRow {
    pub(crate) hour_label: String,
    pub(crate) events: u64,
    pub(crate) allow_pct: String,
    pub(crate) deny_pct: String,
    pub(crate) p95_ms: String,
    pub(crate) outbox_pending: u64,
    pub(crate) error_pct: String,
}

#[derive(Template)]
#[template(path = "llm_panel.html")]
pub(crate) struct LlmPanelTemplate {
    pub(crate) title: String,
    pub(crate) severity: String,
    pub(crate) source: String,
    pub(crate) generated_at: String,
    pub(crate) summary: String,
    pub(crate) bullets: Vec<String>,
}
