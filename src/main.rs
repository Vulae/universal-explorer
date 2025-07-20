mod app;
mod assets;
mod egui_util;
mod loader;
mod tabs;

use app::{App, AppEvent};
use tabs::{Tab, VirtualFsTab};
use util::{OsFs, VirtualFileSystem};

fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let mut app = App::default();

    let fs = OsFs::new_root()?;
    let mut tab = VirtualFsTab::new("OsFs".to_owned(), VirtualFileSystem::new(Box::new(fs)));
    tab.set_directory("/home/vulae/");
    app.push_event(AppEvent::CreateTab(Tab::new(Box::new(tab))));

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
