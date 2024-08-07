
use std::{collections::HashMap, io::{Read, Seek}};
use crate::{app::{Explorer, SharedAppContext}, util::virtual_fs::{FullPath, VirtualFs, VirtualFsEntry, VirtualFsInner}};
use anyhow::Result;
use uuid::Uuid;



#[derive(Debug, Clone, Default)]
pub struct VirtualFsExplorerOptions {
    pub name: Option<String>,
    pub calculate_size: bool,
    pub allow_download: bool,
}



pub struct VirtualFsExplorer<F: Read + Seek, I: VirtualFsInner<F>> {
    app_context: SharedAppContext,
    options: VirtualFsExplorerOptions,
    uuid: Uuid,
    fs: VirtualFs<F, I>,
    expanded: HashMap<FullPath, bool>,
    estimated_size: Option<u64>,
}

impl<F: Read + Seek, I: VirtualFsInner<F>> VirtualFsExplorer<F, I> {
    pub fn new(app_context: SharedAppContext, fs: VirtualFs<F, I>, options: VirtualFsExplorerOptions) -> Self {
        Self {
            app_context,
            options,
            uuid: Uuid::now_v7(),
            fs,
            expanded: HashMap::new(),
            estimated_size: None,
        }
    }

    fn update_node(&mut self, ui: &mut egui::Ui, entry: &mut VirtualFsEntry<F, I>) -> Result<()> {

        match entry {
            VirtualFsEntry::File(file) => {
                if ui.add(
                    egui::Button::image_and_text(
                        egui::Image::new(crate::app::assets::LUCIDE_FILE)
                            .tint(ui.style().visuals.text_color()),
                        file.path().name().unwrap_or(""),
                    )
                ).clicked() {
                    let path = file.path().clone();
                    let file = file.fs_mut().read(path.clone()).unwrap().as_file().unwrap();
                    self.app_context.open_file(file, Some(entry.path().to_string()))?;
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

impl<F: Read + Seek, I: VirtualFsInner<F>> Explorer for VirtualFsExplorer<F, I> {
    fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn name(&mut self) -> String {
        self.options.name.clone().unwrap_or("Virtual Filesystem".to_owned())
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Result<()> {

        crate::util::egui::splitter::Splitter::horizontal(self.uuid).min_size(240.0).show(ui, |ui_a, ui_b| {
            
            // FIXME: Scrollbar isn't fully at right side.
            egui::ScrollArea::vertical().show(ui_a, |ui| {
                // TODO: Implement a Windows-like file explorer alongside this one.
                let mut root = self.fs.root().unwrap().as_entry();
                self.update_node(ui, &mut root).unwrap();
            });

            ui_b.vertical(|ui| {
                if self.options.calculate_size {
                    let estimated_size = self.estimated_size.get_or_insert_with(|| {
                        self.fs.root().unwrap().size().unwrap()
                    });
                    ui.label(format!("File System Size: {}", estimated_size));
                }

                if self.options.allow_download {
                    if ui.button("Extract").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Extract File System")
                            .set_file_name(self.options.name.clone().unwrap_or("archive".to_owned()))
                            .pick_folder()
                        {
                            println!("Save to {:?}", path);
                            self.fs.root().unwrap().save(path).unwrap();
                        }
                    }
                }
            });

        });

        Ok(())
    }
}


