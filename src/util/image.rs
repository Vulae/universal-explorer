
use std::path::PathBuf;

use egui::{ColorImage, Context, TextureFilter, TextureHandle, TextureOptions};
use image::DynamicImage;
use rfd::FileDialog;
use uuid::Uuid;
use anyhow::Result;



pub fn image_egui_handle(image: &DynamicImage, ctx: &Context) -> TextureHandle {
    let pixels_buf = image.to_rgba8();
    let pixels: image::FlatSamples<&[u8]> = pixels_buf.as_flat_samples();
    let image = ColorImage::from_rgba_unmultiplied([ image.width() as _, image.height() as _ ], pixels.as_slice());

    let mut options = TextureOptions::default();
    if (image.width() * image.height()) <= 4096 {
        options.magnification = TextureFilter::Nearest;
    }

    ctx.load_texture(Uuid::now_v7(), image, options)
}



pub fn filename_hint<P: Into<PathBuf>>(path: Option<P>) -> Option<String> {
    match path {
        Some(path) => {
            let path: PathBuf = path.into();
            match path.file_stem() {
                Some(stem) => stem.to_str().map(|s| s.to_string()),
                _ => None,
            }
        },
        _ => None,
    }
}

/// Returns the file location if file was saved.
pub fn save_image(image: &DynamicImage, filename: Option<String>) -> Result<Option<PathBuf>> {
    let mut dialog = FileDialog::new()
        .set_title("Save Image")
        .add_filter("image/png", &["png"])
        .add_filter("image/jpeg", &["jpg", "jpeg"])
        .add_filter("image/webp", &["webp"])
        .add_filter("image/bmp", &["bmp"])
        .add_filter("image/tiff", &["tif", "tiff"])
        .add_filter("image/x-targa", &["tga"])
        .add_filter("image/x-exr", &["exr"]);
    
    if let Some(filename) = filename_hint(filename) {
        dialog = dialog.set_file_name(filename);
    }

    if let Some(path) = dialog.save_file() {
        image.save(&path)?;
        Ok(Some(path))
    } else {
        Ok(None)
    }
}
