
use std::{cell::RefCell, fs::File, io::{Read, Seek}, path::{Path, PathBuf}, rc::Rc};
use anyhow::{anyhow, Result};
use super::explorers::{image::ImageExplorer, renpy::rpa::RenPyArchiveExplorer, source_engine::{vpk::VpkExplorer, vtf::VtfExplorer}, text::TextExplorer};



struct ExplorerTab;

impl egui_dock::TabViewer for ExplorerTab {
    type Tab = SharedExplorer;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        tab.uuid().to_string().into()
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.name().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        if let Err(err) = tab.ui(ui) {
            println!("UI Error: {}", err);
        }
    }
}



// TODO: Add cache for old deleted explorers, to add undo delete functionality.
pub struct AppContext {
    dock_state: egui_dock::DockState<SharedExplorer>,
    explorers_to_add: Vec<SharedExplorer>,
    auto_focus_new_explorers: bool,
    theme: catppuccin_egui::Theme,
    last_frame_time: std::time::Duration,
}

impl AppContext {
    fn new() -> Self {
        Self {
            dock_state: egui_dock::DockState::new(Vec::new()),
            explorers_to_add: Vec::new(),
            auto_focus_new_explorers: true,
            theme: match dark_light::detect() {
                dark_light::Mode::Dark => catppuccin_egui::MOCHA,
                dark_light::Mode::Light => catppuccin_egui::LATTE,
                dark_light::Mode::Default => catppuccin_egui::MOCHA,
            },
            last_frame_time: std::time::Duration::ZERO,
        }
    }

    fn has_explorers(&self) -> bool {
        self.dock_state.iter_all_tabs().count() > 0
    }

    fn new_explorer(&mut self, explorer: impl Explorer + 'static) {
        self.explorers_to_add.push(SharedExplorer(Rc::new(RefCell::new(explorer))));
    }

    fn push_new_explorers_to_dock_state(&mut self) {
        // TODO: Reimplement auto_focus_new_explorers
        while let Some(explorer) = self.explorers_to_add.pop() {
            self.dock_state.push_to_focused_leaf(explorer);
        }
    }
}





#[derive(Clone)]
pub struct SharedAppContext(Rc<RefCell<AppContext>>);

impl SharedAppContext {
    pub fn new() -> Self {
        let app_context = AppContext::new();
        Self(Rc::new(RefCell::new(app_context)))
    }

    pub fn new_explorer(&mut self, explorer: impl Explorer + 'static) {
        self.0.borrow_mut().new_explorer(explorer);
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
        if let Ok(explorer) = TextExplorer::file(&mut file, filename.clone()) {
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
            if let Ok(explorer) = RenPyArchiveExplorer::open(self.clone(), &path) {
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

        let frame_start = std::time::Instant::now();

        egui_extras::install_image_loaders(ctx);

        // TODO: Add config.toml with theme selection. (& with custom theme with catppuccin_egui::Theme)
        catppuccin_egui::set_theme(ctx, self.0.borrow().theme);

        self.0.borrow_mut().auto_focus_new_explorers = !ctx.input(|i| i.modifiers.shift);

        let files = ctx.input(|i| i.raw.dropped_files.clone());
        if !files.is_empty() {
            for file in files {
                if let Some(path) = file.path {
                    self.open(path).expect("Failed to open file");
                }
            }
        }

        self.ui_decorations(ctx);
        self.ui_main(ctx);

        self.0.borrow_mut().last_frame_time = frame_start.elapsed();
    }
}

impl SharedAppContext {
    fn ui_decorations(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("application_decorations")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            )
            .show(ctx, |ui| {
                let interaction = ui.interact(ui.max_rect(), "application_decorations_interaction".into(), egui::Sense::click_and_drag());

                egui::menu::bar(ui, |ui| {
                    ui.image(crate::app::assets::UNIVERSAL_EXPLORER_ICON);
                    ui.hyperlink_to("universal-explorer", "https://github.com/Vulae/universal-explorer");

                    ui.add_space(16.0);
                    
                    ui.menu_button("Settings", |ui| {
                        ui.menu_button("Theme", |ui| {
                            ui.selectable_value(&mut self.0.borrow_mut().theme, catppuccin_egui::LATTE, "Latte");
                            ui.selectable_value(&mut self.0.borrow_mut().theme, catppuccin_egui::FRAPPE, "Frappé");
                            ui.selectable_value(&mut self.0.borrow_mut().theme, catppuccin_egui::MACCHIATO, "Macchiato");
                            ui.selectable_value(&mut self.0.borrow_mut().theme, catppuccin_egui::MOCHA, "Mocha");
                        });
                    });

                    crate::util::egui::virtual_fs::render_dropdown_fs(ui, &mut crate::app::assets::ASSETS_FS.clone(), "Built-In", |entry| {
                        if let Some(file) = entry.as_file() {
                            let name = file.path().name().map(|s| s.to_owned());
                            self.open_file(file, name).expect("Failed to open file");
                        }
                    });

                    ui.label(format!("{:?}", self.0.borrow().last_frame_time));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // FIXME: Wrong ordering.

                        // TODO: Red background on hover
                        if ui.button("🗙").clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                        // if ui.button(if is_maximized { "🗗" } else { "🗖" }).clicked() {
                        if ui.button("🗖").clicked() {
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                        }

                        if ui.button("🗕").clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });

                if interaction.double_clicked() {
                    let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                }
                if interaction.drag_started_by(egui::PointerButton::Primary) {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            });
    }

    fn ui_main(&mut self, ctx: &egui::Context) {
        self.0.borrow_mut().push_new_explorers_to_dock_state();

        if self.0.borrow_mut().has_explorers() {

            // TODO: Probably want to refactor this thing.
            // Having to clone the dock state, then set it to avoid BorrowMutError is just bad.
            let mut dock_state = self.0.borrow_mut().dock_state.clone();
    
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::central_panel(&ctx.style())
                        .multiply_with_opacity(0.5)
                )
                .show(ctx, |ui| {
                    egui_dock::DockArea::new(&mut dock_state)
                        .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                        .show_inside(ui, &mut ExplorerTab);
                });
    
            self.0.borrow_mut().dock_state = dock_state;

        } else {

            egui::CentralPanel::default()
                .frame(
                    egui::Frame::central_panel(&ctx.style())
                        .multiply_with_opacity(0.5)
                        .inner_margin(96.0)
                )
                .show(ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        // FIXME: Text spacing when wrapping.
                        ui.add(egui::Label::new(
                            egui::RichText::new("Drag & drop files to view")
                                .text_style(egui::TextStyle::Heading)
                                .size(64.0)
                                .strong()
                        ));
                    });
                });
            
        }
    }
}



pub trait Explorer {
    fn uuid(&self) -> uuid::Uuid;
    fn name(&mut self) -> String { "Unnamed Tab".to_owned() }
    fn ui(&mut self, ui: &mut egui::Ui) -> Result<()>;
}



#[derive(Clone)]
pub struct SharedExplorer(Rc<RefCell<dyn Explorer>>);

impl Explorer for SharedExplorer {
    fn uuid(&self) -> uuid::Uuid { self.0.borrow().uuid() }
    fn name(&mut self) -> String { self.0.borrow_mut().name() }
    fn ui(&mut self, ui: &mut egui::Ui) -> Result<()> { self.0.borrow_mut().ui(ui) }
}


