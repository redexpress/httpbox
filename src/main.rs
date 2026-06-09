use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use eframe::egui;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

fn init_logging() {
    let filter = EnvFilter::try_from_env("HTTPBOX_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info,httpbox=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();
}

const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "x-auth-token",
    "proxy-authorization",
];

fn redact_value(key: &str, value: &str) -> String {
    if SENSITIVE_HEADERS.iter().any(|h| h.eq_ignore_ascii_case(key)) {
        if value.len() <= 4 {
            return "***".to_string();
        }
        return format!("{}***", &value[..4]);
    }
    value.to_string()
}

fn log_request_summary(method: &str, url: &str, headers: &[KeyValue], body_bytes: usize) {
    info!(method = %method, url = %url, body_bytes, "sending request");
    for kv in headers {
        debug!(
            header = %kv.key,
            value = %redact_value(&kv.key, &kv.value),
            "request header"
        );
    }
}

fn log_response_summary(status: u16, status_text: &str, elapsed_ms: u128, size_bytes: usize) {
    info!(
        status,
        status_text = %status_text,
        elapsed_ms,
        size_bytes,
        "response received"
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl Method {
    fn all() -> [Method; 7] {
        [
            Method::Get,
            Method::Post,
            Method::Put,
            Method::Delete,
            Method::Patch,
            Method::Head,
            Method::Options,
        ]
    }

    fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Patch => "PATCH",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
        }
    }

    fn as_reqwest(self) -> reqwest::Method {
        match self {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Delete => reqwest::Method::DELETE,
            Method::Patch => reqwest::Method::PATCH,
            Method::Head => reqwest::Method::HEAD,
            Method::Options => reqwest::Method::OPTIONS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeyValue {
    enabled: bool,
    key: String,
    value: String,
}

impl KeyValue {
    fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            enabled: true,
            key: key.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum BodyKind {
    None,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HttpRequest {
    method: Method,
    url: String,
    query: Vec<KeyValue>,
    headers: Vec<KeyValue>,
    body_kind: BodyKind,
    body_text: String,
    timeout_secs: u32,
}

impl Default for HttpRequest {
    fn default() -> Self {
        Self {
            method: Method::Get,
            url: String::new(),
            query: Vec::new(),
            headers: Vec::new(),
            body_kind: BodyKind::None,
            body_text: String::new(),
            timeout_secs: 30,
        }
    }
}

#[derive(Debug, Error)]
enum HttpError {
    #[error("URL parse error: {0}")]
    InvalidUrl(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Request timeout ({0}s)")]
    Timeout(u32),
}

#[derive(Debug, Clone)]
struct HttpResponse {
    status: u16,
    status_text: String,
    body: String,
    elapsed_ms: u128,
}

enum SendResult {
    Ok(HttpResponse),
    Err(HttpError),
}

struct HttpboxApp {
    request: HttpRequest,
    sending: bool,
    rx: Option<Receiver<SendResult>>,
    response: Option<HttpResponse>,
    error: Option<String>,
    body_dirty: bool,
}

impl Default for HttpboxApp {
    fn default() -> Self {
        Self {
            request: HttpRequest::default(),
            sending: false,
            rx: None,
            response: None,
            error: None,
            body_dirty: false,
        }
    }
}

impl HttpboxApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn start_send(&mut self) {
        let url = self.request.url.trim().to_string();
        if url.is_empty() {
            self.error = Some("URL cannot be empty".to_string());
            warn!("send clicked with empty URL");
            return;
        }

        self.error = None;
        self.response = None;
        self.sending = true;
        self.body_dirty = false;

        let method = self.request.method;
        let query: Vec<KeyValue> = self
            .request
            .query
            .iter()
            .filter(|kv| kv.enabled && !kv.key.is_empty())
            .cloned()
            .collect();
        let headers: Vec<KeyValue> = self
            .request
            .headers
            .iter()
            .filter(|kv| kv.enabled && !kv.key.is_empty())
            .cloned()
            .collect();
        let body_kind = self.request.body_kind.clone();
        let body_text = self.request.body_text.clone();
        let timeout_secs = self.request.timeout_secs;

        let (tx, rx) = channel();
        self.rx = Some(rx);

        tokio::spawn(async move {
            let start = Instant::now();
            log_request_summary(
                method.as_str(),
                &url,
                &headers,
                match body_kind {
                    BodyKind::None => 0,
                    BodyKind::Json => body_text.len(),
                },
            );

            let result: Result<HttpResponse, HttpError> = async {
                let mut parsed = reqwest::Url::parse(&url)
                    .map_err(|e| HttpError::InvalidUrl(e.to_string()))?;
                {
                    let mut qp = parsed.query_pairs_mut();
                    for kv in &query {
                        qp.append_pair(&kv.key, &kv.value);
                    }
                }

                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(timeout_secs as u64))
                    .build()
                    .map_err(|e| HttpError::Network(e.to_string()))?;

                let mut builder = client.request(method.as_reqwest(), parsed);

                for kv in &headers {
                    builder = builder.header(&kv.key, &kv.value);
                }

                builder = match body_kind {
                    BodyKind::None => builder,
                    BodyKind::Json => {
                        if !has_header(&headers, "Content-Type") {
                            builder = builder.header("Content-Type", "application/json");
                        }
                        builder.body(body_text)
                    }
                };

                let resp = tokio::time::timeout(
                    Duration::from_secs(timeout_secs as u64),
                    builder.send(),
                )
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
            .await;

            let _ = tx.send(match result {
                Ok(resp) => {
                    log_response_summary(
                        resp.status,
                        &resp.status_text,
                        resp.elapsed_ms,
                        resp.body.len(),
                    );
                    SendResult::Ok(resp)
                }
                Err(e) => {
                    error!(error = %e, "request failed");
                    SendResult::Err(e)
                }
            });
        });
    }

    fn poll_response(&mut self) {
        if let Some(rx) = &self.rx {
            if let Ok(result) = rx.try_recv() {
                self.sending = false;
                self.rx = None;
                match result {
                    SendResult::Ok(resp) => {
                        self.response = Some(resp);
                        self.error = None;
                    }
                    SendResult::Err(e) => {
                        self.error = Some(e.to_string());
                        self.response = None;
                    }
                }
            }
        }
    }

    fn render_top_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            egui::ComboBox::from_id_salt("method")
                .selected_text(self.request.method.as_str())
                .width(80.0)
                .show_ui(ui, |ui| {
                    for m in Method::all() {
                        ui.selectable_value(&mut self.request.method, m, m.as_str());
                    }
                });

            ui.add(
                egui::TextEdit::singleline(&mut self.request.url)
                    .hint_text("https://example.com/api")
                    .desired_width(ui.available_width() - 90.0),
            );

            let button_text = if self.sending { "Sending..." } else { "Send" };
            let btn = egui::Button::new(button_text).min_size(egui::vec2(80.0, 0.0));
            if ui.add_enabled(!self.sending, btn).clicked() {
                self.start_send();
            }
        });
    }

    fn render_kv_table(ui: &mut egui::Ui, kvs: &mut Vec<KeyValue>) {
        let mut to_remove: Option<usize> = None;
        egui::Grid::new("kv_table")
            .num_columns(4)
            .spacing([8.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.strong("");
                ui.strong("Key");
                ui.strong("Value");
                ui.strong("");
                ui.end_row();

                for (i, kv) in kvs.iter_mut().enumerate() {
                    ui.checkbox(&mut kv.enabled, "");
                    ui.add(
                        egui::TextEdit::singleline(&mut kv.key)
                            .hint_text("key")
                            .desired_width(160.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut kv.value)
                            .hint_text("value")
                            .desired_width(f32::INFINITY),
                    );
                    if ui.button("x").clicked() {
                        to_remove = Some(i);
                    }
                    ui.end_row();
                }
            });

        if let Some(i) = to_remove {
            kvs.remove(i);
        }

        if ui.button("+ Add").clicked() {
            kvs.push(KeyValue::new("", ""));
        }
    }

    fn render_preset_buttons(ui: &mut egui::Ui, headers: &mut Vec<KeyValue>) {
        ui.horizontal(|ui| {
            ui.label("Quick add:");
            for (name, value) in [
                ("Content-Type", "application/json"),
                ("Authorization", "Bearer "),
                ("User-Agent", "HTTPBox/0.1"),
            ] {
                if ui.small_button(format!("{}: {}", name, value)).clicked() {
                    if !headers.iter().any(|kv| kv.key.eq_ignore_ascii_case(name)) {
                        headers.push(KeyValue::new(name, value));
                    }
                }
            }
        });
    }

    fn render_body(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Content-Type:");
            egui::ComboBox::from_id_salt("body_kind")
                .selected_text(match self.request.body_kind {
                    BodyKind::None => "none",
                    BodyKind::Json => "application/json",
                })
                .show_ui(ui, |ui| {
                    let cur = self.request.body_kind.clone();
                    if ui
                        .selectable_label(matches!(cur, BodyKind::None), "none")
                        .clicked()
                    {
                        self.request.body_kind = BodyKind::None;
                    }
                    if ui
                        .selectable_label(matches!(cur, BodyKind::Json), "application/json")
                        .clicked()
                    {
                        self.request.body_kind = BodyKind::Json;
                        self.sync_content_type_header();
                    }
                });

            if matches!(self.request.body_kind, BodyKind::Json) {
                if ui.button("Format").clicked() {
                    if let Ok(v) =
                        serde_json::from_str::<serde_json::Value>(&self.request.body_text)
                    {
                        if let Ok(s) = serde_json::to_string_pretty(&v) {
                            self.request.body_text = s;
                        }
                    }
                }
            }
        });

        match self.request.body_kind {
            BodyKind::None => {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("(no body)")
                            .color(egui::Color32::GRAY)
                            .italics(),
                    );
                });
            }
            BodyKind::Json => {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut text = self.request.body_text.clone();
                        let resp = ui.add(
                            egui::TextEdit::multiline(&mut text)
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .desired_rows(10)
                                .hint_text("{\"key\": \"value\"}"),
                        );
                        if resp.changed() {
                            self.request.body_text = text;
                            self.body_dirty = true;
                        }

                        if let Err(e) =
                            serde_json::from_str::<serde_json::Value>(&self.request.body_text)
                        {
                            if !self.request.body_text.trim().is_empty() {
                                ui.colored_label(
                                    egui::Color32::from_rgb(220, 80, 80),
                                    format!("JSON error: {}", e),
                                );
                            }
                        }
                    });
            }
        }
    }

    fn sync_content_type_header(&mut self) {
        let already = self
            .request
            .headers
            .iter_mut()
            .find(|kv| kv.key.eq_ignore_ascii_case("Content-Type"));
        match already {
            Some(kv) => {
                kv.value = "application/json".to_string();
                kv.enabled = true;
            }
            None => {
                self.request
                    .headers
                    .push(KeyValue::new("Content-Type", "application/json"));
            }
        }
    }

    fn render_status_line(ui: &mut egui::Ui, resp: &HttpResponse) {
        let color = status_color(resp.status);
        ui.horizontal(|ui| {
            ui.colored_label(color, format!("{} {}", resp.status, resp.status_text));
            ui.label(format!("· {} ms", resp.elapsed_ms));
        });
    }

    fn render_error(ui: &mut egui::Ui, err: &str) {
        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
    }

    fn render_body_response(ui: &mut egui::Ui, body: &str) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut text = body.to_string();
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(20),
                );
            });
    }
}

fn status_color(code: u16) -> egui::Color32 {
    if (200..300).contains(&code) {
        egui::Color32::from_rgb(80, 170, 80)
    } else if (300..400).contains(&code) {
        egui::Color32::from_rgb(80, 140, 200)
    } else if (400..500).contains(&code) {
        egui::Color32::from_rgb(220, 170, 60)
    } else if code >= 500 {
        egui::Color32::from_rgb(220, 80, 80)
    } else {
        egui::Color32::GRAY
    }
}

fn has_header(headers: &[KeyValue], name: &str) -> bool {
    headers
        .iter()
        .any(|kv| kv.enabled && kv.key.eq_ignore_ascii_case(name))
}

impl eframe::App for HttpboxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_response();

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            self.render_top_bar(ui);
        });

        egui::SidePanel::left("request_panel")
            .resizable(true)
            .default_width(420.0)
            .min_width(280.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(4.0);

                        ui.collapsing("Query Params", |ui| {
                            Self::render_kv_table(ui, &mut self.request.query);
                        });

                        ui.collapsing("Headers", |ui| {
                            Self::render_preset_buttons(ui, &mut self.request.headers);
                            Self::render_kv_table(ui, &mut self.request.headers);
                        });

                        ui.collapsing("Body", |ui| {
                            self.render_body(ui);
                        });

                        ui.collapsing("Settings", |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Timeout (s):");
                                ui.add(
                                    egui::DragValue::new(&mut self.request.timeout_secs)
                                        .range(1..=600)
                                        .speed(1),
                                );
                            });
                        });
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(err) = &self.error {
                Self::render_error(ui, err);
            } else if let Some(resp) = &self.response {
                Self::render_status_line(ui, resp);
                ui.separator();
                Self::render_body_response(ui, &resp.body);
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(
                        egui::RichText::new("Click \"Send\" to view the response")
                            .color(egui::Color32::GRAY),
                    );
                });
            }
        });

        if self.sending {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

#[tokio::main]
async fn main() -> eframe::Result<()> {
    init_logging();
    info!("HTTPBox starting");
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1100.0, 700.0])
        .with_title("HTTPBox");
    eframe::run_native(
        "HTTPBox",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(HttpboxApp::new(cc)))),
    )
}
