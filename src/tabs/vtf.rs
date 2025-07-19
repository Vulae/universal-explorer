use std::{collections::HashMap, fmt::Debug};

use egui::Widget as _;
use source_engine::{Vtf, VtfContainer};

use crate::egui_util::ImageSourceWithTextureHandle;

use super::TabTrait;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct VtfTextureIndex {
    mipmap: u8,
    frame: u16,
}

#[derive(Debug)]
pub struct VtfTab {
    name: String,
    vtf: Vtf,
    thumbnail_texture: Option<ImageSourceWithTextureHandle<'static>>,
    textures: HashMap<VtfTextureIndex, ImageSourceWithTextureHandle<'static>>,
    current_frame: u16,
}

impl VtfTab {
    pub fn new(name: String, vtf: Vtf) -> Self {
        Self {
            name,
            vtf,
            thumbnail_texture: None,
            textures: HashMap::new(),
            current_frame: 0,
        }
    }

    fn get_texture_source(
        &mut self,
        ctx: &egui::Context,
        index: VtfTextureIndex,
    ) -> &egui::ImageSource<'static> {
        &self
            .textures
            .entry(index)
            .or_insert_with(|| {
                let VtfContainer::Frames(frames) = self.vtf.container() else {
                    unimplemented!();
                };

                ImageSourceWithTextureHandle::from_image(
                    frames
                        .texture(index.mipmap, index.frame)
                        .unwrap()
                        .to_image(),
                    ctx,
                )
            })
            .source
    }
}

impl TabTrait for VtfTab {
    fn name(&self) -> egui::WidgetText {
        self.name.as_str().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
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

        if let Some(thumbnail) = self.vtf.lowres() {
            let source = &self
                .thumbnail_texture
                .get_or_insert_with(|| {
                    ImageSourceWithTextureHandle::from_image(thumbnail.to_image(), ui.ctx())
                })
                .source;
            ui.horizontal(|ui| {
                let rect = ui.label("thumbnail: ").rect;
                ui.add(
                    egui::Image::new(source.clone())
                        .fit_to_exact_size(egui::Vec2::INFINITY)
                        .max_height(rect.height()),
                );
            });
        }

        if frames.num_frames() > 1 {
            egui::DragValue::new(&mut self.current_frame)
                .range(0..=(frames.num_frames() - 1))
                .ui(ui);
        }

        let mut view_sources = Vec::new();
        for mipmap in 0..frames.num_mipmaps() {
            view_sources.push(
                self.get_texture_source(
                    ui.ctx(),
                    VtfTextureIndex {
                        mipmap,
                        frame: self.current_frame,
                    },
                )
                .clone(),
            );
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            view_sources.into_iter().for_each(|source| {
                ui.add(egui::Image::new(source).max_width(ui.available_width()));
            });
        });
    }
}
