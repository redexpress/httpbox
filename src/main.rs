mod app;
mod http;
mod model;
mod ui;
mod util;

use tracing::info;

use crate::app::HttpboxApp;
use crate::ui::theme::init_logging;

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
