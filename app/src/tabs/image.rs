use std::{fmt::Debug, io::Seek};

use util_vfs::{VirtualFileSystemFile, VirtualFileSystemPath};

use crate::{
    app_util::save_image,
    egui_util::{image_handle, load_image, ImageSourceWithTextureHandle},
};

use super::TabTrait;

#[derive(Debug)]
enum ImageLoadState {
    Unloaded(VirtualFileSystemFile),
    Error(image::ImageError),
    Loaded {
        texture: ImageSourceWithTextureHandle<'static>,
        image: image::DynamicImage,
        path: VirtualFileSystemPath,
        filesize: u64,
    },
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
    fn name(&self) -> egui::WidgetText {
        self.name.as_str().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        match &mut self.state {
            ImageLoadState::Unloaded(file) => match load_image(file) {
                Ok(image) => {
                    let handle = image_handle(image.clone(), ui.ctx());
                    let source =
                        egui::ImageSource::Texture(egui::load::SizedTexture::from_handle(&handle));
                    self.state = ImageLoadState::Loaded {
                        texture: ImageSourceWithTextureHandle { source, handle },
                        image,
                        path: file.path().clone(),
                        filesize: file.seek(std::io::SeekFrom::End(0)).unwrap_or(0),
                    };
                }
                Err(err) => {
                    self.state = ImageLoadState::Error(err);
                }
            },
            ImageLoadState::Error(error) => {
                ui.label(format!("{error}"));
            }
            ImageLoadState::Loaded {
                texture,
                image,
                path,
                filesize,
            } => {
                ui.centered_and_justified(|ui| {
                    ui.add(
                        egui::Image::new(texture.source.clone())
                            .fit_to_exact_size(ui.available_size())
                            .sense(egui::Sense::CLICK),
                    )
                    .context_menu(|ui| {
                        if let Some(name) = path.name() {
                            ui.horizontal(|ui| {
                                ui.set_max_width(256.0);
                                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                                ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
                                if let Some(ext_pos) = name
                                    .char_indices()
                                    .rev()
                                    .find_map(|(i, c)| (c == '.').then_some(i))
                                {
                                    let (name, ext) = name.split_at(ext_pos);
                                    ui.label(name);
                                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                                    ui.label(ext);
                                } else {
                                    ui.label(name);
                                }
                            });
                        }
                        ui.label(format!(
                            "{:.2}KiB {}x{}",
                            (*filesize as f64) / 1024.0,
                            image.width(),
                            image.height(),
                        ));
                        if ui.button("Save").clicked() {
                            if let Err(err) = save_image(image, path.name()) {
                                log::error!("Failed to save image: {err}");
                            }
                        }
                    });
                });
            }
        }
    }
}
