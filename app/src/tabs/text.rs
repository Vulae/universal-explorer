use egui::Widget as _;

use crate::app::AppEvent;

use super::TabTrait;

#[derive(Debug)]
pub struct TextTab {
    name: String,
    content: String,
}

impl TextTab {
    pub fn new(name: String, content: String) -> Self {
        Self { name, content }
    }
}

impl TabTrait for TextTab {
    fn name(&self) -> egui::WidgetText {
        self.name.as_str().into()
    }

    fn next_event(&mut self) -> Option<AppEvent> {
        None
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::TextEdit::multiline(&mut self.content.as_str())
                .desired_width(f32::INFINITY)
                .ui(ui);
        });
    }
}
