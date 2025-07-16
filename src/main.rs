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

    let mut tab = VirtualFsTab::new(
        "OsFs".to_owned(),
        VirtualFileSystem::new(Box::new(OsFs::new_root()?)),
    );
    tab.set_directory("/home/vulae/.local/share/Steam/steamapps/common/Team Fortress 2/tf/");
    // tab.set_directory("/home/vulae/repos/universal-explorer/.testing/");

    app.event(AppEvent::CreateTab(Tab::new(Box::new(tab))))?;

    eframe::run_native(
        "universal-explorer",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("universal-explorer")
                .with_min_inner_size([100.0, 100.0]),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .unwrap();

    Ok(())
}
