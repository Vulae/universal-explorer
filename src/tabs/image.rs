use std::fmt::Debug;

use util::VirtualFileSystemFile;

use crate::egui_util::{image_handle, load_image, ImageSourceWithTextureHandle};

use super::TabTrait;

#[derive(Debug)]
enum ImageLoadState {
    Unloaded(VirtualFileSystemFile),
    Error(image::ImageError),
    Loaded(ImageSourceWithTextureHandle<'static>),
}

#[derive(Debug)]
pub struct ImageTab {
    name: String,
    state: ImageLoadState,
}

impl ImageTab {
    pub fn new(name: String, file: VirtualFileSystemFile) -> Self {
        Self {
            name,
            state: ImageLoadState::Unloaded(file),
        }
    }
}

impl TabTrait for ImageTab {
    fn name(&self) -> &str {
        &self.name
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        match &mut self.state {
            ImageLoadState::Unloaded(file) => match load_image(file) {
                Ok(image) => {
                    let handle = image_handle(image, ui.ctx());
                    let source =
                        egui::ImageSource::Texture(egui::load::SizedTexture::from_handle(&handle));
                    self.state =
                        ImageLoadState::Loaded(ImageSourceWithTextureHandle { source, handle });
                }
                Err(err) => {
                    self.state = ImageLoadState::Error(err);
                }
            },
            ImageLoadState::Error(error) => {
                ui.label(format!("{error}"));
            }
            ImageLoadState::Loaded(ImageSourceWithTextureHandle { source, .. }) => {
                ui.add(egui::Image::new(source.clone()).fit_to_exact_size(ui.available_size()));
            }
        }
    }
}
