use util::{VirtualFileSystem, VirtualFileSystemPath};

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

    if source_engine::may_be_vtf_file(&path)
        && source_engine::is_vtf_file(&path, fs.open_file(&path)?)
    {
        let vtf = source_engine::Vtf::load(fs.open_file(&path)?)?;
        return Ok(Some(Tab::new(Box::new(tabs::VtfTab::new(
            path.name()
                .map(|v| v.to_owned())
                .unwrap_or(path.to_string()),
            vtf,
        )))));
    }

    if source_engine::is_vpk_file(&path) {
        let vpk = source_engine::VPKArchiveFiles::locate_archives(fs, &path)?.load()?;
        return Ok(Some(Tab::new(Box::new(tabs::VirtualFsTab::new(
            path.name()
                .map(|v| v.to_owned())
                .unwrap_or(path.to_string()),
            VirtualFileSystem::new(Box::new(vpk)),
        )))));
    }

    if path.is_file() {
        return Ok(Some(Tab::new(Box::new(tabs::HexTab::new(
            path.name()
                .map(|v| v.to_owned())
                .unwrap_or(path.to_string()),
            fs.open_file(&path)?,
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
    fs: &mut VirtualFileSystem,
    path: P,
    target_size: (u32, u32),
) -> Result<Option<LoaderImage>, anyhow::Error> {
    let path: VirtualFileSystemPath = path.into();
    let (target_width, target_height) = target_size;

    if path
        .extension()
        .and_then(image::ImageFormat::from_extension)
        .is_some()
    {
        let image = load_image(&mut fs.open_file(&path)?)?;
        let image = image.resize(
            target_width,
            target_height,
            image::imageops::FilterType::Triangle,
        );
        return Ok(Some(LoaderImage::Image(image)));
    }

    if source_engine::may_be_vtf_file(&path)
        && source_engine::is_vtf_file(&path, fs.open_file(&path)?)
    {
        let vtf = source_engine::Vtf::load(fs.open_file(&path)?)?;
        let image = vtf.texture_best().to_image();
        let image = image.resize(
            target_width,
            target_height,
            image::imageops::FilterType::Triangle,
        );
        return Ok(Some(LoaderImage::Image(image)));
    }

    if source_engine::is_vpk_file(&path) {
        return Ok(Some(LoaderImage::Source(
            assets::LUCIDE_FILE_ARCHIVE.to_owned(),
        )));
    }

    Ok(None)
}
