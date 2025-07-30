use std::{fmt::Debug, io::Seek as _};

use util_vfs::VirtualFileSystemFile;

pub struct ImageSourceWithTextureHandle<'a> {
    pub source: egui::ImageSource<'a>,
    #[allow(unused)]
    pub handle: egui::TextureHandle,
}

impl Debug for ImageSourceWithTextureHandle<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageSourceWithTextureHandle")
            .field("source", &self.source)
            .field("handle", &"TextureHandle")
            .finish()
    }
}

impl ImageSourceWithTextureHandle<'static> {
    pub fn from_image(image: image::DynamicImage, ctx: &egui::Context) -> Self {
        let handle = image_handle(image, ctx);
        let source = egui::ImageSource::Texture(egui::load::SizedTexture::from_handle(&handle));
        Self { source, handle }
    }
}

pub fn image_handle(image: image::DynamicImage, ctx: &egui::Context) -> egui::TextureHandle {
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.into_rgba8().into_flat_samples().as_slice(),
    );
    let mut options = egui::TextureOptions::default();
    if image.width() * image.height() <= 96 * 96 {
        options.magnification = egui::TextureFilter::Nearest;
    }
    ctx.load_texture(uuid::Uuid::now_v7(), image, options)
}

pub fn load_image(
    file: &mut VirtualFileSystemFile,
) -> Result<image::DynamicImage, image::ImageError> {
    let Some(format) = file
        .path()
        .extension()
        .and_then(image::ImageFormat::from_extension)
    else {
        return image::ImageReader::new(std::io::BufReader::new(file))
            .with_guessed_format()?
            .decode();
    };
    let mut reader = image::ImageReader::new(std::io::BufReader::new(&mut *file));
    reader.set_format(format);
    match reader.decode() {
        Err(err) => {
            log::error!("Failed to load image, retrying read with auto format: {err}",);
            file.rewind()?;
            Ok(image::ImageReader::new(std::io::BufReader::new(file))
                .with_guessed_format()?
                .decode()?)
        }
        Ok(image) => Ok(image),
    }
}
