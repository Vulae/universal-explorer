
use std::{collections::HashMap, io::{Read, Seek}, sync::{Arc, Mutex}};
use crate::{app::{Explorer, SharedAppContext}, util::virtual_fs::{FullPath, VirtualFs, VirtualFsDirectory, VirtualFsEntry, VirtualFsInner}};
use anyhow::Result;
use uuid::Uuid;



#[derive(Debug, Clone, Default)]
pub struct VirtualFsExplorerOptions {
    pub name: Option<String>,
    pub allow_download: bool,
}



pub struct VirtualFsExplorer<F: Read + Seek, I: VirtualFsInner<F>> {
    app_context: SharedAppContext,
    options: VirtualFsExplorerOptions,
    uuid: Uuid,

    fs: VirtualFs<F, I>,
    view_directory: VirtualFsDirectory<F, I>,
    new_view_directory: Option<VirtualFsDirectory<F, I>>,

    search: String,

    cached_icons: HashMap<FullPath, Option<egui::ImageSource<'static>>>,
    cached_icon_handles: Vec<egui::TextureHandle>,
    load_icons: Arc<Mutex<Vec<(FullPath, crate::app::loader::LoadedThumbnail)>>>,
}

impl<F: Read + Seek + 'static, I: VirtualFsInner<F> + 'static> VirtualFsExplorer<F, I> {
    pub fn new(app_context: SharedAppContext, mut fs: VirtualFs<F, I>, options: VirtualFsExplorerOptions) -> Result<Self> {
        let view_directory = fs.root()?;
        Ok(Self {
            app_context,
            options,
            uuid: Uuid::now_v7(),
            fs,
            view_directory,
            new_view_directory: None,
            search: String::new(),
            cached_icons: HashMap::new(),
            cached_icon_handles: Vec::new(),
            load_icons: Arc::new(Mutex::new(Vec::new())),
        })
    }

    fn update_loaded_icons(&mut self, ctx: &egui::Context) {
        let mut load_icons = self.load_icons.lock().unwrap();

        while let Some((path, icon)) = load_icons.pop() {
            match icon {
                crate::app::loader::LoadedThumbnail::None => {
                    self.cached_icons.insert(path, None);
                },
                crate::app::loader::LoadedThumbnail::Image(image) => {
                    let handle = crate::util::image::image_egui_handle(&image, ctx);
                    let source = egui::ImageSource::Texture(egui::load::SizedTexture::from_handle(&handle));
                    self.cached_icon_handles.push(handle);
                    self.cached_icons.insert(path, Some(source));
                },
                crate::app::loader::LoadedThumbnail::ImageSource(source) => {
                    self.cached_icons.insert(path, Some(source));
                },
            }
        }
    }

    fn get_icon(&mut self, entry: &VirtualFsEntry<F, I>) -> Option<egui::ImageSource> {
        let hint = crate::util::image::SizeHint::Pixels((EntryDisplay::THUMBNAIL_SIZE.x * EntryDisplay::THUMBNAIL_SIZE.y * 1.5) as u64);
        let path = entry.path().clone();

        if let Some(cached_icon) = self.cached_icons.get(&path) {
            return cached_icon.clone().or(Some(crate::app::assets::LUCIDE_FILE));
        }

        self.cached_icons.insert(path.clone(), None);

        match entry {
            VirtualFsEntry::Directory(_) => {
                self.load_icons.lock().unwrap().push((path, crate::app::loader::LoadedThumbnail::ImageSource(crate::app::assets::LUCIDE_FOLDER)));
            },
            VirtualFsEntry::File(file) => {
                let name = file.path().name().map(|s| s.to_owned());
                let threaded_file = crate::util::file::ThreadedFile::new(Arc::new(Mutex::new(file.clone())));
                let load_icons = Arc::clone(&self.load_icons);
                std::thread::spawn(move || {
                    // TODO: Use multiple threads to load thumbnails.
                    match crate::app::loader::thumbnail_file(
                        threaded_file,
                        name,
                        hint,
                    ) {
                        Ok(thumbnail) => {
                            load_icons.lock().unwrap().push((path, thumbnail));
                        },
                        Err(err) => {
                            println!("Failed to load icon {}", err);
                        },
                    }
                });
            },
        }

        None
    }

    fn entry_display(&mut self, ui: &mut egui::Ui, entry: VirtualFsEntry<F, I>) {
        let path = entry.path().clone();
        let name = path.name().unwrap_or("Error");
        let icon = self.get_icon(&entry);

        let response = ui.add(EntryDisplay::new(name, icon.as_ref()));

        if response.clicked() {
            match &entry {
                VirtualFsEntry::File(file) => {
                    let name = file.path().name().map(|s| s.to_owned());
                    self.app_context.open_file(file.clone(), name).unwrap();
                },
                VirtualFsEntry::Directory(directory) => {
                    self.new_view_directory = Some(directory.clone());
                },
            }
        }

        response.context_menu(|ui| {
            if ui.button("Copy Path").clicked() {
                ui.output_mut(|o| o.copied_text = path.to_string());
            }
            if ui.button("Extract").clicked() {

                let dialog = rfd::FileDialog::new()
                    .set_title(format!("Extract {}", path))
                    .set_file_name(path.name().unwrap_or("archive"))
                    .set_can_create_directories(true);

                match entry {
                    VirtualFsEntry::File(mut file) => {
                        let save_name = path.name().unwrap_or("error");
                        if let Some(save_path) = dialog.set_file_name(save_name).save_file() {
                            println!("Extract file to {:?}", save_path);
                            file.save(save_path).expect("Failed to save file");
                        }
                    },
                    VirtualFsEntry::Directory(directory) => {
                        if let Some(save_path) = dialog.pick_folder() {
                            // save_path.push(path.name().unwrap_or("archive"));
                            println!("Extract directory to {:?}", save_path);
                            directory.save(save_path).expect("Failed to save directory");
                        }
                    }
                }

            }
        });

    }
}

impl<F: Read + Seek + 'static, I: VirtualFsInner<F> + 'static> Explorer for VirtualFsExplorer<F, I> {
    fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn name(&mut self) -> String {
        self.options.name.clone().unwrap_or("Virtual Filesystem".to_owned())
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Result<()> {
        self.update_loaded_icons(ui.ctx());

        let view_entries = self.view_directory.entries().collect::<Result<Vec<_>>>().map(|view_entries| {
            if self.search.len() > 0 {
                view_entries.into_iter().filter(|entry| {
                    if let Some(name) = entry.path().name() {
                        glob_match::glob_match(&format!("*{}*", self.search.to_lowercase()), &name.to_lowercase())
                    } else {
                        false
                    }
                }).collect()
            } else {
                view_entries
            }
        });

        egui::containers::Frame::none()
            .inner_margin(egui::Margin {
                left: 16.0, right: 16.0,
                top: 8.0,
                bottom: 32.0,
            })
            .show(ui, |ui| {
                // FIXME: Auto size so that the second element is max size while keeping the rest inside the rect.
                ui.horizontal(|ui| {

                    ui.style_mut().text_styles.get_mut(&egui::TextStyle::Button).unwrap().size = 16.0;
                    ui.style_mut().text_styles.get_mut(&egui::TextStyle::Body).unwrap().size = 16.0;

                    ui.group(|ui| {
                        let parent = self.view_directory.path().parent();
        
                        if parent.is_none() {
                            ui.disable();
                        }
        
                        if ui.button("Back").clicked() {
                            if let Some(parent) = parent {
                                if let Ok(VirtualFsEntry::Directory(directory)) = self.fs.read(parent) {
                                    self.new_view_directory = Some(directory);
                                } else {
                                    self.new_view_directory = Some(self.fs.root().unwrap()); // Fallback
                                }
                            }
                        }
                    });
        
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.group(|ui| {
                            let mut segments: Vec<(String, FullPath)> = Vec::new();
                            let mut path = self.view_directory.path().clone();
                            segments.push((path.name().unwrap_or("root").to_owned(), path.clone()));
                            while let Some(parent) = path.parent() {
                                path = parent;
                                segments.push((path.name().unwrap_or("root").to_owned(), path.clone()));
                            }
                
                            for (index, (name, path)) in segments.into_iter().rev().enumerate() {
                                if index > 0 {
                                    ui.label(">");
                                }
                
                                if ui.button(name).clicked() {
                                    if let Ok(VirtualFsEntry::Directory(directory)) = self.fs.read(path) {
                                        self.new_view_directory = Some(directory);
                                    } else {
                                        self.new_view_directory = Some(self.fs.root().unwrap()); // Fallback
                                    }
                                }
                            }
                        });
                    });

                    ui.group(|ui| {

                        ui.add({
                            let search_empty = self.search.is_empty();
                            egui::TextEdit::singleline(&mut self.search)
                                // FIXME: Why is this not grayed out?
                                .text_color_opt(if !search_empty { None } else { Some(ui.visuals().text_color().gamma_multiply(0.5)) })
                                .hint_text("Search")
                                .desired_width(128.0)
                        });

                        if let Ok(entries) = &view_entries {
                            let mut num_directories: u64 = 0;
                            let mut num_files: u64 = 0;
    
                            for entry in entries {
                                match entry {
                                    VirtualFsEntry::Directory(_) => num_directories += 1,
                                    VirtualFsEntry::File(_) => num_files += 1,
                                }
                            }
    
                            ui.label(format!("Directories: {} - Files: {}", num_directories, num_files));
                        }
                    });
                    
                });
        
                egui::ScrollArea::vertical().show(ui, |ui| {
            
                    if let Ok(entries) = view_entries {

                        // TODO: There has to be a better way to do this.
                        // I need a grid that will auto columns, while keeping every element the same size.
                        const GRID_SPACING: egui::Vec2 = egui::Vec2::new(16.0, 16.0);
                        let num_columns = (ui.available_width() / (EntryDisplay::SIZE.x + GRID_SPACING.x + 16.0)).floor().max(1.0) as usize;
                        egui::Grid::new(self.uuid)
                            .num_columns(num_columns)
                            .spacing(GRID_SPACING)
                            .show(ui, |ui| {
                                for (i, entry) in entries.into_iter().enumerate() {
                                    ui.allocate_ui_with_layout(
                                        EntryDisplay::SIZE + GRID_SPACING,
                                        egui::Layout::top_down(egui::Align::Center),
                                        |ui| {
                                            self.entry_display(ui, entry);
                                        }
                                    );
                                    if (i + 1) % num_columns == 0 {
                                        ui.end_row();
                                    }
                                }
                            });

                    } else {

                        ui.colored_label(ui.style().visuals.error_fg_color, "Failed to get entries");

                    }
                    
                });
                
            });

        if let Some(new_view_directory) = self.new_view_directory.take() {
            self.view_directory = new_view_directory;
            self.search.clear();
        }

        // FIXME: Don't render continuously!
        // Because the loading of thumbnails is multithreaded,
        // Some thumbnails may be fully loaded but not yet rendered until the user interacts.
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));

        Ok(())
    }
}



struct EntryDisplay<'a> {
    name: &'a str,
    icon: Option<&'a egui::ImageSource<'a>>,
}

impl<'a> EntryDisplay<'a> {
    const THUMBNAIL_SIZE: egui::Vec2 = egui::Vec2::new(64.0, 64.0);
    const SIZE: egui::Vec2 = egui::Vec2::new(Self::THUMBNAIL_SIZE.x * 1.25, Self::THUMBNAIL_SIZE.y * 1.5);

    pub fn new(name: &'a str, icon: Option<&'a egui::ImageSource>) -> Self {
        Self { name, icon }
    }
}

impl<'a> egui::Widget for EntryDisplay<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let response = ui.push_id(self.name, |ui| {
            ui.vertical_centered_justified(|ui| {
                if let Some(icon) = self.icon {
                    ui.add_sized(
                        Self::THUMBNAIL_SIZE,
                        egui::Image::new(icon.clone()) // FIXME: Don't clone!
                            .max_size(Self::THUMBNAIL_SIZE),
                    );
                } else {
                    ui.add_sized(
                        Self::THUMBNAIL_SIZE,
                        egui::Image::new(crate::app::assets::ERROR)
                            .texture_options(egui::TextureOptions::NEAREST)
                            .max_size(Self::THUMBNAIL_SIZE),
                    );
                }
                ui.add(
                    egui::Label::new(egui::RichText::new(self.name).size(12.0)).truncate()
                );
            });
        }).response;

        if response.hovered() {
            let painter = ui.painter();
            let stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
            painter.rect_stroke(response.rect, 0.0, stroke);
        }
        
        response.on_hover_cursor(egui::CursorIcon::PointingHand).interact(egui::Sense::click())
    }
}


