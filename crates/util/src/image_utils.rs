use image::DynamicImage;
use std::path::PathBuf;

pub fn filename_hint<P: Into<PathBuf>>(path: Option<P>) -> Option<String> {
    match path {
        Some(path) => {
            let path: PathBuf = path.into();
            match path.file_stem() {
                Some(stem) => stem.to_str().map(|s| s.to_string()),
                _ => None,
            }
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SizeHint {
    SizeBoth(u32, u32),
    SizeEither(u32, u32),
    Pixels(u64),
}

impl SizeHint {
    /// If width & height satisfies this SizeHint's constraints.
    pub fn satisfies(&self, width: u32, height: u32) -> bool {
        match self {
            SizeHint::SizeBoth(hint_width, hint_height) => {
                width <= *hint_width && height <= *hint_height
            }
            SizeHint::SizeEither(hint_width, hint_height) => {
                width <= *hint_width || height <= *hint_height
            }
            SizeHint::Pixels(hint_pixels) => (width as u64) * (height as u64) <= *hint_pixels,
        }
    }

    /// Rescale width & height to biggest size of constraints.
    ///
    /// Will keep aspect ratio, but not perfectly due to integer imprecision.
    pub fn rescale(&self, width: u32, height: u32) -> (u32, u32) {
        match self {
            SizeHint::SizeBoth(max_width, max_height) => {
                let aspect_ratio = width as f64 / height as f64;
                if width > *max_width || height > *max_height {
                    if aspect_ratio > *max_width as f64 / *max_height as f64 {
                        (*max_width, (*max_width as f64 / aspect_ratio) as u32)
                    } else {
                        ((*max_height as f64 * aspect_ratio) as u32, *max_height)
                    }
                } else {
                    (width, height)
                }
            }
            SizeHint::SizeEither(max_width, max_height) => {
                let aspect_ratio = width as f64 / height as f64;
                if width > *max_width {
                    (*max_width, (*max_width as f64 / aspect_ratio) as u32)
                } else if height > *max_height {
                    ((*max_height as f64 * aspect_ratio) as u32, *max_height)
                } else {
                    (width, height)
                }
            }
            SizeHint::Pixels(max_pixels) => {
                let current_pixels = (width as u64) * (height as u64);
                if current_pixels > *max_pixels {
                    let scale_factor = (*max_pixels as f64 / current_pixels as f64).sqrt();
                    (
                        (width as f64 * scale_factor) as u32,
                        (height as f64 * scale_factor) as u32,
                    )
                } else {
                    (width, height)
                }
            }
        }
    }

    /// Downscale image to SizeHint::rescale size.
    ///
    /// If size is already smaller than the new downscale size, will return original image.
    pub fn downscale_image(
        &self,
        image: DynamicImage,
        filter: image::imageops::FilterType,
    ) -> DynamicImage {
        let (new_width, new_height) = self.rescale(image.width(), image.height());
        if new_width < image.width() || new_height < image.height() {
            image.resize_exact(new_width, new_height, filter)
        } else {
            image
        }
    }
}
