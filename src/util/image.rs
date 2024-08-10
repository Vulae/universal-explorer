
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
    pub fn test(&self, width: u32, height: u32) -> bool {
        match self {
            SizeHint::SizeBoth(hint_width, hint_height) => width <= *hint_width && height <= *hint_height,
            SizeHint::SizeEither(hint_width, hint_height) => width <= *hint_width || height <= *hint_height,
            SizeHint::Pixels(hint_pixels) => (width as u64) * (height as u64) <= *hint_pixels,
        }
    }
}


