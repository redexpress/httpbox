use eframe::egui;
use tracing_subscriber::EnvFilter;

use crate::model::request::Method;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(30, 30, 35);
pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(140, 145, 160);
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(80, 140, 240);
pub const DANGER: egui::Color32 = egui::Color32::from_rgb(220, 80, 80);

pub fn init_logging() {
    let filter = EnvFilter::try_from_env("HTTPBOX_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info,httpbox=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();
}

pub fn apply_theme(ctx: &egui::Context) {
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
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 140, 220));

    visuals.window_fill = egui::Color32::from_rgb(252, 252, 254);
    visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(160, 165, 180));
    visuals.window_rounding = egui::Rounding::same(6.0);

    ctx.set_visuals(visuals);
}

pub fn method_color(m: Method) -> egui::Color32 {
    match m {
        Method::Get => egui::Color32::from_rgb(80, 140, 200),
        Method::Post => egui::Color32::from_rgb(60, 165, 90),
        Method::Put => egui::Color32::from_rgb(220, 150, 50),
        Method::Delete => egui::Color32::from_rgb(200, 70, 70),
        Method::Patch => egui::Color32::from_rgb(150, 110, 200),
        Method::Head => egui::Color32::from_rgb(110, 110, 130),
        Method::Options => egui::Color32::from_rgb(110, 110, 130),
    }
}

pub fn status_color(code: u16) -> egui::Color32 {
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
