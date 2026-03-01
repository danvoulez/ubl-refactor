use reqwest::Client;
use serde_json::json;
use tracing::warn;
use ubl_runtime::durable_store::OutboxEvent;

pub(crate) fn outbox_endpoint_from_env() -> Option<String> {
    std::env::var("UBL_OUTBOX_ENDPOINT")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub(crate) async fn deliver_emit_receipt_event(
    client: &Client,
    endpoint: Option<&str>,
    event: OutboxEvent,
) -> Result<(), String> {
    let Some(endpoint) = endpoint else {
        warn!(
            event_id = event.id,
            "outbox: no endpoint configured, emit_receipt dropped"
        );
        return Ok(());
    };

    let payload = json!({
        "event_id": event.id,
        "event_type": event.event_type,
        "attempt": event.attempts.saturating_add(1),
        "payload": event.payload_json,
    });

    let response = client
        .post(endpoint)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("outbox http send failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<unreadable body>".to_string());
        let body_snippet: String = body.chars().take(240).collect();
        return Err(format!(
            "outbox endpoint returned {} body={}",
            status, body_snippet
        ));
    }

    Ok(())
}
