
use std::io::{Read, Seek};
use image::{DynamicImage, GrayAlphaImage, GrayImage, ImageBuffer, LumaA, Pixel, Rgb, RgbImage, Rgba, RgbaImage};
use anyhow::{anyhow, Result};
use bitflags::bitflags;
use crate::util::image::SizeHint;



bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct TextureFlags: u32 {
        // Flags from the *.txt config file
        const POINTSAMPLE = 0x00000001;
        const TRILINEAR = 0x00000002;
        const CLAMPS = 0x00000004;
        const CLAMPT = 0x00000008;
        const ANISOTROPIC = 0x00000010;
        const HINT_DXT5 = 0x00000020;
        const PWL_CORRECTED = 0x00000040;
        const NORMAL = 0x00000080;
        const NOMIP = 0x00000100;
        const NOLOD = 0x00000200;
        const ALL_MIPS = 0x00000400;
        const PROCEDURAL = 0x00000800;

        // These are automatically generated by vtex from the texture data.
        const ONEBITALPHA = 0x00001000;
        const EIGHTBITALPHA = 0x00002000;

        // Newer flags from the *.txt config file
        const ENVMAP = 0x00004000;
        const RENDERTARGET = 0x00008000;
        const DEPTHRENDERTARGET = 0x00010000;
        const NODEBUGOVERRIDE = 0x00020000;
        const SINGLECOPY	= 0x00040000;
        const PRE_SRGB = 0x00080000;
        
        const UNUSED_00100000 = 0x00100000;
        const UNUSED_00200000 = 0x00200000;
        const UNUSED_00400000 = 0x00400000;

        const NODEPTHBUFFER = 0x00800000;

        const UNUSED_01000000 = 0x01000000;

        const CLAMPU = 0x02000000;
        const VERTEXTEXTURE = 0x04000000;
        const SSBUMP = 0x08000000;

        const UNUSED_10000000 = 0x10000000;

        const BORDER = 0x20000000;

        const UNUSED_40000000 = 0x40000000;
        const UNUSED_80000000 = 0x80000000;
    }
}



#[derive(Debug, Clone, Copy)]
#[repr(i32)]
#[allow(non_camel_case_types)]
pub enum TextureFormat {
    NONE = -1, // TODO: Probably want to do something different with NONE format.
	RGBA8888 = 0,
	ABGR8888 = 1,
	RGB888 = 2,
	BGR888 = 3,
	RGB565 = 4,
	I8 = 5,
	IA88 = 6,
	P8 = 7, // ?????
	A8 = 8,
	RGB888_BLUESCREEN = 9,
	BGR888_BLUESCREEN = 10,
	ARGB8888 = 11,
	BGRA8888 = 12,
	DXT1 = 13,
	DXT3 = 14,
	DXT5 = 15,
	BGRX8888 = 16,
	BGR565 = 17,
	BGRX5551 = 18,
	BGRA4444 = 19,
	DXT1_ONEBITALPHA = 20,
	BGRA5551 = 21,
	UV88 = 22,
	UVWQ8888 = 23,
	RGBA16161616F = 24,
	RGBA16161616 = 25,
	UVLX8888 = 26,
}

impl TextureFormat {
    fn try_from(value: i32) -> Result<TextureFormat> {
        if value < (TextureFormat::NONE as i32) || value > (TextureFormat::UVLX8888 as i32) {
            return Err(anyhow!("Texture with format invalid {}", value));
        }
        Ok(unsafe { std::mem::transmute(value) })
    }

    /// Fix DXT1 & DXT1_ONEBITALPHA format.
    fn fix(&self, _flags: &TextureFlags) -> TextureFormat {
        // This doesn't make any sense, why is it always DXT1_ONEBITALPHA?????????
        match self {
            TextureFormat::DXT1 | TextureFormat::DXT1_ONEBITALPHA => {
                // if flags.intersects(TextureFlags::ONEBITALPHA) {
                //     TextureFormat::DXT1_ONEBITALPHA
                // } else {
                //     TextureFormat::DXT1
                // }
                TextureFormat::DXT1_ONEBITALPHA
            },
            format => *format,
        }
    }

    pub fn texture_byte_size(&self, width: u32, height: u32) -> u64 {
        let width = width as u64;
        let height = height as u64;
        match self {
            TextureFormat::NONE => 0,
            TextureFormat::RGBA8888 => width * height * 4,
            TextureFormat::ABGR8888 => width * height * 4,
            TextureFormat::RGB888 => width * height * 3,
            TextureFormat::BGR888 => width * height * 3,
            TextureFormat::RGB565 => width * height * 2,
            TextureFormat::I8 => width * height,
            TextureFormat::IA88 => width * height * 2,
            TextureFormat::P8 => 256 * 4 + width * height, // ?????
            TextureFormat::A8 => width * height,
            TextureFormat::RGB888_BLUESCREEN => width * height * 3,
            TextureFormat::BGR888_BLUESCREEN => width * height * 3,
            TextureFormat::ARGB8888 => width * height * 4,
            TextureFormat::BGRA8888 => width * height * 4,
            TextureFormat::DXT1 => width.div_ceil(4) * height.div_ceil(4) * 8,
            TextureFormat::DXT3 => width.div_ceil(4) * height.div_ceil(4) * 16,
            TextureFormat::DXT5 => width.div_ceil(4) * height.div_ceil(4) * 16,
            TextureFormat::BGRX8888 => width * height * 4,
            TextureFormat::BGR565 => width * height * 2,
            TextureFormat::BGRX5551 => width * height * 2,
            TextureFormat::BGRA4444 => width * height * 2,
            TextureFormat::DXT1_ONEBITALPHA => width.div_ceil(4) * height.div_ceil(4) * 8,
            TextureFormat::BGRA5551 => width * height * 2,
            TextureFormat::UV88 => width * height * 2,
            TextureFormat::UVWQ8888 => width * height * 4,
            TextureFormat::RGBA16161616F => width * height * 8,
            TextureFormat::RGBA16161616 => width * height * 8,
            TextureFormat::UVLX8888 => width * height * 4,
        }
    }
}



#[derive(Debug, Clone)]
pub struct VtfTexture {
    width: u32,
    height: u32,
    format: TextureFormat,
    data: Vec<u8>,
}

impl VtfTexture {
    pub fn new(width: u32, height: u32, format: TextureFormat, data: &[u8]) -> VtfTexture {
        let expected_bytes = format.texture_byte_size(width, height) as usize;
        if data.len() != expected_bytes {
            panic!("VTF texture created with invalid data buffer length. {:?} {}x{} {} bytes != {} buffer bytes", format, width, height, expected_bytes, data.len());
        }
        VtfTexture { width, height, format, data: Vec::from(data) }
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
    pub fn format(&self) -> TextureFormat { self.format }



    pub fn to_image(&self) -> DynamicImage {
        
        // The unsupported image is just temporary for until all image formats are supported.
        fn unsupported_format_image() -> DynamicImage {
            image::load_from_memory(include_bytes!("./unsupported_format.png")).unwrap()
        }

        fn swizzle_image<const S: usize, T, P, F>(data: &[u8], width: u32, height: u32, mut swizzle: F) -> ImageBuffer<P, Vec<T>>
        where
            T: image::Primitive,
            P: Pixel<Subpixel = T> + 'static,
            F: FnMut(&[u8; S]) -> P,
        {
            let mut image = ImageBuffer::<P, Vec<T>>::new(width, height);
            let mut offset: usize = 0;
            // TODO: Rayon
            image.pixels_mut().for_each(|pixel| {
                let color = swizzle(&data[offset..(offset + S)].try_into().unwrap());
                *pixel = color;
                offset += S;
            });
            image
        }

        // TODO: Generalize with generics
        // TODO: Use const generics
        #[inline]
        fn extract(v: u16, offset: u8, length: u8) -> u8 {
            #[inline]
            fn expand(v: u8, source_bits: u8) -> u8 {
                // TODO: Probably just want to hard code each source_bits possibility for best performance.

                // let shift_amount = 8 - source_bits;
                // let mut result = ((v << shift_amount) & 0xFF) as u8;
                // for _ in 0..shift_amount {
                //     result |= result >> source_bits;
                // }
                // result
                let mut dest: u8 = 0;
                let mut dest_bits: u8 = 8;
                while dest_bits >= source_bits {
                    dest <<= source_bits;
                    dest |= v;
                    dest_bits -= source_bits;
                }
                if dest_bits > 0 {
                    let temp = v >> (source_bits - dest_bits);
                    dest <<= dest_bits;
                    dest |= temp;
                }
                dest
            }

            let mask: u16 = ((1 << length) - 1) << offset;
            let value: u8 = ((v & mask) >> offset) as u8;
            expand(value, length)
        }

        #[inline]
        fn bluescreen(c: Rgb<u8>) -> Rgba<u8> {
            if c == Rgb([ 0, 0, 255 ]) {
                Rgba([ 0, 0, 0, 0 ])
            } else {
                c.to_rgba()
            }
        }

        match self.format {
            TextureFormat::NONE => unsupported_format_image(),
            TextureFormat::RGBA8888 => DynamicImage::ImageRgba8(RgbaImage::from_raw(self.width, self.height, self.data.clone()).unwrap()),
            TextureFormat::ABGR8888 => DynamicImage::ImageRgba8(swizzle_image(&self.data, self.width, self.height, |c: &[u8; 4]| Rgba([ c[3], c[2], c[1], c[0] ]))),
            TextureFormat::RGB888 => DynamicImage::ImageRgb8(RgbImage::from_raw(self.width, self.height, self.data.clone()).unwrap()),
            TextureFormat::BGR888 => DynamicImage::ImageRgb8(swizzle_image(&self.data, self.width, self.height, |c: &[u8; 3]| Rgb([ c[2], c[1], c[0] ]))),
            TextureFormat::RGB565 => DynamicImage::ImageRgb8(
                swizzle_image(&self.data, self.width, self.height, |c: &[u8; 2]| {
                    let v = u16::from_le_bytes(*c);
                    Rgb([ extract(v, 0, 5), extract(v, 5, 6), extract(v, 11, 5) ])
                })
            ),
            TextureFormat::I8 => DynamicImage::ImageLuma8(GrayImage::from_raw(self.width, self.height, self.data.clone()).unwrap()),
            TextureFormat::IA88 => DynamicImage::ImageLumaA8(GrayAlphaImage::from_raw(self.width, self.height, self.data.clone()).unwrap()),
            TextureFormat::P8 => unsupported_format_image(), // ?????
            // There's no such thing as ImageA8, So we just use ImageLumaA8 & set Luma to be 0.
            TextureFormat::A8 => DynamicImage::ImageLumaA8(swizzle_image(&self.data, self.width, self.height, |c: &[u8; 1]| LumaA([ 0, c[0] ]))),
            TextureFormat::RGB888_BLUESCREEN => DynamicImage::ImageRgba8(swizzle_image(&self.data, self.width, self.height, |c: &[u8; 3]| bluescreen(Rgb([ c[0], c[1], c[2] ])))),
            TextureFormat::BGR888_BLUESCREEN => DynamicImage::ImageRgba8(swizzle_image(&self.data, self.width, self.height, |c: &[u8; 3]| bluescreen(Rgb([ c[2], c[1], c[0] ])))),
            TextureFormat::ARGB8888 => DynamicImage::ImageRgba8(swizzle_image(&self.data, self.width, self.height, |c: &[u8; 4]| Rgba([ c[3], c[0], c[1], c[2] ]))),
            // FIXME: This may also be an HDR texture based on texture flags.
            TextureFormat::BGRA8888 => DynamicImage::ImageRgba8(swizzle_image(&self.data, self.width, self.height, |c: &[u8; 4]| Rgba([ c[2], c[1], c[0], c[3] ]))),
            TextureFormat::DXT1 => DynamicImage::ImageRgba8(crate::util::texture::bc::decode_bc1(&self.data, self.width(), self.height(), image::Rgba([ 0, 0, 0, 255 ]))),
            TextureFormat::DXT3 => DynamicImage::ImageRgba8(crate::util::texture::bc::decode_bc2(&self.data, self.width(), self.height())),
            TextureFormat::DXT5 => DynamicImage::ImageRgba8(crate::util::texture::bc::decode_bc3(&self.data, self.width(), self.height())),
            TextureFormat::BGRX8888 => DynamicImage::ImageRgb8({
                let mut malformed = false;
                swizzle_image(&self.data, self.width, self.height, |c: &[u8; 4]| {
                    if !malformed && c[3] != 0 {
                        malformed = true;
                        println!("BGRX8888 is malformed, has alpha channel data.");
                    }
                    Rgb([ c[2], c[1], c[0] ])
                })
            }),
            TextureFormat::BGR565 => DynamicImage::ImageRgb8(
                swizzle_image(&self.data, self.width, self.height, |c: &[u8; 2]| {
                    let v = u16::from_le_bytes(*c);
                    Rgb([ extract(v, 11, 5), extract(v, 5, 6), extract(v, 0, 5) ])
                })
            ),
            TextureFormat::BGRX5551 => DynamicImage::ImageRgb8({
                let mut malformed = false;
                swizzle_image(&self.data, self.width, self.height, |c: &[u8; 2]| {
                    let v = u16::from_le_bytes(*c);
                    if !malformed && extract(v, 15, 1) != 0 {
                        malformed = true;
                        println!("BGRX5551 is malformed, has alpha channel data.");
                    }
                    Rgb([ extract(v, 10, 5), extract(v, 5, 5), extract(v, 0, 5) ])
                })
            }),
            TextureFormat::BGRA4444 => DynamicImage::ImageRgba8(
                swizzle_image(&self.data, self.width, self.height, |c: &[u8; 2]| {
                    let v = u16::from_le_bytes(*c);
                    Rgba([ extract(v, 8, 4), extract(v, 4, 4), extract(v, 0, 4), extract(v, 12, 4) ])
                })
            ),
            TextureFormat::DXT1_ONEBITALPHA => DynamicImage::ImageRgba8(crate::util::texture::bc::decode_bc1(&self.data, self.width(), self.height(), image::Rgba([ 0, 0, 0, 0 ]))),
            TextureFormat::BGRA5551 => DynamicImage::ImageRgba8(
                swizzle_image(&self.data, self.width, self.height, |c: &[u8; 2]| {
                    let v = u16::from_le_bytes(*c);
                    Rgba([ extract(v, 10, 5), extract(v, 5, 5), extract(v, 0, 5), extract(v, 15, 1) ])
                })
            ),
            TextureFormat::UV88 => DynamicImage::ImageRgb8(swizzle_image(&self.data, self.width, self.height, |c: &[u8; 2]| Rgb([ c[0], c[1], 0 ]))),
            TextureFormat::UVWQ8888 =>  DynamicImage::ImageRgba8(RgbaImage::from_raw(self.width, self.height, self.data.clone()).unwrap()),
            TextureFormat::RGBA16161616F => DynamicImage::ImageRgba16( // Is this supposed to be the same as RGBA16161616 ???
                swizzle_image(&self.data, self.width, self.height, |c: &[u8; 8]| {
                    Rgba([ u16::from_le_bytes([ c[0], c[1] ]), u16::from_le_bytes([ c[2], c[3] ]), u16::from_le_bytes([ c[4], c[5] ]), u16::from_le_bytes([ c[6], c[7] ]) ])
                })
            ),
            TextureFormat::RGBA16161616 => DynamicImage::ImageRgba16(
                swizzle_image(&self.data, self.width, self.height, |c: &[u8; 8]| {
                    Rgba([ u16::from_le_bytes([ c[0], c[1] ]), u16::from_le_bytes([ c[2], c[3] ]), u16::from_le_bytes([ c[4], c[5] ]), u16::from_le_bytes([ c[6], c[7] ]) ])
                })
            ),
            TextureFormat::UVLX8888 => DynamicImage::ImageRgba8(RgbaImage::from_raw(self.width, self.height, self.data.clone()).unwrap()), // TODO: Probably warn for malformed because X component.
        }
    }
}



#[derive(Debug, Clone)]
pub struct Vtf {
    thumbnail: Option<VtfTexture>,
    format: TextureFormat,
    width: u32,
    height: u32,
    mipmaps: u8,
    frames: u16,
    first_frame: u16,
    faces: u8,
    slices: u16,
    textures: Vec<VtfTexture>,
}

impl Vtf {

    /// May have different format than Vtf::format()
    pub fn thumbnail(&self) -> Option<&VtfTexture> { self.thumbnail.as_ref() }
    pub fn format(&self) -> TextureFormat { self.format }
    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
    pub fn mipmaps(&self) -> u32 { self.mipmaps as u32 }
    pub fn frames(&self) -> u32 { self.frames as u32 }
    pub fn faces(&self) -> u32 { self.faces as u32 }
    pub fn slices(&self) -> u32 { self.slices as u32 }

    pub fn texture_index(&self, mipmap: u32, frame: u32, face: u32, slice: u32) -> Option<usize> {
        if mipmap >= self.mipmaps() || frame >= self.frames() || face >= self.faces() || slice >= self.slices() {
            return None;
        }

        let mipmap = (self.mipmaps() as usize) - 1 - (mipmap as usize);
        let frame = ((frame + (self.first_frame as u32)) % self.frames()) as usize;
        let face = face as usize;
        let slice = slice as usize;

        Some(
            slice +
            face * (self.slices() as usize) +
            frame * (self.faces() as usize) * (self.slices() as usize) +
            mipmap * (self.frames() as usize) * (self.faces() as usize) * (self.slices() as usize)
        )
    }

    pub fn total_num_textures(&self) -> usize {
        (self.mipmaps() as usize) * (self.frames() as usize) * (self.faces() as usize) * (self.slices() as usize)
    }

    pub fn texture(&self, mipmap: u32, frame: u32, face: u32, slice: u32) -> Option<&VtfTexture> {
        self.textures.get(self.texture_index(mipmap, frame, face, slice)?)
    }

    fn into_texture(self, mipmap: u32, frame: u32, face: u32, slice: u32) -> Option<VtfTexture> {
        let index = self.texture_index(mipmap, frame, face, slice)?;
        self.textures.into_iter().nth(index)
    }



    pub fn load(mut data: impl Read + Seek) -> Result<Vtf> {
        data.rewind()?;
        let header = VtfHeader::load(&mut data)?;

        let mut thumbnail = None;
        if let Some(lowres_offset) = header.lowres_offset {
            data.seek(std::io::SeekFrom::Start(lowres_offset as u64))?;
            thumbnail = Vtf::read_thumbnail(&mut data, header.lowres_format, header.lowres_width as u32, header.lowres_height as u32)?;
        }
        data.seek(std::io::SeekFrom::Start(header.highres_offset as u64))?;
        let textures = Vtf::read_textures(&mut data, header.highres_format, header.width as u32, header.height as u32, header.mipmaps as u32, header.frames as u32, header.faces as u32, header.slices as u32)?;

        Ok(Vtf {
            format: header.highres_format,
            width: header.width as u32,
            height: header.height as u32,
            thumbnail,
            mipmaps: header.mipmaps,
            frames: header.frames,
            first_frame: header.first_frame,
            faces: header.faces,
            slices: header.slices,
            textures,
        })
    }

    fn read_texture(mut data: impl Read, format: TextureFormat, width: u32, height: u32) -> Result<VtfTexture> {
        let size = format.texture_byte_size(width, height);
        let mut buf = vec![0u8; size as usize];
        data.read(&mut buf)?;
        Ok(VtfTexture::new(width, height, format, &buf))
    }

    fn read_thumbnail(data: impl Read, format: TextureFormat, width: u32, height: u32) -> Result<Option<VtfTexture>> {
        let size = format.texture_byte_size(width, height);
        if size > 0 {
            Ok(Some(Vtf::read_texture(data, format, width, height)?))
        } else { Ok(None) }
    }

    fn read_textures(
        mut data: impl Read,
        format: TextureFormat,
        width: u32, height: u32,
        mipmaps: u32, frames: u32, faces: u32, slices: u32,
    ) -> Result<Vec<VtfTexture>> {
        let mut textures: Vec<VtfTexture> = Vec::new();
        for mipmap in (0..mipmaps).rev() {
            let mip_width = (width >> mipmap).max(1);
            let mip_height = (height >> mipmap).max(1);
            for _frame in 0..frames {
                for _face in 0..faces {
                    for _slice in 0..slices {
                        textures.push(Vtf::read_texture(&mut data, format, mip_width, mip_height)?);
                    }
                }
            }
        }
        Ok(textures)
    }

    fn read_specific_texture(
        mut data: impl Read + Seek,
        format: TextureFormat,
        width: u32, height: u32,
        mipmaps: u32, frames: u32, faces: u32, slices: u32,
        target_mipmap: u32, target_frame: u32, target_face: u32, target_slice: u32,
    ) -> Result<VtfTexture> {
        let mut offset: u64 = 0;
        for mipmap in (0..mipmaps).rev() {
            let mip_width = (width >> mipmap).max(1);
            let mip_height = (height >> mipmap).max(1);
            for frame in 0..frames {
                for face in 0..faces {
                    for slice in 0..slices {
                        let size = format.texture_byte_size(mip_width, mip_height);

                        if mipmap != target_mipmap || frame != target_frame || face != target_face || slice != target_slice {
                            offset += size;
                            continue;
                        }
                        
                        data.seek_relative(offset as i64)?;
                        return Ok(Vtf::read_texture(&mut data, format, mip_width, mip_height)?);
                    }
                }
            }
        }
        Err(anyhow!("Failed to read specific texture"))
    }



    pub fn load_thumbnail(mut data: impl Read + Seek, hint: SizeHint) -> Result<Option<VtfTexture>> {
        data.rewind()?;
        if let Ok(header) = VtfHeader::load(&mut data) {

            let mut mipmap = 0;
            while mipmap < header.mipmaps {
                if hint.satisfies((header.width as u32) >> mipmap, (header.height as u32) >> mipmap) {
                    if mipmap > 0 {
                        // Go to previous mipmap so scaling is a bit more clean.
                        mipmap -= 1;
                    }
                    break;
                }
                mipmap += 1;
            }

            data.seek(std::io::SeekFrom::Start(header.highres_offset as u64))?;
            Ok(Some(Vtf::read_specific_texture(
                data,
                header.highres_format,
                header.width as u32, header.height as u32,
                header.mipmaps as u32, header.frames as u32, header.faces as u32, header.slices as u32,
                mipmap as u32, 0, 0, 0,
            )?))
        } else {
            Ok(None)
        }
    }
}





enum VtfResource {
    Unknown([u8; 3], u8, u32),
    LowRes(u32),
    HighRes(u32),
}

impl VtfResource {
    pub fn lowres_offset(&self) -> Option<u32> {
        if let VtfResource::LowRes(offset) = self {
            Some(*offset)
        } else {
            None
        }
    }
    pub fn highres_offset(&self) -> Option<u32> {
        if let VtfResource::HighRes(offset) = self {
            Some(*offset)
        } else {
            None
        }
    }
}



struct VtfHeader {
    version: [u32; 2],
    header_size: u32,
    width: u16,
    height: u16,
    flags: TextureFlags,
    frames: u16,
    first_frame: u16,
    reflectivity: [f32; 3],
    bumpmap_scale: f32,
    highres_format: TextureFormat,
    mipmaps: u8,
    faces: u8,
    lowres_format: TextureFormat,
    lowres_width: u8,
    lowres_height: u8,
    slices: u16,

    is_resource_format: bool,
    resources: Vec<VtfResource>,
    highres_offset: u32,
    lowres_offset: Option<u32>,
}

impl VtfHeader {
    pub fn load(data: impl Read) -> Result<VtfHeader> {
        let mut reader = crate::util::reader::Reader::new_le(data);

        if &reader.read::<[u8; 4]>()? != b"VTF\0" {
            return Err(anyhow!("Invalid VTF identifier"));
        }
        let version = reader.read::<[u32; 2]>()?;
        let header_size = reader.read::<u32>()?;
        let width = reader.read::<u16>()?;
        let height = reader.read::<u16>()?;
        let flags = TextureFlags::from_bits_retain(reader.read::<u32>()?);
        let frames = reader.read::<u16>()?;
        let mut first_frame = reader.read::<u16>()?;
        reader.skip(4)?;
        let reflectivity = reader.read::<[f32; 3]>()?;
        reader.skip(4)?;
        let bumpmap_scale = reader.read::<f32>()?;
        let highres_format = TextureFormat::try_from(reader.read::<i32>()?)?.fix(&flags);
        let mipmaps = reader.read::<u8>()?;
        let faces = if flags.intersects(TextureFlags::ENVMAP) {
            if version < [7, 5] && first_frame == 0xFFFF {
                first_frame = 0;
                7
            } else { 6 }
        } else { 1 };
        let lowres_format = TextureFormat::try_from(reader.read::<i32>()?)?.fix(&flags);
        let lowres_width = reader.read::<u8>()?;
        let lowres_height = reader.read::<u8>()?;
        let slices: u16 = if version > [7, 2] { reader.read()? } else { 1 };

        let is_resource_format: bool;
        let mut resources: Vec<VtfResource> = Vec::new();
        let highres_offset: u32;
        let mut lowres_offset: Option<u32> = None;
        if version < [7, 3] {
            is_resource_format = false;

            let mut offset = header_size;

            let lowres_size = lowres_format.texture_byte_size(lowres_width as u32, lowres_height as u32);
            if lowres_size > 0 {
                lowres_offset = Some(offset);
                offset += lowres_size as u32;
            }

            highres_offset = offset;

        } else {
            is_resource_format = true;

            reader.skip(3)?;
            let num_resources = reader.read::<u32>()?;
            reader.skip(8)?;

            for _ in 0..num_resources {
                let tag = reader.read::<[u8; 3]>()?;
                let flags = reader.read::<u8>()?;
                let offset = reader.read::<u32>()?;

                resources.push(match &tag {
                    b"\x01\0\0" => VtfResource::LowRes(offset),
                    b"\x30\0\0" => VtfResource::HighRes(offset),
                    _ => VtfResource::Unknown(tag, flags, offset),
                });
            }

            if let Some(resource_lowres) = resources.iter().find(|r| r.lowres_offset().is_some()) {
                lowres_offset = resource_lowres.lowres_offset();
            }

            let resource_highres = resources.iter().find(|r| r.highres_offset().is_some()).ok_or(anyhow!("VTF texture does not have highres image data."))?;
            highres_offset = resource_highres.highres_offset().unwrap();
        }

        Ok(VtfHeader {
            version,
            header_size,
            width,
            height,
            flags,
            frames,
            first_frame,
            reflectivity,
            bumpmap_scale,
            highres_format,
            mipmaps,
            faces,
            lowres_format,
            lowres_width,
            lowres_height,
            slices,
            is_resource_format,
            resources,
            highres_offset,
            lowres_offset,
        })
    }
}


