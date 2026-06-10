use eframe::egui;

use crate::app::HttpboxApp;
use crate::model::response::HttpResponse;
use crate::ui::theme::{status_color, DANGER};

pub fn render_status_line(ui: &mut egui::Ui, resp: &HttpResponse) {
    let color = status_color(resp.status);
    ui.horizontal(|ui| {
        ui.colored_label(color, format!("{} {}", resp.status, resp.status_text));
        ui.label(format!("· {} ms", resp.elapsed_ms));
    });
}

pub fn render_error(ui: &mut egui::Ui, err: &str) {
    ui.colored_label(DANGER, err);
}

pub fn render_body_response(ui: &mut egui::Ui, body: &str) {
    egui::ScrollArea::vertical()
        .id_salt("response_scroll")
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

pub fn render_response_panel(app: &HttpboxApp, ui: &mut egui::Ui) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(6.0))
        .show(ui, |ui| {
            if let Some(err) = &app.error {
                render_error(ui, err);
            } else if let Some(resp) = &app.response {
                render_status_line(ui, resp);
                ui.separator();
                render_body_response(ui, &resp.body);
            } else {
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Click \"Send\" to view the response")
                            .color(egui::Color32::GRAY),
                    );
                });
            }
        });
}
