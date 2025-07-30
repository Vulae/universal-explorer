use tab::{EntriesContainer, ViewingType};
use util_vfs::{VirtualFileSystem, VirtualFileSystemPath};

use crate::{app::AppEvent, loader::try_open_tab_from_fs_and_path};

mod entry;
mod tab;

#[derive(Debug)]
pub struct VirtualFsTab {
    name: String,
    fs: VirtualFileSystem,
    directory: VirtualFileSystemPath,
    entries: EntriesContainer,
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
}
