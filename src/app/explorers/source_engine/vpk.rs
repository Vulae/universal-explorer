
use std::{fs::File, io::{Read, Seek}, path::PathBuf};
use crate::{app::{explorers::virtual_fs::{VirtualFsExplorer, VirtualFsExplorerOptions}, Explorer, SharedAppContext}, explorers::source_engine::vpk::{VpkArchive, VpkArchiveFiles, VpkFile}, util::virtual_fs::VirtualFs};
use anyhow::Result;
use uuid::Uuid;



pub struct VpkExplorer<F: Read + Seek> {
    explorer: VirtualFsExplorer<VpkFile<F>, VpkArchive<F>>,
}

impl<F: Read + Seek> VpkExplorer<F> {
    pub fn new(app_context: SharedAppContext, vpk: VpkArchive<F>, name: Option<String>) -> Result<VpkExplorer<F>> {
        Ok(VpkExplorer {
            explorer: VirtualFsExplorer::new(
                app_context,
                VirtualFs::new(vpk),
                VirtualFsExplorerOptions {
                    name,
                    calculate_size: true,
                    allow_download: true,
                    ..Default::default()
                },
            ),
        })
    }
}

impl VpkExplorer<File> {
    pub fn open<P: Into<PathBuf>>(app_context: SharedAppContext, path: P) -> Result<VpkExplorer<File>> {
        let path: PathBuf = path.into();
        let (archive_name, vpk_archive_files) = VpkArchiveFiles::locate(&path)?;
        let vpk = VpkArchive::<File>::open(vpk_archive_files)?;
        VpkExplorer::new(
            app_context,
            vpk,
            crate::util::filename(&archive_name),
        )
    }
}

impl<F: Read + Seek> Explorer for VpkExplorer<F> {
    fn uuid(&self) -> Uuid {
        self.explorer.uuid()
    }

    fn name(&mut self) -> String {
        self.explorer.name()
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Result<()> {
        self.explorer.ui(ui)
    }
}


