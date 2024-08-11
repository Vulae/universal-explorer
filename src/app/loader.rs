
use std::{fs::File, io::{Read, Seek}, path::{Path, PathBuf}};
use super::{Explorer, SharedAppContext};
use anyhow::{anyhow, Result};
use crate::{app::explorers, util::{file::FileSize, image::SizeHint}};



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
            crate::util::file::filename(&path),
        )?;
    }

    Ok(None)
}





fn image_source(image: image::DynamicImage, ctx: &egui::Context, hint: SizeHint) -> (egui::ImageSource<'static>, Option<egui::TextureHandle>) {
    let image = hint.downscale_image(image, image::imageops::FilterType::Nearest);
    let handle = crate::util::image::image_egui_handle(&image, ctx);
    let source = egui::ImageSource::Texture(egui::load::SizedTexture::from_handle(&handle));
    (source, Some(handle))
}

#[cfg_attr(debug_assertions, allow(unused))]
pub fn thumbnail_file(mut file: impl Read + Seek, filename: Option<String>, ctx: &egui::Context, hint: SizeHint) -> Option<(egui::ImageSource<'static>, Option<egui::TextureHandle>)> {
    let file_size = FileSize::from_file(&mut file).ok()?;

    if let Some(filename) = &filename {
        if let Ok(format) = image::ImageFormat::from_path(filename) {
            // #[cfg(not(debug_assertions))] // Decoding images is VERY slow on debug build.
            if file_size < FileSize::from_mebibytes(3) {
                if let Ok(image) = image::ImageReader::with_format(std::io::BufReader::new(&mut file), format).decode() {
                    return Some(image_source(image, ctx, hint));
                }
            }

            return Some((crate::app::assets::LUCIDE_FILE_IMAGE, None));
        }

        if filename.ends_with(".vtf") {
            if let Some(texture) = crate::explorers::source_engine::vtf::Vtf::load_thumbnail(file, hint) {
                return Some(image_source(texture.to_image(), ctx, hint));
            }

            return Some((crate::app::assets::LUCIDE_FILE_IMAGE, None));
        }

    }

    None
}


