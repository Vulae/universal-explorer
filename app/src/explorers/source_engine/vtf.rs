
use std::{fs::File, io::{Read, Seek}, path::PathBuf};
use anyhow::Result;
use source_engine::vtf::Vtf;
use uuid::Uuid;

use crate::{app::Explorer, app_util};



#[derive(PartialEq, Debug, Clone, Copy)]
enum RenderedTextureType {
    Texture(u32, u32, u32, u32),
    Thumbnail,
}



pub struct VtfExplorer {
    name: Option<String>,
    uuid: Uuid,

    vtf: Vtf,
    mipmap: u32,
    frame: u32,
    face: u32,
    slice: u32,

    thumbnail: Option<egui::TextureHandle>,
    textures: Vec<Option<egui::TextureHandle>>,
}

impl VtfExplorer {
    pub fn new(vtf: Vtf, name: Option<String>) -> VtfExplorer {
        VtfExplorer {
            name,
            uuid: Uuid::now_v7(),
            textures: vec![None; vtf.total_num_textures()],
            vtf,
            mipmap: 0,
            frame: 0,
            face: 0,
            slice: 0,
            thumbnail: None,
        }
    }
    
    pub fn file<F: Read + Seek>(mut file: F, filename: Option<String>) -> Result<VtfExplorer> {
        file.rewind()?;
        Ok(VtfExplorer::new(
            Vtf::load(file)?,
            filename.map(|f| util::file_utils::filename(&f)).flatten()
        ))
    }

    pub fn open<P: Into<PathBuf>>(path: P) -> Result<VtfExplorer> {
        let path: PathBuf = path.into();
        VtfExplorer::file(
            File::open(&path)?,
            util::file_utils::filename(&path),
        )
    }
}

impl Explorer for VtfExplorer {
    fn uuid(&self) -> &Uuid {
        &self.uuid
    }

    fn title(&self) -> String {
        self.name.clone().unwrap_or("VTF Texture".to_owned())
    }

    fn ui(&mut self, ui: &mut egui::Ui) {

        app_util::splitter::Splitter::horizontal(self.uuid).min_size(240.0).show(ui, |ui_a, ui_b| {

            ui_a.vertical(|ui| {
                ui.label("VTF Information");
                ui.label(format!("Format: {:?}", self.vtf.format()));
                ui.label(format!("Size: {}x{}", self.vtf.width(), self.vtf.height()));
    
                if self.vtf.total_num_textures() > 1 {
                    ui.add_space(32.0);
                }
    
                ui.horizontal(|ui| {
                    if self.vtf.mipmaps() > 1 {
                        ui.menu_button(format!("Mipmap {}", self.mipmap), |ui| {
                            for mipmap in 0..self.vtf.mipmaps() {
                                if ui.button(format!("Mipmap {}", mipmap)).clicked() {
                                    self.mipmap = mipmap;
                                }
                            }
                        });
                    }
                    if self.vtf.faces() > 1 {
                        ui.menu_button(format!("Face {}", self.face), |ui| {
                            for face in 0..self.vtf.faces() {
                                if ui.button(format!("Face {}", face)).clicked() {
                                    self.face = face;
                                }
                            }
                        });
                    }
                    if self.vtf.slices() > 1 {
                        ui.menu_button(format!("Slice {}", self.slice), |ui| {
                            for slice in 0..self.vtf.slices() {
                                if ui.button(format!("Slice {}", slice)).clicked() {
                                    self.slice = slice;
                                }
                            }
                        });
                    }
                });
    
                if self.vtf.frames() > 1 {
                    ui.add(egui::Slider::new(&mut self.frame, 0..=(self.vtf.frames() - 1)).text("Frame"));
                }
    
                if let Some(thumbnail) = self.vtf.thumbnail() {
                    let thumbnail_source = self.thumbnail.get_or_insert_with(|| {
                        app_util::image_utils::image_egui_handle(&thumbnail.to_image(), ui.ctx())
                    });
    
                    ui.add_space(32.0);
                    ui.horizontal(|ui| {
                        ui.image(egui::ImageSource::Texture(egui::load::SizedTexture::from_handle(thumbnail_source)));
                        ui.label("Thumbnail");
                    });
                    ui.label(format!("Format: {:?}", thumbnail.format()));
                    ui.label(format!("Size: {}x{}", thumbnail.width(), thumbnail.height()));
                }
            });

            if let Some(texture_handle_index) = self.vtf.texture_index(self.mipmap, self.frame, self.face, self.slice) {
                if let Some(texture) = self.vtf.texture(self.mipmap, self.frame, self.face, self.slice) {
                    let texture_handle = self.textures[texture_handle_index].get_or_insert_with(|| {
                        app_util::image_utils::image_egui_handle(&texture.to_image(), ui_b.ctx())
                    });
                    ui_b.add_sized(
                        ui_b.available_size(),
                        egui::Image::new(egui::ImageSource::Texture(egui::load::SizedTexture::from_handle(&texture_handle))).shrink_to_fit(),
                    ).context_menu(|ui| {
                        if ui.button("Save Texture").clicked() {
                            app_util::image_utils::save_image(
                                &texture.to_image(),
                                self.name.clone().map(|filename| filename.trim_end_matches(".vtf").to_owned()),
                            ).expect("Failed to save VTF image");
                        }
                    });
                }
            }

        });
    }
}


