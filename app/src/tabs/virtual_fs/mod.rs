use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use icon::EntryIcon;
use ui::ViewingType;
use util_vfs::{VirtualFileSystem, VirtualFileSystemError, VirtualFileSystemPath};

use crate::{app::AppEvent, app_util::save_stream, loader::try_open_tab_from_fs_and_path};

mod icon;
mod ui;

#[derive(Debug)]
enum TabEvent {
    SetDirectory(VirtualFileSystemPath),
    OpenEntry(VirtualFileSystemPath),
    SaveEntry(VirtualFileSystemPath),
}

#[derive(Debug)]
pub enum EntriesContainer {
    NeedLoading,
    Error(VirtualFileSystemError),
    Entries(Box<[VirtualFileSystemPath]>),
}

#[derive(Debug)]
pub enum EntryIconLoadState {
    Loading,
    Loaded(EntryIcon),
}

#[derive(Debug)]
pub struct VirtualFsTab {
    name: String,
    fs: VirtualFileSystem,
    directory: VirtualFileSystemPath,
    entries: EntriesContainer,
    icons: Arc<Mutex<HashMap<VirtualFileSystemPath, EntryIconLoadState>>>,
    events: Vec<AppEvent>,
    viewing_type: ViewingType,
    path_search: String,
    path_search_focused: bool,
}

impl VirtualFsTab {
    pub fn new(name: String, fs: VirtualFileSystem) -> Self {
        Self {
            name,
            fs,
            directory: "/".into(),
            entries: EntriesContainer::NeedLoading,
            icons: Arc::new(Mutex::new(HashMap::new())),
            events: Vec::new(),
            viewing_type: ViewingType::list_default(),
            path_search: String::new(),
            path_search_focused: false,
        }
    }

    pub fn set_directory<P: Into<VirtualFileSystemPath>>(&mut self, path: P) {
        self.directory = path.into();
        self.entries = EntriesContainer::NeedLoading;
        self.path_search.clear();
    }

    fn open_entry(&mut self, entry: &VirtualFileSystemPath) -> Result<(), anyhow::Error> {
        if let Some(tab) = try_open_tab_from_fs_and_path(&mut self.fs, entry)? {
            self.events.push(AppEvent::CreateTab(tab));
        }
        Ok(())
    }

    fn execute_event(&mut self, event: TabEvent) -> Result<(), anyhow::Error> {
        match event {
            TabEvent::SetDirectory(directory) => self.set_directory(directory),
            TabEvent::OpenEntry(entry) => {
                self.open_entry(&entry)?;
            }
            TabEvent::SaveEntry(entry) => {
                if entry.is_file() {
                    let file = self.fs.open_file(&entry)?;
                    save_stream(file, Some(entry.name().unwrap_or("unnamed")))?;
                } else if entry.is_directory() {
                    return Err(anyhow::anyhow!("Saving directories is not yet supported"));
                }
            }
        }
        Ok(())
    }

    fn execute_events(&mut self, events: Vec<TabEvent>) {
        for event in events.into_iter() {
            if let Err(err) = self.execute_event(event) {
                log::error!("VirtualFsTab error while executing event: {err}");
            }
        }
    }
}
