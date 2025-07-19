mod hex;
mod image;
mod other;
mod virtual_fs;
mod vtf;
pub use hex::*;
pub use image::*;
pub use other::*;
pub use virtual_fs::*;
pub use vtf::*;

use crate::app::AppEvent;

pub trait TabTrait: std::fmt::Debug {
    fn name(&self) -> egui::WidgetText;
    fn next_event(&mut self) -> Option<AppEvent> {
        None
    }
    fn ui(&mut self, ui: &mut egui::Ui);
}

#[derive(Debug)]
pub struct Tab {
    inner: Box<dyn TabTrait>,
    uuid: uuid::Uuid,
}

impl Tab {
    pub fn new(inner: Box<dyn TabTrait>) -> Self {
        Self {
            inner,
            uuid: uuid::Uuid::now_v7(),
        }
    }

    pub fn id(&self) -> uuid::Uuid {
        self.uuid
    }

    pub fn name(&self) -> egui::WidgetText {
        self.inner.name()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.inner.ui(ui)
    }

    pub fn events(&mut self) -> Vec<AppEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.inner.next_event() {
            events.push(event);
        }
        events
    }
}
