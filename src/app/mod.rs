
mod app;


pub use app::SharedAppContext;
pub use app::Explorer;
pub mod explorers;





use anyhow::Result;
use std::path::PathBuf;

pub fn run_app(open_files: &Vec<PathBuf>) -> Result<()> {

    let mut app_context = SharedAppContext::new();

    for file in open_files {
        app_context.open(file)?;
    }

    eframe::run_native(
        "universal-unpacker",
        eframe::NativeOptions {
            run_and_return: true,
            viewport: egui::ViewportBuilder::default()
                .with_title("universal-unpacker")
                .with_active(true)
                .with_min_inner_size([ 480.0, 320.0 ])
                // .with_transparent(true) // TODO: Window transparency with blur.
                .with_maximized(true),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(app_context))),
    ).unwrap();

    Ok(())
}


