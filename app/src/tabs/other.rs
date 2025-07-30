use super::TabTrait;

#[derive(Debug)]
pub struct TextureInfoTab;

impl TabTrait for TextureInfoTab {
    fn name(&self) -> egui::WidgetText {
        "egui texture info".into()
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.ctx().clone().texture_ui(ui);
    }
}
