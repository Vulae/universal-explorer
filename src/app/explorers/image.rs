
use std::{fs::File, io::{BufReader, Read, Seek}, path::PathBuf};
use crate::app::Explorer;
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

    pub fn file<F: Read + Seek>(file: F, filename: Option<String>) -> Result<ImageExplorer> {
        let image: DynamicImage = match &filename {
            Some(filename) => image::ImageReader::with_format(
                BufReader::new(file),
                image::ImageFormat::from_path(filename)?,
            ).decode()?,
            None => image::ImageReader::new(BufReader::new(file))
                .with_guessed_format()?
                .decode()?,
        };
        Ok(ImageExplorer::new(image, filename))
    }

    pub fn open<P: Into<PathBuf>>(path: P) -> Result<ImageExplorer> {
        let path: PathBuf = path.into();
        ImageExplorer::file(
            File::open(&path)?,
            path.file_name().map(|s| s.to_str().map(|s| s.to_owned())).flatten(),
        )
    }
}

impl Explorer for ImageExplorer {
    fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn name(&mut self) -> String {
        self.name.clone().unwrap_or("Image".to_owned())
    }

    fn update(&mut self, ui: &mut egui::Ui) -> Result<()> {
        let texture = self.texture.get_or_insert_with(|| {
            crate::util::image::image_egui_handle(&self.image, ui.ctx())
        });
        ui.add_sized(
            ui.available_size(),
            egui::Image::new(egui::ImageSource::Texture(egui::load::SizedTexture::from_handle(&texture))).shrink_to_fit()
        ).context_menu(|ui| {
            if ui.button("Save Image").clicked() {
                crate::util::image::save_image(
                    &self.image,
                    self.name.clone(),
                ).expect("Failed to save image");
            }
        });
        Ok(())
    }
}


