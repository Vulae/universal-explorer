
use std::{fs::File, io::{Read, Seek}, path::PathBuf};
use crate::{app::{Explorer, SharedAppContext}, util::{source_engine::vpk::{VpkArchive, VpkArchiveFiles}, TryClone}};
use anyhow::Result;
use uuid::Uuid;



pub struct VpkExplorer<F: Read + Seek + TryClone> {
    app_context: SharedAppContext,
    name: Option<String>,
    vpk: VpkArchive<F>,
    uuid: Uuid,
}

impl<F: Read + Seek + TryClone> VpkExplorer<F> {
    pub fn new(app_context: SharedAppContext, vpk: VpkArchive<F>, name: Option<String>) -> VpkExplorer<F> {
        VpkExplorer {
            app_context,
            name,
            vpk,
            uuid: Uuid::now_v7(),
        }
    }
}

impl VpkExplorer<File> {
    pub fn open<P: Into<PathBuf>>(app_context: SharedAppContext, path: P) -> Result<VpkExplorer<File>> {
        let path: PathBuf = path.into();
        let vpk_archive_files = VpkArchiveFiles::locate(&path)?;
        let vpk = VpkArchive::<File>::open(vpk_archive_files)?;
        Ok(VpkExplorer::new(
            app_context,
            vpk,
            path.file_name().map(|s| s.to_str().map(|s| s.to_owned())).flatten(),
        ))
    }
}

impl<F: Read + Seek + TryClone> Explorer for VpkExplorer<F> {
    fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn name(&mut self) -> String {
        self.name.clone().unwrap_or("VPK Archive".to_owned())
    }

    fn update(&mut self, ui: &mut egui::Ui) -> Result<()> {
        ui.vertical(|ui| {
            for file in self.vpk.files.iter_mut() {
                if ui.button(file.path()).clicked() {
                    file.rewind().expect("VTF failed to reset file stream position.");
                    self.app_context.open_file(file.try_clone().expect("Failed to clone VPK file."), Some(file.path())).expect("Failed to open VTF file");
                }
            }
        });
        Ok(())
    }
}


