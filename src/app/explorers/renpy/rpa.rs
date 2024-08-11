
use std::{fs::File, io::{Read, Seek}, path::PathBuf};
use crate::{app::{explorers::virtual_fs::{VirtualFsExplorer, VirtualFsExplorerOptions}, Explorer, SharedAppContext}, explorers::renpy::rpa::RenPyArchive, util::{file::InnerFile, virtual_fs::VirtualFs}};
use anyhow::Result;
use uuid::Uuid;



pub struct RenPyArchiveExplorer<F: Read + Seek> {
    explorer: VirtualFsExplorer<InnerFile<F>, RenPyArchive<F>>,
}

impl<F: Read + Seek + 'static> RenPyArchiveExplorer<F> {
    pub fn new(app_context: SharedAppContext, rpa: RenPyArchive<F>, name: Option<String>) -> Result<Self> {
        Ok(RenPyArchiveExplorer {
            explorer: VirtualFsExplorer::new(
                app_context,
                VirtualFs::new(rpa),
                VirtualFsExplorerOptions {
                    name,
                    allow_download: true,
                    ..Default::default()
                },
            )?,
        })
    }

    pub fn file(app_context: SharedAppContext, mut file: F, filename: Option<String>) -> Result<Self> {
        file.rewind()?;
        Ok(RenPyArchiveExplorer::new(
            app_context,
            RenPyArchive::load(file)?,
            filename.map(|f| crate::util::file::filename(&f)).flatten()
        )?)
    }
}

impl RenPyArchiveExplorer<File> {
    pub fn open<P: Into<PathBuf>>(app_context: SharedAppContext, path: P) -> Result<RenPyArchiveExplorer<File>> {
        let path: PathBuf = path.into();
        let rpa = RenPyArchive::load(File::open(&path)?)?;
        RenPyArchiveExplorer::new(
            app_context,
            rpa,
            crate::util::file::filename(path),
        )
    }
}

impl<F: Read + Seek + 'static> Explorer for RenPyArchiveExplorer<F> {
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


