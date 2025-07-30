/// https://developer.valvesoftware.com/wiki/VTF_(Valve_Texture_Format)
use std::io::{Read, Seek};

use thiserror::Error;
use util_general::ReadExt as _;
use util_codec::{bcn::{decode_bc1, decode_bc2, decode_bc3}, decode_f16, decode_rgb565, upscale_lower_u8};
use util_vfs::VirtualFileSystemPath;

#[derive(Debug, Error)]
pub enum VtfError {
    #[error("VTF file has invalid identifier")]
    InvalidIdentifier,
    #[error("VTF file has invalid version {0:?}")]
    #[allow(private_interfaces)]
    InvalidVersion(VtfVersion),
    #[error("VTF invalid dimensions {0}x{1}")]
    InvalidDimensions(u16, u16),
    #[error("VTF must have atleast 1 frame")]
    NoFrames,
    #[error("VTF must have atleast 1 mipmap")]
    NoMipmaps,
    #[error("VTF has too many mipmaps, {1}x{2} with {0} mipmaps")]
    TooManyMipmaps(u8, u16, u16),
    #[error("VTF texture invalid format \"{0}\"")]
    InvalidFormat(u32),
    #[error("VTF 7.3+ requires atleast 1 resource")]
    NoResources,
    #[error("VTF 7.3+ no highres resource")]
    NoHighresResource,
    #[error("VTF feature not yet supported: {0}")]
    NotSupported(&'static str),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

const VTF_IDENTIFIER: [u8; 4] = *b"VTF\0";

/// Some VTF files don't have the .vtf extension and instead have .dat
/// So use source_engine::is_vtf_file to check if it is actually a VTF file.
pub fn may_be_vtf_file(path: &VirtualFileSystemPath) -> bool {
    path.is_file()
        && path
            .name()
            .map(|name| name.ends_with(".vtf") || name.ends_with(".dat"))
            .unwrap_or(false)
}

/// Some VTF files don't have the .vtf extension and instead have .dat
pub fn is_vtf_file<F: Read + Seek>(path: &VirtualFileSystemPath, mut file: F) -> bool {
    if !may_be_vtf_file(path) {
        return false;
    }
    let Some(name) = path.name() else {
        return false;
    };
    if name.ends_with(".vtf") {
        return true;
    }
    if name.ends_with(".dat") {
        if let Err(err) = file.rewind() {
            log::error!("Error while detecting VTF file {err}");
            return false;
        }
        match file.read_const::<4>() {
            Ok(VTF_IDENTIFIER) => return true,
            Ok(_) => return false,
            Err(err) => {
                log::error!("Error while detecting VTF file {err}");
                return false;
            }
        }
    }
    false
}

// const TEXTUREFLAGS_POINTSAMPLE: u32 = 0x00000001;
// const TEXTUREFLAGS_TRILINEAR: u32 = 0x00000002;
// const TEXTUREFLAGS_CLAMPS: u32 = 0x00000004;
// const TEXTUREFLAGS_CLAMPT: u32 = 0x00000008;
// const TEXTUREFLAGS_ANISOTROPIC: u32 = 0x00000010;
// const TEXTUREFLAGS_HINT_DXT5: u32 = 0x00000020;
// const TEXTUREFLAGS_PWL_CORRECTED: u32 = 0x00000040;
// const TEXTUREFLAGS_NORMAL: u32 = 0x00000080;
// const TEXTUREFLAGS_NOMIP: u32 = 0x00000100;
// const TEXTUREFLAGS_NOLOD: u32 = 0x00000200;
// const TEXTUREFLAGS_ALL_MIPS: u32 = 0x00000400;
// const TEXTUREFLAGS_PROCEDURAL: u32 = 0x00000800;
// const TEXTUREFLAGS_ONEBITALPHA: u32 = 0x00001000;
// const TEXTUREFLAGS_EIGHTBITALPHA: u32 = 0x00002000;
const TEXTUREFLAGS_ENVMAP: u32 = 0x00004000;
// const TEXTUREFLAGS_RENDERTARGET: u32 = 0x00008000;
// const TEXTUREFLAGS_DEPTHRENDERTARGET: u32 = 0x00010000;
// const TEXTUREFLAGS_NODEBUGOVERRIDE: u32 = 0x00020000;
// const TEXTUREFLAGS_SINGLECOPY: u32 = 0x00040000;
// const TEXTUREFLAGS_PRE_SRGB: u32 = 0x00080000;
// const TEXTUREFLAGS_UNUSED_00100000: u32 = 0x00100000;
// const TEXTUREFLAGS_UNUSED_00200000: u32 = 0x00200000;
// const TEXTUREFLAGS_UNUSED_00400000: u32 = 0x00400000;
// const TEXTUREFLAGS_NODEPTHBUFFER: u32 = 0x00800000;
// const TEXTUREFLAGS_UNUSED_01000000: u32 = 0x01000000;
// const TEXTUREFLAGS_CLAMPU: u32 = 0x02000000;
// const TEXTUREFLAGS_VERTEXTEXTURE: u32 = 0x04000000;
// const TEXTUREFLAGS_SSBUMP: u32 = 0x08000000;
// const TEXTUREFLAGS_UNUSED_10000000: u32 = 0x10000000;
// const TEXTUREFLAGS_BORDER: u32 = 0x20000000;
// const TEXTUREFLAGS_UNUSED_40000000: u32 = 0x40000000;
// const TEXTUREFLAGS_UNUSED_80000000: u32 = 0x80000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum VtfTextureFormat {
    RGBA8888,
    ABGR8888,
    RGB888,
    BGR888,
    RGB565,
    I8,
    IA88,
    // P8,
    A8,
    RGB888_BLUESCREEN,
    BGR888_BLUESCREEN,
    ARGB8888,
    BGRA8888,
    DXT1,
    DXT3,
    DXT5,
    BGRX8888,
    BGR565,
    BGRX5551,
    BGRA4444,
    DXT1_ONEBITALPHA,
    BGRA5551,
    UV88,
    UVWQ8888,
    RGBA16161616F,
    RGBA16161616,
    UVLX8888,
}

impl VtfTextureFormat {
    fn from_id(id: u32) -> Option<Self> {
        Some(match id {
            0 => Self::RGBA8888,
            1 => Self::ABGR8888,
            2 => Self::RGB888,
            3 => Self::BGR888,
            4 => Self::RGB565,
            5 => Self::I8,
            6 => Self::IA88,
            8 => Self::A8,
            9 => Self::RGB888_BLUESCREEN,
            10 => Self::BGR888_BLUESCREEN,
            11 => Self::ARGB8888,
            12 => Self::BGRA8888,
            13 => Self::DXT1,
            14 => Self::DXT3,
            15 => Self::DXT5,
            16 => Self::BGRX8888,
            17 => Self::BGR565,
            18 => Self::BGRX5551,
            19 => Self::BGRA4444,
            20 => Self::DXT1_ONEBITALPHA,
            21 => Self::BGRA5551,
            22 => Self::UV88,
            23 => Self::UVWQ8888,
            24 => Self::RGBA16161616F,
            25 => Self::RGBA16161616,
            26 => Self::UVLX8888,
            _ => return None,
        })
    }

    fn texture_byte_size(&self, width: u32, height: u32) -> usize {
        match self {
            Self::I8 | Self::A8 => (width as usize) * (height as usize),
            Self::RGB565
            | Self::IA88
            | Self::BGR565
            | Self::BGRX5551
            | Self::BGRA4444
            | Self::BGRA5551
            | Self::UV88 => (width as usize) * (height as usize) * 2,
            Self::RGB888 | Self::BGR888 | Self::RGB888_BLUESCREEN | Self::BGR888_BLUESCREEN => {
                (width as usize) * (height as usize) * 3
            }
            Self::RGBA8888
            | Self::ABGR8888
            | Self::ARGB8888
            | Self::BGRA8888
            | Self::BGRX8888
            | Self::UVWQ8888
            | Self::UVLX8888 => (width as usize) * (height as usize) * 4,
            Self::RGBA16161616 | Self::RGBA16161616F => (width as usize) * (height as usize) * 8,
            VtfTextureFormat::DXT1 | Self::DXT1_ONEBITALPHA => {
                (width.div_ceil(4) as usize) * (height.div_ceil(4) as usize) * 8
            }
            VtfTextureFormat::DXT3 | Self::DXT5 => {
                (width.div_ceil(4) as usize) * (height.div_ceil(4) as usize) * 16
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct VtfTexture {
    format: VtfTextureFormat,
    width: u32,
    height: u32,
    bytes: Box<[u8]>,
}

impl VtfTexture {
    pub fn format(&self) -> VtfTextureFormat {
        self.format
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn to_image(&self) -> image::DynamicImage {
        // TODO: All formats that have alpha (Except 1 bit alpha), maybe instead be HDR, using the alpha channel to
        // multiply color channels.
        // https://developer.valvesoftware.com/wiki/VTF_(Valve_Texture_Format)#HDR_compression
        // There seems to be no texture flags to detect this.

        assert_eq!(
            self.bytes.len(),
            self.format.texture_byte_size(self.width, self.height),
        );

        fn iter_pixel_sections<const N: usize>(
            buf: &[u8],
            width: u32,
            height: u32,
        ) -> impl Iterator<Item = ([u8; N], u32, u32)> {
            assert_eq!(buf.len(), (width as usize) * (height as usize) * N);
            let mut item = [0u8; N];
            buf.chunks(N).enumerate().map(move |(i, chunk)| {
                item.clone_from_slice(chunk);
                (
                    item,
                    (i % (width as usize)) as u32,
                    (i / (width as usize)) as u32,
                )
            })
        }

        match self.format {
            VtfTextureFormat::RGBA8888 => image::DynamicImage::from(
                image::RgbaImage::from_raw(self.width, self.height, self.bytes.to_vec()).unwrap(),
            ),
            VtfTextureFormat::ABGR8888 => {
                let mut image = image::RgbaImage::new(self.width, self.height);
                iter_pixel_sections(&self.bytes, self.width, self.height).for_each(
                    |([a, b, g, r], x, y)| {
                        image.get_pixel_mut(x, y).0 = [r, g, b, a];
                    },
                );
                image::DynamicImage::from(image)
            }
            VtfTextureFormat::RGB888 => image::DynamicImage::from(
                image::RgbImage::from_raw(self.width, self.height, self.bytes.to_vec()).unwrap(),
            ),
            VtfTextureFormat::BGR888 => {
                let mut image = image::RgbImage::new(self.width, self.height);
                iter_pixel_sections(&self.bytes, self.width, self.height).for_each(
                    |([b, g, r], x, y)| {
                        image.get_pixel_mut(x, y).0 = [r, g, b];
                    },
                );
                image::DynamicImage::from(image)
            }
            VtfTextureFormat::RGB565 => {
                let mut image = image::RgbImage::new(self.width, self.height);
                iter_pixel_sections(&self.bytes, self.width, self.height).for_each(|(p, x, y)| {
                    *image.get_pixel_mut(x, y) = decode_rgb565(u16::from_le_bytes(p));
                });
                image::DynamicImage::from(image)
            }
            VtfTextureFormat::I8 | VtfTextureFormat::A8 => image::DynamicImage::from(
                image::GrayImage::from_raw(self.width, self.height, self.bytes.to_vec()).unwrap(),
            ),
            VtfTextureFormat::IA88 => image::DynamicImage::from(
                image::GrayAlphaImage::from_raw(self.width, self.height, self.bytes.to_vec())
                    .unwrap(),
            ),
            VtfTextureFormat::RGB888_BLUESCREEN => {
                let mut image = image::RgbaImage::new(self.width, self.height);
                iter_pixel_sections(&self.bytes, self.width, self.height).for_each(
                    |([r, g, b], x, y)| {
                        image.get_pixel_mut(x, y).0 =
                            [r, g, b, if [r, g, b] == [0, 0, 255] { 0 } else { 255 }];
                    },
                );
                image::DynamicImage::from(image)
            }
            VtfTextureFormat::BGR888_BLUESCREEN => {
                let mut image = image::RgbaImage::new(self.width, self.height);
                iter_pixel_sections(&self.bytes, self.width, self.height).for_each(
                    |([b, g, r], x, y)| {
                        image.get_pixel_mut(x, y).0 =
                            [r, g, b, if [r, g, b] == [0, 0, 255] { 0 } else { 255 }];
                    },
                );
                image::DynamicImage::from(image)
            }
            VtfTextureFormat::ARGB8888 => {
                let mut image = image::RgbaImage::new(self.width, self.height);
                iter_pixel_sections(&self.bytes, self.width, self.height).for_each(
                    |([a, r, g, b], x, y)| {
                        image.get_pixel_mut(x, y).0 = [r, g, b, a];
                    },
                );
                image::DynamicImage::from(image)
            }
            VtfTextureFormat::BGRA8888 | VtfTextureFormat::BGRX8888 => {
                let mut image = image::RgbaImage::new(self.width, self.height);
                iter_pixel_sections(&self.bytes, self.width, self.height).for_each(
                    |([b, g, r, a], x, y)| {
                        image.get_pixel_mut(x, y).0 = [r, g, b, a];
                    },
                );
                image::DynamicImage::from(image)
            }
            VtfTextureFormat::DXT1 | VtfTextureFormat::DXT1_ONEBITALPHA => {
                image::DynamicImage::from(decode_bc1::<true>(&self.bytes, self.width, self.height))
            }
            VtfTextureFormat::DXT3 => {
                image::DynamicImage::from(decode_bc2(&self.bytes, self.width, self.height))
            }
            VtfTextureFormat::DXT5 => {
                image::DynamicImage::from(decode_bc3(&self.bytes, self.width, self.height))
            }
            VtfTextureFormat::BGR565 => {
                let mut image = image::RgbImage::new(self.width, self.height);
                iter_pixel_sections(&self.bytes, self.width, self.height).for_each(|(p, x, y)| {
                    let [b, g, r] = decode_rgb565(u16::from_le_bytes(p)).0;
                    image.get_pixel_mut(x, y).0 = [r, g, b];
                });
                image::DynamicImage::from(image)
            }
            VtfTextureFormat::BGRX5551 | VtfTextureFormat::BGRA5551 => {
                let mut image = image::RgbaImage::new(self.width, self.height);
                iter_pixel_sections(&self.bytes, self.width, self.height).for_each(|(p, x, y)| {
                    let p = u16::from_le_bytes(p);
                    let r = upscale_lower_u8::<5>(((p & 0b1111100000000000) >> 11) as u8);
                    let g = upscale_lower_u8::<5>(((p & 0b0000011111000000) >> 6) as u8);
                    let b = upscale_lower_u8::<5>(((p & 0b0000000000111110) >> 1) as u8);
                    let a = if (p & 0b0000000000000001) == 0 {
                        0
                    } else {
                        255
                    };
                    image.get_pixel_mut(x, y).0 = [r, g, b, a];
                });
                image::DynamicImage::from(image)
            }
            VtfTextureFormat::BGRA4444 => {
                let mut image = image::RgbaImage::new(self.width, self.height);
                iter_pixel_sections(&self.bytes, self.width, self.height).for_each(|(p, x, y)| {
                    let p = u16::from_le_bytes(p);
                    let r = upscale_lower_u8::<4>(((p & 0b1111000000000000) >> 12) as u8);
                    let g = upscale_lower_u8::<4>(((p & 0b0000111100000000) >> 8) as u8);
                    let b = upscale_lower_u8::<4>(((p & 0b0000000011110000) >> 4) as u8);
                    let a = upscale_lower_u8::<4>((p & 0b0000000000001111) as u8);
                    image.get_pixel_mut(x, y).0 = [r, g, b, a];
                });
                image::DynamicImage::from(image)
            }
            VtfTextureFormat::UV88 => {
                let mut image = image::RgbImage::new(self.width, self.height);
                iter_pixel_sections(&self.bytes, self.width, self.height).for_each(
                    |([u, v], x, y)| {
                        // FIXME: Ordering is wrong.
                        image.get_pixel_mut(x, y).0 = [u, v, 0];
                    },
                );
                image::DynamicImage::from(image)
            }
            VtfTextureFormat::UVWQ8888 | VtfTextureFormat::UVLX8888 => {
                let mut image = image::RgbaImage::new(self.width, self.height);
                iter_pixel_sections(&self.bytes, self.width, self.height).for_each(
                    |([u, v, w, q], x, y)| {
                        // FIXME: Ordering is probably wrong.
                        image.get_pixel_mut(x, y).0 = [u, v, w, q];
                    },
                );
                image::DynamicImage::from(image)
            }
            VtfTextureFormat::RGBA16161616F => {
                let mut image = image::Rgba32FImage::new(self.width, self.height);
                iter_pixel_sections(&self.bytes, self.width, self.height).for_each(
                    |([r1, r2, g1, g2, b1, b2, a1, a2], x, y)| {
                        let r = decode_f16(u16::from_le_bytes([r1, r2]));
                        let g = decode_f16(u16::from_le_bytes([g1, g2]));
                        let b = decode_f16(u16::from_le_bytes([b1, b2]));
                        let a = decode_f16(u16::from_le_bytes([a1, a2]));
                        image.get_pixel_mut(x, y).0 = [r, g, b, a];
                    },
                );
                image::DynamicImage::from(image)
            }
            VtfTextureFormat::RGBA16161616 => image::DynamicImage::ImageRgba16(
                image::ImageBuffer::from_raw(
                    self.width,
                    self.height,
                    self.bytes
                        .to_vec()
                        .chunks(2)
                        .map(|v| u16::from_le_bytes([v[0], v[1]]))
                        .collect(),
                )
                .unwrap(),
            ),
        }
    }
}

#[derive(Debug, Clone)]
struct VtfContainerInner {
    format: VtfTextureFormat,
    width: u32,
    height: u32,
    mipmaps: u8,
    frames: u16,
    first_frame: u16,
    faces: u8,
    depth: u16,
    textures: Box<[VtfTexture]>,
}

impl VtfContainerInner {
    fn texture_index(&self, mipmap: u8, frame: u16, face: u8, slice: u16) -> Option<usize> {
        if mipmap > self.mipmaps || frame > self.frames || face > self.faces || slice > self.depth {
            return None;
        }

        let mipmap = (self.mipmaps as usize) - 1 - (mipmap as usize);
        let frame = ((frame as usize) + (self.first_frame as usize)) % (self.frames as usize);
        let face = face as usize;
        let slice = slice as usize;

        Some(
            slice
                + face * (self.depth as usize)
                + frame * (self.faces as usize) * (self.depth as usize)
                + mipmap * (self.frames as usize) * (self.faces as usize) * (self.depth as usize),
        )
    }

    fn get(&self, mipmap: u8, frame: u16, face: u8, slice: u16) -> Option<&VtfTexture> {
        self.textures
            .get(self.texture_index(mipmap, frame, face, slice)?)
    }
}

#[derive(Debug, Clone)]
pub struct VtfContainerFrames(VtfContainerInner);

impl VtfContainerFrames {
    pub fn format(&self) -> VtfTextureFormat {
        self.0.format
    }

    pub fn width(&self) -> u32 {
        self.0.width
    }

    pub fn height(&self) -> u32 {
        self.0.height
    }

    pub fn num_mipmaps(&self) -> u8 {
        self.0.mipmaps
    }

    pub fn num_frames(&self) -> u16 {
        self.0.frames
    }

    pub fn texture(&self, mipmap: u8, frame: u16) -> Option<&VtfTexture> {
        self.0.get(mipmap, frame, 0, 0)
    }

    pub fn texture_best(&self) -> &VtfTexture {
        self.texture(0, 0).unwrap()
    }
}

struct IterTexturesEntry {
    width: u32,
    height: u32,
    mipmap: u8,
    frame: u16,
    face: u8,
    slice: u16,
}

fn iter_textures(
    width: u32,
    height: u32,
    mipmaps: u8,
    frames: u16,
    faces: u8,
    depth: u16,
) -> impl Iterator<Item = IterTexturesEntry> {
    (0..mipmaps)
        .rev()
        .flat_map(move |mipmap| {
            (0..frames).flat_map(move |frame| {
                (0..faces)
                    .flat_map(move |face| (0..depth).map(move |slice| (mipmap, frame, face, slice)))
            })
        })
        .map(move |(mipmap, frame, face, slice)| IterTexturesEntry {
            width: (width >> mipmap).max(1),
            height: (height >> mipmap).max(1),
            mipmap,
            frame,
            face,
            slice,
        })
}

#[derive(Debug, Clone)]
pub enum VtfContainer {
    Frames(VtfContainerFrames),
    Envmap,
    Depth,
}

impl VtfContainer {
    #[allow(clippy::too_many_arguments)]
    fn read<R: Read>(
        mut reader: R,
        format: VtfTextureFormat,
        width: u16,
        height: u16,
        mipmaps: u8,
        frames: u16,
        first_frame: u16,
        faces: u8,
        depth: u16,
    ) -> Result<Self, VtfError> {
        let textures = iter_textures(width as u32, height as u32, mipmaps, frames, faces, depth)
            .map(|IterTexturesEntry { width, height, .. }| {
                Ok(VtfTexture {
                    format,
                    width,
                    height,
                    bytes: reader.read_var(format.texture_byte_size(width, height))?,
                })
            })
            .collect::<Result<Vec<_>, VtfError>>()?;

        let inner = VtfContainerInner {
            format,
            width: width as u32,
            height: height as u32,
            mipmaps,
            frames,
            first_frame,
            faces,
            depth,
            textures: textures.into_boxed_slice(),
        };

        if faces != 1 || depth != 1 {
            return Err(VtfError::NotSupported("envmap or depth not supported"));
        }

        Ok(Self::Frames(VtfContainerFrames(inner)))
    }

    pub fn format(&self) -> VtfTextureFormat {
        match self {
            VtfContainer::Frames(inner) => inner.format(),
            VtfContainer::Envmap => todo!(),
            VtfContainer::Depth => todo!(),
        }
    }

    pub fn texture_best(&self) -> &VtfTexture {
        match self {
            VtfContainer::Frames(inner) => inner.texture_best(),
            VtfContainer::Envmap => todo!(),
            VtfContainer::Depth => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct VtfVersion(u32, u32);

#[derive(Debug, Clone)]
pub struct Vtf {
    container: VtfContainer,
    reflectivity: (f32, f32, f32),
    bumpmap_scale: f32,
    lowres: Option<VtfTexture>,
}

#[derive(Debug)]
struct VtfHeaderLowresInfo {
    format: VtfTextureFormat,
    width: u8,
    height: u8,
    offset: usize,
}

#[derive(Debug)]
#[allow(unused)]
enum VtfHeaderResource {
    Lowres {
        offset: u32,
    },
    Highres {
        offset: u32,
    },
    Other {
        tag: [u8; 3],
        flags: u8,
        offset_or_data: u32,
    },
}

#[derive(Debug)]
#[allow(unused)]
struct VtfHeader {
    version: VtfVersion,
    header_size: u32,
    width: u16,
    height: u16,
    flags: u32,
    frames: u16,
    first_frame: u16,
    faces: u8,
    reflectivity: (f32, f32, f32),
    bumpmap_scale: f32,
    format: VtfTextureFormat,
    mipmaps: u8,
    lowres: Option<VtfHeaderLowresInfo>,
    depth: u16,
    highres_offset: usize,
    resources: Box<[VtfHeaderResource]>,
}

impl VtfHeader {
    fn load<F: Read + Seek>(mut reader: F) -> Result<Self, VtfError> {
        reader.rewind()?;

        if reader.read_const()? != VTF_IDENTIFIER {
            return Err(VtfError::InvalidIdentifier);
        }

        let version = VtfVersion(
            u32::from_le_bytes(reader.read_const()?),
            u32::from_le_bytes(reader.read_const()?),
        );

        match version.0 {
            7 => match version.1 {
                0..=5 => {}
                6 => {
                    return Err(VtfError::NotSupported("VTF version 7.6 not yet supported"));
                }
                _ => return Err(VtfError::InvalidVersion(version)),
            },
            _ => return Err(VtfError::InvalidVersion(version)),
        }

        let header_size = u32::from_le_bytes(reader.read_const()?);
        // if header_size % 16 != 0 {
        //     log::warn!("VTF Header size is expected to be a multiple of 16: {header_size}");
        // }

        let width = u16::from_le_bytes(reader.read_const()?);
        let height = u16::from_le_bytes(reader.read_const()?);
        if width == 0 || height == 0 {
            return Err(VtfError::InvalidDimensions(width, height));
        }

        let flags = u32::from_le_bytes(reader.read_const()?);

        let frames = u16::from_le_bytes(reader.read_const()?);
        if frames == 0 {
            return Err(VtfError::NoFrames);
        }
        let first_frame = u16::from_le_bytes(reader.read_const()?);

        let faces: u8 = if flags & TEXTUREFLAGS_ENVMAP != 0 {
            if first_frame == 0xFFFF {
                7
            } else {
                6
            }
        } else {
            1
        };

        reader.read_const::<4>()?;

        let reflectivity = (
            f32::from_le_bytes(reader.read_const()?),
            f32::from_le_bytes(reader.read_const()?),
            f32::from_le_bytes(reader.read_const()?),
        );

        reader.read_const::<4>()?;

        let bumpmap_scale = f32::from_le_bytes(reader.read_const()?);

        let format_id = u32::from_le_bytes(reader.read_const()?);
        let format =
            VtfTextureFormat::from_id(format_id).ok_or(VtfError::InvalidFormat(format_id))?;

        let mipmaps = u8::from_le_bytes(reader.read_const()?);
        if mipmaps == 0 {
            return Err(VtfError::NoMipmaps);
        }
        if (width >> (mipmaps - 1)) == 0 && (height >> (mipmaps - 1)) == 0 {
            // return Err(VtfError::TooManyMipmaps(mipmaps, width, height));

            // Alot of textures seem to just not care about having 0 dimension mipmaps, so we just
            // warn instead or erroring.
            log::warn!("VTF has too many mipmaps, {width}x{height} with {mipmaps} mipmaps");
        }

        let lowres_format_id = u32::from_le_bytes(reader.read_const()?);
        let lowres_format = (lowres_format_id != 0xFFFFFFFF)
            .then_some(
                VtfTextureFormat::from_id(lowres_format_id)
                    .ok_or(VtfError::InvalidFormat(lowres_format_id)),
            )
            .transpose()?;
        if let Some(lowres_format) = lowres_format {
            if lowres_format != VtfTextureFormat::DXT1
                && lowres_format != VtfTextureFormat::DXT1_ONEBITALPHA
            {
                log::warn!(
                    "VTF lowres image is {lowres_format:?} instead of {:?}",
                    VtfTextureFormat::DXT1,
                );
            }
        }
        let lowres_width = u8::from_le_bytes(reader.read_const()?);
        let lowres_height = u8::from_le_bytes(reader.read_const()?);

        let depth = if version >= VtfVersion(7, 2) {
            u16::from_le_bytes(reader.read_const()?)
        } else {
            1
        };

        if version < VtfVersion(7, 3) {
            let lowres = if let Some(lowres_format) = lowres_format
                && lowres_width != 0
                && lowres_height != 0
            {
                Some(VtfHeaderLowresInfo {
                    format: lowres_format,
                    width: lowres_width,
                    height: lowres_height,
                    offset: header_size as usize,
                })
            } else {
                None
            };

            let highres_offset = lowres
                .as_ref()
                .map(|lowres| {
                    lowres.offset
                        + lowres
                            .format
                            .texture_byte_size(lowres.width as u32, lowres.height as u32)
                })
                .unwrap_or(header_size as usize);

            Ok(Self {
                version,
                header_size,
                width,
                height,
                flags,
                frames,
                first_frame,
                faces,
                reflectivity,
                bumpmap_scale,
                format,
                mipmaps,
                lowres,
                depth,
                highres_offset,
                resources: Vec::new().into_boxed_slice(),
            })
        } else {
            reader.read_const::<3>()?;
            let num_resources = u32::from_le_bytes(reader.read_const()?);
            if num_resources > 32 {
                log::warn!("VTF with more resources than officially supported");
            }
            reader.read_const::<8>()?;

            let resources = (0..num_resources)
                .map(|_| {
                    let tag: [u8; 3] = reader.read_const()?;
                    let flags = u8::from_le_bytes(reader.read_const()?);
                    let offset = u32::from_le_bytes(reader.read_const()?);
                    Ok(match &tag {
                        b"\x01\0\0" => VtfHeaderResource::Lowres { offset },
                        b"\x30\0\0" => VtfHeaderResource::Highres { offset },
                        _ => VtfHeaderResource::Other {
                            tag,
                            flags,
                            offset_or_data: offset,
                        },
                    })
                })
                .collect::<Result<Box<[_]>, VtfError>>()?;

            let lowres_offset = resources
                .iter()
                .find_map(|resource| {
                    if let VtfHeaderResource::Lowres { offset } = resource {
                        Some(offset)
                    } else {
                        None
                    }
                })
                .cloned();
            let highres_offset = *resources
                .iter()
                .find_map(|resource| {
                    if let VtfHeaderResource::Highres { offset } = resource {
                        Some(offset)
                    } else {
                        None
                    }
                })
                .ok_or(VtfError::NoHighresResource)?;

            Ok(Self {
                version,
                header_size,
                width,
                height,
                flags,
                frames,
                first_frame,
                faces,
                reflectivity,
                bumpmap_scale,
                format,
                mipmaps,
                lowres: if let Some(lowres_offset) = lowres_offset
                    && let Some(lowres_format) = lowres_format
                    && lowres_width != 0
                    && lowres_height != 0
                {
                    Some(VtfHeaderLowresInfo {
                        format: lowres_format,
                        width: lowres_width,
                        height: lowres_height,
                        offset: lowres_offset as usize,
                    })
                } else {
                    None
                },
                depth,
                highres_offset: highres_offset as usize,
                resources,
            })
        }
    }
}

impl Vtf {
    pub fn load<F: Read + Seek>(mut reader: F) -> Result<Self, VtfError> {
        let header = VtfHeader::load(&mut reader)?;
        Ok(Self {
            lowres: if let Some(lowres) = header.lowres {
                reader.seek(std::io::SeekFrom::Start(lowres.offset as u64))?;
                Some(VtfTexture {
                    format: lowres.format,
                    bytes: reader.read_var(
                        lowres
                            .format
                            .texture_byte_size(lowres.width as u32, lowres.height as u32),
                    )?,
                    width: lowres.width as u32,
                    height: lowres.height as u32,
                })
            } else {
                None
            },
            container: {
                reader.seek(std::io::SeekFrom::Start(header.highres_offset as u64))?;
                VtfContainer::read(
                    reader,
                    header.format,
                    header.width,
                    header.height,
                    header.mipmaps,
                    header.frames,
                    header.first_frame,
                    header.faces,
                    header.depth,
                )?
            },
            reflectivity: header.reflectivity,
            bumpmap_scale: header.bumpmap_scale,
        })
    }

    pub fn load_single_texture<F: Read + Seek>(
        mut reader: F,
        target_width: u32,
        target_height: u32,
    ) -> Result<VtfTexture, VtfError> {
        let header = VtfHeader::load(&mut reader)?;

        let target_mipmap = (0..header.mipmaps)
            .rev()
            .find(|mipmap| {
                let width = ((header.width as u32) >> mipmap).max(1);
                let height = ((header.height as u32) >> mipmap).max(1);
                width >= target_width && height >= target_height
            })
            .unwrap_or(0);

        let texture_offset: usize = iter_textures(
            header.width as u32,
            header.height as u32,
            header.mipmaps,
            header.frames,
            header.faces,
            header.depth,
        )
        .take_while(
            |IterTexturesEntry {
                 mipmap,
                 frame,
                 face,
                 slice,
                 ..
             }| {
                *mipmap != target_mipmap || *frame != 0 || *face != 0 || *slice != 0
            },
        )
        .map(|IterTexturesEntry { width, height, .. }| {
            header.format.texture_byte_size(width, height)
        })
        .sum();

        let actual_offset = header.highres_offset + texture_offset;

        let width = ((header.width as u32) >> target_mipmap).max(1);
        let height = ((header.height as u32) >> target_mipmap).max(1);
        let size = header.format.texture_byte_size(width, height);
        reader.seek(std::io::SeekFrom::Start(actual_offset as u64))?;
        Ok(VtfTexture {
            format: header.format,
            width,
            height,
            bytes: reader.read_var(size)?,
        })
    }

    pub fn container(&self) -> &VtfContainer {
        &self.container
    }

    pub fn reflectivity(&self) -> (f32, f32, f32) {
        self.reflectivity
    }

    pub fn bumpmap_scale(&self) -> f32 {
        self.bumpmap_scale
    }

    pub fn lowres(&self) -> Option<&VtfTexture> {
        self.lowres.as_ref()
    }

    pub fn texture_best(&self) -> &VtfTexture {
        self.container.texture_best()
    }
}
