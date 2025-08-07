use std::io::{Read, Seek};

use util_general::is_likely_utf8;
use util_vfs::{VirtualFileSystem, VirtualFileSystemFile, VirtualFileSystemPath};

use crate::{
    assets,
    egui_util::load_image,
    tabs::{self, Tab},
};

pub fn try_open_tab_from_fs_and_path<P: Into<VirtualFileSystemPath>>(
    fs: &mut VirtualFileSystem,
    path: P,
) -> Result<Option<Tab>, anyhow::Error> {
    let path: VirtualFileSystemPath = path.into();

    if let Some(_format) = path
        .extension()
        .and_then(image::ImageFormat::from_extension)
    {
        return Ok(Some(Tab::new(Box::new(tabs::ImageTab::new(
            "Image".to_owned(),
            fs.open_file(path)?,
        )))));
    }

    #[cfg(feature = "archive")]
    if util_archive::is_zip_file(&path) {
        let zip = util_archive::Zip::load(fs.open_file(&path)?)?;
        return Ok(Some(Tab::new(Box::new(tabs::VirtualFsTab::new(
            path.name()
                .map(|v| v.to_owned())
                .unwrap_or(path.to_string()),
            VirtualFileSystem::new(Box::new(zip)),
        )))));
    }

    #[cfg(feature = "source_engine")]
    if source_engine::may_be_vtf_file(&path) {
        let mut file = fs.open_file(&path)?;
        if source_engine::is_vtf_file(&path, &mut file) {
            let vtf = source_engine::Vtf::load(file)?;
            let filename = path.name().unwrap();
            return Ok(Some(Tab::new(Box::new(tabs::VtfTab::new(
                filename.to_owned(),
                filename.to_owned(),
                vtf,
            )))));
        }
    }

    #[cfg(feature = "source_engine")]
    if source_engine::is_vpk_file(&path) {
        let vpk = source_engine::VPKArchiveFiles::locate_archives(fs, &path)?.load()?;
        return Ok(Some(Tab::new(Box::new(tabs::VirtualFsTab::new(
            path.name()
                .map(|v| v.to_owned())
                .unwrap_or(path.to_string()),
            VirtualFileSystem::new(Box::new(vpk)),
        )))));
    }

    #[cfg(all(feature = "source_engine", feature = "archive"))]
    if source_engine::is_bsp_file(&path) {
        if let Some(bsp) = source_engine::bsp_open_pakfile(fs.open_file(&path)?)? {
            let zip = util_archive::Zip::load(VirtualFileSystemFile::__debug_new_from_trait(
                Box::new(bsp),
            ))?;
            return Ok(Some(Tab::new(Box::new(tabs::VirtualFsTab::new(
                path.name()
                    .map(|v| v.to_owned())
                    .unwrap_or(path.to_string()),
                VirtualFileSystem::new(Box::new(zip)),
            )))));
        }
    }

    #[cfg(feature = "renpy")]
    if renpy::is_rpa_file(&path) {
        let rpa = renpy::RenPyArchive::new(fs.open_file(&path)?)?;
        return Ok(Some(Tab::new(Box::new(tabs::VirtualFsTab::new(
            path.name()
                .map(|v| v.to_owned())
                .unwrap_or(path.to_string()),
            VirtualFileSystem::new(Box::new(rpa)),
        )))));
    }

    if path.is_file() {
        let mut file = fs.open_file(&path)?;

        let mut preview = [0u8; 64];
        let preview_length = file.read(&mut preview)?;
        file.rewind()?;

        if is_likely_utf8(&preview[0..preview_length]) {
            let mut full = Vec::new();
            file.read_to_end(&mut full)?;
            match String::from_utf8(full) {
                Ok(string) => {
                    return Ok(Some(Tab::new(Box::new(tabs::TextTab::new(
                        path.name()
                            .map(|v| v.to_owned())
                            .unwrap_or(path.to_string()),
                        string,
                    )))));
                }
                Err(err) => {
                    log::warn!("Text file detection guessed wrong: {err}");
                    file.rewind()?;
                }
            }
        }

        return Ok(Some(Tab::new(Box::new(tabs::HexTab::new(
            path.name()
                .map(|v| v.to_owned())
                .unwrap_or(path.to_string()),
            file,
        )))));
    }

    Ok(None)
}

#[derive(Debug)]
pub enum LoaderImage {
    Image(image::DynamicImage),
    Source(egui::ImageSource<'static>),
}

pub fn entry_icon<P: Into<VirtualFileSystemPath>>(
    fs: &VirtualFileSystem,
    path: P,
    target_width: u32,
    target_height: u32,
) -> Result<Option<LoaderImage>, anyhow::Error> {
    let path: VirtualFileSystemPath = path.into();

    if path
        .extension()
        .and_then(image::ImageFormat::from_extension)
        .is_some()
    {
        let image = load_image(&mut fs.open_file(&path)?)?;
        return Ok(Some(LoaderImage::Image(image)));
    }

    #[cfg(feature = "archive")]
    if util_archive::is_zip_file(&path) {
        return Ok(Some(LoaderImage::Source(
            assets::LUCIDE_FILE_ARCHIVE.to_owned(),
        )));
    }

    #[cfg(feature = "source_engine")]
    if source_engine::may_be_vtf_file(&path) {
        let mut file = fs.open_file(&path)?;
        if source_engine::is_vtf_file(&path, &mut file) {
            // let vtf = source_engine::Vtf::load(file)?;
            // let texture = vtf.texture_best();
            let texture =
                source_engine::Vtf::load_single_texture(file, target_width, target_height)?;
            let image = texture.to_image();
            return Ok(Some(LoaderImage::Image(image)));
        }
    }

    #[cfg(feature = "source_engine")]
    if source_engine::is_vpk_file(&path) {
        return Ok(Some(LoaderImage::Source(
            assets::LUCIDE_FILE_ARCHIVE.to_owned(),
        )));
    }

    #[cfg(feature = "renpy")]
    if renpy::is_rpa_file(&path) {
        return Ok(Some(LoaderImage::Source(
            assets::LUCIDE_FILE_ARCHIVE.to_owned(),
        )));
    }

    Ok(None)
}
