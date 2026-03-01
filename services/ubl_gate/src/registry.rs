//! Registry page handlers and materialize_registry builder.

use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};

use axum::http::HeaderMap;
use crate::chip::submit_chip_bytes;
use crate::console::{render_html, split_rows};
use crate::state::AppState;
use crate::templates::{
    RegistryKatRow, RegistryKatResultTemplate, RegistryKatTestForm, RegistryRow,
    RegistryTableTemplate, RegistryTemplate, RegistryTypeTemplate, RegistryTypeVersionRow,
    RegistryTypeView, RegistryVersionView, RegistryView,
};

pub(crate) async fn registry_page(
    Query(query): Query<std::collections::BTreeMap<String, String>>,
) -> Response {
    let world = query
        .get("world")
        .cloned()
        .unwrap_or_else(|| "*".to_string());
    render_html(&RegistryTemplate { world })
}

pub(crate) async fn registry_table_partial(
    State(state): State<AppState>,
    Query(query): Query<std::collections::BTreeMap<String, String>>,
) -> Response {
    let world = query
        .get("world")
        .map(|w| w.as_str())
        .filter(|w| !w.trim().is_empty() && *w != "*");
    let registry = match materialize_registry(&state, world).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "@type":"ubl/error",
                    "code":"INTERNAL_ERROR",
                    "message": format!("registry materialization failed: {}", e),
                })),
            )
                .into_response();
        }
    };
    let rows: Vec<RegistryRow> = registry
        .types
        .values()
        .map(|view| RegistryRow {
            chip_type: view.chip_type.clone(),
            latest_version: view
                .latest_version
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            deprecated: view.deprecated,
            has_kats: view.has_kats,
            required_cap: view.required_cap.clone().unwrap_or_else(|| "-".to_string()),
            last_updated_at: view
                .last_updated_at
                .clone()
                .unwrap_or_else(|| "-".to_string()),
        })
        .collect();
    let (visible_rows, hidden_rows) = split_rows(rows, 6);
    render_html(&RegistryTableTemplate {
        visible_rows,
        hidden_rows,
    })
}

pub(crate) async fn registry_type_page(
    State(state): State<AppState>,
    Path(chip_type): Path<String>,
) -> Response {
    let normalized_type = chip_type.trim_start_matches('/').to_string();
    let registry = match materialize_registry(&state, None).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "@type":"ubl/error",
                    "code":"INTERNAL_ERROR",
                    "message": format!("registry materialization failed: {}", e),
                })),
            )
                .into_response();
        }
    };
    let Some(view) = registry.types.get(&normalized_type) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "@type":"ubl/error",
                "code":"NOT_FOUND",
                "message": format!("Registry type '{}' not found", normalized_type),
            })),
        )
            .into_response();
    };

    let versions: Vec<RegistryTypeVersionRow> = view
        .versions
        .values()
        .map(|ver| {
            let kats = ver
                .kats
                .iter()
                .enumerate()
                .map(|(index, kat)| {
                    let label = kat
                        .get("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or("kat")
                        .to_string();
                    let expected_decision = kat
                        .get("expected_decision")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-")
                        .to_string();
                    let expected_error = kat
                        .get("expected_error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-")
                        .to_string();
                    let input_json_preview = kat
                        .get("input")
                        .map(Value::to_string)
                        .map(|s| {
                            if s.len() > 240 {
                                format!("{}...", &s[..240])
                            } else {
                                s
                            }
                        })
                        .unwrap_or_else(|| "-".to_string());
                    RegistryKatRow {
                        index,
                        label,
                        expected_decision,
                        expected_error,
                        input_json_preview,
                    }
                })
                .collect();
            RegistryTypeVersionRow {
                version: ver.version.clone(),
                required_cap: ver.required_cap.clone().unwrap_or_else(|| "-".to_string()),
                kats_count: ver.kats.len(),
                register_cid: ver.register_cid.clone().unwrap_or_else(|| "-".to_string()),
                updated_at: ver.updated_at.clone().unwrap_or_else(|| "-".to_string()),
                kats,
            }
        })
        .collect();
    let deprecation_json = view
        .deprecation
        .as_ref()
        .map(Value::to_string)
        .unwrap_or_else(|| "-".to_string());

    render_html(&RegistryTypeTemplate {
        chip_type: view.chip_type.clone(),
        latest_version: view
            .latest_version
            .clone()
            .unwrap_or_else(|| "-".to_string()),
        deprecated: view.deprecated,
        description: view.description.clone().unwrap_or_else(|| "-".to_string()),
        docs_url: view.docs_url.clone(),
        deprecation_json,
        versions,
    })
}

pub(crate) async fn registry_kat_test(
    State(state): State<AppState>,
    Form(form): Form<RegistryKatTestForm>,
) -> Response {
    let registry = match materialize_registry(&state, None).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "@type":"ubl/error",
                    "code":"INTERNAL_ERROR",
                    "message": format!("registry materialization failed: {}", e),
                })),
            )
                .into_response();
        }
    };
    let Some(type_view) = registry.types.get(&form.chip_type) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "@type":"ubl/error",
                "code":"NOT_FOUND",
                "message": format!("Registry type '{}' not found", form.chip_type),
            })),
        )
            .into_response();
    };
    let Some(version_view) = type_view.versions.get(&form.version) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "@type":"ubl/error",
                "code":"NOT_FOUND",
                "message": format!(
                    "Registry version '{}' not found for type '{}'",
                    form.version, form.chip_type
                ),
            })),
        )
            .into_response();
    };
    let Some(kat) = version_view.kats.get(form.kat_index) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "@type":"ubl/error",
                "code":"NOT_FOUND",
                "message": format!(
                    "KAT index '{}' not found for type '{}' version '{}'",
                    form.kat_index, form.chip_type, form.version
                ),
            })),
        )
            .into_response();
    };

    let kat_label = kat
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("kat")
        .to_string();
    let expected_decision = kat
        .get("expected_decision")
        .and_then(|v| v.as_str())
        .unwrap_or("-")
        .to_string();
    let expected_error = kat
        .get("expected_error")
        .and_then(|v| v.as_str())
        .unwrap_or("-")
        .to_string();
    let Some(input_chip) = kat.get("input") else {
        return render_html(&RegistryKatResultTemplate {
            status_code: 400,
            kat_label,
            expected_decision,
            expected_error,
            actual_decision: "-".to_string(),
            actual_error: "missing_kat_input".to_string(),
            receipt_cid: "-".to_string(),
            pass: false,
            response_json: "{}".to_string(),
            message: "KAT input missing".to_string(),
        });
    };
    let body = match serde_json::to_vec(input_chip) {
        Ok(v) => v,
        Err(e) => {
            return render_html(&RegistryKatResultTemplate {
                status_code: 500,
                kat_label,
                expected_decision,
                expected_error,
                actual_decision: "-".to_string(),
                actual_error: "kat_input_serialize_error".to_string(),
                receipt_cid: "-".to_string(),
                pass: false,
                response_json: "{}".to_string(),
                message: format!("KAT input serialization failed: {}", e),
            });
        }
    };

    let (status, _headers, payload): (StatusCode, HeaderMap, Value) = submit_chip_bytes(&state, None, true, &body).await;
    let actual_decision = payload
        .get("decision")
        .and_then(|v| v.as_str())
        .unwrap_or("-")
        .to_string();
    let actual_error = payload
        .get("code")
        .and_then(|v| v.as_str())
        .or_else(|| {
            payload
                .get("receipt")
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("-")
        .to_string();
    let receipt_cid = payload
        .get("receipt_cid")
        .and_then(|v| v.as_str())
        .or_else(|| {
            payload
                .get("receipt")
                .and_then(|v| v.get("receipt_cid"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("-")
        .to_string();

    let decision_match = expected_decision == "-"
        || actual_decision
            .to_ascii_lowercase()
            .contains(&expected_decision.to_ascii_lowercase());
    let error_match = expected_error == "-" || actual_error == expected_error;
    let pass = status.is_success() && decision_match && error_match;
    let response_json = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string());
    let message = if pass {
        "KAT passed".to_string()
    } else {
        "KAT failed".to_string()
    };

    render_html(&RegistryKatResultTemplate {
        status_code: status.as_u16(),
        kat_label,
        expected_decision,
        expected_error,
        actual_decision,
        actual_error,
        receipt_cid,
        pass,
        response_json,
        message,
    })
}

pub(crate) async fn registry_types(
    State(state): State<AppState>,
    Query(query): Query<std::collections::BTreeMap<String, String>>,
) -> Response {
    let world = query.get("world").map(|s| s.as_str());
    let registry = match materialize_registry(&state, world).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "@type":"ubl/error",
                    "code":"INTERNAL_ERROR",
                    "message": format!("registry materialization failed: {}", e),
                })),
            )
                .into_response();
        }
    };

    let mut types: Vec<Value> = Vec::with_capacity(registry.types.len());
    for view in registry.types.values() {
        types.push(json!({
            "type": view.chip_type,
            "latest_version": view.latest_version,
            "deprecated": view.deprecated,
            "has_kats": view.has_kats,
            "required_cap": view.required_cap,
            "last_cid": view.last_cid,
            "last_updated_at": view.last_updated_at,
            "versions_count": view.versions.len(),
        }));
    }

    (
        StatusCode::OK,
        Json(json!({
            "@type": "ubl/registry.types",
            "count": types.len(),
            "types": types,
        })),
    )
        .into_response()
}

pub(crate) async fn registry_type_detail(
    State(state): State<AppState>,
    Path(chip_type): Path<String>,
) -> Response {
    let registry = match materialize_registry(&state, None).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "@type":"ubl/error",
                    "code":"INTERNAL_ERROR",
                    "message": format!("registry materialization failed: {}", e),
                })),
            )
                .into_response();
        }
    };
    let Some(view) = registry.types.get(&chip_type) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "@type":"ubl/error",
                "code":"NOT_FOUND",
                "message": format!("Registry type '{}' not found", chip_type),
            })),
        )
            .into_response();
    };

    let versions: Vec<Value> = view
        .versions
        .values()
        .map(|ver| {
            json!({
                "version": ver.version,
                "schema": ver.schema,
                "kats": ver.kats,
                "required_cap": ver.required_cap,
                "register_cid": ver.register_cid,
                "updated_at": ver.updated_at,
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(json!({
            "@type": "ubl/registry.type",
            "type": view.chip_type,
            "latest_version": view.latest_version,
            "deprecated": view.deprecated,
            "description": view.description,
            "docs_url": view.docs_url,
            "deprecation": view.deprecation,
            "has_kats": view.has_kats,
            "required_cap": view.required_cap,
            "last_cid": view.last_cid,
            "last_updated_at": view.last_updated_at,
            "versions": versions,
        })),
    )
        .into_response()
}

pub(crate) async fn registry_type_version(
    State(state): State<AppState>,
    Path((chip_type, ver)): Path<(String, String)>,
) -> Response {
    let registry = match materialize_registry(&state, None).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "@type":"ubl/error",
                    "code":"INTERNAL_ERROR",
                    "message": format!("registry materialization failed: {}", e),
                })),
            )
                .into_response();
        }
    };
    let Some(view) = registry.types.get(&chip_type) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "@type":"ubl/error",
                "code":"NOT_FOUND",
                "message": format!("Registry type '{}' not found", chip_type),
            })),
        )
            .into_response();
    };
    let Some(version) = view.versions.get(&ver) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "@type":"ubl/error",
                "code":"NOT_FOUND",
                "message": format!("Registry version '{}' not found for type '{}'", ver, chip_type),
            })),
        )
            .into_response();
    };

    (
        StatusCode::OK,
        Json(json!({
            "@type": "ubl/registry.version",
            "type": chip_type,
            "version": version.version,
            "schema": version.schema,
            "kats": version.kats,
            "required_cap": version.required_cap,
            "register_cid": version.register_cid,
            "updated_at": version.updated_at,
            "deprecated": view.deprecated,
            "deprecation": view.deprecation,
        })),
    )
        .into_response()
}

pub(crate) async fn materialize_registry(
    state: &AppState,
    world_filter: Option<&str>,
) -> Result<RegistryView, String> {
    fn world_matches(chip: &ubl_chipstore::StoredChip, world_filter: Option<&str>) -> bool {
        let Some(expected) = world_filter else {
            return true;
        };
        chip.chip_data
            .get("@world")
            .and_then(|v| v.as_str())
            .map(|w| w == expected)
            .unwrap_or(false)
    }

    fn type_entry<'a>(
        map: &'a mut std::collections::BTreeMap<String, RegistryTypeView>,
        chip_type: &str,
    ) -> &'a mut RegistryTypeView {
        map.entry(chip_type.to_string())
            .or_insert_with(|| RegistryTypeView {
                chip_type: chip_type.to_string(),
                latest_version: None,
                deprecated: false,
                has_kats: false,
                required_cap: None,
                description: None,
                docs_url: None,
                deprecation: None,
                last_cid: None,
                last_updated_at: None,
                versions: std::collections::BTreeMap::new(),
            })
    }

    let mut types = std::collections::BTreeMap::<String, RegistryTypeView>::new();

    let mut registers = state
        .chip_store
        .get_chips_by_type("ubl/meta.register")
        .await
        .map_err(|e| e.to_string())?;
    registers.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    for chip in registers {
        if !world_matches(&chip, world_filter) {
            continue;
        }
        let Ok(parsed) = ubl_runtime::meta_chip::parse_register(&chip.chip_data) else {
            continue;
        };
        let entry = type_entry(&mut types, &parsed.target_type);
        entry.latest_version = Some(parsed.type_version.clone());
        entry.description = Some(parsed.description.clone());
        entry.has_kats = entry.has_kats || !parsed.kats.is_empty();
        entry.required_cap = parsed.schema.required_cap.clone();
        entry.last_cid = Some(chip.cid.to_string());
        entry.last_updated_at = Some(chip.created_at.clone());
        entry.versions.insert(
            parsed.type_version.clone(),
            RegistryVersionView {
                version: parsed.type_version,
                schema: serde_json::to_value(parsed.schema.clone()).ok(),
                kats: parsed
                    .kats
                    .iter()
                    .filter_map(|k| serde_json::to_value(k).ok())
                    .collect(),
                required_cap: parsed.schema.required_cap.clone(),
                register_cid: Some(chip.cid.to_string()),
                updated_at: Some(chip.created_at.clone()),
            },
        );
    }

    let mut describes = state
        .chip_store
        .get_chips_by_type("ubl/meta.describe")
        .await
        .map_err(|e| e.to_string())?;
    describes.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    for chip in describes {
        if !world_matches(&chip, world_filter) {
            continue;
        }
        let Ok(parsed) = ubl_runtime::meta_chip::parse_describe(&chip.chip_data) else {
            continue;
        };
        let entry = type_entry(&mut types, &parsed.target_type);
        entry.description = Some(parsed.description);
        entry.docs_url = parsed.docs_url;
        entry.last_cid = Some(chip.cid.to_string());
        entry.last_updated_at = Some(chip.created_at.clone());
        if !parsed.kats.is_empty() {
            entry.has_kats = true;
            if let Some(ver) = entry.latest_version.clone() {
                if let Some(version_entry) = entry.versions.get_mut(&ver) {
                    version_entry.kats = parsed
                        .kats
                        .iter()
                        .filter_map(|k| serde_json::to_value(k).ok())
                        .collect();
                    version_entry.updated_at = Some(chip.created_at.clone());
                }
            }
        }
    }

    let mut deprecates = state
        .chip_store
        .get_chips_by_type("ubl/meta.deprecate")
        .await
        .map_err(|e| e.to_string())?;
    deprecates.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    for chip in deprecates {
        if !world_matches(&chip, world_filter) {
            continue;
        }
        let Ok(parsed) = ubl_runtime::meta_chip::parse_deprecate(&chip.chip_data) else {
            continue;
        };
        let entry = type_entry(&mut types, &parsed.target_type);
        entry.deprecated = true;
        entry.deprecation = Some(json!({
            "reason": parsed.reason,
            "replacement_type": parsed.replacement_type,
            "sunset_at": parsed.sunset_at,
            "cid": chip.cid.to_string(),
        }));
        entry.last_cid = Some(chip.cid.to_string());
        entry.last_updated_at = Some(chip.created_at.clone());
    }

    Ok(RegistryView { types })
}
