mod app;
mod colors;
mod config;
mod models;
mod vault;
mod pty;
mod ssh_parse;
mod theme;
mod ui;

use app::SshedApp;


fn main() -> eframe::Result<()> {
    // Embed the 256px icon into the binary
    let icon_bytes = include_bytes!("../assets/icons/shellkeeper_256.png");
    let icon_image = image::load_from_memory(icon_bytes)
        .expect("invalid icon")
        .to_rgba8();
    let w = icon_image.width();
    let h = icon_image.height();
    let icon = egui::IconData {
        rgba: icon_image.into_raw(),
        width: w,
        height: h,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("⚡ ShellKeeper — SSH Manager")
            .with_min_inner_size([800.0, 500.0])
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };

    eframe::run_native(
        "ShellKeeper",
        options,
        Box::new(|cc| Ok(Box::new(SshedApp::new(cc)))),
    )
}
