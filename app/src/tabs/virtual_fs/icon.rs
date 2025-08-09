use util_vfs::{VirtualFileSystem, VirtualFileSystemPath};

use crate::{
    assets,
    egui_util::{image_handle, ImageSourceWithTextureHandle},
    loader::{entry_icon, LoaderImage},
};

const ENTRY_THUMBNAIL_WIDTH: u32 = 192;
const ENTRY_THUMBNAIL_HEIGHT: u32 = 192;

#[derive(Debug)]
pub enum EntryIcon {
    ImageSource(egui::ImageSource<'static>),
    ImageSourceWithTextureHandle(ImageSourceWithTextureHandle<'static>),
}

impl EntryIcon {
    pub fn image_source(&self) -> &egui::ImageSource {
        match self {
            EntryIcon::ImageSource(image_source) => image_source,
            EntryIcon::ImageSourceWithTextureHandle(image_source_with_texture_handle) => {
                &image_source_with_texture_handle.source
            }
        }
    }

    pub fn load<P: Into<VirtualFileSystemPath>>(
        fs: &VirtualFileSystem,
        path: P,
        ctx: &egui::Context,
    ) -> Self {
        let path: VirtualFileSystemPath = path.into();

        let image = match entry_icon(fs, &path, ENTRY_THUMBNAIL_WIDTH, ENTRY_THUMBNAIL_HEIGHT) {
            Ok(image) => image,
            Err(err) => {
                log::error!("Error while loading thumbnail: {err}");
                return EntryIcon::ImageSource(assets::NOTEXTURE.to_owned());
            }
        };

        match image {
            None => EntryIcon::ImageSource(if path.is_directory() {
                assets::LUCIDE_FOLDER.to_owned()
            } else {
                assets::LUCIDE_FILE.to_owned()
            }),
            Some(LoaderImage::Image(image)) => {
                let image = if image.width() > ENTRY_THUMBNAIL_WIDTH
                    || image.height() > ENTRY_THUMBNAIL_HEIGHT
                {
                    image.resize(
                        ENTRY_THUMBNAIL_WIDTH,
                        ENTRY_THUMBNAIL_HEIGHT,
                        image::imageops::FilterType::Triangle,
                    )
                } else {
                    image
                };
                let handle = image_handle(image, ctx);
                let source =
                    egui::ImageSource::Texture(egui::load::SizedTexture::from_handle(&handle));
                EntryIcon::ImageSourceWithTextureHandle(ImageSourceWithTextureHandle {
                    source,
                    handle,
                })
            }
            Some(LoaderImage::Source(image_source)) => EntryIcon::ImageSource(image_source),
        }
    }
}
