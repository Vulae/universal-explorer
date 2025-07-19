mod app;
mod assets;
mod egui_util;
mod loader;
mod tabs;

use app::App;

fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let app = App::default();

    eframe::run_native(
        "universal-explorer",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("universal-explorer")
                .with_min_inner_size([100.0, 100.0])
                .with_decorations(false),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .unwrap();

    Ok(())
}
