use eframe::egui;

use crate::model::request::{BodyKind, HttpRequest, KeyValue};
use crate::ui::theme::DANGER;

pub fn render_body_editor(ui: &mut egui::Ui, req: &mut HttpRequest, body_dirty: &mut bool) {
    ui.horizontal(|ui| {
        ui.label("Content-Type:");
        egui::ComboBox::from_id_salt("body_kind")
            .selected_text(match req.body_kind {
                BodyKind::None => "none",
                BodyKind::Json => "application/json",
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(matches!(req.body_kind, BodyKind::None), "none")
                    .clicked()
                {
                    req.body_kind = BodyKind::None;
                }
                if ui
                    .selectable_label(matches!(req.body_kind, BodyKind::Json), "application/json")
                    .clicked()
                {
                    req.body_kind = BodyKind::Json;
                    sync_content_type_header(&mut req.headers);
                }
            });

        if matches!(req.body_kind, BodyKind::Json) {
            if ui.button("Format").clicked() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&req.body_text) {
                    if let Ok(s) = serde_json::to_string_pretty(&v) {
                        req.body_text = s;
                    }
                }
            }
        }
    });

    ui.add_space(4.0);

    match req.body_kind {
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
            let mut text = req.body_text.clone();
            let resp = ui.add(
                egui::TextEdit::multiline(&mut text)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(8)
                    .hint_text("{\"key\": \"value\"}"),
            );
            if resp.changed() {
                req.body_text = text;
                *body_dirty = true;
            }

            if let Err(e) = serde_json::from_str::<serde_json::Value>(&req.body_text) {
                if !req.body_text.trim().is_empty() {
                    ui.colored_label(DANGER, format!("JSON error: {}", e));
                }
            }
        }
    }
}

pub fn sync_content_type_header(headers: &mut Vec<KeyValue>) {
    let already = headers
        .iter_mut()
        .find(|kv| kv.key.eq_ignore_ascii_case("Content-Type"));
    match already {
        Some(kv) => {
            kv.value = "application/json".to_string();
            kv.enabled = true;
        }
        None => {
            headers.push(KeyValue::new("Content-Type", "application/json"));
        }
    }
}
