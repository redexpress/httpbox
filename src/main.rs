use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use eframe::egui;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(30, 30, 35);
const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(140, 145, 160);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(80, 140, 240);
const DANGER: egui::Color32 = egui::Color32::from_rgb(220, 80, 80);

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
    name: String,
    method: Method,
    url: String,
    query: Vec<KeyValue>,
    headers: Vec<KeyValue>,
    body_kind: BodyKind,
    body_text: String,
    timeout_secs: u32,
}

impl HttpRequest {
    fn new_named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
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

impl Default for HttpRequest {
    fn default() -> Self {
        Self::new_named("Request 1")
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
    requests: Vec<HttpRequest>,
    selected: usize,
    sending: bool,
    rx: Option<Receiver<SendResult>>,
    response: Option<HttpResponse>,
    error: Option<String>,
    body_dirty: bool,
    about_open: bool,
    confirm_new_open: bool,
    rename_target: Option<usize>,
    rename_buffer: String,
}

impl Default for HttpboxApp {
    fn default() -> Self {
        let first = HttpRequest::default();
        Self {
            requests: vec![first],
            selected: 0,
            sending: false,
            rx: None,
            response: None,
            error: None,
            body_dirty: false,
            about_open: false,
            confirm_new_open: false,
            rename_target: None,
            rename_buffer: String::new(),
        }
    }
}

impl HttpboxApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::apply_theme(&cc.egui_ctx);
        Self::default()
    }

    fn apply_theme(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::light();

        let border = egui::Stroke::new(1.0, TEXT_MUTED);
        let border_hover = egui::Stroke::new(1.5, egui::Color32::from_rgb(80, 130, 220));
        let border_focus = egui::Stroke::new(2.0, ACCENT);

        visuals.override_text_color = Some(TEXT_PRIMARY);
        visuals.extreme_bg_color = egui::Color32::from_rgb(255, 255, 255);
        visuals.panel_fill = egui::Color32::from_rgb(248, 248, 250);
        visuals.faint_bg_color = egui::Color32::from_rgb(240, 240, 244);

        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(255, 255, 255);
        visuals.widgets.noninteractive.bg_stroke = border;
        visuals.widgets.noninteractive.fg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 50));
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);

        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(255, 255, 255);
        visuals.widgets.inactive.bg_stroke = border;
        visuals.widgets.inactive.fg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 30, 40));
        visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);

        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(245, 248, 255);
        visuals.widgets.hovered.bg_stroke = border_hover;
        visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);

        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(230, 240, 255);
        visuals.widgets.active.bg_stroke = border_focus;
        visuals.widgets.active.rounding = egui::Rounding::same(4.0);

        visuals.widgets.open.bg_fill = egui::Color32::from_rgb(245, 248, 255);
        visuals.widgets.open.bg_stroke = border_hover;
        visuals.widgets.open.rounding = egui::Rounding::same(4.0);

        visuals.selection.bg_fill = egui::Color32::from_rgb(200, 220, 255);
        visuals.selection.stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 140, 220));

        visuals.window_fill = egui::Color32::from_rgb(252, 252, 254);
        visuals.window_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(160, 165, 180));
        visuals.window_rounding = egui::Rounding::same(6.0);

        ctx.set_visuals(visuals);
    }

    fn start_send(&mut self) {
        let req = self.current().clone();
        let url = req.url.trim().to_string();
        if url.is_empty() {
            self.error = Some("URL cannot be empty".to_string());
            warn!("send clicked with empty URL");
            return;
        }

        self.error = None;
        self.response = None;
        self.sending = true;
        self.body_dirty = false;

        let method = req.method;
        let query: Vec<KeyValue> = req
            .query
            .iter()
            .filter(|kv| kv.enabled && !kv.key.is_empty())
            .cloned()
            .collect();
        let headers: Vec<KeyValue> = req
            .headers
            .iter()
            .filter(|kv| kv.enabled && !kv.key.is_empty())
            .cloned()
            .collect();
        let body_kind = req.body_kind.clone();
        let body_text = req.body_text.clone();
        let timeout_secs = req.timeout_secs;

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

    fn current(&self) -> &HttpRequest {
        &self.requests[self.selected]
    }

    fn current_mut(&mut self) -> &mut HttpRequest {
        &mut self.requests[self.selected]
    }

    fn request_is_empty(&self) -> bool {
        let r = self.current();
        r.url.trim().is_empty()
            && r.query.iter().all(|kv| kv.key.is_empty() && kv.value.is_empty())
            && r.headers.iter().all(|kv| kv.key.is_empty() && kv.value.is_empty())
            && r.body_text.trim().is_empty()
    }

    fn reset_request(&mut self) {
        self.requests[self.selected] = HttpRequest::new_named(self.current().name.clone());
        self.response = None;
        self.error = None;
        self.body_dirty = false;
    }

    fn new_request(&mut self) {
        let next = format!("Request {}", self.requests.len() + 1);
        self.requests.push(HttpRequest::new_named(next));
        self.selected = self.requests.len() - 1;
        self.response = None;
        self.error = None;
        self.body_dirty = false;
    }

    fn select_request(&mut self, idx: usize) {
        if idx == self.selected || idx >= self.requests.len() {
            return;
        }
        self.selected = idx;
        self.response = None;
        self.error = None;
        self.body_dirty = false;
    }

    fn delete_request(&mut self, idx: usize) {
        if self.requests.len() <= 1 {
            self.requests[idx] = HttpRequest::new_named("Request 1");
            self.selected = 0;
        } else {
            self.requests.remove(idx);
            if self.selected >= self.requests.len() {
                self.selected = self.requests.len() - 1;
            } else if idx < self.selected {
                self.selected -= 1;
            }
        }
        self.response = None;
        self.error = None;
        self.body_dirty = false;
    }

    fn render_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui
                    .add(egui::Button::new("New Request").shortcut_text("Ctrl+N"))
                    .clicked()
                {
                    ui.close_menu();
                    if self.request_is_empty() {
                        self.reset_request();
                    } else {
                        self.confirm_new_open = true;
                    }
                }
                ui.separator();
                if ui
                    .add(egui::Button::new("Quit").shortcut_text("Ctrl+Q"))
                    .clicked()
                {
                    ui.close_menu();
                    info!("quit requested from menu");
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("About").clicked() {
                    ui.close_menu();
                    self.about_open = true;
                }
            });
        });
    }

    fn render_request_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            let method_str = self.current().method.as_str();
            egui::ComboBox::from_id_salt("method")
                .selected_text(method_str)
                .width(80.0)
                .show_ui(ui, |ui| {
                    let cur = self.current().method;
                    for m in Method::all() {
                        if ui.selectable_label(cur == m, m.as_str()).clicked() {
                            self.current_mut().method = m;
                        }
                    }
                });

            let remaining = ui.available_width() - 90.0;
            let edit_width = remaining.max(100.0);
            ui.allocate_ui(egui::vec2(edit_width, ui.available_height()), |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.current_mut().url)
                        .hint_text("https://example.com/api")
                        .desired_width(f32::INFINITY),
                );
            });

            let button_text = if self.sending { "Sending..." } else { "Send" };
            let btn = egui::Button::new(button_text).min_size(egui::vec2(80.0, 0.0));
            if ui
                .add_enabled(!self.sending, btn)
                .on_hover_text("Ctrl+Enter")
                .clicked()
            {
                self.start_send();
            }
        });
    }

    fn render_top_bar(&mut self, ui: &mut egui::Ui) {
        self.render_menu_bar(ui);
    }

    fn render_request_list(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Requests");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("+ New").clicked() {
                    self.new_request();
                }
            });
        });

        let mut to_select: Option<usize> = None;
        let mut to_delete: Option<usize> = None;
        let mut to_rename: Option<usize> = None;

        for (i, req) in self.requests.iter().enumerate() {
            ui.horizontal(|ui| {
                let label = if req.url.trim().is_empty() {
                    format!("{}  (no url)", req.name)
                } else {
                    format!("{}  {} {}", req.name, req.method.as_str(), req.url)
                };
                let selected = i == self.selected;
                if ui.selectable_label(selected, label).clicked() {
                    to_select = Some(i);
                }
                if ui.small_button("R").on_hover_text("Rename").clicked() {
                    to_rename = Some(i);
                }
                if ui.small_button("x").on_hover_text("Delete").clicked() {
                    to_delete = Some(i);
                }
            });
        }

        if let Some(i) = to_select {
            self.select_request(i);
        }
        if let Some(i) = to_delete {
            self.delete_request(i);
        }
        if let Some(i) = to_rename {
            self.rename_buffer = self.requests[i].name.clone();
            self.rename_target = Some(i);
        }
    }

    fn render_section<F>(ui: &mut egui::Ui, title: &str, add_contents: F)
    where
        F: FnOnce(&mut egui::Ui),
    {
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(title).strong().size(13.0));
                    ui.add_space(4.0);
                    add_contents(ui);
                });
            });
    }

    fn render_editor(&mut self, ui: &mut egui::Ui) {
        let body_kind = self.current().body_kind.clone();
        let body_text = self.current().body_text.clone();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(4.0);

                Self::render_section(ui, "Headers", |ui| {
                    Self::render_kv_table(ui, &mut self.current_mut().headers);
                });
                ui.add_space(6.0);

                Self::render_section(ui, "Body", |ui| {
                    self.render_body_editor(ui, &body_kind, &body_text);
                });
            });
    }

    fn render_rename_window(&mut self, ctx: &egui::Context) {
        let Some(target) = self.rename_target else { return };
        let mut open = true;
        let mut done = false;
        let mut cancelled = false;

        egui::Window::new("Rename Request")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Name:");
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.rename_buffer)
                        .desired_width(240.0),
                );
                if resp.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                {
                    done = true;
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        done = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancelled = true;
                    }
                });
            });

        if done {
            let new_name = self.rename_buffer.trim();
            if !new_name.is_empty() {
                self.requests[target].name = new_name.to_string();
            }
            self.rename_target = None;
        } else if cancelled || !open {
            self.rename_target = None;
        }
    }

    fn render_about_window(&mut self, ctx: &egui::Context) {
        let mut open = self.about_open;
        egui::Window::new("About HTTPBox")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.heading("HTTPBox");
                    ui.label(format!("Version {}", APP_VERSION));
                    ui.label("A lightweight HTTP API client.");
                    ui.add_space(8.0);
                    ui.label("Built with Rust + egui.");
                    ui.add_space(8.0);
                    ui.hyperlink_to("Project docs", "https://github.com/");
                });
            });
        if !open {
            self.about_open = false;
        }
    }

    fn render_confirm_new_window(&mut self, ctx: &egui::Context) {
        if !self.confirm_new_open {
            return;
        }
        let mut open = self.confirm_new_open;
        egui::Window::new("Discard current request?")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("The current request will be discarded.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Discard").clicked() {
                        self.reset_request();
                        self.confirm_new_open = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.confirm_new_open = false;
                    }
                });
            });
        if !open {
            self.confirm_new_open = false;
        }
    }

    fn render_kv_table(ui: &mut egui::Ui, kvs: &mut Vec<KeyValue>) {
        let mut to_remove: Option<usize> = None;
        let available_w = ui.available_width();
        let key_w = (available_w * 0.28).clamp(120.0, 260.0);
        let x_btn_w = 32.0;
        let gap = 6.0;
        let value_w = (available_w - key_w - x_btn_w - 24.0 - 32.0 - gap * 4.0).max(120.0);

        if kvs.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("No entries. Click \"+ Add\" to create one.")
                        .color(egui::Color32::from_rgb(120, 120, 130))
                        .italics()
                        .size(12.0),
                );
                ui.add_space(12.0);
            });
        } else {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.colored_label(
                    egui::Color32::from_rgb(90, 95, 110),
                    egui::RichText::new("KEY").size(11.0).strong(),
                );
                ui.add_space(key_w - 24.0);
                ui.colored_label(
                    egui::Color32::from_rgb(90, 95, 110),
                    egui::RichText::new("VALUE").size(11.0).strong(),
                );
            });
            ui.add_space(4.0);
        }

        for (i, kv) in kvs.iter_mut().enumerate() {
            let mut key = kv.key.clone();
            let mut value = kv.value.clone();
            let mut enabled = kv.enabled;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.checkbox(&mut enabled, "");

                let idle_stroke = egui::Stroke::new(1.0, TEXT_MUTED);
                let focus_stroke = egui::Stroke::new(1.0, ACCENT);
                let rounding = egui::Rounding::same(4.0);

                let mut key_frame = egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(idle_stroke)
                    .rounding(rounding)
                    .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                    .begin(ui);
                let key_edit_resp = {
                    let fui = &mut key_frame.content_ui;
                    fui.add(
                        egui::TextEdit::singleline(&mut key)
                            .hint_text("Header-Name")
                            .desired_width(key_w)
                            .frame(false)
                            .font(egui::TextStyle::Monospace),
                    )
                };
                let key_frame_resp = key_frame.end(ui);
                if key_edit_resp.has_focus() {
                    ui.painter().rect_stroke(
                        key_frame_resp.rect,
                        rounding,
                        focus_stroke,
                    );
                    ui.ctx().request_repaint();
                }

                let mut value_frame = egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(idle_stroke)
                    .rounding(rounding)
                    .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                    .begin(ui);
                let value_edit_resp = {
                    let fui = &mut value_frame.content_ui;
                    fui.add(
                        egui::TextEdit::singleline(&mut value)
                            .hint_text("header value")
                            .desired_width(value_w)
                            .frame(false)
                            .font(egui::TextStyle::Monospace),
                    )
                };
                let value_frame_resp = value_frame.end(ui);
                if value_edit_resp.has_focus() {
                    ui.painter().rect_stroke(
                        value_frame_resp.rect,
                        rounding,
                        focus_stroke,
                    );
                    ui.ctx().request_repaint();
                }

                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("×").size(14.0).strong().color(
                                egui::Color32::from_rgb(180, 50, 60),
                            ),
                        )
                        .min_size(egui::vec2(x_btn_w, 0.0))
                        .fill(egui::Color32::from_rgb(250, 235, 238))
                        .rounding(egui::Rounding::same(4.0)),
                    )
                    .clicked()
                {
                    to_remove = Some(i);
                }
            });

            kv.key = key;
            kv.value = value;
            kv.enabled = enabled;
            ui.add_space(4.0);
        }

        if let Some(i) = to_remove {
            kvs.remove(i);
        }

        ui.add_space(4.0);
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("+ Add").size(13.0).strong(),
                )
                .min_size(egui::vec2(80.0, 24.0))
                .fill(egui::Color32::from_rgb(55, 90, 140))
                .rounding(egui::Rounding::same(4.0)),
            )
            .clicked()
        {
            kvs.push(KeyValue::new("", ""));
        }
    }

    fn render_body_editor(&mut self, ui: &mut egui::Ui, body_kind: &BodyKind, body_text: &str) {
        ui.horizontal(|ui| {
            ui.label("Content-Type:");
            egui::ComboBox::from_id_salt("body_kind")
                .selected_text(match body_kind {
                    BodyKind::None => "none",
                    BodyKind::Json => "application/json",
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(matches!(body_kind, BodyKind::None), "none")
                        .clicked()
                    {
                        self.current_mut().body_kind = BodyKind::None;
                    }
                    if ui
                        .selectable_label(matches!(body_kind, BodyKind::Json), "application/json")
                        .clicked()
                    {
                        self.current_mut().body_kind = BodyKind::Json;
                        self.sync_content_type_header();
                    }
                });

            if matches!(body_kind, BodyKind::Json) {
                if ui.button("Format").clicked() {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body_text) {
                        if let Ok(s) = serde_json::to_string_pretty(&v) {
                            self.current_mut().body_text = s;
                        }
                    }
                }
            }
        });

        ui.add_space(4.0);

        match body_kind {
            BodyKind::None => {
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("(no body)")
                            .color(egui::Color32::GRAY)
                            .italics(),
                    );
                });
            }
            BodyKind::Json => {
                let mut text = self.current().body_text.clone();
                let resp = ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(8)
                        .hint_text("{\"key\": \"value\"}"),
                );
                if resp.changed() {
                    self.current_mut().body_text = text;
                    self.body_dirty = true;
                }

                if let Err(e) =
                    serde_json::from_str::<serde_json::Value>(&self.current().body_text)
                {
                    if !self.current().body_text.trim().is_empty() {
                        ui.colored_label(
                            DANGER,
                            format!("JSON error: {}", e),
                        );
                    }
                }
            }
        }
    }

    fn sync_content_type_header(&mut self) {
        let already = self
            .current_mut()
            .headers
            .iter_mut()
            .find(|kv| kv.key.eq_ignore_ascii_case("Content-Type"));
        match already {
            Some(kv) => {
                kv.value = "application/json".to_string();
                kv.enabled = true;
            }
            None => {
                self.current_mut()
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
        ui.colored_label(DANGER, err);
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
        DANGER
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

        let ctrl_n = ctx.input(|i| i.key_pressed(egui::Key::N) && i.modifiers.command);
        if ctrl_n && self.rename_target.is_none() {
            if self.request_is_empty() {
                self.new_request();
            } else {
                self.confirm_new_open = true;
            }
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            self.render_top_bar(ui);
        });

        egui::SidePanel::left("request_list")
            .resizable(true)
            .default_width(220.0)
            .min_width(160.0)
            .max_width(360.0)
            .show(ctx, |ui| {
                self.render_request_list(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_request_bar(ui);
            ui.separator();

            ui.vertical(|ui| {
                let available = ui.available_size();
                let editor_height = (available.y * 0.55).clamp(200.0, available.y - 120.0);

                egui::Resize::default()
                    .default_size([available.x, editor_height])
                    .resizable(true)
                    .min_height(150.0)
                    .show(ui, |ui| {
                        self.render_editor(ui);
                    });

                ui.separator();

                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        if let Some(err) = &self.error {
                            Self::render_error(ui, err);
                        } else if let Some(resp) = &self.response {
                            Self::render_status_line(ui, resp);
                            ui.separator();
                            Self::render_body_response(ui, &resp.body);
                        } else {
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new("Click \"Send\" to view the response")
                                        .color(egui::Color32::GRAY),
                                );
                            });
                        }
                    });
            });
        });

        if self.sending {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        self.render_about_window(ctx);
        self.render_confirm_new_window(ctx);
        self.render_rename_window(ctx);
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
