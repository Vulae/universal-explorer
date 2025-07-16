use crate::tabs::Tab;

#[derive(Debug)]
pub enum AppEvent {
    CreateTab(Tab),
}

#[derive(Debug, Default)]
pub struct App {
    tabs: Vec<Tab>,
    selected_tab: uuid::Uuid,
}

impl App {
    pub fn process_events(&mut self) -> Result<(), anyhow::Error> {
        let events: Vec<AppEvent> = self
            .tabs
            .iter_mut()
            .flat_map(|tab| tab.events().into_iter())
            .collect();
        for event in events {
            self.event(event)?;
        }
        Ok(())
    }

    pub fn event(&mut self, event: AppEvent) -> Result<(), anyhow::Error> {
        match event {
            AppEvent::CreateTab(tab) => {
                log::info!("Create tab \"{}\" with ID of {}", tab.name(), tab.id());
                self.tabs.push(tab);
            }
        }
        Ok(())
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        // TODO: Gracefully fail
        self.process_events().expect("Failed to process events");

        // egui::SidePanel::right("right").show(ctx, |ui| {
        //     ctx.texture_ui(ui);
        // });

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.tabs.iter().for_each(|tab| {
                    ui.push_id(tab.id(), |ui| {
                        if ui.button(tab.name()).clicked() {
                            self.selected_tab = tab.id();
                        }
                    });
                });
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let Some(tab) = self
                .tabs
                .iter_mut()
                .find(|tab| tab.id() == self.selected_tab)
            else {
                return;
            };
            ui.push_id(tab.id(), |ui| {
                tab.ui(ui);
            });
        });
    }
}
