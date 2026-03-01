//! LLM panel handlers, context builders, heuristic analysis, and real LLM calls.

use async_stream::stream;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{sse::{Event as SseEvent, KeepAlive, Sse}, IntoResponse, Response},
    Json,
};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::time::Duration;

use crate::advisor::build_advisor_snapshot;
use crate::audit::query_audit_rows;
use crate::console::{build_mock_24h_rows, normalize_console_tab, normalize_mock_profile, render_html};
use crate::events::LlmPanelQuery;
use crate::registry::materialize_registry;
use crate::state::AppState;
use crate::templates::LlmPanelTemplate;
use crate::utils::env_bool;

// ── Panel handler ─────────────────────────────────────────────────────────────

pub(crate) async fn ui_llm_panel(
    State(state): State<AppState>,
    Query(query): Query<LlmPanelQuery>,
) -> Response {
    let page = query.page.unwrap_or_else(|| "console".to_string());
    let tab = normalize_console_tab(query.tab.as_deref().unwrap_or("live"));
    let world = query.world.unwrap_or_else(|| "*".to_string());
    let profile = normalize_mock_profile(query.profile.as_deref().unwrap_or("normal"));
    let kind = query.kind.unwrap_or_else(|| "reports".to_string());
    let chip_type = query.chip_type.unwrap_or_default();
    let cid = query.cid.unwrap_or_default();

    let context = build_llm_context(
        &state, &page, &tab, &world, &profile, &kind, &chip_type, &cid,
    )
    .await;

    let (mut severity, mut summary, mut bullets) = heuristic_analysis(&page, &context);
    let mut source = "heuristic local mock".to_string();

    if llm_is_enabled() {
        if let Ok(text) = call_real_llm(&state.http_client, &page, &context).await {
            let (llm_summary, llm_bullets) = parse_llm_text(&text);
            summary = llm_summary;
            if !llm_bullets.is_empty() {
                bullets = llm_bullets;
            }
            severity = "LLM".to_string();
            source = llm_source_label();
        }
    }

    let title = match page.as_str() {
        "registry" => "Registry".to_string(),
        "registry_type" => format!("Registry Type {}", chip_type),
        "audit" => format!("Audit {}", kind),
        "receipt" => format!("Receipt {}", cid),
        _ => format!("Console {}", tab),
    };

    render_html(&LlmPanelTemplate {
        title,
        severity,
        source,
        generated_at: chrono::Utc::now().to_rfc3339(),
        summary,
        bullets,
    })
}

pub(crate) async fn ui_llm_panel_stream(
    State(state): State<AppState>,
    Query(query): Query<LlmPanelQuery>,
) -> Response {
    let page = query.page.unwrap_or_else(|| "console".to_string());
    let tab = normalize_console_tab(query.tab.as_deref().unwrap_or("live"));
    let world = query.world.unwrap_or_else(|| "*".to_string());
    let profile = normalize_mock_profile(query.profile.as_deref().unwrap_or("normal"));
    let kind = query.kind.unwrap_or_else(|| "reports".to_string());
    let chip_type = query.chip_type.unwrap_or_default();
    let cid = query.cid.unwrap_or_default();

    let context = build_llm_context(
        &state, &page, &tab, &world, &profile, &kind, &chip_type, &cid,
    )
    .await;

    call_real_llm_stream_sse(state.http_client.clone(), page, context).await
}

// ── Context builder ───────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(crate) async fn build_llm_context(
    state: &AppState,
    page: &str,
    tab: &str,
    world: &str,
    profile: &str,
    kind: &str,
    chip_type: &str,
    cid: &str,
) -> Value {
    let world_filter = if world.trim().is_empty() || world == "*" {
        None
    } else {
        Some(world)
    };

    match page {
        "registry" => match materialize_registry(state, world_filter).await {
            Ok(registry) => {
                let total = registry.types.len();
                let deprecated = registry.types.values().filter(|v| v.deprecated).count();
                let without_kats = registry.types.values().filter(|v| !v.has_kats).count();
                json!({
                    "page": "registry",
                    "world": world,
                    "types_total": total,
                    "deprecated_total": deprecated,
                    "without_kats_total": without_kats,
                })
            }
            Err(e) => json!({
                "page": "registry",
                "world": world,
                "error": e
            }),
        },
        "registry_type" => match materialize_registry(state, None).await {
            Ok(registry) => {
                let view = registry.types.get(chip_type);
                json!({
                    "page": "registry_type",
                    "chip_type": chip_type,
                    "exists": view.is_some(),
                    "deprecated": view.map(|v| v.deprecated).unwrap_or(false),
                    "versions_total": view.map(|v| v.versions.len()).unwrap_or(0),
                    "has_kats": view.map(|v| v.has_kats).unwrap_or(false),
                })
            }
            Err(e) => json!({
                "page": "registry_type",
                "chip_type": chip_type,
                "error": e
            }),
        },
        "audit" => match query_audit_rows(state, kind, world_filter, 100).await {
            Ok(rows) => {
                let count = rows.len();
                json!({
                    "page": "audit",
                    "kind": kind,
                    "world": world,
                    "rows": count,
                    "latest_cid": rows.first().map(|r| r.cid.clone()).unwrap_or_else(|| "-".to_string())
                })
            }
            Err(e) => json!({
                "page": "audit",
                "kind": kind,
                "world": world,
                "error": e
            }),
        },
        "receipt" => {
            let exists = if cid.is_empty() {
                false
            } else {
                state
                    .chip_store
                    .get_chip(cid)
                    .await
                    .ok()
                    .flatten()
                    .is_some()
            };
            json!({
                "page": "receipt",
                "cid": cid,
                "exists": exists,
            })
        }
        _ => {
            let mock_rows = build_mock_24h_rows(profile, world);
            let sample = mock_rows.iter().take(6).collect::<Vec<_>>();
            let deny_avg = sample
                .iter()
                .filter_map(|r| r.deny_pct.parse::<f64>().ok())
                .sum::<f64>()
                / sample.len().max(1) as f64;
            let p95_max = sample
                .iter()
                .filter_map(|r| r.p95_ms.parse::<f64>().ok())
                .fold(0.0f64, f64::max);
            let outbox_max = sample.iter().map(|r| r.outbox_pending).max().unwrap_or(0);
            let events_sum: u64 = sample.iter().map(|r| r.events).sum();

            let mut base = json!({
                "page": "console",
                "tab": tab,
                "world": world,
                "profile": profile,
                "mock_rollup": {
                    "sample_hours": sample.len(),
                    "events_sum": events_sum,
                    "deny_avg_pct": deny_avg,
                    "p95_max_ms": p95_max,
                    "outbox_max": outbox_max
                }
            });

            if let Some(store) = state.event_store.as_ref() {
                if let Ok(snapshot) = build_advisor_snapshot(
                    state,
                    store,
                    world_filter,
                    Duration::from_secs(300),
                    5000,
                ) {
                    if let Some(obj) = base.as_object_mut() {
                        obj.insert("live_snapshot".to_string(), snapshot);
                    }
                }
            }
            base
        }
    }
}

// ── Heuristic analysis ────────────────────────────────────────────────────────

pub(crate) fn heuristic_analysis(page: &str, context: &Value) -> (String, String, Vec<String>) {
    let mut severity = "green".to_string();
    let summary: String;
    let mut bullets = Vec::new();

    match page {
        "registry" => {
            let total = context
                .get("types_total")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let deprecated = context
                .get("deprecated_total")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let no_kats = context
                .get("without_kats_total")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            summary = format!(
                "Registry possui {} tipos. {} deprecated e {} sem KAT.",
                total, deprecated, no_kats
            );
            if no_kats > 0 {
                severity = "yellow".to_string();
                bullets.push("Priorizar KAT para tipos sem cobertura.".to_string());
            }
            if deprecated > 0 {
                bullets.push("Revisar plano de sunset dos tipos deprecated.".to_string());
            }
        }
        "registry_type" => {
            let exists = context
                .get("exists")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let versions = context
                .get("versions_total")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let has_kats = context
                .get("has_kats")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !exists {
                severity = "yellow".to_string();
                summary = "Tipo nao encontrado no registry materializado.".to_string();
            } else {
                summary = format!("Tipo ativo com {} versoes registradas.", versions);
                if !has_kats {
                    severity = "yellow".to_string();
                    bullets.push("Adicionar KATs para validar regressao de politica.".to_string());
                }
            }
        }
        "audit" => {
            let rows = context.get("rows").and_then(|v| v.as_u64()).unwrap_or(0);
            let kind = context
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("reports");
            summary = format!("Audit {} retornou {} artefatos.", kind, rows);
            if rows == 0 {
                severity = "yellow".to_string();
                bullets.push("Sem artefatos recentes: revisar emissao de auditoria.".to_string());
            }
        }
        "receipt" => {
            let exists = context
                .get("exists")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if exists {
                summary = "Receipt localizado; trilha de trace e narrate disponivel.".to_string();
            } else {
                severity = "yellow".to_string();
                summary = "Receipt nao localizado no store local.".to_string();
            }
        }
        _ => {
            let deny_avg = context
                .get("mock_rollup")
                .and_then(|v| v.get("deny_avg_pct"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let p95_max = context
                .get("mock_rollup")
                .and_then(|v| v.get("p95_max_ms"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let outbox_max = context
                .get("mock_rollup")
                .and_then(|v| v.get("outbox_max"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let events_sum = context
                .get("mock_rollup")
                .and_then(|v| v.get("events_sum"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let profile = context
                .get("profile")
                .and_then(|v| v.as_str())
                .unwrap_or("normal");
            let tab = context
                .get("tab")
                .and_then(|v| v.as_str())
                .unwrap_or("live");

            summary = format!(
                "Console {} com perfil {}: {} eventos no recorte recente; deny medio {:.2}%, p95 max {:.1}ms.",
                tab, profile, events_sum, deny_avg, p95_max
            );

            if deny_avg >= 7.0 || p95_max >= 240.0 || outbox_max >= 80 {
                severity = "red".to_string();
                bullets.push(
                    "Sinal de degradacao: validar pipeline CHECK/TR e fila outbox.".to_string(),
                );
            } else if deny_avg >= 4.0 || p95_max >= 170.0 || outbox_max >= 35 {
                severity = "yellow".to_string();
                bullets.push(
                    "Tendencia de risco moderado: aumentar observabilidade por stage.".to_string(),
                );
            }

            if profile == "chaos" || profile == "degraded" {
                bullets.push(
                    "Perfil mock agressivo ativo; usar para testar auto-remediacao.".to_string(),
                );
            }
        }
    }

    if bullets.is_empty() {
        bullets.push("Continuar monitorando variacao de latencia e deny rate.".to_string());
    }
    (severity, summary, bullets)
}

// ── LLM backend helpers ───────────────────────────────────────────────────────

pub(crate) fn llm_is_enabled() -> bool {
    env_bool("UBL_ENABLE_REAL_LLM", false)
        || std::env::var("UBL_LLM_BASE_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .is_some()
}

pub(crate) fn llm_source_label() -> String {
    if std::env::var("UBL_LLM_BASE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some()
    {
        let model = std::env::var("UBL_LLM_MODEL").unwrap_or_else(|_| "qwen3:4b".to_string());
        format!("local ollama ({})", model)
    } else {
        let model = std::env::var("UBL_LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
        format!("openai ({})", model)
    }
}

pub(crate) fn resolve_llm_endpoint() -> Result<(String, String, Option<String>), String> {
    let base_url = std::env::var("UBL_LLM_BASE_URL")
        .ok()
        .filter(|s| !s.is_empty());
    if let Some(ref base) = base_url {
        let model = std::env::var("UBL_LLM_MODEL").unwrap_or_else(|_| "qwen3:4b".to_string());
        let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
        Ok((url, model, None))
    } else {
        let key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| "Neither UBL_LLM_BASE_URL nor OPENAI_API_KEY is configured".to_string())?;
        let model = std::env::var("UBL_LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
        Ok(("https://api.openai.com/v1/chat/completions".to_string(), model, Some(key)))
    }
}

pub(crate) fn llm_system_prompt(page: &str) -> &'static str {
    match page {
        "receipt" => concat!(
            "Voc\u{ea} \u{e9} o UBL Advisory Engine. ",
            "Narre este receipt de forma clara para o operador: ",
            "o que aconteceu, por que importa, o que observar. ",
            "Responda em portugu\u{ea}s. 1 par\u{e1}grafo curto, m\u{e1}ximo 80 palavras. Sem bullets."
        ),
        _ => concat!(
            "Voc\u{ea} \u{e9} um analista t\u{e9}cnico do sistema UBL (Universal Business Leverage). ",
            "O UBL usa um pipeline determin\u{ed}stico KNOCK\u{2192}WA\u{2192}CHECK\u{2192}TR\u{2192}WF com chips, receipts e pol\u{ed}ticas. ",
            "Responda em portugu\u{ea}s. Formato: 1 linha de resumo, depois at\u{e9} 3 bullets acion\u{e1}veis com '\u{2022}'. ",
            "Seja preciso, t\u{e9}cnico e conciso. M\u{e1}ximo 200 palavras."
        ),
    }
}

pub(crate) async fn call_real_llm(
    client: &reqwest::Client,
    page: &str,
    context: &Value,
) -> Result<String, String> {
    let (endpoint, model, api_key) = resolve_llm_endpoint()?;
    let timeout_ms = std::env::var("UBL_LLM_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(12_000);

    let user_msg = format!("Contexto (página '{}'): {}", page, context);
    let payload = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": llm_system_prompt(page)},
            {"role": "user",   "content": user_msg}
        ],
        "max_tokens": 400,
        "temperature": 0.3,
        "stream": false
    });

    let mut req = client
        .post(&endpoint)
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .json(&payload);
    if let Some(key) = api_key {
        req = req.bearer_auth(key);
    }

    let res = req.send().await.map_err(|e| e.to_string())?;
    let status = res.status();
    let body: Value = res.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("llm status {}: {}", status, body));
    }

    body.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "empty LLM output".to_string())
}

pub(crate) async fn call_real_llm_stream_sse(
    client: reqwest::Client,
    page: String,
    context: Value,
) -> Response {
    let (endpoint, model, api_key) = match resolve_llm_endpoint() {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"@type":"ubl/error","code":"LLM_UNAVAILABLE","message":e})),
            )
                .into_response();
        }
    };

    let timeout_ms = std::env::var("UBL_LLM_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30_000);

    let user_msg = format!("Contexto (página '{}'): {}", page, context);
    let payload = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": llm_system_prompt(&page)},
            {"role": "user",   "content": user_msg}
        ],
        "max_tokens": 400,
        "temperature": 0.3,
        "stream": true
    });

    let mut req = client
        .post(&endpoint)
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .json(&payload);
    if let Some(key) = api_key {
        req = req.bearer_auth(key);
    }

    let res = match req.send().await {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            let status = r.status().as_u16();
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({"@type":"ubl/error","code":"LLM_ERROR","message":format!("upstream status {}", status)})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({"@type":"ubl/error","code":"LLM_UNREACHABLE","message":e.to_string()})),
            )
                .into_response();
        }
    };

    let byte_stream = res.bytes_stream();
    let mut buf = String::new();

    let sse_stream = stream! {
        futures_util::pin_mut!(byte_stream);
        while let Some(chunk) = byte_stream.next().await {
            let bytes = match chunk {
                Ok(b) => b,
                Err(_) => break,
            };
            let Ok(text) = std::str::from_utf8(&bytes) else { continue };
            buf.push_str(text);

            while let Some(nl) = buf.find('\n') {
                let line = buf[..nl].trim().to_string();
                buf = buf[nl + 1..].to_string();

                let Some(data) = line.strip_prefix("data: ") else { continue };
                if data == "[DONE]" { return; }

                let Ok(v) = serde_json::from_str::<Value>(data) else { continue };
                let Some(token) = v
                    .get("choices").and_then(|c| c.get(0))
                    .and_then(|c| c.get("delta"))
                    .and_then(|d| d.get("content"))
                    .and_then(|c| c.as_str())
                    .filter(|s| !s.is_empty())
                else { continue };

                yield Ok::<SseEvent, Infallible>(
                    SseEvent::default().event("token").data(token.to_string())
                );
            }
        }
        yield Ok::<SseEvent, Infallible>(
            SseEvent::default().event("done").data("")
        );
    };

    Sse::new(sse_stream)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(5))
                .text(":"),
        )
        .into_response()
}

pub(crate) fn parse_llm_text(text: &str) -> (String, Vec<String>) {
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return (
            "LLM respondeu sem conteudo, mantendo analise local.".to_string(),
            Vec::new(),
        );
    }

    let summary = lines.remove(0).to_string();
    let bullets = lines
        .into_iter()
        .take(3)
        .map(|line| {
            line.trim_start_matches('-')
                .trim_start_matches('*')
                .trim()
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    (summary, bullets)
}
