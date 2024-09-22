use crate::{
    app::{Explorer, SharedAppContext},
    assets, explorers,
};
use anyhow::{anyhow, Result};
use std::{
    fs::File,
    io::{Read, Seek},
    path::{Path, PathBuf},
};
use util::{
    file_utils::{self, FileSize},
    image_utils::SizeHint,
};

pub fn open_file<F: Read + Seek>(
    _app_context: SharedAppContext,
    mut file: F,
    filename: Option<String>,
) -> Result<Option<Box<dyn Explorer>>> {
    // FIXME: Do not clone filename.
    if let Ok(explorer) = explorers::image::ImageExplorer::file(&mut file, filename.clone()) {
        return Ok(Some(Box::new(explorer)));
    }
    #[cfg(feature = "source_engine")]
    if let Ok(explorer) =
        explorers::source_engine::vtf::VtfExplorer::file(&mut file, filename.clone())
    {
        return Ok(Some(Box::new(explorer)));
    }
    #[cfg(feature = "godot")]
    if let Ok(explorer) = explorers::godot::tex::GodotTexExplorer::file(&mut file, filename.clone())
    {
        return Ok(Some(Box::new(explorer)));
    }
    if let Ok(explorer) = explorers::text::TextExplorer::file(&mut file, filename.clone()) {
        return Ok(Some(Box::new(explorer)));
    }

    Ok(None)
}

pub fn open<P: AsRef<Path>>(
    app_context: SharedAppContext,
    path: P,
) -> Result<Option<Box<dyn Explorer>>> {
    let path: PathBuf = path.as_ref().into();

    if !path.try_exists()? {
        return Err(anyhow!("Failed to open path."));
    }

    if path.is_file() {
        #[cfg(feature = "source_engine")]
        if let Ok(explorer) =
            explorers::source_engine::vpk::VpkExplorer::open(app_context.clone(), &path)
        {
            return Ok(Some(Box::new(explorer)));
        }
        #[cfg(feature = "renpy")]
        if let Ok(explorer) =
            explorers::renpy::rpa::RenPyArchiveExplorer::open(app_context.clone(), &path)
        {
            return Ok(Some(Box::new(explorer)));
        }
        #[cfg(feature = "godot")]
        if let Ok(explorer) =
            explorers::godot::pck::GodotPckExplorer::open(app_context.clone(), &path)
        {
            return Ok(Some(Box::new(explorer)));
        }

        return Ok(open_file(
            app_context,
            File::open(&path)?,
            file_utils::filename(&path),
        )?);
    }

    Ok(None)
}

pub enum LoadedThumbnail {
    None,
    Image(image::DynamicImage),
    ImageSource(egui::ImageSource<'static>),
}

const DEFAULT_DOWNSCALE_FILTER: image::imageops::FilterType = image::imageops::FilterType::Nearest;
const MAX_THUMBNAIL_LOAD_FILESIZE: FileSize = FileSize::from_mebibytes(10);

pub fn thumbnail_file(
    mut file: impl Read + Seek,
    filename: Option<String>,
    hint: SizeHint,
) -> Result<LoadedThumbnail> {
    file.rewind()?;
    let file_size = FileSize::from_file(&mut file)?;

    if let Some(filename) = &filename {
        if let Ok(_) = image::ImageFormat::from_path(filename) {
            if file_size < MAX_THUMBNAIL_LOAD_FILESIZE {
                if let Ok(image) = image::ImageReader::new(std::io::BufReader::new(&mut file))
                    .with_guessed_format()?
                    .decode()
                {
                    return Ok(LoadedThumbnail::Image(
                        hint.downscale_image(image, DEFAULT_DOWNSCALE_FILTER),
                    ));
                }
            }

            return Ok(LoadedThumbnail::ImageSource(assets::LUCIDE_FILE_IMAGE));
        }

        #[cfg(feature = "godot")]
        if filename.ends_with(".stex") || filename.ends_with(".ctex") {
            if file_size < MAX_THUMBNAIL_LOAD_FILESIZE {
                if let Ok(image) = godot::tex::godot_extract_texture(&mut file) {
                    return Ok(LoadedThumbnail::Image(
                        hint.downscale_image(image, DEFAULT_DOWNSCALE_FILTER),
                    ));
                }
            }
            return Ok(LoadedThumbnail::ImageSource(assets::LUCIDE_FILE_IMAGE));
        }

        #[cfg(feature = "source_engine")]
        if filename.ends_with(".vtf") {
            if let Ok(Some(texture)) = source_engine::vtf::Vtf::load_thumbnail(file, hint) {
                return Ok(LoadedThumbnail::Image(
                    hint.downscale_image(texture.to_image(), DEFAULT_DOWNSCALE_FILTER),
                ));
            }

            return Ok(LoadedThumbnail::ImageSource(assets::LUCIDE_FILE_IMAGE));
        }
    }

    Ok(LoadedThumbnail::None)
}
