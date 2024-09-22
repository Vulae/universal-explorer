use super::loader;
use crate::{app_util, assets};
use anyhow::Result;
use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{Read, Seek},
    path::Path,
    rc::Rc,
    sync::{Arc, Mutex},
};

struct ExplorerPane;

impl ExplorerPane {
    fn tab_icon_for_pane(&mut self, pane: &SharedExplorer) -> Option<egui::ImageSource<'static>> {
        pane.icon()
    }

    fn tab_icon_for_tile(
        &mut self,
        tiles: &egui_tiles::Tiles<SharedExplorer>,
        tile_id: egui_tiles::TileId,
    ) -> Option<egui::ImageSource<'static>> {
        tiles
            .get(tile_id)
            .map(|tile| {
                if let egui_tiles::Tile::Pane(pane) = tile {
                    Some(pane)
                } else {
                    None
                }
            })
            .flatten()
            .map(|pane| self.tab_icon_for_pane(pane))
            .flatten()
    }
}

impl egui_tiles::Behavior<SharedExplorer> for ExplorerPane {
    fn tab_title_for_pane(&mut self, pane: &SharedExplorer) -> egui::WidgetText {
        pane.title().into()
    }

    fn is_tab_closable(
        &self,
        _tiles: &egui_tiles::Tiles<SharedExplorer>,
        _tile_id: egui_tiles::TileId,
    ) -> bool {
        true
    }

    // Taken from egui_tiles::Behavior::tab_ui
    // But with ability to display Explorer::icon
    // And the ability to close with middle click
    fn tab_ui(
        &mut self,
        tiles: &mut egui_tiles::Tiles<SharedExplorer>,
        ui: &mut egui::Ui,
        id: egui::Id,
        tile_id: egui_tiles::TileId,
        state: &egui_tiles::TabState,
    ) -> egui::Response {
        let text = self.tab_title_for_tile(tiles, tile_id);
        let icon_size = egui::Vec2::splat(16.0);
        let icon_right_padding = 4.0;
        let icon = self
            .tab_icon_for_tile(tiles, tile_id)
            .map(|icon| egui::Image::new(icon).fit_to_exact_size(icon_size));
        let close_btn_size = egui::Vec2::splat(self.close_button_outer_size());
        let close_btn_left_padding = 4.0;
        let font_id = egui::TextStyle::Button.resolve(ui.style());
        let galley = text.into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, font_id);

        let x_margin = self.tab_title_spacing(ui.visuals());

        let button_width = galley.size().x
            + 2.0 * x_margin
            + f32::from(state.closable) * (close_btn_left_padding + close_btn_size.x)
            + if icon.is_some() {
                icon_size.x + icon_right_padding
            } else {
                0.0
            };
        let (_, tab_rect) = ui.allocate_space(egui::vec2(button_width, ui.available_height()));

        let tab_response = ui
            .interact(tab_rect, id, egui::Sense::click_and_drag())
            .on_hover_cursor(egui::CursorIcon::Grab);

        // Close with middle click
        if tab_response.middle_clicked() {
            if self.on_tab_close(tiles, tile_id) {
                tiles.remove(tile_id);
            }

            return self.on_tab_button(tiles, tile_id, tab_response);
        }

        // Show a gap when dragged
        if ui.is_rect_visible(tab_rect) && !state.is_being_dragged {
            let bg_color = self.tab_bg_color(ui.visuals(), tiles, tile_id, state);
            let stroke = self.tab_outline_stroke(ui.visuals(), tiles, tile_id, state);
            ui.painter()
                .rect(tab_rect.shrink(0.5), 0.0, bg_color, stroke);

            if state.active {
                // Make the tab name area connect with the tab ui area:
                ui.painter().hline(
                    tab_rect.x_range(),
                    tab_rect.bottom(),
                    egui::Stroke::new(stroke.width + 1.0, bg_color),
                );
            }

            let mut text_icon_position = egui::Align2::LEFT_CENTER
                .align_size_within_rect(galley.size(), tab_rect.shrink(x_margin))
                .min;

            // Render icon
            if let Some(icon) = icon {
                icon.paint_at(
                    ui,
                    egui::Rect::from_min_size(text_icon_position, egui::Vec2::new(16.0, 16.0)),
                );
                text_icon_position += egui::Vec2::new(icon_size.x + icon_right_padding, 0.0);
            }

            // Render the title
            let text_color = self.tab_text_color(ui.visuals(), tiles, tile_id, state);
            ui.painter().galley(text_icon_position, galley, text_color);

            // Conditionally render the close button
            if state.closable {
                let close_btn_rect = egui::Align2::RIGHT_CENTER
                    .align_size_within_rect(close_btn_size, tab_rect.shrink(x_margin));

                // Allocate
                let close_btn_id = ui.auto_id_with("tab_close_btn");
                let close_btn_response = ui
                    .interact(close_btn_rect, close_btn_id, egui::Sense::click_and_drag())
                    .on_hover_cursor(egui::CursorIcon::Default);

                let visuals = ui.style().interact(&close_btn_response);

                // Scale based on the interaction visuals
                let rect = close_btn_rect
                    .shrink(self.close_button_inner_margin())
                    .expand(visuals.expansion);
                let stroke = visuals.fg_stroke;

                // paint the crossed lines
                ui.painter() // paints \
                    .line_segment([rect.left_top(), rect.right_bottom()], stroke);
                ui.painter() // paints /
                    .line_segment([rect.right_top(), rect.left_bottom()], stroke);

                // Give the user a chance to react to the close button being clicked
                // Only close if the user returns true (handled)
                if close_btn_response.clicked() {
                    // Close the tab if the implementation wants to
                    if self.on_tab_close(tiles, tile_id) {
                        tiles.remove(tile_id);
                    }
                }
            }
        }

        self.on_tab_button(tiles, tile_id, tab_response)
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut SharedExplorer,
    ) -> egui_tiles::UiResponse {
        // FIXME: Force frame size to be available space.
        egui::ScrollArea::new(true).show(ui, |ui| {
            egui::Frame::central_panel(ui.style())
                .inner_margin(16.0)
                .stroke(egui::Stroke::new(
                    1.0,
                    ui.style().visuals.widgets.active.bg_fill,
                ))
                .show(ui, |ui| {
                    let available_rect = ui.available_rect_before_wrap();
                    ui.expand_to_include_rect(available_rect);
                    ui.set_max_size(available_rect.size());
                    pane.ui(ui);
                });
        });
        Default::default()
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            prune_empty_tabs: true,
            prune_empty_containers: true,
            prune_single_child_tabs: true,
            prune_single_child_containers: true,
            all_panes_must_have_tabs: true,
            join_nested_linear_containers: false,
        }
    }
}

pub enum AppContextEvent {
    NewExplorer(Box<dyn Explorer>),
}

struct AppContextEventReceiver {
    queue: Arc<Mutex<VecDeque<AppContextEvent>>>,
}

impl AppContextEventReceiver {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn create_sender(&self) -> AppContextEventSender {
        AppContextEventSender {
            queue: Arc::clone(&self.queue),
        }
    }

    pub fn pop(&mut self) -> Option<AppContextEvent> {
        self.queue.lock().unwrap().pop_front()
    }
}

#[derive(Clone)]
pub struct AppContextEventSender {
    queue: Arc<Mutex<VecDeque<AppContextEvent>>>,
}

impl AppContextEventSender {
    pub fn push(&mut self, event: AppContextEvent) {
        self.queue.lock().unwrap().push_back(event)
    }
}

// TODO: Add cache for old deleted explorers, to add undo delete functionality.
pub struct AppContext {
    tree: egui_tiles::Tree<SharedExplorer>,
    event_receiver: AppContextEventReceiver,
    auto_focus_new_explorers: bool,
    theme: catppuccin_egui::Theme,
    last_frame_time: std::time::Duration,
}

impl AppContext {
    fn new() -> Self {
        let mut tiles = egui_tiles::Tiles::default();
        let root = tiles.insert_tab_tile(Vec::new());
        Self {
            tree: egui_tiles::Tree::new("root_tree", root, tiles),
            event_receiver: AppContextEventReceiver::new(),
            auto_focus_new_explorers: true,
            theme: match dark_light::detect() {
                dark_light::Mode::Dark => catppuccin_egui::MOCHA,
                dark_light::Mode::Light => catppuccin_egui::LATTE,
                dark_light::Mode::Default => catppuccin_egui::MOCHA,
            },
            last_frame_time: std::time::Duration::ZERO,
        }
    }

    fn execute_events(&mut self) {
        while let Some(event) = self.event_receiver.pop() {
            match event {
                AppContextEvent::NewExplorer(explorer) => {
                    // TODO: Better tree insertion, Allow insertion into a container with a specific ID.
                    let id = self.tree.tiles.insert_pane(SharedExplorer {
                        uuid: *explorer.uuid(),
                        explorer: Rc::new(RefCell::new(explorer)),
                    });
                    let _ = self.tree.tiles.insert_tab_tile(vec![id]);
                    // TODO: Am I just dumb and missed something in the docs?
                    // Why isn't there a way to force a default root?
                    let target = self.tree.root().unwrap_or_else(|| {
                        let root_id =
                            self.tree
                                .tiles
                                .insert_container(egui_tiles::Container::Tabs(egui_tiles::Tabs {
                                    children: Vec::new(),
                                    active: None,
                                }));
                        // Is this stupid?
                        self.tree.root = Some(root_id);
                        root_id
                    });
                    self.tree
                        .move_tile_to_container(id, target, usize::MAX, true);
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct SharedAppContext {
    context: Rc<RefCell<AppContext>>,
    event_sender: AppContextEventSender,
}

impl SharedAppContext {
    pub fn new() -> Self {
        let app_context = AppContext::new();
        Self {
            event_sender: app_context.event_receiver.create_sender(),
            context: Rc::new(RefCell::new(app_context)),
        }
    }

    pub fn new_explorer(&mut self, explorer: Box<dyn Explorer>) {
        self.event_sender
            .push(AppContextEvent::NewExplorer(explorer));
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

        self.context.borrow_mut().execute_events();

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
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                let interaction = ui.interact(
                    ui.max_rect(),
                    "application_decorations_interaction".into(),
                    egui::Sense::click_and_drag(),
                );

                egui::menu::bar(ui, |ui| {
                    ui.image(assets::UNIVERSAL_EXPLORER_ICON);
                    ui.hyperlink_to(
                        "universal-explorer",
                        "https://github.com/Vulae/universal-explorer",
                    );

                    ui.add_space(16.0);

                    ui.menu_button("Settings", |ui| {
                        ui.menu_button("Theme", |ui| {
                            ui.selectable_value(
                                &mut self.context.borrow_mut().theme,
                                catppuccin_egui::LATTE,
                                "Latte",
                            );
                            ui.selectable_value(
                                &mut self.context.borrow_mut().theme,
                                catppuccin_egui::FRAPPE,
                                "Frappé",
                            );
                            ui.selectable_value(
                                &mut self.context.borrow_mut().theme,
                                catppuccin_egui::MACCHIATO,
                                "Macchiato",
                            );
                            ui.selectable_value(
                                &mut self.context.borrow_mut().theme,
                                catppuccin_egui::MOCHA,
                                "Mocha",
                            );
                        });
                    });

                    app_util::virtual_fs::render_dropdown_fs(
                        ui,
                        &mut crate::app::assets::ASSETS_FS.clone(),
                        "Built-In",
                        |entry| {
                            if let Some(file) = entry.as_file() {
                                let name = file.path().name().map(|s| s.to_owned());
                                self.open_file(file, name).expect("Failed to open file");
                            }
                        },
                    );

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
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                        }

                        if ui.button("🗕").clicked() {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });

                if interaction.double_clicked() {
                    let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                }
                if interaction.drag_started_by(egui::PointerButton::Primary) {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            });
    }

    fn ui_main(&mut self, ctx: &egui::Context) {
        if !self.context.borrow().tree.is_empty() {
            egui::CentralPanel::default()
                .frame(egui::Frame::central_panel(&ctx.style()).multiply_with_opacity(0.5))
                .show(ctx, |ui| {
                    self.context.borrow_mut().tree.ui(&mut ExplorerPane, ui);
                });
        } else {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::central_panel(&ctx.style())
                        .multiply_with_opacity(0.5)
                        .inner_margin(96.0),
                )
                .show(ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        // FIXME: Text spacing when wrapping.
                        ui.add(egui::Label::new(
                            egui::RichText::new("Drag & drop files to view")
                                .text_style(egui::TextStyle::Heading)
                                .size(64.0)
                                .strong(),
                        ));
                    });
                });
        }
    }
}

pub trait Explorer {
    fn uuid(&self) -> &uuid::Uuid;
    fn title(&self) -> String {
        "Unnamed".to_owned()
    }
    fn icon(&self) -> Option<egui::ImageSource<'static>> {
        None
    }
    fn ui(&mut self, ui: &mut egui::Ui);
}

#[derive(Clone)]
pub struct SharedExplorer {
    explorer: Rc<RefCell<Box<dyn Explorer>>>,
    uuid: uuid::Uuid,
}

impl Explorer for SharedExplorer {
    fn uuid(&self) -> &uuid::Uuid {
        &self.uuid
    }
    fn title(&self) -> String {
        self.explorer.borrow().title().clone()
    }
    fn icon(&self) -> Option<egui::ImageSource<'static>> {
        self.explorer.borrow().icon()
    }
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.explorer.borrow_mut().ui(ui)
    }
}
