use util::{OsFs, VirtualFileSystem};

use crate::{
    assets,
    tabs::{Tab, TextureInfoTab, VirtualFsTab},
};

#[derive(Debug)]
pub enum AppEvent {
    CreateTab(Tab),
}

#[derive(Debug)]
pub struct App {
    sys: sysinfo::System,
    pid: sysinfo::Pid,
    last_update_time: std::time::Duration,
    debug_stats_enabled: bool,
    tree: egui_dock::DockState<Tab>,
    events: Vec<AppEvent>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            sys: sysinfo::System::new_with_specifics(
                sysinfo::RefreshKind::nothing().with_processes(
                    sysinfo::ProcessRefreshKind::nothing()
                        // .with_cpu()
                        .with_memory(),
                ),
            ),
            pid: sysinfo::get_current_pid().expect("Failed to get this process identifier"),
            last_update_time: std::time::Duration::MAX,
            debug_stats_enabled: true,
            tree: egui_dock::DockState::new(Vec::new()),
            events: Vec::new(),
        }
    }
}

impl App {
    fn process_events(&mut self) {
        let mut tab_events: Vec<AppEvent> = self
            .tree
            .iter_all_tabs_mut()
            .flat_map(|(_, tab)| tab.events().into_iter())
            .collect();
        self.events.append(&mut tab_events);
        for event in self.events.drain(..).collect::<Vec<_>>().into_iter() {
            if let Err(err) = self.event(event) {
                log::error!("Error while processing event: {err}");
            }
        }
    }

    pub fn event(&mut self, event: AppEvent) -> Result<(), anyhow::Error> {
        match event {
            AppEvent::CreateTab(tab) => {
                log::info!(
                    "Create tab \"{}\" with ID of {}",
                    tab.name().text(),
                    tab.id(),
                );
                self.tree.push_to_focused_leaf(tab);
            }
        }
        Ok(())
    }

    fn window_decorations(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("universal-explorer_decorations")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ctx, |ui| {
                let interaction = ui.interact(
                    ui.max_rect(),
                    "universal-explorer_decorations_interaction".into(),
                    egui::Sense::click_and_drag(),
                );

                egui::MenuBar::new().ui(ui, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.image(assets::UNIVERSAL_EXPLORER_ICON.clone());
                        ui.hyperlink_to(
                            "universal-explorer",
                            "https://github.com/Vulae/universal-explorer",
                        );
                    });
                    ui.add_space(16.0);

                    ui.menu_button(" + ", |ui| {
                        ui.menu_button("DEBUG", |ui| {
                            if ui
                                .button(format!(
                                    "stats: {}",
                                    if self.debug_stats_enabled {
                                        "ON"
                                    } else {
                                        "OFF"
                                    }
                                ))
                                .clicked()
                            {
                                self.debug_stats_enabled = !self.debug_stats_enabled;
                            }
                            if ui.button("egui texture info").clicked() {
                                self.events
                                    .push(AppEvent::CreateTab(Tab::new(Box::new(TextureInfoTab))));
                            }
                            if ui.button("OS file system: /").clicked() {
                                let fs =
                                    OsFs::new_root().expect("Error while creating new OsFs tab");
                                self.events.push(AppEvent::CreateTab(Tab::new(Box::new(
                                    VirtualFsTab::new(
                                        "OsFs".to_owned(),
                                        VirtualFileSystem::new(Box::new(fs)),
                                    ),
                                ))));
                            }
                            if ui.button("OS file system: ~/").clicked() {
                                let path = home::home_dir().expect("Error while creating new OsFs tab: Could not get home directory");
                                let fs =
                                    OsFs::new(path).expect("Error while creating new OsFs tab");
                                self.events.push(AppEvent::CreateTab(Tab::new(Box::new(
                                    VirtualFsTab::new(
                                        "OsFs".to_owned(),
                                        VirtualFileSystem::new(Box::new(fs)),
                                    ),
                                ))));
                            }
                        });
                    });
                    ui.add_space(16.0);

                    if self.debug_stats_enabled {
                        self.sys
                            .refresh_processes(sysinfo::ProcessesToUpdate::Some(&[self.pid]), true);
                        let process = self
                            .sys
                            .process(self.pid)
                            .expect("Somehow this process doesn't exist???");
                        ui.label(format!("MEM: {}MIB", process.memory() / 1048576));
                        ui.add_space(8.0);
                        ui.label(format!("DT: {:.2?}", self.last_update_time));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗙").clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        if ui.button("🗖").clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(
                                !ui.input(|i| i.viewport().maximized.unwrap_or(false)),
                            ));
                        }

                        if ui.button("🗕").clicked() {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });

                if interaction.double_clicked_by(egui::PointerButton::Primary) {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(
                        !ui.input(|i| i.viewport().maximized.unwrap_or(false)),
                    ));
                }
                if interaction.drag_started_by(egui::PointerButton::Primary) {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let start = std::time::Instant::now();

        egui_extras::install_image_loaders(ctx);

        self.process_events();

        self.window_decorations(ctx);

        struct TabViewer;
        impl egui_dock::TabViewer for TabViewer {
            type Tab = Tab;

            fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
                egui::Id::new(tab.id())
            }

            fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
                tab.name()
            }

            fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
                ui.push_id(tab.id(), |ui| {
                    tab.ui(ui);
                });
            }

            // This is handled inside the tab
            fn scroll_bars(&self, _tab: &Self::Tab) -> [bool; 2] {
                [false, false]
            }
        }

        egui_dock::DockArea::new(&mut self.tree)
            .style(egui_dock::Style::from_egui(ctx.style().as_ref()))
            .show_leaf_collapse_buttons(false)
            .show_leaf_close_all_buttons(false)
            .show(ctx, &mut TabViewer);

        self.last_update_time = std::time::Instant::now().duration_since(start);
    }
}
