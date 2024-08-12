
use std::{fs::File, io::{Read, Seek}, path::PathBuf};
use crate::{app::{explorers::virtual_fs::{VirtualFsExplorer, VirtualFsExplorerOptions}, Explorer, SharedAppContext}, explorers::godot::pck::GodotPck, util::{file::InnerFile, virtual_fs::VirtualFs}};
use anyhow::Result;
use uuid::Uuid;



pub struct GodotPckExplorer<F: Read + Seek> {
    explorer: VirtualFsExplorer<InnerFile<F>, GodotPck<F>>,
}

impl<F: Read + Seek + 'static> GodotPckExplorer<F> {
    pub fn new(app_context: SharedAppContext, pck: GodotPck<F>, name: Option<String>) -> Result<Self> {
        Ok(GodotPckExplorer {
            explorer: VirtualFsExplorer::new(
                app_context,
                VirtualFs::new(pck),
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
        Ok(GodotPckExplorer::new(
            app_context,
            GodotPck::load(file)?,
            filename.map(|f| crate::util::file::filename(&f)).flatten()
        )?)
    }
}

impl GodotPckExplorer<File> {
    pub fn open<P: Into<PathBuf>>(app_context: SharedAppContext, path: P) -> Result<GodotPckExplorer<File>> {
        let path: PathBuf = path.into();
        GodotPckExplorer::file(app_context, File::open(&path)?, crate::util::file::filename(path))
    }
}

impl<F: Read + Seek + 'static> Explorer for GodotPckExplorer<F> {
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


