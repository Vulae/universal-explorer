
// I have tried my best at optimizing this without making it very ugly.
// I think the only thing left to optimize without uglifying the whole thing is UnsafeImageWriter.
// 
// Performance on my CPU (AMD Ryzen 5 5600G) (12 threads) decoding on release build:
//     4096x4096 bc3 texture decode in ~17.2ms.
//     (Can comfortably decode an animated 1920x1080 bc1 texture in real time.)

use std::sync::atomic::AtomicU32;
use image::{ImageBuffer, Rgb, Rgba, RgbaImage};
use rayon::iter::{IntoParallelIterator, ParallelIterator};



#[inline(always)]
fn lerp_u8<const N: u16, const D: u16>(a: u8, b: u8) -> u8 {
    if N == 0 {
        a
    } else if N == D {
        b
    } else if D == (N << 1) {
        (a & b) + ((a ^ b) >> 1)
    } else {
        (((a as u16) * (D - N) + (b as u16) * N) / D) as u8
    }
}

#[inline(always)]
fn upscale_lower_u8<const N: u8>(x: u8) -> u8 {
    let shift = 8 - N;
    (x << shift) | (x >> (N - shift))
}

#[inline(always)]
fn decode_rgb565(d: u16) -> Rgb<u8> {
    Rgb([
        upscale_lower_u8::<5>((d >> 11 & 0b00011111) as u8),
        upscale_lower_u8::<6>((d >> 5 & 0b00111111) as u8),
        upscale_lower_u8::<5>((d & 0b00011111) as u8),
    ])
}

#[inline(always)]
fn rgb888_lerp<const N: u16, const D: u16>(a: Rgb<u8>, b: Rgb<u8>) -> Rgb<u8> {
    Rgb([
        lerp_u8::<N, D>(a.0[0], b.0[0]),
        lerp_u8::<N, D>(a.0[1], b.0[1]),
        lerp_u8::<N, D>(a.0[2], b.0[2]),
    ])
}



macro_rules! _join_le_bytes_inner {
    ($type:ty, $counter:expr; $first:expr $(, $rest:expr)*) => {
        (($first as $type) << $counter) | _join_le_bytes_inner!($type, $counter + 8; $($rest),*)
    };
    ($type:ty, $counter:expr;) => {
        0
    };
}

macro_rules! join_le_bytes {
    ($type:ty; $($bytes:expr),*) => {
        _join_le_bytes_inner!($type, 0; $($bytes),*)
    };
}



#[inline(always)]
fn rgba8888_encode_u32(rgba8888: Rgba<u8>) -> u32 {
    join_le_bytes!(u32; rgba8888.0[0], rgba8888.0[1], rgba8888.0[2], rgba8888.0[3])
}

#[inline(always)]
fn u32_decode_rgba8888(value: u32) -> Rgba<u8> {
    Rgba([ (value >> 24) as u8, (value >> 16) as u8, (value >> 8) as u8, value as u8 ])
}

#[inline(always)]
fn u32_alpha_bitmask(alpha: u8) -> u32 {
    (alpha as u32) << 24
}

#[inline(always)]
fn rgb888_to_rgba8888<const A: bool>(rgb888: Rgb<u8>) -> Rgba<u8> {
    Rgba([ rgb888.0[0], rgb888.0[1], rgb888.0[2], if A { 255 } else { 0 } ])
}



// TODO: Rewrite to be faster by not using atomics
struct UnsafeImageWriter {
    width: u32,
    height: u32,
    data: Vec<AtomicU32>,
}

unsafe impl Send for UnsafeImageWriter {}

impl UnsafeImageWriter {
    pub fn new(width: u32, height: u32) -> Self {
        let num_pixels = (width as usize) * (height as usize);
        Self {
            width, height,
            data: (0..num_pixels).map(|_| AtomicU32::default()).collect(),
        }
    }

    #[inline(always)]
    fn in_bounds(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height
    }

    #[inline(always)]
    fn index(&self, x: u32, y: u32) -> usize {
        (x as usize) + ((y as usize) * (self.width as usize))
    }

    #[inline(always)]
    pub fn set(&self, x: u32, y: u32, color: Rgba<u8>) {
        if self.in_bounds(x, y) {
            self.data[self.index(x, y)].store(rgba8888_encode_u32(color), std::sync::atomic::Ordering::Relaxed);
        }
    }

    #[inline(always)]
    pub fn or_alpha(&self, x: u32, y: u32, alpha: u8) {
        if self.in_bounds(x, y) {
            self.data[self.index(x, y)].fetch_or(u32_alpha_bitmask(alpha), std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub fn into_image(self) -> RgbaImage {
        let mut img: RgbaImage = ImageBuffer::new(self.width, self.height);
        img.par_enumerate_pixels_mut().for_each(|(x, y, pixel)| {
            let value = self.data[self.index(x, y)].load(std::sync::atomic::Ordering::Relaxed);
            pixel.0 = value.to_le_bytes();
        });
        img
    }
}



fn decode_bc1_block<const A: bool>(data: &[u8], extra_color: &Rgba<u8>, out: &UnsafeImageWriter, block_x: u32, block_y: u32) {
    let q0 = join_le_bytes!(u16; data[0], data[1]);
    let q1 = join_le_bytes!(u16; data[2], data[3]);
    let rgb0 = decode_rgb565(q0);
    let rgb1 = decode_rgb565(q1);

    let palette: [Rgba<u8>; 4] = if q0 > q1 { [
        rgb888_to_rgba8888::<A>(rgb0),
        rgb888_to_rgba8888::<A>(rgb1),
        rgb888_to_rgba8888::<A>(rgb888_lerp::<1, 3>(rgb0, rgb1)),
        rgb888_to_rgba8888::<A>(rgb888_lerp::<2, 3>(rgb0, rgb1)),
    ] } else { [
        rgb888_to_rgba8888::<A>(rgb0),
        rgb888_to_rgba8888::<A>(rgb1),
        rgb888_to_rgba8888::<A>(rgb888_lerp::<1, 2>(rgb0, rgb1)),
        extra_color.clone(),
    ] };

    let indices = join_le_bytes!(u32; data[4], data[5], data[6], data[7]);

    for pi in 0..16 {
        let index = (indices >> (pi << 1)) & 0b11;
        let color = palette[index as usize];

        let x = (block_x << 2) | (pi & 0b11);
        let y = (block_y << 2) | (pi >> 2);

        out.set(x, y, color);
    }
}

fn decode_bc2_alpha_block(data: &[u8], out: &UnsafeImageWriter, block_x: u32, block_y: u32) {
    let alphas = join_le_bytes!(u64; data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]);
    for pi in 0..16 {
        let alpha = upscale_lower_u8::<4>(((alphas >> (pi << 2)) & 0b1111) as u8);

        let x = (block_x << 2) | (pi & 0b11);
        let y = (block_y << 2) | (pi >> 2);

        out.or_alpha(x, y, alpha);
    }
}

fn decode_bc3_alpha_block(data: &[u8], out: &UnsafeImageWriter, block_x: u32, block_y: u32) {
    let a0 = data[0];
    let a1 = data[1];
    
    let palette: [u8; 8] = if a0 > a1 { [
        lerp_u8::<0, 7>(a0, a1),
        lerp_u8::<7, 7>(a0, a1),
        lerp_u8::<1, 7>(a0, a1),
        lerp_u8::<2, 7>(a0, a1),
        lerp_u8::<3, 7>(a0, a1),
        lerp_u8::<4, 7>(a0, a1),
        lerp_u8::<5, 7>(a0, a1),
        lerp_u8::<6, 7>(a0, a1),
    ] } else { [
        lerp_u8::<0, 5>(a0, a1),
        lerp_u8::<5, 5>(a0, a1),
        lerp_u8::<1, 5>(a0, a1),
        lerp_u8::<2, 5>(a0, a1),
        lerp_u8::<3, 5>(a0, a1),
        lerp_u8::<4, 5>(a0, a1),
        0, 255,
    ] };

    let indices = join_le_bytes!(u64; data[2], data[3], data[4], data[5], data[6], data[7], 0, 0);
    for pi in 0..16 {
        let index = (indices >> (pi * 3)) & 0b111;
        let alpha = palette[index as usize];

        let x = (block_x << 2) | (pi & 0b11);
        let y = (block_y << 2) | (pi >> 2);

        out.or_alpha(x, y, alpha);
    }
}



pub fn decode_bc1(data: &[u8], width: u32, height: u32, extra_color: Rgba<u8>) -> RgbaImage {
    let num_blocks_x = width.div_ceil(4);
    let num_blocks_y = height.div_ceil(4);

    let img = UnsafeImageWriter::new(width, height);

    (0..num_blocks_y).into_par_iter().for_each(|block_y| {
        (0..num_blocks_x).for_each(|block_x| {
            let data_offset = ((block_x as usize) + (block_y as usize) * (num_blocks_x as usize)) << 3;
            decode_bc1_block::<true>(&data[data_offset..], &extra_color, &img, block_x, block_y);
        });
    });

    img.into_image()
}

pub fn decode_bc2(data: &[u8], width: u32, height: u32) -> RgbaImage {
    let num_blocks_x = width.div_ceil(4);
    let num_blocks_y = height.div_ceil(4);

    let img = UnsafeImageWriter::new(width, height);

    (0..num_blocks_y).into_par_iter().for_each(|block_y| {
        (0..num_blocks_x).for_each(|block_x| {
            let data_offset = ((block_x as usize) + (block_y as usize) * (num_blocks_x as usize)) << 4;
            decode_bc1_block::<false>(&data[(data_offset + 8)..], &Rgba([ 0, 0, 0, 0 ]), &img, block_x, block_y);
            decode_bc2_alpha_block(&data[data_offset..], &img, block_x, block_y);
        });
    });

    img.into_image()
}

pub fn decode_bc3(data: &[u8], width: u32, height: u32) -> RgbaImage {
    let num_blocks_x = width.div_ceil(4);
    let num_blocks_y = height.div_ceil(4);

    let img = UnsafeImageWriter::new(width, height);

    (0..num_blocks_y).into_par_iter().for_each(|block_y| {
        (0..num_blocks_x).for_each(|block_x| {
            let data_offset = ((block_x as usize) + (block_y as usize) * (num_blocks_x as usize)) << 4;
            decode_bc1_block::<false>(&data[(data_offset + 8)..], &Rgba([ 0, 0, 0, 0 ]), &img, block_x, block_y);
            decode_bc3_alpha_block(&data[data_offset..], &img, block_x, block_y);
        });
    });

    img.into_image()
}


