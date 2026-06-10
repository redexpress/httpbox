use std::time::{Duration, Instant};

use base64::Engine;
use tracing::{debug, info};

use crate::model::error::HttpError;
use crate::model::request::{AuthConfig, BodyKind, KeyValue, Method};
use crate::model::response::HttpResponse;

pub type HttpResult = Result<HttpResponse, HttpError>;

const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "x-auth-token",
    "proxy-authorization",
];

pub(crate) fn redact_value(key: &str, value: &str) -> String {
    if SENSITIVE_HEADERS
        .iter()
        .any(|h| h.eq_ignore_ascii_case(key))
    {
        if value.len() <= 4 {
            return "***".to_string();
        }
        return format!("{}***", &value[..4]);
    }
    value.to_string()
}

pub(crate) fn log_request_summary(
    method: &str,
    url: &str,
    headers: &[KeyValue],
    body_bytes: usize,
) {
    info!(method = %method, url = %url, body_bytes, "sending request");
    for kv in headers {
        debug!(
            header = %kv.key,
            value = %redact_value(&kv.key, &kv.value),
            "request header"
        );
    }
}

pub(crate) fn log_response_summary(
    status: u16,
    status_text: &str,
    elapsed_ms: u128,
    size_bytes: usize,
) {
    info!(
        status,
        status_text = %status_text,
        elapsed_ms,
        size_bytes,
        "response received"
    );
}

pub(crate) fn has_header(headers: &[KeyValue], name: &str) -> bool {
    headers
        .iter()
        .any(|kv| kv.enabled && kv.key.eq_ignore_ascii_case(name))
}

pub(crate) fn apply_auth(
    builder: reqwest::RequestBuilder,
    auth: &AuthConfig,
    headers: &[KeyValue],
) -> reqwest::RequestBuilder {
    if has_header(headers, "Authorization") {
        return builder;
    }
    match &auth.kind {
        crate::model::request::AuthKind::None => builder,
        crate::model::request::AuthKind::Bearer => {
            if auth.bearer_token.is_empty() {
                builder
            } else {
                builder.header("Authorization", format!("Bearer {}", auth.bearer_token))
            }
        }
        crate::model::request::AuthKind::Basic => {
            let raw = format!("{}:{}", auth.basic_user, auth.basic_password);
            let encoded = base64::engine::general_purpose::STANDARD.encode(raw.as_bytes());
            builder.header("Authorization", format!("Basic {}", encoded))
        }
    }
}

pub async fn execute_request(
    method: Method,
    url: &str,
    query: &[KeyValue],
    headers: &[KeyValue],
    body_kind: &BodyKind,
    body_text: &str,
    timeout_secs: u32,
    auth: &AuthConfig,
) -> HttpResult {
    let start = Instant::now();

    let mut parsed = reqwest::Url::parse(url).map_err(|e| HttpError::InvalidUrl(e.to_string()))?;
    {
        let mut qp = parsed.query_pairs_mut();
        for kv in query {
            qp.append_pair(&kv.key, &kv.value);
        }
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs as u64))
        .build()
        .map_err(|e| HttpError::Network(e.to_string()))?;

    let mut builder = client.request(method.as_reqwest(), parsed);

    for kv in headers {
        builder = builder.header(&kv.key, &kv.value);
    }

    builder = apply_auth(builder, auth, headers);

    builder = match body_kind {
        BodyKind::None => builder,
        BodyKind::Json => {
            if !has_header(headers, "Content-Type") {
                builder = builder.header("Content-Type", "application/json");
            }
            builder.body(body_text.to_string())
        }
    };

    let resp = tokio::time::timeout(Duration::from_secs(timeout_secs as u64), builder.send())
        .await
        .map_err(|_| HttpError::Timeout(timeout_secs))?
        .map_err(|e| HttpError::Network(e.to_string()))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| HttpError::Network(e.to_string()))?;
    Ok(HttpResponse {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        body,
        elapsed_ms: start.elapsed().as_millis(),
    })
}
