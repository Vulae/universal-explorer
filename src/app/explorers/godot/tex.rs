
use std::{fs::File, io::{Read, Seek}, path::PathBuf};
use crate::app::{explorers::image::ImageExplorer, Explorer};
use anyhow::Result;
use image::DynamicImage;
use uuid::Uuid;



pub struct GodotTexExplorer {
    explorer: ImageExplorer,
}

impl GodotTexExplorer {
    pub fn new(image: DynamicImage, name: Option<String>) -> Self {
        Self { explorer: ImageExplorer::new(image, name) }
    }

    pub fn file<F: Read + Seek>(mut file: F, filename: Option<String>) -> Result<Self> {
        file.rewind()?;
        let image = crate::explorers::godot::tex::godot_extract_texture(file)?;
        Ok(Self::new(
            image,
            filename.map(|f| crate::util::file::filename(&f)).flatten(),
        ))
    }

    pub fn open<P: Into<PathBuf>>(path: P) -> Result<Self> {
        let path: PathBuf = path.into();
        Self::file(
            &mut File::open(&path)?,
            crate::util::file::filename(&path),
        )
    }
}

impl Explorer for GodotTexExplorer {
    fn uuid(&self) -> Uuid {
        self.explorer.uuid()
    }

    fn name(&mut self) -> String {
        self.explorer.name()
    }

    fn ui(&mut self, ui: &mut egui::Ui) -> Result<()> {
        self.explorer.ui(ui)
    }
}


