
use std::io::{Read, Seek};
use anyhow::{anyhow, Result};
use image::DynamicImage;
use bitflags::bitflags;



bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct DataFormatBits: u32 {
        const PNG = 1 << 20;
        const WEBP = 1 << 21;
        const STREAM = 1 << 22;
        const HAS_MIPMAPS = 1 << 23;
        const DETECT_3D = 1 << 24;
        const DETECT_SRGB = 1 << 25;
        const DETECT_NORMAL = 1 << 26;
    }
}

impl DataFormatBits {
    pub fn image_format(&self) -> Option<image::ImageFormat> {
        if self.contains(DataFormatBits::PNG | DataFormatBits::WEBP) {
            None
        } else if self.intersects(DataFormatBits::PNG) {
            Some(image::ImageFormat::Png)
        } else if self.intersects(DataFormatBits::WEBP) {
            Some(image::ImageFormat::WebP)
        } else {
            None
        }
    }
}



pub fn godot_extract_texture(mut file: impl Read + Seek) -> Result<DynamicImage> {
    file.rewind()?;
    let mut reader = crate::util::reader::Reader::new_le(file);

    match &reader.read::<[u8; 4]>()? {
        b"GDST" => {
            let _texture_width = reader.read::<u16>()?;
            let _image_width = reader.read::<u16>()?;
            let _texture_height = reader.read::<u16>()?;
            let _image_height = reader.read::<u16>()?;
            let _flags = reader.read::<u32>()?;
            let data_format = DataFormatBits::from_bits_retain(reader.read::<u32>()?);

            let image_format = data_format
                .image_format()
                .ok_or(anyhow!("Godot texture that isn't PNG or WEBP data format not supported."))?;

            if data_format.intersects(DataFormatBits::HAS_MIPMAPS) {
                println!("Godot texture with extra mipmaps ignored.");
            }

            let _mipmaps = reader.read::<u32>()?;

            // Read first mipmap
            let size = reader.read::<u32>()?;
            match image_format {
                image::ImageFormat::WebP => if &reader.read::<[u8; 4]>()? != b"WEBP" { return Err(anyhow!("Godot texture mipmap expected webp.")) },
                _ => return Err(anyhow!("Godot texture unsupported image format.")),
            }
            let data = reader.read_buf((size as usize) - 4)?;
            let image = image::load_from_memory_with_format(&data, image_format)?;
            Ok(image)
        },
        b"GD3T" => return Err(anyhow!("Godot 3d texture not supported.")),
        b"GDAT" => return Err(anyhow!("Godot array texture not supported.")),
        _ => return Err(anyhow!("File is not a texture file!")),
    }
}


