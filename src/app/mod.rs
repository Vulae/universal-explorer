
pub mod app;
pub mod assets;
pub mod loader;

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
                .with_title("Universal Explorer")
                .with_icon({
                    let bytes = match crate::app::assets::UNIVERSAL_EXPLORER_ICON {
                        egui::ImageSource::Bytes { uri: _, bytes } => bytes,
                        _ => unreachable!("crate::app::assets::UNIVERSAL_EXPLORER_ICON should always be egui::ImageSource::Bytes"),
                    };
                    let image = image::load_from_memory(bytes.as_ref())?;
                    egui::IconData {
                        width: image.width(),
                        height: image.height(),
                        rgba: image.into_rgba8().as_raw().to_vec(),
                    }
                })
                // .with_active(true)
                .with_min_inner_size([ 480.0, 320.0 ])
                .with_transparent(true) // TODO: Window transparency with blur.
                // Decorations force resizable to false on Windows
                // https://github.com/emilk/egui/issues/4345
                // https://github.com/rust-windowing/winit/issues/3730
                .with_decorations(false)
                .with_resizable(true)
                .with_drag_and_drop(true)
                .with_maximized(true),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(app_context))),
    ).unwrap();

    Ok(())
}


