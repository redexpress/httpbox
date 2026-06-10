use eframe::egui;

use crate::model::request::KeyValue;
use crate::ui::theme::{ACCENT, TEXT_MUTED};

pub fn render_kv_table(ui: &mut egui::Ui, kvs: &mut Vec<KeyValue>) {
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
                ui.painter()
                    .rect_stroke(key_frame_resp.rect, rounding, focus_stroke);
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
                ui.painter()
                    .rect_stroke(value_frame_resp.rect, rounding, focus_stroke);
                ui.ctx().request_repaint();
            }

            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("×")
                            .size(14.0)
                            .strong()
                            .color(egui::Color32::from_rgb(180, 50, 60)),
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
            egui::Button::new(egui::RichText::new("+ Add").size(13.0).strong())
                .min_size(egui::vec2(80.0, 24.0))
                .fill(egui::Color32::from_rgb(55, 90, 140))
                .rounding(egui::Rounding::same(4.0)),
        )
        .clicked()
    {
        kvs.push(KeyValue::new("", ""));
    }
}
