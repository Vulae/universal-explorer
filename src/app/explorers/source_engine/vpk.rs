
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
    estimated_size: Option<u64>,
}

impl<F: Read + Seek> VpkExplorer<F> {
    pub fn new(app_context: SharedAppContext, vpk: VpkArchive<F>, name: Option<String>) -> Result<VpkExplorer<F>> {
        Ok(VpkExplorer {
            app_context,
            name,
            uuid: Uuid::now_v7(),
            vpk: VirtualFs::new(vpk),
            expanded: HashMap::new(),
            estimated_size: None,
        })
    }

    fn update_node(&mut self, ui: &mut egui::Ui, entry: &mut VirtualFsEntry<VpkFile<F>, VpkArchive<F>>) -> Result<()> {

        match entry {
            VirtualFsEntry::File(file) => {
                if ui.add(
                    egui::Button::image_and_text(
                        egui::Image::new(crate::app::assets::LUCIDE_FILE)
                            .tint(ui.style().visuals.text_color()),
                        file.path().name().unwrap_or(""),
                    )
                ).clicked() {
                    file.rewind()?;
                    self.app_context.open_file(file.clone(), Some(entry.path().to_string()))?;
                }
            },
            VirtualFsEntry::Directory(directory) => {
                if ui.add(
                    egui::Button::image_and_text(
                        egui::Image::new(crate::app::assets::LUCIDE_FOLDER)
                            .tint(ui.style().visuals.text_color()),
                        directory.path().name().unwrap_or(""),
                    )
                ).clicked() {
                    self.expanded.insert(directory.path().clone(), !*self.expanded.get(directory.path()).unwrap_or(&false));
                }
                if *self.expanded.get(directory.path()).unwrap_or(&false) {
                    let entries_iter = directory.entries();
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            for entry in entries_iter {
                                match entry {
                                    Ok(mut entry) => { self.update_node(ui, &mut entry).unwrap(); },
                                    Err(err) => { ui.colored_label(egui::Color32::RED, err.to_string()); },
                                }
                            }
                        });
                    });
                }
            },
        }
        
        Ok(())
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

        crate::util::egui::splitter::Splitter::horizontal(self.uuid).min_size(240.0).show(ui, |ui_a, ui_b| {
            
            // TODO: Implement a Windows-like file explorer alongside this one.
            let mut root = self.vpk.root().unwrap().as_entry();
            self.update_node(ui_a, &mut root).unwrap();

            ui_b.vertical(|ui| {
                let estimated_size = self.estimated_size.get_or_insert_with(|| {
                    self.vpk.root().unwrap().size().unwrap()
                });
                ui.label(format!("File System Size: {}", estimated_size));

                if ui.button("Extract").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Extract File System")
                        .set_file_name(self.name.clone().unwrap_or("archive".to_owned()))
                        .pick_folder()
                    {
                        println!("Save to {:?}", path);
                        self.vpk.root().unwrap().save(path).unwrap();
                    }
                }
            });

        });

        Ok(())
    }
}


