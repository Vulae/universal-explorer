use util::{VirtualFileSystem, VirtualFileSystemError, VirtualFileSystemPath};

use crate::app::AppEvent;

use super::{try_open_tab_from_fs_and_path, TabTrait};

const ENTRY_SIZE: egui::Vec2 = egui::Vec2::new(64.0, 86.0);
const ENTRIES_SPACING: egui::Vec2 = egui::Vec2::new(16.0, 16.0);

#[derive(Debug)]
pub struct VirtualFsTab {
    name: String,
    fs: VirtualFileSystem,
    directory: VirtualFileSystemPath,
    view_directory: Option<VirtualFileSystemPath>,
    cached_entries: Box<[VirtualFileSystemPath]>,
    events: Vec<AppEvent>,
}

impl VirtualFsTab {
    pub fn new(name: String, fs: VirtualFileSystem) -> Self {
        Self {
            name,
            fs,
            directory: "/".into(),
            view_directory: None,
            cached_entries: Vec::new().into_boxed_slice(),
            events: Vec::new(),
        }
    }

    pub fn set_directory<P: Into<VirtualFileSystemPath>>(&mut self, path: P) {
        self.directory = path.into();
    }

    fn render_entry(
        &mut self,
        ui: &mut egui::Ui,
        entry: VirtualFileSystemPath,
    ) -> Result<(), VirtualFileSystemError> {
        let frame = egui::Frame::group(ui.style()).show(ui, |ui| {
            let Some(name) = entry.name() else {
                return;
            };
            ui.label(name);
        });
        let interact = ui.interact(
            frame.response.rect,
            entry.to_str().to_owned().into(),
            egui::Sense::all(),
        );
        if entry.is_directory() {
            if interact.clicked() {
                self.directory = entry.clone();
            }
        } else if entry.is_file() && interact.clicked() {
            if let Some(tab) = try_open_tab_from_fs_and_path(&mut self.fs, &entry)? {
                self.events.push(AppEvent::CreateTab(tab));
            }
        }
        Ok(())
    }

    fn render(&mut self, ui: &mut egui::Ui) -> Result<(), VirtualFileSystemError> {
        let mut update_cache = false;

        if let Some(view_directory) = self.view_directory.as_ref() {
            if view_directory != &self.directory {
                update_cache = true;
            }
        } else {
            update_cache = true;
        }
        self.view_directory = Some(self.directory.clone());

        if update_cache {
            self.cached_entries = self.fs.read_directory(&self.directory)?;
            self.cached_entries.sort();
            self.cached_entries.sort_by_key(|a| !a.is_directory());
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                let num_cols = ((ui.available_width() / (ENTRY_SIZE.x + ENTRIES_SPACING.x)).floor()
                    as usize)
                    .max(1);
                egui::Grid::new(&self.name)
                    .num_columns(num_cols)
                    .spacing(ENTRIES_SPACING)
                    .show(ui, |ui| {
                        self.cached_entries
                            .clone()
                            .into_iter()
                            .enumerate()
                            .for_each(|(i, entry)| {
                                ui.allocate_ui_with_layout(
                                    ENTRY_SIZE,
                                    egui::Layout::top_down(egui::Align::Center),
                                    |ui| {
                                        ui.push_id(entry.to_string(), |ui| {
                                            // TODO: Error handling
                                            self.render_entry(ui, entry)
                                                .expect("Failed to render filesystem entry");
                                        });
                                    },
                                );
                                if ((i + 1) % num_cols) == 0 {
                                    ui.end_row();
                                }
                            });
                    });
            });
        });

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
        ui.label(&self.name);

        ui.label(self.directory.to_str());

        if let Some(parent) = self.directory.parent() {
            if ui.button("^").clicked() {
                self.directory = parent;
            }
        }

        if let Err(err) = self.render(ui) {
            ui.small(err.to_string());
        }
    }
}
