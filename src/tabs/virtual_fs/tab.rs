use egui::Widget as _;
use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
use util::{LevenshteinDistance, VirtualFileSystemError, VirtualFileSystemPath};

use crate::{app::AppEvent, tabs::TabTrait};

use super::{entry::Entry, VirtualFsTab};

fn path_parents(path: &VirtualFileSystemPath) -> Vec<VirtualFileSystemPath> {
    let mut paths = Vec::new();
    let mut dir = Some(path.clone());
    while let Some(some_dir) = dir {
        paths.push(some_dir.clone());
        dir = some_dir.parent();
    }
    paths
}

fn search_score(search: &str, entry: &str) -> usize {
    // TODO: Make scoring system better.
    ((search.chars().count())..(entry.chars().count()))
        .map(|m| &entry[..m])
        .map(|slice| LevenshteinDistance::new(1, 1, 1).distance(search, slice))
        .min()
        .unwrap_or(LevenshteinDistance::new(1, 1, 1).distance(search, entry))
}

#[derive(Debug)]
enum TabEvent {
    SetDirectory(VirtualFileSystemPath),
    OpenEntry(VirtualFileSystemPath),
}

#[derive(Debug)]
pub enum EntriesContainer {
    NeedLoading,
    Error(VirtualFileSystemError),
    Entries(Vec<Entry>),
}

#[derive(Debug)]
pub enum ViewingType {
    Grid { size: f32 },
    List { height: f32, cols: Option<usize> },
}

impl ViewingType {
    fn is_grid(&self) -> bool {
        matches!(self, Self::Grid { .. })
    }

    fn is_list(&self) -> bool {
        matches!(self, Self::List { .. })
    }

    pub fn grid_default() -> Self {
        Self::Grid { size: 96.0 }
    }

    pub fn list_default() -> Self {
        Self::List {
            height: 24.0,
            cols: None,
        }
    }
}

impl VirtualFsTab {
    fn execute_events(&mut self, events: Vec<TabEvent>) {
        events.into_iter().for_each(|event| match event {
            TabEvent::SetDirectory(directory) => self.set_directory(directory),
            TabEvent::OpenEntry(entry) => {
                if let Err(err) = self.open_entry(&entry) {
                    log::error!("Error while opening entry \"{entry}\": {err}");
                }
            }
        });
    }

    fn update_entries_list(&mut self, ctx: &egui::Context) {
        if !matches!(self.entries, EntriesContainer::NeedLoading) {
            return;
        }
        match self.fs.read_directory(&self.directory) {
            Ok(mut entries) => {
                entries.sort();
                entries.sort_by_key(|a| !a.is_directory());
                // TODO: Non-blocking multi-threaded loading.
                self.entries = EntriesContainer::Entries(
                    entries
                        .into_par_iter()
                        .map(|path| Entry::load(&self.fs, path, ctx))
                        .collect(),
                );
            }
            Err(err) => {
                self.entries = EntriesContainer::Error(err);
            }
        }
    }

    fn ui_entry(&self, entry: &Entry, ui: &mut egui::Ui, events: &mut Vec<TabEvent>) {
        let base = match self.viewing_type {
            ViewingType::Grid { .. } => ui.group(|ui| {
                ui.set_min_size(ui.available_size());
                ui.vertical_centered(|ui| {
                    egui::Image::new(entry.icon().image_source().to_owned())
                        .max_size(ui.available_size())
                        .ui(ui);
                    if let Some(name) = entry.path().name() {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                        ui.label(name);
                    }
                });
            }),
            ViewingType::List { .. } => ui.horizontal_centered(|ui| {
                ui.set_min_width(ui.available_width());
                egui::Image::new(entry.icon().image_source().to_owned())
                    .max_size(ui.available_size())
                    .ui(ui);
                if let Some(name) = entry.path().name() {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                    ui.label(name);
                }
            }),
        };

        let interact = ui.interact(
            base.response.rect,
            entry.path().to_str().to_owned().into(),
            egui::Sense::click(),
        );
        if interact.clicked() {
            if entry.path().is_directory() {
                events.push(TabEvent::SetDirectory(entry.path().clone()));
            } else if entry.path().is_file() {
                events.push(TabEvent::OpenEntry(entry.path().clone()));
            }
        }
    }

    fn ui_entries(&self, ui: &mut egui::Ui, events: &mut Vec<TabEvent>) {
        match &self.entries {
            EntriesContainer::NeedLoading => {
                unreachable!();
            }
            EntriesContainer::Error(err) => {
                ui.label(format!("ERROR: {err:?}"));
            }
            EntriesContainer::Entries(entries) => match self.viewing_type {
                ViewingType::Grid { size: grid_size } => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.set_min_size(ui.available_size());

                        let size = egui::Vec2::new(grid_size, grid_size * (4.0 / 3.0));
                        let spacing = egui::Vec2::splat(grid_size / 16.0);

                        let num_cols = ((ui.available_width() / (size.x + spacing.x * 2.0)).floor()
                            as usize)
                            .max(1);

                        egui::Grid::new(&self.name)
                            .num_columns(num_cols)
                            .spacing(spacing)
                            .min_col_width(size.x)
                            .max_col_width(size.x)
                            .min_row_height(size.y)
                            .show(ui, |ui| {
                                entries.iter().enumerate().for_each(|(i, entry)| {
                                    ui.push_id(entry.path().to_str(), |ui| {
                                        ui.set_max_size(size);
                                        self.ui_entry(entry, ui, events);
                                    });

                                    if ((i + 1) % num_cols) == 0 {
                                        ui.end_row();
                                    }
                                });
                            });
                    });
                }
                ViewingType::List {
                    height: list_height,
                    cols: list_cols,
                } => {
                    let num_cols = list_cols
                        .unwrap_or((ui.available_width() / 256.0).floor() as usize)
                        .max(1);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new(&self.name)
                            .num_columns(num_cols)
                            .striped(true)
                            .min_row_height(list_height)
                            .max_col_width(ui.available_width() / (num_cols as f32))
                            .show(ui, |ui| {
                                entries.iter().enumerate().for_each(|(i, entry)| {
                                    ui.push_id(entry.path().to_str(), |ui| {
                                        self.ui_entry(entry, ui, events);
                                    });

                                    if ((i + 1) % num_cols) == 0 {
                                        ui.end_row();
                                    }
                                });
                            });
                    });
                }
            },
        }
    }

    fn ui_path(&mut self, ui: &mut egui::Ui, events: &mut Vec<TabEvent>) {
        let search_id = ui.auto_id_with("search");

        let segments = path_parents(&self.directory)
            .into_iter()
            .map(|path| {
                Some((
                    path.is_root().then_some("ROOT").or(path.name())?.to_owned(),
                    path,
                ))
            })
            .collect::<Option<Vec<_>>>();

        let Some(segments) = segments else {
            ui.label("ERROR GETTING PATH SEGMENTS TO SHOW, WTFFFFF");
            return;
        };

        ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
            segments.iter().rev().for_each(|(name, path)| {
                if ui.button(format!("{name}/")).clicked() && path != &self.directory {
                    events.push(TabEvent::SetDirectory(path.clone()));
                }
            });

            if let EntriesContainer::Entries(entries) = &mut self.entries {
                let mut dirs = entries
                    .iter()
                    .filter_map(|dir| {
                        (dir.path().is_directory() && dir.path().name().is_some())
                            .then_some(dir.path())
                    })
                    .map(|dir| (dir, search_score(&self.path_search, dir.name().unwrap())))
                    .filter(|(_, score)| *score <= 7)
                    .collect::<Vec<_>>();
                dirs.sort_by_key(|(dir, _)| dir.to_str());
                dirs.sort_by_key(|(_, score)| *score);

                let was_empty = self.path_search.is_empty();

                if self.path_search_focused {
                    ui.ctx().memory_mut(|m| m.request_focus(search_id));
                }

                let search_response = egui::TextEdit::singleline(&mut self.path_search)
                    .id(search_id)
                    .hint_text("Path Search")
                    .desired_width(128.0)
                    .ui(ui);

                if search_response.has_focus() {
                    self.path_search_focused = true;
                }

                if search_response.lost_focus() {
                    if search_response
                        .ctx
                        .input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        if let Some(dir) = dirs.first() {
                            events.push(TabEvent::SetDirectory(dir.0.clone()));
                        } else {
                            self.path_search.clear();
                        }
                    } else {
                        self.path_search_focused = false;
                    }
                }

                if search_response.has_focus()
                    && was_empty
                    && search_response
                        .ctx
                        .input(|i| i.key_pressed(egui::Key::Backspace))
                {
                    if let Some(parent) = self.directory.parent() {
                        events.push(TabEvent::SetDirectory(parent));
                    }
                }

                // FIXME: Popup buttons aren't clickable.
                // It becomes not open before button is able to be clicked.
                if let Some(egui::InnerResponse {
                    inner: Some(selected),
                    ..
                }) = egui::Popup::menu(&search_response)
                    .open(self.path_search_focused)
                    .anchor(egui::PopupAnchor::ParentRect(search_response.rect))
                    .align(egui::RectAlign::RIGHT_START)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show(|ui| {
                        ui.set_min_width(128.0);
                        if dirs.is_empty() {
                            ui.colored_label(
                                ui.style().visuals.weak_text_color(),
                                "No Directories",
                            );
                        }

                        let mut selected = None;
                        dirs.into_iter().for_each(|(dir, _)| {
                            if ui.button(dir.name().unwrap()).clicked() {
                                selected = Some(dir);
                            }
                        });
                        selected
                    })
                {
                    events.push(TabEvent::SetDirectory(selected.clone()));
                }
            }
        });
    }

    fn ui_options_view(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("View", |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    if ui.radio(self.viewing_type.is_grid(), "Grid").clicked() {
                        self.viewing_type = ViewingType::grid_default();
                    }
                    if ui.radio(self.viewing_type.is_list(), "List").clicked() {
                        self.viewing_type = ViewingType::list_default();
                    }
                });
            });
            ui.separator();
            ui.vertical(|ui| match &mut self.viewing_type {
                ViewingType::Grid { size } => {
                    ui.horizontal(|ui| {
                        ui.label("Size: ");
                        egui::DragValue::new(size).range((64.0)..=192.0).ui(ui);
                    });
                }
                ViewingType::List { height, cols } => {
                    ui.horizontal(|ui| {
                        ui.label("Size: ");
                        egui::DragValue::new(height).range((16.0)..=96.0).ui(ui);
                    });
                    ui.horizontal(|ui| {
                        if let Some(some_cols) = cols {
                            if ui.button("Cols: ").clicked() {
                                *cols = None;
                            } else {
                                egui::DragValue::new(some_cols).range(1..=8).ui(ui);
                            }
                        } else if ui.button("Cols: Auto").clicked() {
                            *cols = Some(1);
                        }
                    });
                }
            });
        });
    }

    fn ui_top(&mut self, ui: &mut egui::Ui, events: &mut Vec<TabEvent>) {
        ui.group(|ui| {
            egui_flex::Flex::new()
                .width(ui.available_width())
                .justify(egui_flex::FlexJustify::SpaceBetween)
                .show(ui, |flex| {
                    flex.add_ui(egui_flex::item(), |ui| self.ui_path(ui, events));
                    flex.add_ui(egui_flex::item(), |ui| self.ui_options_view(ui));
                });
        });
    }
}

impl TabTrait for VirtualFsTab {
    fn name(&self) -> egui::WidgetText {
        self.name.as_str().into()
    }

    fn next_event(&mut self) -> Option<AppEvent> {
        self.events.pop()
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut events = Vec::new();

        ui.vertical_centered_justified(|ui| {
            self.ui_top(ui, &mut events);
            self.update_entries_list(ui.ctx());
            self.ui_entries(ui, &mut events);
        });

        self.execute_events(events);
    }
}
