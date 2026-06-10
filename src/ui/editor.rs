use eframe::egui;

use crate::app::HttpboxApp;
use crate::model::request::EditorTab;
use crate::ui::auth::render_auth_section;
use crate::ui::body_editor::render_body_editor;
use crate::ui::kv_table::render_kv_table;

pub fn render_section<F>(ui: &mut egui::Ui, title: &str, add_contents: F)
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

pub fn render_editor(app: &mut HttpboxApp, ui: &mut egui::Ui) {
    let tab = app.current().tab;

    render_editor_tabs(app, ui, tab);

    egui::ScrollArea::vertical()
        .id_salt("editor_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(4.0);

            match tab {
                EditorTab::Body => {
                    render_section(ui, "Headers", |ui| {
                        render_kv_table(ui, &mut app.current_mut().headers);
                    });
                    ui.add_space(6.0);

                    let mut body_dirty = app.body_dirty;
                    render_section(ui, "Body", |ui| {
                        render_body_editor(ui, app.current_mut(), &mut body_dirty);
                    });
                    app.body_dirty = body_dirty;
                }
                EditorTab::Param => {
                    render_section(ui, "Query Parameters", |ui| {
                        render_kv_table(ui, &mut app.current_mut().query);
                    });
                }
                EditorTab::Auth => {
                    render_auth_section(ui, app.current_mut());
                }
            }
        });
}

fn render_editor_tabs(app: &mut HttpboxApp, ui: &mut egui::Ui, current: EditorTab) {
    let count_headers = app
        .current()
        .headers
        .iter()
        .filter(|kv| !kv.key.is_empty())
        .count();
    let count_params = app
        .current()
        .query
        .iter()
        .filter(|kv| !kv.key.is_empty())
        .count();

    let tab_top = ui.cursor().min;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(18.0, 0.0);
        ui.add_space(4.0);

        if tab_button(
            ui,
            "Headers",
            current == EditorTab::Body,
            Some(count_headers),
        )
        .clicked()
        {
            app.current_mut().tab = EditorTab::Body;
        }
        if tab_button(ui, "Param", current == EditorTab::Param, Some(count_params)).clicked() {
            app.current_mut().tab = EditorTab::Param;
        }
        if tab_button(ui, "Auth", current == EditorTab::Auth, None).clicked() {
            app.current_mut().tab = EditorTab::Auth;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(4.0);
            let (icon, tooltip) = match app.response_layout {
                crate::model::request::ResponseLayout::Bottom => (
                    "\u{2194}",
                    "Response on the right (currently at the bottom)",
                ),
                crate::model::request::ResponseLayout::Right => (
                    "\u{2195}",
                    "Response at the bottom (currently on the right)",
                ),
            };
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(icon).size(14.0).strong())
                        .min_size(egui::vec2(28.0, 22.0))
                        .rounding(egui::Rounding::same(4.0)),
                )
                .on_hover_text(tooltip)
                .clicked()
            {
                app.response_layout = match app.response_layout {
                    crate::model::request::ResponseLayout::Bottom => {
                        crate::model::request::ResponseLayout::Right
                    }
                    crate::model::request::ResponseLayout::Right => {
                        crate::model::request::ResponseLayout::Bottom
                    }
                };
            }
        });
    });

    let tab_bottom = ui.cursor().min.y;
    let sep_rect = egui::Rect::from_min_max(
        egui::pos2(tab_top.x, tab_bottom - 1.0),
        egui::pos2(ui.max_rect().right(), tab_bottom),
    );
    ui.painter()
        .rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(220, 224, 232));

    ui.add_space(4.0);
}

fn tab_button(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    badge: Option<usize>,
) -> egui::Response {
    let selected_color = egui::Color32::from_rgb(35, 95, 200);
    let idle_color = egui::Color32::from_rgb(80, 85, 100);

    let text = if selected {
        egui::RichText::new(label)
            .color(selected_color)
            .size(13.0)
            .strong()
    } else {
        egui::RichText::new(label).color(idle_color).size(13.0)
    };

    let pill_id = ui.id().with(("tab_v2", label));
    let desired_h = 26.0;
    let start_min = ui.cursor().min;

    ui.allocate_ui(egui::vec2(0.0, desired_h), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.add(egui::Label::new(text.clone()).selectable(false));
            if let Some(n) = badge {
                ui.label(
                    egui::RichText::new(n.to_string())
                        .color(idle_color)
                        .size(11.0),
                );
            }
        });
    });

    let end_x = ui.cursor().min.x;
    let rect = egui::Rect::from_min_max(start_min, egui::pos2(end_x, start_min.y + desired_h));

    if selected {
        let underline = egui::Rect::from_min_max(
            egui::pos2(rect.left(), rect.bottom() - 2.0),
            egui::pos2(rect.right(), rect.bottom()),
        );
        ui.painter().rect_filled(underline, 0.0, selected_color);
    }

    ui.interact(rect, pill_id, egui::Sense::click())
}
