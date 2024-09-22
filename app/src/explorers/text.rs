use crate::app::Explorer;
use anyhow::{anyhow, Result};
use std::{
    fs::File,
    io::{Read, Seek},
    path::PathBuf,
};
use uuid::Uuid;

fn is_text_file<F: Read + Seek>(file: &mut F) -> Result<bool> {
    let position = file.stream_position()?;
    file.rewind()?;
    let mut str = String::new();
    // TODO: Haven't tested this, but this may fail on multi-byte characters at the end of the sample data.
    let is_text_file = file.take(4096).read_to_string(&mut str).is_ok();
    file.seek(std::io::SeekFrom::Start(position))?;
    Ok(is_text_file)
}

pub struct TextExplorer {
    name: Option<String>,
    uuid: Uuid,
    text: String,
}

impl TextExplorer {
    pub fn new(text: String, name: Option<String>) -> TextExplorer {
        TextExplorer {
            name,
            uuid: Uuid::now_v7(),
            text,
        }
    }

    pub fn file<F: Read + Seek>(mut file: F, filename: Option<String>) -> Result<TextExplorer> {
        file.rewind()?;
        if !is_text_file(&mut file)? {
            return Err(anyhow!("File is not text file."));
        }
        let mut str = String::new();
        file.read_to_string(&mut str)?;
        Ok(TextExplorer::new(
            str,
            filename.map(|f| util::file_utils::filename(&f)).flatten(),
        ))
    }

    pub fn open<P: Into<PathBuf>>(path: P) -> Result<TextExplorer> {
        let path: PathBuf = path.into();
        TextExplorer::file(&mut File::open(&path)?, util::file_utils::filename(&path))
    }
}

impl Explorer for TextExplorer {
    fn uuid(&self) -> &Uuid {
        &self.uuid
    }

    fn title(&self) -> String {
        self.name.clone().unwrap_or("Text".to_owned())
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        // TODO: Don't use egui::TextEdit, this should not be editable.
        ui.add(
            egui::TextEdit::multiline(&mut self.text)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY),
        );
    }
}
