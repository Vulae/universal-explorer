
use std::{cell::RefCell, io::{Read, Seek}, path::Path, rc::Rc};
use anyhow::Result;
use crate::{app_util, assets};
use super::loader;



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
        tab.ui(ui);
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

    fn push_new_explorers_to_dock_state(&mut self) {
        // TODO: Reimplement auto_focus_new_explorers
        while let Some(explorer) = self.explorers_to_add.pop() {
            self.dock_state.push_to_focused_leaf(explorer);
        }
    }
}





#[derive(Clone)]
pub struct SharedAppContext {
    context: Rc<RefCell<AppContext>>,
}

impl SharedAppContext {
    pub fn new() -> Self {
        let app_context = AppContext::new();
        Self { context: Rc::new(RefCell::new(app_context)) }
    }

    pub fn new_explorer(&mut self, explorer: Box<dyn Explorer>) {
        let shared_explorer = SharedExplorer {
            uuid: *explorer.uuid(),
            explorer: Rc::new(RefCell::new(explorer)),
        };
        self.context.borrow_mut().explorers_to_add.push(shared_explorer);
    }

    pub fn open_file<F: Read + Seek>(&mut self, file: F, filename: Option<String>) -> Result<()> {
        if let Some(explorer) = loader::open_file(self.clone(), file, filename)? {
            self.new_explorer(explorer);
        }
        Ok(())
    }

    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        if let Some(explorer) = loader::open(self.clone(), path)? {
            self.new_explorer(explorer);
        }
        Ok(())
    }
}

impl eframe::App for SharedAppContext {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        let frame_start = std::time::Instant::now();

        egui_extras::install_image_loaders(ctx);

        // TODO: Add config.toml with theme selection. (& with custom theme with catppuccin_egui::Theme)
        catppuccin_egui::set_theme(ctx, self.context.borrow().theme);

        self.context.borrow_mut().auto_focus_new_explorers = !ctx.input(|i| i.modifiers.shift);

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

        self.context.borrow_mut().last_frame_time = frame_start.elapsed();
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
                    ui.image(assets::UNIVERSAL_EXPLORER_ICON);
                    ui.hyperlink_to("universal-explorer", "https://github.com/Vulae/universal-explorer");

                    ui.add_space(16.0);
                    
                    ui.menu_button("Settings", |ui| {
                        ui.menu_button("Theme", |ui| {
                            ui.selectable_value(&mut self.context.borrow_mut().theme, catppuccin_egui::LATTE, "Latte");
                            ui.selectable_value(&mut self.context.borrow_mut().theme, catppuccin_egui::FRAPPE, "Frappé");
                            ui.selectable_value(&mut self.context.borrow_mut().theme, catppuccin_egui::MACCHIATO, "Macchiato");
                            ui.selectable_value(&mut self.context.borrow_mut().theme, catppuccin_egui::MOCHA, "Mocha");
                        });
                    });

                    app_util::virtual_fs::render_dropdown_fs(ui, &mut crate::app::assets::ASSETS_FS.clone(), "Built-In", |entry| {
                        if let Some(file) = entry.as_file() {
                            let name = file.path().name().map(|s| s.to_owned());
                            self.open_file(file, name).expect("Failed to open file");
                        }
                    });

                    ui.label(format!("{:?}", self.context.borrow().last_frame_time));

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
        self.context.borrow_mut().push_new_explorers_to_dock_state();

        if self.context.borrow_mut().has_explorers() {

            // TODO: Probably want to refactor this thing.
            // Having to clone the dock state, then set it to avoid BorrowMutError is just bad.
            let mut dock_state = self.context.borrow_mut().dock_state.clone();
    
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
    
            self.context.borrow_mut().dock_state = dock_state;

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
    fn uuid(&self) -> &uuid::Uuid;
    fn name(&mut self) -> String { "Unnamed".to_owned() }
    fn ui(&mut self, ui: &mut egui::Ui);
}



#[derive(Clone)]
pub struct SharedExplorer {
    explorer: Rc<RefCell<Box<dyn Explorer>>>,
    uuid: uuid::Uuid,
}

impl Explorer for SharedExplorer {
    fn uuid(&self) -> &uuid::Uuid { &self.uuid }
    fn name(&mut self) -> String { self.explorer.borrow_mut().name().clone() }
    fn ui(&mut self, ui: &mut egui::Ui) { self.explorer.borrow_mut().ui(ui) }
}


