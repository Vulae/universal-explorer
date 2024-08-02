
use std::{cell::RefCell, fs::File, io::{Read, Seek}, path::{Path, PathBuf}, rc::Rc};
use anyhow::{anyhow, Result};
use uuid::Uuid;

use super::explorers::{image::ImageExplorer, source_engine::{vpk::VpkExplorer, vtf::VtfExplorer}};



pub struct AppContext {
    pub explorers: Vec<SharedExplorer>,
    open_explorer: Uuid,
}

impl AppContext {
    fn new() -> AppContext {
        AppContext {
            explorers: Vec::new(),
            open_explorer: Uuid::nil(),
        }
    }
}





#[derive(Clone)]
pub struct SharedAppContext {
    app_context: Rc<RefCell<AppContext>>,
}

impl SharedAppContext {
    pub fn new() -> SharedAppContext {
        let app_context = AppContext::new();
        SharedAppContext {
            app_context: Rc::new(RefCell::new(app_context)),
        }
    }

    pub fn explorers(&self) -> Vec<SharedExplorer> {
        // This sucks, please do not collect!
        self.app_context.borrow_mut().explorers.iter().map(|v| v.clone()).collect::<Vec<_>>()
    }

    pub fn current_explorer(&self) -> Option<SharedExplorer> {
        let open_explorer_uuid = self.app_context.borrow().open_explorer;
        self.explorers()
            .iter()
            .find(|explorer| explorer.uuid() == open_explorer_uuid)
            .map(|explorer| explorer.clone())
    }

    pub fn select_explorer(&self, uuid: Uuid) {
        self.app_context.borrow_mut().open_explorer = uuid;
    }

    pub fn new_explorer(&mut self, explorer: impl Explorer + 'static) {
        self.app_context.borrow_mut().explorers.push(SharedExplorer(Rc::new(RefCell::new(explorer))));
    }

    pub fn remove_explorer(&mut self, uuid: Uuid) {
        // TODO: Clean up. What in the heck is this?
        self.app_context.borrow_mut().explorers = self.explorers().into_iter().filter(|e| e.uuid() != uuid).collect::<Vec<_>>();
    }



    pub fn open_file<F: Read + Seek>(&mut self, mut file: F, filename: Option<String>) -> Result<()> {
        // FIXME: Do not clone filename.
        if let Ok(explorer) = ImageExplorer::file(&mut file, filename.clone()) {
            self.new_explorer(explorer);
            return Ok(());
        }
        if let Ok(explorer) = VtfExplorer::file(&mut file, filename.clone()) {
            self.new_explorer(explorer);
            return Ok(());
        }

        Ok(())
    }

    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path: PathBuf = path.as_ref().into();
    
        if !path.try_exists()? {
            return Err(anyhow!("Failed to open path."));
        }
    
        if path.is_file() {
            if let Ok(explorer) = VpkExplorer::open(self.clone(), &path) {
                self.new_explorer(explorer);
                return Ok(())
            }

            self.open_file(
                File::open(&path)?,
                crate::util::filename(&path),
            )?;
        }
    
        Ok(())
    }
}

impl eframe::App for SharedAppContext {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let files = ctx.input(|i| i.raw.dropped_files.clone());
        if !files.is_empty() {
            for file in files {
                if let Some(path) = file.path {
                    self.open(path).expect("Failed to open file.");
                }
            }
        }

        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {

            egui::SidePanel::left("tab_list").show(ui.ctx(), |ui| {
                ui.vertical(|ui| {
                    ui.label("Tab List");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.vertical(|ui| {
                            let selected_explorer: Option<Uuid> = self.current_explorer().map(|e| e.uuid());
                            for explorer in self.explorers().iter_mut() {
                                let is_selected_explorer = selected_explorer.map(|u| u == explorer.uuid()).unwrap_or(false);

                                ui.horizontal(|ui| {
                                    ui.style_mut().spacing.item_spacing = egui::Vec2::new(0.0, 0.0);

                                    if ui.add(
                                        egui::Button::new(explorer.name())
                                            .selected(is_selected_explorer)
                                    ).clicked() {
                                        self.select_explorer(explorer.uuid());
                                    }

                                    if ui.add(
                                        egui::Button::new("X")
                                            .selected(is_selected_explorer)
                                    ).clicked() {
                                        self.remove_explorer(explorer.uuid())
                                    }
                                });
                            }
                        });
                    });
                });
            });

            egui::CentralPanel::default()
                .frame(egui::Frame::central_panel(ui.style()).multiply_with_opacity(0.5))
                .show(ui.ctx(), |ui| {
                    egui::ScrollArea::both()
                        .auto_shrink([ false, false ])
                        .show(ui, |ui| {
                            if let Some(mut explorer) = self.current_explorer() {
                                if let Err(err) = explorer.update(ui) {
                                    println!("{}", err);
                                }
                            }
                        });
                });

        });
    }
}



pub trait Explorer {
    fn uuid(&self) -> uuid::Uuid;
    fn name(&mut self) -> String { "Unnamed Tab".to_owned() }
    fn update(&mut self, ui: &mut egui::Ui) -> Result<()>;
}



#[derive(Clone)]
pub struct SharedExplorer(Rc<RefCell<dyn Explorer>>);

impl Explorer for SharedExplorer {
    fn uuid(&self) -> uuid::Uuid { self.0.borrow().uuid() }
    fn name(&mut self) -> String { self.0.borrow_mut().name() }
    fn update(&mut self, ui: &mut egui::Ui) -> Result<()> { self.0.borrow_mut().update(ui) }
}


