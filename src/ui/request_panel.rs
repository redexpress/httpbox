use eframe::egui;
use tracing::info;

use crate::app::HttpboxApp;
use crate::model::request::{HttpRequest, Method};
use crate::ui::theme::{method_color, ACCENT, APP_VERSION, TEXT_MUTED, TEXT_PRIMARY};

pub fn render_bordered_input(
    ui: &mut egui::Ui,
    value: &mut String,
    hint: &str,
    width: f32,
) -> egui::Response {
    let idle_stroke = egui::Stroke::new(1.0, TEXT_MUTED);
    let focus_stroke = egui::Stroke::new(1.0, ACCENT);
    let rounding = egui::Rounding::same(4.0);

    let mut frame = egui::Frame::none()
        .fill(egui::Color32::WHITE)
        .stroke(idle_stroke)
        .rounding(rounding)
        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
        .begin(ui);
    let edit_resp = {
        let fui = &mut frame.content_ui;
        fui.add(
            egui::TextEdit::singleline(value)
                .hint_text(hint)
                .desired_width(width)
                .frame(false)
                .font(egui::TextStyle::Monospace),
        )
    };
    let frame_resp = frame.end(ui);
    if edit_resp.has_focus() {
        ui.painter()
            .rect_stroke(frame_resp.rect, rounding, focus_stroke);
        ui.ctx().request_repaint();
    }
    edit_resp
}

pub fn sync_auto_name(req: &mut HttpRequest) {
    if req.name_auto {
        req.name = HttpRequest::name_from_url(&req.url);
    }
}

pub fn render_request_bar(app: &mut HttpboxApp, ui: &mut egui::Ui) {
    egui::menu::bar(ui, |ui| {
        let method_str = app.current().method.as_str();
        egui::ComboBox::from_id_salt("method")
            .selected_text(method_str)
            .width(80.0)
            .show_ui(ui, |ui| {
                let cur = app.current().method;
                for m in Method::all() {
                    if ui.selectable_label(cur == m, m.as_str()).clicked() {
                        app.current_mut().method = m;
                    }
                }
            });

        let remaining = ui.available_width() - 90.0;
        let edit_width = remaining.max(100.0);
        ui.allocate_ui(egui::vec2(edit_width, ui.available_height()), |ui| {
            let resp = render_bordered_input(
                ui,
                &mut app.current_mut().url,
                "https://example.com/api",
                f32::INFINITY,
            );
            if resp.changed() {
                sync_auto_name(app.current_mut());
            }
        });

        let button_text = if app.sending { "Sending..." } else { "Send" };
        let btn = egui::Button::new(button_text).min_size(egui::vec2(80.0, 0.0));
        if ui
            .add_enabled(!app.sending, btn)
            .on_hover_text("Ctrl+Enter")
            .clicked()
        {
            app.start_send();
        }
    });
}

pub fn render_menu_bar(app: &mut HttpboxApp, ui: &mut egui::Ui) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui
                .add(egui::Button::new("New Request").shortcut_text("Ctrl+N"))
                .clicked()
            {
                ui.close_menu();
                if app.request_is_empty() {
                    app.reset_request();
                } else {
                    app.confirm_new_open = true;
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
                app.about_open = true;
            }
        });
    });
}

pub fn render_top_bar(app: &mut HttpboxApp, ui: &mut egui::Ui) {
    render_menu_bar(app, ui);
}

pub fn render_request_list(app: &mut HttpboxApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.strong("Requests");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("+ New").clicked() {
                app.new_request();
            }
        });
    });

    let mut to_select: Option<usize> = None;
    let mut context_target: Option<usize> = None;
    let mut context_pos: Option<egui::Pos2> = None;

    for (i, req) in app.requests.iter().enumerate() {
        let selected = i == app.selected;
        let badge_color = method_color(req.method);
        let badge_text = req.method.as_str();
        let name = if req.name.is_empty() {
            "(no url)".to_string()
        } else {
            req.name.clone()
        };
        let has_url = !req.url.trim().is_empty();

        let row_h = if has_url { 46.0 } else { 28.0 };
        let row_w = ui.available_width();
        let (row_rect, row_resp) =
            ui.allocate_exact_size(egui::vec2(row_w, row_h), egui::Sense::click());
        if row_resp.clicked() {
            to_select = Some(i);
        }
        if row_resp.secondary_clicked() {
            context_target = Some(i);
            context_pos = Some(
                row_resp
                    .interact_pointer_pos()
                    .unwrap_or(row_rect.left_top()),
            );
        }

        if selected {
            let bg = ui.visuals().selection.bg_fill;
            ui.painter().rect_filled(row_rect, 4.0, bg);
        }

        ui.painter().rect_stroke(
            row_rect,
            4.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(225, 228, 235)),
        );

        let child = egui::UiBuilder::new()
            .max_rect(row_rect.shrink2(egui::vec2(6.0, 2.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center));
        ui.allocate_new_ui(child, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                egui::Frame::none()
                    .fill(badge_color)
                    .rounding(egui::Rounding::same(3.0))
                    .inner_margin(egui::Margin::symmetric(5.0, 1.0))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(badge_text)
                                .color(egui::Color32::WHITE)
                                .strong()
                                .size(10.0)
                                .font(egui::FontId::monospace(10.0)),
                        );
                    });

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.label(egui::RichText::new(name).strong().color(if selected {
                        egui::Color32::WHITE
                    } else {
                        TEXT_PRIMARY
                    }));
                    if has_url {
                        let url = req.url.trim();
                        let url_color = if selected {
                            egui::Color32::from_rgb(225, 230, 240)
                        } else {
                            TEXT_MUTED
                        };
                        let url_text = egui::RichText::new(url).color(url_color).size(10.5);
                        ui.add(egui::Label::new(url_text).truncate().selectable(false));
                    }
                });
            });
        });
    }

    if let Some(i) = to_select {
        app.select_request(i);
    }

    if let (Some(i), Some(pos)) = (context_target, context_pos) {
        render_request_context_menu(app, ui.ctx(), i, pos);
    }
}

fn render_request_context_menu(
    app: &mut HttpboxApp,
    ctx: &egui::Context,
    idx: usize,
    pos: egui::Pos2,
) {
    let mut to_rename = false;
    let mut to_delete = false;

    egui::Area::new(egui::Id::new("request_context_menu").with(idx))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(140.0);
                if ui.button("Rename").clicked() {
                    to_rename = true;
                }
                if ui.button("Delete").clicked() {
                    to_delete = true;
                }
            });
        });

    if to_rename {
        app.rename_buffer = app.requests[idx].name.clone();
        app.rename_target = Some(idx);
    }
    if to_delete {
        app.delete_request(idx);
    }
}

pub fn render_rename_window(app: &mut HttpboxApp, ctx: &egui::Context) {
    let Some(target) = app.rename_target else {
        return;
    };
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
            let resp =
                ui.add(egui::TextEdit::singleline(&mut app.rename_buffer).desired_width(240.0));
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
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
        let new_name = app.rename_buffer.trim();
        if !new_name.is_empty() {
            app.requests[target].name = new_name.to_string();
            app.requests[target].name_auto = false;
        }
        app.rename_target = None;
    } else if cancelled || !open {
        app.rename_target = None;
    }
}

pub fn render_about_window(app: &mut HttpboxApp, ctx: &egui::Context) {
    let mut open = app.about_open;
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
        app.about_open = false;
    }
}

pub fn render_confirm_new_window(app: &mut HttpboxApp, ctx: &egui::Context) {
    if !app.confirm_new_open {
        return;
    }
    let mut open = app.confirm_new_open;
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
                    app.reset_request();
                    app.confirm_new_open = false;
                }
                if ui.button("Cancel").clicked() {
                    app.confirm_new_open = false;
                }
            });
        });
    if !open {
        app.confirm_new_open = false;
    }
}
