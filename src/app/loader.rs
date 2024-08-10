
use std::{fs::File, io::{Read, Seek}, path::{Path, PathBuf}};
use super::{Explorer, SharedAppContext};
use anyhow::{anyhow, Result};
use crate::{app::explorers, util::image::SizeHint};



pub fn open_file<F: Read + Seek>(_app_context: SharedAppContext, mut file: F, filename: Option<String>) -> Result<Option<Box<dyn Explorer>>> {
    // FIXME: Do not clone filename.
    if let Ok(explorer) = explorers::image::ImageExplorer::file(&mut file, filename.clone()) {
        return Ok(Some(Box::new(explorer)));
    }
    if let Ok(explorer) = explorers::source_engine::vtf::VtfExplorer::file(&mut file, filename.clone()) {
        return Ok(Some(Box::new(explorer)));
    }
    if let Ok(explorer) = explorers::text::TextExplorer::file(&mut file, filename.clone()) {
        return Ok(Some(Box::new(explorer)));
    }

    Ok(None)
}

pub fn open<P: AsRef<Path>>(app_context: SharedAppContext, path: P) -> Result<Option<Box<dyn Explorer>>> {
    let path: PathBuf = path.as_ref().into();

    if !path.try_exists()? {
        return Err(anyhow!("Failed to open path."));
    }

    if path.is_file() {
        if let Ok(explorer) = explorers::source_engine::vpk::VpkExplorer::open(app_context.clone(), &path) {
            return Ok(Some(Box::new(explorer)));
        }
        if let Ok(explorer) = explorers::renpy::rpa::RenPyArchiveExplorer::open(app_context.clone(), &path) {
            return Ok(Some(Box::new(explorer)));
        }

        open_file(
            app_context,
            File::open(&path)?,
            crate::util::filename(&path),
        )?;
    }

    Ok(None)
}



pub fn thumbnail_file(file: impl Read + Seek, filename: Option<String>, ctx: &egui::Context, hint: SizeHint) -> Option<egui::ImageSource<'static>> {

    if let Some(filename) = &filename {
        if let Ok(_) = image::ImageFormat::from_path(filename) {
            return Some(crate::app::assets::LUCIDE_FILE_IMAGE);
        }

        if filename.ends_with(".vtf") {
            if let Some(texture) = crate::explorers::source_engine::vtf::Vtf::load_thumbnail(file, hint) {
                let image = texture.to_image();
                let handle = crate::util::image::image_egui_handle(&image, ctx);
                let source = egui::ImageSource::Texture(egui::load::SizedTexture::from_handle(&handle));

                // FIXME: Don't do this!
                // Texture handle gets dropped after returned, causing texture to immediately go poof.
                // So I just leak the texture so it never goes poof.
                std::mem::forget(handle);

                return Some(source);
            } else {
                return Some(crate::app::assets::LUCIDE_FILE_IMAGE);
            }
        }

    }

    None
}


