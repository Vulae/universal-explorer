
use std::{collections::HashMap, fs::File, io::{Read, Seek}, path::PathBuf};
use crate::{app::{Explorer, SharedAppContext}, util::{source_engine::vpk::{VpkArchive, VpkArchiveFiles, VpkFile}, virtual_fs::{FullPath, VirtualFs, VirtualFsEntry}}};
use anyhow::Result;
use uuid::Uuid;



pub struct VpkExplorer<F: Read + Seek> {
    app_context: SharedAppContext,
    name: Option<String>,
    uuid: Uuid,
    vpk: VirtualFs<VpkFile<F>, VpkArchive<F>>,
    expanded: HashMap<FullPath, bool>,
}

impl<F: Read + Seek> VpkExplorer<F> {
    pub fn new(app_context: SharedAppContext, vpk: VpkArchive<F>, name: Option<String>) -> Result<VpkExplorer<F>> {
        Ok(VpkExplorer {
            app_context,
            name,
            uuid: Uuid::now_v7(),
            vpk: VirtualFs::new(vpk),
            expanded: HashMap::new(),
        })
    }

    fn update_node(&mut self, ui: &mut egui::Ui, entry: &mut VirtualFsEntry<VpkFile<F>, VpkArchive<F>>) {
        match entry {
            VirtualFsEntry::File(_, path, file) => {
                if ui.add(
                    egui::Button::image_and_text(
                        egui::Image::new(crate::app::assets::LUCIDE_FILE)
                            .tint(ui.style().visuals.text_color()),
                        path.name().unwrap_or(""),
                    )
                ).clicked() {
                    file.rewind().expect("VTF failed to reset file stream position.");
                    self.app_context.open_file(file.clone(), Some(path.to_string())).expect("Failed to open VTF file");
                }
            },
            VirtualFsEntry::Directory(_, path, _) => {
                if ui.add(
                    egui::Button::image_and_text(
                        egui::Image::new(crate::app::assets::LUCIDE_FOLDER)
                            .tint(ui.style().visuals.text_color()),
                        path.name().unwrap_or(""),
                    )
                ).clicked() {
                    self.expanded.insert(path.clone(), !*self.expanded.get(path).unwrap_or(&false));
                }
                if *self.expanded.get(path).unwrap_or(&false) {
                    let entries = entry.children().unwrap();
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            for mut entry in entries {
                                self.update_node(ui, &mut entry);
                            }
                        });
                    });
                }
            },
        }
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
        self.uuid
    }

    fn name(&mut self) -> String {
        self.name.clone().unwrap_or("VPK Archive".to_owned())
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Result<()> {
        // TODO: Implement a Windows-like file explorer alongside this one.
        let mut entry = self.vpk.read("")?;
        self.update_node(ui, &mut entry);
        Ok(())
    }
}


