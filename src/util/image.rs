
use std::path::PathBuf;
use egui::{Color32, ColorImage, Context, Pos2, Rect, TextureFilter, TextureHandle, TextureOptions, Vec2, Widget};
use image::DynamicImage;
use rfd::FileDialog;
use uuid::Uuid;
use anyhow::Result;



pub fn image_egui_handle(image: &DynamicImage, ctx: &Context) -> TextureHandle {
    // TODO: Probably want to make my own texture loader because the built in one iterates over every pixel to transform them, Which is not necessary I think.
    let image = match image {
        DynamicImage::ImageRgba8(rgba8) => {
            ColorImage::from_rgba_unmultiplied(
            [ rgba8.width() as usize, rgba8.height() as usize ],
            rgba8.as_flat_samples().as_slice(),
        )},
        image => ColorImage::from_rgba_unmultiplied(
            [ image.width() as usize, image.height() as usize ],
            image.to_rgba8().as_flat_samples().as_slice(),
        ),
    };

    let mut options = TextureOptions::default();
    if (image.width() * image.height()) <= 4096 {
        options.magnification = TextureFilter::Nearest;
    }

    ctx.load_texture(Uuid::now_v7(), image, options)
}



/// This sucks! Don't use until it is better.
pub struct EguiTransparentImage<'a> {
    image: egui::Image<'a>,
    sense: egui::Sense,
}

impl<'a> EguiTransparentImage<'a> {
    pub fn new(image: egui::Image<'a>, sense: egui::Sense) -> EguiTransparentImage {
        EguiTransparentImage { image, sense }
    }
}

impl<'a> Widget for EguiTransparentImage<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let tlr = self.image.load_for_size(ui.ctx(), ui.available_size());
        let original_image_size = tlr.as_ref().ok().and_then(|t| t.size());
        let ui_size = self.image.calc_size(ui.available_size(), original_image_size);

        let (rect, response) = ui.allocate_exact_size(ui_size, self.sense);
        if ui.is_rect_visible(rect) {
            let mut child = ui.child_ui(rect, ui.layout().clone(), None);
            
            // Create the checkered background
            // ui.image(egui::include_image!("../../assets/transparent.png"));
            let painter = child.painter();
            let checkered_size = 32.0;
            let rows = (rect.height() / checkered_size).ceil() as usize;
            let cols = (rect.width() / checkered_size).ceil() as usize;

            for row in 0..rows {
                for col in 0..cols {
                    let color = if (row + col) % 2 == 0 { Color32::LIGHT_GRAY } else { Color32::WHITE };
                    let top_left = Pos2::new(rect.left() + col as f32 * checkered_size, rect.top() + row as f32 * checkered_size);
                    let bottom_right = (top_left + Vec2::new(checkered_size, checkered_size)).min(rect.max);
                    painter.rect_filled(Rect::from_two_pos(top_left, bottom_right), 0.0, color);
                }
            }

            self.image.ui(&mut child);
        }
        response
    }
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
            SizeHint::SizeBoth(hint_width, hint_height) => width <= *hint_width && height <= *hint_height,
            SizeHint::SizeEither(hint_width, hint_height) => width <= *hint_width || height <= *hint_height,
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
    pub fn downscale_image(&self, image: DynamicImage, filter: image::imageops::FilterType) -> DynamicImage {
        let (new_width, new_height) = self.rescale(image.width(), image.height());
        if new_width < image.width() || new_height < image.height() {
            image.resize_exact(new_width, new_height, filter)
        } else {
            image
        }
    }
}


