use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use eframe::egui;
use tracing::{error, warn};

use crate::http::client::{execute_request, log_request_summary, log_response_summary, HttpResult};
use crate::model::request::{HttpRequest, KeyValue, ResponseLayout};
use crate::model::response::HttpResponse;
use crate::ui::editor::render_editor;
use crate::ui::request_panel::{
    render_about_window, render_confirm_new_window, render_rename_window, render_request_bar,
    render_request_list, render_top_bar,
};
use crate::ui::response_panel::render_response_panel;
use crate::ui::theme::apply_theme;

pub struct HttpboxApp {
    pub requests: Vec<HttpRequest>,
    pub selected: usize,
    pub sending: bool,
    pub rx: Option<Receiver<HttpResult>>,
    pub response: Option<HttpResponse>,
    pub error: Option<String>,
    pub body_dirty: bool,
    pub about_open: bool,
    pub confirm_new_open: bool,
    pub rename_target: Option<usize>,
    pub rename_buffer: String,
    pub response_layout: ResponseLayout,
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
            response_layout: ResponseLayout::Bottom,
        }
    }
}

impl HttpboxApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_theme(&cc.egui_ctx);
        Self::default()
    }

    pub fn start_send(&mut self) {
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
        let auth = req.auth.clone();

        let (tx, rx) = channel();
        self.rx = Some(rx);

        let body_bytes = match body_kind {
            crate::model::request::BodyKind::None => 0,
            crate::model::request::BodyKind::Json => body_text.len(),
        };

        tokio::spawn(async move {
            let start = Instant::now();
            log_request_summary(method.as_str(), &url, &headers, body_bytes);

            let result = execute_request(
                method,
                &url,
                &query,
                &headers,
                &body_kind,
                &body_text,
                timeout_secs,
                &auth,
            )
            .await;

            let result = match result {
                Ok(mut resp) => {
                    resp.elapsed_ms = start.elapsed().as_millis();
                    log_response_summary(
                        resp.status,
                        &resp.status_text,
                        resp.elapsed_ms,
                        resp.body.len(),
                    );
                    Ok(resp)
                }
                Err(e) => {
                    error!(error = %e, "request failed");
                    Err(e)
                }
            };

            let _ = tx.send(result);
        });
    }

    pub fn poll_response(&mut self) {
        if let Some(rx) = &self.rx {
            if let Ok(result) = rx.try_recv() {
                self.sending = false;
                self.rx = None;
                match result {
                    Ok(resp) => {
                        self.response = Some(resp);
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e.to_string());
                        self.response = None;
                    }
                }
            }
        }
    }

    pub fn current(&self) -> &HttpRequest {
        &self.requests[self.selected]
    }

    pub fn current_mut(&mut self) -> &mut HttpRequest {
        &mut self.requests[self.selected]
    }

    pub fn request_is_empty(&self) -> bool {
        let r = self.current();
        r.url.trim().is_empty()
            && r.query
                .iter()
                .all(|kv| kv.key.is_empty() && kv.value.is_empty())
            && r.headers
                .iter()
                .all(|kv| kv.key.is_empty() && kv.value.is_empty())
            && r.body_text.trim().is_empty()
    }

    pub fn reset_request(&mut self) {
        let prev = self.current();
        let name = prev.name.clone();
        let name_auto = prev.name_auto;
        self.requests[self.selected] = HttpRequest::new_named(name);
        self.requests[self.selected].name_auto = name_auto;
        self.response = None;
        self.error = None;
        self.body_dirty = false;
    }

    pub fn new_request(&mut self) {
        let next = format!("Request {}", self.requests.len() + 1);
        self.requests.push(HttpRequest::new_named(next));
        self.selected = self.requests.len() - 1;
        self.response = None;
        self.error = None;
        self.body_dirty = false;
    }

    pub fn select_request(&mut self, idx: usize) {
        if idx == self.selected || idx >= self.requests.len() {
            return;
        }
        self.selected = idx;
        self.response = None;
        self.error = None;
        self.body_dirty = false;
    }

    pub fn delete_request(&mut self, idx: usize) {
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
            render_top_bar(self, ui);
            ui.separator();
            render_request_bar(self, ui);
        });

        egui::SidePanel::left("request_list")
            .resizable(true)
            .default_width(220.0)
            .min_width(160.0)
            .max_width(360.0)
            .show(ctx, |ui| {
                render_request_list(self, ui);
            });

        if matches!(self.response_layout, ResponseLayout::Right) {
            egui::SidePanel::right("response_side")
                .resizable(true)
                .default_width(420.0)
                .min_width(240.0)
                .max_width(900.0)
                .show(ctx, |ui| {
                    render_response_panel(self, ui);
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if matches!(self.response_layout, ResponseLayout::Bottom) {
                let available = ui.available_size();
                let editor_height = (available.y * 0.55).clamp(200.0, available.y - 120.0);

                egui::Resize::default()
                    .id_salt("editor_resize_bottom")
                    .default_size([available.x, editor_height])
                    .resizable(true)
                    .min_height(150.0)
                    .with_stroke(false)
                    .show(ui, |ui| {
                        render_editor(self, ui);
                    });

                ui.separator();
                render_response_panel(self, ui);
            } else {
                render_editor(self, ui);
            }
        });

        if self.sending {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        render_about_window(self, ctx);
        render_confirm_new_window(self, ctx);
        render_rename_window(self, ctx);
    }
}
