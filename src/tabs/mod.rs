use std::sync::atomic::AtomicUsize;

use util::{VirtualFileSystem, VirtualFileSystemPath};

mod hex;
mod virtual_fs;
pub use hex::*;
pub use virtual_fs::*;

use crate::app::AppEvent;

pub trait TabTrait: std::fmt::Debug {
    fn name(&self) -> &str;
    fn next_event(&mut self) -> Option<AppEvent> {
        None
    }
    fn ui(&mut self, ui: &mut egui::Ui);
}

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
pub struct Tab {
    inner: Box<dyn TabTrait>,
    id: usize,
}

impl Tab {
    pub fn new(inner: Box<dyn TabTrait>) -> Self {
        Self {
            inner,
            id: ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn name(&self) -> &str {
        self.inner.name()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.inner.ui(ui)
    }

    pub fn events(&mut self) -> Vec<AppEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.inner.next_event() {
            events.push(event);
        }
        events
    }
}

pub fn try_open_tab_from_fs_and_path<P: Into<VirtualFileSystemPath>>(
    fs: &mut VirtualFileSystem,
    path: P,
) -> Result<Option<Tab>, anyhow::Error> {
    let path: VirtualFileSystemPath = path.into();

    if source_engine::is_vpk_file(&path) {
        let vpk = source_engine::VPKArchiveFiles::locate_archives(fs, &path)?.load()?;
        return Ok(Some(Tab::new(Box::new(VirtualFsTab::new(
            path.to_string(),
            VirtualFileSystem::new(Box::new(vpk)),
        )))));
    }

    if path.is_file() {
        return Ok(Some(Tab::new(Box::new(HexTab::new(
            path.to_string(),
            fs.open_file(&path)?,
        )))));
    }

    Ok(None)
}
