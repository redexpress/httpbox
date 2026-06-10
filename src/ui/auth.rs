use eframe::egui;

use crate::model::request::{AuthKind, HttpRequest};
use crate::ui::theme::TEXT_MUTED;

pub fn render_auth_section(ui: &mut egui::Ui, req: &mut HttpRequest) {
    let kind = req.auth.kind.clone();
    let bearer = req.auth.bearer_token.clone();
    let basic_user = req.auth.basic_user.clone();
    let basic_password = req.auth.basic_password.clone();

    ui.horizontal(|ui| {
        ui.label("Type:");
        egui::ComboBox::from_id_salt("auth_kind")
            .selected_text(match kind {
                AuthKind::None => "No Auth",
                AuthKind::Bearer => "Bearer Token",
                AuthKind::Basic => "Basic Auth",
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(matches!(kind, AuthKind::None), "No Auth")
                    .clicked()
                {
                    req.auth.kind = AuthKind::None;
                }
                if ui
                    .selectable_label(matches!(kind, AuthKind::Bearer), "Bearer Token")
                    .clicked()
                {
                    req.auth.kind = AuthKind::Bearer;
                }
                if ui
                    .selectable_label(matches!(kind, AuthKind::Basic), "Basic Auth")
                    .clicked()
                {
                    req.auth.kind = AuthKind::Basic;
                }
            });
    });

    match req.auth.kind {
        AuthKind::None => {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("This request does not use any authorization.")
                        .color(TEXT_MUTED)
                        .italics(),
                );
                ui.add_space(8.0);
            });
        }
        AuthKind::Bearer => {
            let mut token = bearer;
            let resp = ui.add(
                egui::TextEdit::singleline(&mut token)
                    .hint_text("Token")
                    .desired_width(f32::INFINITY)
                    .password(true)
                    .font(egui::TextStyle::Monospace),
            );
            if resp.changed() {
                req.auth.bearer_token = token;
            }
        }
        AuthKind::Basic => {
            let mut user = basic_user;
            let mut pass = basic_password;
            let resp_u = ui.add(
                egui::TextEdit::singleline(&mut user)
                    .hint_text("Username")
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace),
            );
            if resp_u.changed() {
                req.auth.basic_user = user;
            }
            let resp_p = ui.add(
                egui::TextEdit::singleline(&mut pass)
                    .hint_text("Password")
                    .desired_width(f32::INFINITY)
                    .password(true)
                    .font(egui::TextStyle::Monospace),
            );
            if resp_p.changed() {
                req.auth.basic_password = pass;
            }
        }
    }
}
