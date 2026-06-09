use std::sync::mpsc::{channel, Receiver};
use std::time::Instant;

use eframe::egui;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Method {
    Get,
    Post,
}

impl Method {
    fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
        }
    }

    fn as_reqwest(self) -> reqwest::Method {
        match self {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
        }
    }
}

#[derive(Debug, Error)]
enum HttpError {
    #[error("URL 解析失败: {0}")]
    InvalidUrl(String),
    #[error("网络错误: {0}")]
    Network(String),
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
    method: Method,
    url: String,
    sending: bool,
    rx: Option<Receiver<SendResult>>,
    response: Option<HttpResponse>,
    error: Option<String>,
}

impl Default for HttpboxApp {
    fn default() -> Self {
        Self {
            method: Method::Get,
            url: String::new(),
            sending: false,
            rx: None,
            response: None,
            error: None,
        }
    }
}

impl HttpboxApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn start_send(&mut self) {
        let url = self.url.trim().to_string();
        if url.is_empty() {
            self.error = Some("URL cannot be empty".to_string());
            return;
        }

        self.error = None;
        self.response = None;
        self.sending = true;

        let method = self.method;
        let (tx, rx) = channel();
        self.rx = Some(rx);

        tokio::spawn(async move {
            let start = Instant::now();
            let result: Result<HttpResponse, HttpError> = async {
                let parsed = reqwest::Url::parse(&url)
                    .map_err(|e| HttpError::InvalidUrl(e.to_string()))?;
                let client = reqwest::Client::builder()
                    .build()
                    .map_err(|e| HttpError::Network(e.to_string()))?;
                let resp = client
                    .request(method.as_reqwest(), parsed)
                    .send()
                    .await
                    .map_err(|e| HttpError::Network(e.to_string()))?;
                let status = resp.status();
                let body = resp
                    .text()
                    .await
                    .map_err(|e| HttpError::Network(e.to_string()))?;
                Ok(HttpResponse {
                    status: status.as_u16(),
                    status_text: status
                        .canonical_reason()
                        .unwrap_or("")
                        .to_string(),
                    body,
                    elapsed_ms: start.elapsed().as_millis(),
                })
            }
            .await;

            let _ = tx.send(match result {
                Ok(resp) => SendResult::Ok(resp),
                Err(e) => SendResult::Err(e),
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
                self.sending = false;
            }
        }
    }

    fn render_top_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            egui::ComboBox::from_label("")
                .selected_text(self.method.as_str())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.method, Method::Get, "GET");
                    ui.selectable_value(&mut self.method, Method::Post, "POST");
                });

            ui.add(
                egui::TextEdit::singleline(&mut self.url)
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

    fn render_status_line(ui: &mut egui::Ui, resp: &HttpResponse) {
        let color = status_color(resp.status);
        ui.horizontal(|ui| {
            ui.colored_label(color, format!("{} {}", resp.status, resp.status_text));
            ui.label(format!("· {} ms", resp.elapsed_ms));
        });
    }

    fn render_error(ui: &mut egui::Ui, err: &str) {
        ui.colored_label(egui::Color32::RED, err);
    }

    fn render_body(ui: &mut egui::Ui, body: &str) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut body.to_string())
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

impl eframe::App for HttpboxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_response();

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            self.render_top_bar(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(err) = &self.error {
                Self::render_error(ui, err);
            } else if let Some(resp) = &self.response {
                Self::render_status_line(ui, resp);
                ui.separator();
                Self::render_body(ui, &resp.body);
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
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

#[tokio::main]
async fn main() -> eframe::Result<()> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([900.0, 600.0])
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
