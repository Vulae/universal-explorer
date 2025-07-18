use util::{VirtualFileSystem, VirtualFileSystemPath};

use crate::{
    app::AppEvent,
    assets::{self, NOTEXTURE},
    egui_util::image_handle,
    loader::{entry_icon, try_open_tab_from_fs_and_path, LoaderImage},
};

use super::TabTrait;

const ENTRY_SIZE: egui::Vec2 = egui::Vec2::new(64.0, 96.0);
const ENTRY_THUMB_SIZE: egui::Vec2 = egui::Vec2::new(64.0, 64.0);
const ENTRIES_SPACING: egui::Vec2 = egui::Vec2::new(16.0, 16.0);

struct Entry {
    path: VirtualFileSystemPath,
    icon: Option<egui::ImageSource<'static>>,
    _icon_handle: Option<egui::TextureHandle>,
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry")
            .field("path", &self.path)
            // .field("icon", &self.icon)
            .finish()
    }
}

#[derive(Debug)]
pub struct VirtualFsTab {
    name: String,
    fs: VirtualFileSystem,
    directory: VirtualFileSystemPath,
    entries: Option<Box<[Entry]>>,
    events: Vec<AppEvent>,
}

impl VirtualFsTab {
    pub fn new(name: String, fs: VirtualFileSystem) -> Self {
        Self {
            name,
            fs,
            directory: "/".into(),
            entries: None,
            events: Vec::new(),
        }
    }

    pub fn set_directory<P: Into<VirtualFileSystemPath>>(&mut self, path: P) {
        self.directory = path.into();
        self.entries = None;
    }

    fn open_entry(&mut self, entry: &VirtualFileSystemPath) -> Result<(), anyhow::Error> {
        if let Some(tab) = try_open_tab_from_fs_and_path(&mut self.fs, entry)? {
            self.events.push(AppEvent::CreateTab(tab));
        }
        Ok(())
    }

    fn render(&mut self, ui: &mut egui::Ui) -> Result<(), anyhow::Error> {
        if self.entries.is_none() {
            let mut entries = self.fs.read_directory(&self.directory)?;
            entries.sort();
            entries.sort_by_key(|a| !a.is_directory());
            self.entries = Some(
                entries
                    .into_iter()
                    .map(|path| {
                        let image = match entry_icon(
                            &mut self.fs,
                            &path,
                            ENTRY_THUMB_SIZE.x as u32,
                            ENTRY_THUMB_SIZE.y as u32,
                        ) {
                            Ok(image) => image,
                            Err(err) => {
                                log::error!("Error while loading thumbnail: {err}");
                                return Entry {
                                    path,
                                    icon: None,
                                    _icon_handle: None,
                                };
                            }
                        };
                        match image {
                            None => Entry {
                                icon: Some(if path.is_directory() {
                                    assets::LUCIDE_FOLDER.to_owned()
                                } else {
                                    assets::LUCIDE_FILE.to_owned()
                                }),
                                _icon_handle: None,
                                path,
                            },
                            Some(LoaderImage::Image(dynamic_image)) => {
                                let handle = image_handle(dynamic_image, ui.ctx());
                                let source = egui::ImageSource::Texture(
                                    egui::load::SizedTexture::from_handle(&handle),
                                );
                                Entry {
                                    path,
                                    icon: Some(source),
                                    _icon_handle: Some(handle),
                                }
                            }
                            Some(LoaderImage::Source(image_source)) => Entry {
                                path,
                                icon: Some(image_source),
                                _icon_handle: None,
                            },
                        }
                    })
                    .collect(),
            );
        }

        let mut new_directory: Option<VirtualFileSystemPath> = None;
        let mut open_entry: Option<VirtualFileSystemPath> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            // FIXME: Doesn't center at all.
            ui.vertical_centered(|ui| {
                ui.set_min_size(ui.available_size());

                let num_cols = ((ui.available_width() / (ENTRY_SIZE.x + ENTRIES_SPACING.x * 2.0))
                    .floor() as usize)
                    .max(1);

                egui::Grid::new(&self.name)
                    .num_columns(num_cols)
                    .spacing(ENTRIES_SPACING)
                    .show(ui, |ui| {
                        self.entries
                            .as_ref()
                            .unwrap()
                            .into_iter()
                            .enumerate()
                            .for_each(|(i, entry)| {
                                ui.allocate_ui_with_layout(
                                    ENTRY_SIZE,
                                    egui::Layout::top_down(egui::Align::Center),
                                    |ui| {
                                        ui.push_id(entry.path.to_string(), |ui| {
                                            let base = ui.group(|ui| {
                                                ui.set_min_size(ENTRY_SIZE);
                                                egui_flex::Flex::vertical()
                                                    .size(ui.available_size())
                                                    .justify(egui_flex::FlexJustify::SpaceBetween)
                                                    .show(ui, |flex| {
                                                        let Some(name) = entry.path.name() else {
                                                            return;
                                                        };

                                                        flex.add_ui(egui_flex::item(), |ui| {
                                                            ui.label(name)
                                                        });

                                                        flex.add_ui(egui_flex::item(), |ui| {
                                                            if let Some(icon) = entry.icon.as_ref()
                                                            {
                                                                ui.add(
                                                                    egui::Image::new(
                                                                        icon.to_owned(),
                                                                    )
                                                                    .max_size(ENTRY_THUMB_SIZE),
                                                                );
                                                            } else {
                                                                ui.add(
                                                                    egui::Image::new(
                                                                        NOTEXTURE.to_owned(),
                                                                    )
                                                                    .max_size(ENTRY_THUMB_SIZE),
                                                                );
                                                            }
                                                        });
                                                    });
                                            });

                                            let interact = ui.interact(
                                                base.response.rect,
                                                entry.path.to_str().to_owned().into(),
                                                egui::Sense::all(),
                                            );
                                            if entry.path.is_directory() {
                                                if interact.clicked() {
                                                    new_directory = Some(entry.path.clone());
                                                }
                                            } else if entry.path.is_file() && interact.clicked() {
                                                open_entry = Some(entry.path.clone());
                                            }
                                        });
                                    },
                                );
                                if ((i + 1) % num_cols) == 0 {
                                    ui.end_row();
                                }
                            });
                    });
            })
        });

        if let Some(new_directory) = new_directory {
            self.set_directory(new_directory);
        }

        if let Some(open_file) = open_entry {
            if let Err(err) = self.open_entry(&open_file) {
                log::error!("{err}");
            }
        }

        Ok(())
    }
}

impl TabTrait for VirtualFsTab {
    fn name(&self) -> &str {
        &self.name
    }

    fn next_event(&mut self) -> Option<AppEvent> {
        self.events.pop()
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            let mut paths = Vec::new();
            let mut dir = Some(self.directory.clone());
            while let Some(some_dir) = dir {
                paths.push(some_dir.clone());
                dir = some_dir.parent();
            }

            ui.horizontal(|ui| {
                for path in paths.iter().rev() {
                    let name = path
                        .is_root()
                        .then_some("ROOT")
                        .or(path.name())
                        .unwrap_or("PATH SEGMENT ERROR");
                    if ui.button(name).clicked() {
                        self.set_directory(path);
                    }
                    ui.label("/");
                }
            });
        });

        if let Err(err) = self.render(ui) {
            ui.small(err.to_string());
        }
    }
}
