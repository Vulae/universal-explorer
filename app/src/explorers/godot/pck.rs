
use std::{fs::File, io::{Read, Seek}, path::PathBuf};
use anyhow::Result;
use godot::pck::GodotPck;
use util::{file_utils::InnerFile, virtual_fs::VirtualFs};
use uuid::Uuid;
use crate::{app::{Explorer, SharedAppContext}, explorers::virtual_fs::{VirtualFsExplorer, VirtualFsExplorerOptions}};



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
            filename.map(|f| util::file_utils::filename(&f)).flatten()
        )?)
    }
}

impl GodotPckExplorer<File> {
    pub fn open<P: Into<PathBuf>>(app_context: SharedAppContext, path: P) -> Result<GodotPckExplorer<File>> {
        let path: PathBuf = path.into();
        GodotPckExplorer::file(app_context, File::open(&path)?, util::file_utils::filename(path))
    }
}

impl<F: Read + Seek + 'static> Explorer for GodotPckExplorer<F> {
    fn uuid(&self) -> &Uuid {
        self.explorer.uuid()
    }

    fn title(&self) -> String {
        self.explorer.title()
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        self.explorer.ui(ui);
    }
}


