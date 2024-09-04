
use std::{fs::File, io::{BufReader, Read, Seek}, path::PathBuf};
use crate::{app::Explorer, app_util};
use anyhow::Result;
use image::DynamicImage;
use uuid::Uuid;



pub struct ImageExplorer {
    name: Option<String>,
    uuid: Uuid,
    image: DynamicImage,
    texture: Option<egui::TextureHandle>,
}

impl ImageExplorer {
    pub fn new(image: DynamicImage, name: Option<String>) -> ImageExplorer {
        ImageExplorer {
            name,
            uuid: Uuid::now_v7(),
            image,
            texture: None,
        }
    }

    pub fn file<F: Read + Seek>(mut file: F, filename: Option<String>) -> Result<ImageExplorer> {
        file.rewind()?;

        let mut decoder = image::ImageReader::new(BufReader::new(file));
        // if let Some(filename) = &filename {
        //     decoder.set_format(image::ImageFormat::from_path(filename)?);
        // } else {
        //     decoder = decoder.with_guessed_format()?;
        // }
        decoder = decoder.with_guessed_format()?;

        let image = decoder.decode()?;

        Ok(ImageExplorer::new(
            image,
            filename.map(|f| util::file_utils::filename(&f)).flatten(),
        ))
    }

    pub fn open<P: Into<PathBuf>>(path: P) -> Result<ImageExplorer> {
        let path: PathBuf = path.into();
        ImageExplorer::file(
            File::open(&path)?,
            util::file_utils::filename(&path),
        )
    }
}

impl Explorer for ImageExplorer {
    fn uuid(&self) -> &Uuid {
        &self.uuid
    }

    fn name(&mut self) -> String {
        self.name.clone().unwrap_or("Image".to_owned())
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if self.texture.is_none() {
            self.texture = Some(app_util::image_utils::image_egui_handle(&self.image, ui.ctx()));
        }
        let texture = self.texture.as_ref().unwrap();
        ui.add_sized(
            ui.available_size(),
            egui::Image::new(egui::ImageSource::Texture(egui::load::SizedTexture::from_handle(&texture))).shrink_to_fit()
        ).context_menu(|ui| {
            if ui.button("Save Image").clicked() {
                app_util::image_utils::save_image(
                    &self.image,
                    self.name.clone(),
                ).expect("Failed to save image");
            }
        });
    }
}


