use std::{collections::HashMap, fmt::Debug};

use egui::Widget as _;
use source_engine::{Vtf, VtfContainer, VtfTexture};

use crate::{
    app_util::{change_extension, save_image},
    egui_util::ImageSourceWithTextureHandle,
};

use super::TabTrait;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct VtfTextureIndex {
    mipmap: u8,
    frame: u16,
}

#[derive(Debug)]
struct VtfTextureView {
    texture: VtfTexture,
    handle: ImageSourceWithTextureHandle<'static>,
}

#[derive(Debug)]
struct VtfTextureLoadState {
    thumbnail: Option<VtfTextureView>,
    textures: HashMap<VtfTextureIndex, VtfTextureView>,
}

#[derive(Debug)]
pub struct VtfTab {
    name: String,
    filename: String,
    vtf: Vtf,
    current_frame: u16,
    state: Option<VtfTextureLoadState>,
}

impl VtfTab {
    pub fn new(name: String, filename: String, vtf: Vtf) -> Self {
        Self {
            name,
            filename,
            vtf,
            current_frame: 0,
            state: None,
        }
    }

    fn load_state(&mut self, ctx: &egui::Context) {
        if self.state.is_some() {
            return;
        }

        let VtfContainer::Frames(frames) = self.vtf.container() else {
            unimplemented!();
        };

        let thumbnail = self.vtf.lowres().cloned().map(|thumbnail| VtfTextureView {
            handle: ImageSourceWithTextureHandle::from_image(thumbnail.to_image(), ctx),
            texture: thumbnail,
        });

        let mut textures = HashMap::new();
        for mipmap in 0..frames.num_mipmaps() {
            for frame in 0..frames.num_frames() {
                let texture = frames.texture(mipmap, frame).unwrap().clone();
                textures.insert(
                    VtfTextureIndex { mipmap, frame },
                    VtfTextureView {
                        handle: ImageSourceWithTextureHandle::from_image(texture.to_image(), ctx),
                        texture,
                    },
                );
            }
        }
        self.state = Some(VtfTextureLoadState {
            thumbnail,
            textures,
        });
    }
}

impl TabTrait for VtfTab {
    fn name(&self) -> egui::WidgetText {
        self.name.as_str().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        self.load_state(ui.ctx());
        let state = self.state.as_ref().unwrap();

        let VtfContainer::Frames(frames) = self.vtf.container() else {
            unimplemented!();
        };

        ui.label(format!("{:?}", frames.format()));
        ui.label(format!("{}x{}", frames.width(), frames.height()));
        ui.label(format!(
            "mips: {}, frames: {}",
            frames.num_mipmaps(),
            frames.num_frames(),
        ));

        if let Some(thumbnail) = &state.thumbnail {
            ui.horizontal(|ui| {
                let rect = ui.label("thumbnail: ").rect;
                ui.add(
                    egui::Image::new(thumbnail.handle.source.clone())
                        .fit_to_exact_size(egui::Vec2::INFINITY)
                        .max_height(rect.height())
                        .sense(egui::Sense::CLICK),
                )
                .context_menu(|ui| {
                    ui.label(format!(
                        "{}x{} {:?}",
                        thumbnail.texture.width(),
                        thumbnail.texture.height(),
                        thumbnail.texture.format(),
                    ));
                    if ui.button("save").clicked() {
                        if let Err(err) = save_image(
                            &thumbnail.texture.to_image(),
                            Some(&change_extension(&self.filename, "png")),
                        ) {
                            log::error!("Failed to save image: {err}");
                        }
                    }
                });
            });
        }

        if frames.num_frames() > 1 {
            egui::DragValue::new(&mut self.current_frame)
                .range(0..=(frames.num_frames() - 1))
                .ui(ui);
        }

        let mut views = Vec::new();
        for mipmap in 0..frames.num_mipmaps() {
            views.push(
                state
                    .textures
                    .get(&VtfTextureIndex {
                        mipmap,
                        frame: self.current_frame,
                    })
                    .unwrap(),
            );
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            views.into_iter().for_each(|view| {
                ui.add(
                    egui::Image::new(view.handle.source.clone())
                        .max_width(ui.available_width())
                        .sense(egui::Sense::CLICK),
                )
                .context_menu(|ui| {
                    ui.label(format!(
                        "{}x{} {:?}",
                        view.texture.width(),
                        view.texture.height(),
                        view.texture.format(),
                    ));
                    if ui.button("save").clicked() {
                        if let Err(err) = save_image(
                            &view.texture.to_image(),
                            Some(&change_extension(&self.filename, "png")),
                        ) {
                            log::error!("Failed to save image: {err}");
                        }
                    }
                });
            });
        });
    }
}
