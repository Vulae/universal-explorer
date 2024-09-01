
// Alot of code stolen from: https://github.com/UniversalGameExtraction/texture2ddecoder
// TODO: Make decoding faster.
// There's probably alot of tiny optimizations to be made.
// Definitely when loading values from data
// & writing to &mut out
// & decoded_rows_to_image can go faster by coping data by pixel rows.

use image::{Pixel, Rgb, Rgba, RgbaImage};
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



fn decode_bc1_block(data: &[u8], extra_color: &Rgba<u8>, out: &mut [u8], num_blocks_x: u32, block_x: u32) {
    let q0 = join_le_bytes!(u16; data[0], data[1]);
    let q1 = join_le_bytes!(u16; data[2], data[3]);
    let rgb0 = decode_rgb565(q0);
    let rgb1 = decode_rgb565(q1);

    let palette: [Rgba<u8>; 4] = if q0 > q1 { [
        rgb0.to_rgba(),
        rgb1.to_rgba(),
        rgb888_lerp::<1, 3>(rgb0, rgb1).to_rgba(),
        rgb888_lerp::<2, 3>(rgb0, rgb1).to_rgba(),
    ] } else { [
        rgb0.to_rgba(),
        rgb1.to_rgba(),
        rgb888_lerp::<1, 2>(rgb0, rgb1).to_rgba(),
        extra_color.clone(),
    ] };

    let indices = join_le_bytes!(u32; data[4], data[5], data[6], data[7]);

    for pi in 0..16 {
        let index = (indices >> (pi << 1)) & 0b11;
        let color = palette[index as usize];

        let x = (block_x << 2) + (pi & 0b11);
        let y = pi >> 2;
        let out_index = ((x + y * (num_blocks_x << 2)) << 2) as usize;
        out[out_index + 0] = color.0[0];
        out[out_index + 1] = color.0[1];
        out[out_index + 2] = color.0[2];
        out[out_index + 3] = color.0[3];
    }
}

fn decode_bc2_alpha_block(data: &[u8], out: &mut [u8], num_blocks_x: u32, block_x: u32) {
    let alphas = join_le_bytes!(u64; data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]);
    for pi in 0..16 {
        // FIXME: The upsampling doesn't seem to be correct.
        let alpha = (((alphas >> (pi << 2)) & 0b1111) as u8) * 17;

        let x = (block_x << 2) + (pi & 0b11);
        let y = pi >> 2;
        let out_index = (x + y * (num_blocks_x << 2)) << 2;
        out[out_index as usize + 3] = alpha;
    }
}

fn decode_bc3_alpha_block(data: &[u8], out: &mut [u8], num_blocks_x: u32, block_x: u32) {
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

        let x = (block_x << 2) + (pi & 0b11);
        let y = pi >> 2;
        let out_index = (x + y * (num_blocks_x << 2)) << 2;
        out[out_index as usize + 3] = alpha;
    }
}



pub fn decoded_rows_to_image(decoded_rows: Vec<Vec<u8>>, width: u32, height: u32) -> RgbaImage {
    let num_blocks_x = width.div_ceil(4);

    let mut out = RgbaImage::new(width, height);
    
    out.par_enumerate_pixels_mut().for_each(|(x, y, pixel)| {
        let decoded_row = &decoded_rows[(y >> 2) as usize];
        let decoded_index = ((x + (y & 0b11) * (num_blocks_x << 2)) << 2) as usize;
        pixel.0[0] = decoded_row[decoded_index + 0];
        pixel.0[1] = decoded_row[decoded_index + 1];
        pixel.0[2] = decoded_row[decoded_index + 2];
        pixel.0[3] = decoded_row[decoded_index + 3];
    });

    out
}



pub fn decode_bc1(data: &[u8], width: u32, height: u32, extra_color: Rgba<u8>) -> RgbaImage {
    let num_blocks_x = width.div_ceil(4);
    let num_blocks_y = height.div_ceil(4);

    let decoded_rows = (0..num_blocks_y).into_par_iter().map(|block_y| {
        let mut decoded_row = vec![0u8; (num_blocks_x as usize) << 6];

        (0..num_blocks_x).for_each(|block_x| {
            let data_offset = ((block_x as usize) + (block_y as usize) * (num_blocks_x as usize)) << 3;
            decode_bc1_block(&data[data_offset..], &extra_color, &mut decoded_row, num_blocks_x, block_x);
        });

        decoded_row
    }).collect::<Vec<_>>();

    decoded_rows_to_image(decoded_rows, width, height)
}

pub fn decode_bc2(data: &[u8], width: u32, height: u32) -> RgbaImage {
    let num_blocks_x = width.div_ceil(4);
    let num_blocks_y = height.div_ceil(4);

    let decoded_rows = (0..num_blocks_y).into_par_iter().map(|block_y| {
        let mut decoded_row = vec![0u8; (num_blocks_x as usize) << 6];

        (0..num_blocks_x).for_each(|block_x| {
            let data_offset = ((block_x as usize) + (block_y as usize) * (num_blocks_x as usize)) << 4;
            decode_bc1_block(&data[(data_offset + 8)..], &Rgba([ 0, 0, 0, 255 ]), &mut decoded_row, num_blocks_x, block_x);
            decode_bc2_alpha_block(&data[data_offset..], &mut decoded_row, num_blocks_x, block_x);
        });

        decoded_row
    }).collect::<Vec<_>>();

    decoded_rows_to_image(decoded_rows, width, height)
}

pub fn decode_bc3(data: &[u8], width: u32, height: u32) -> RgbaImage {
    let num_blocks_x = width.div_ceil(4);
    let num_blocks_y = height.div_ceil(4);

    let decoded_rows = (0..num_blocks_y).into_par_iter().map(|block_y| {
        let mut decoded_row = vec![0u8; (num_blocks_x as usize) << 6];

        (0..num_blocks_x).for_each(|block_x| {
            let data_offset = ((block_x as usize) + (block_y as usize) * (num_blocks_x as usize)) << 4;
            decode_bc1_block(&data[(data_offset + 8)..], &Rgba([ 0, 0, 0, 255 ]), &mut decoded_row, num_blocks_x, block_x);
            decode_bc3_alpha_block(&data[data_offset..], &mut decoded_row, num_blocks_x, block_x);
        });

        decoded_row
    }).collect::<Vec<_>>();

    decoded_rows_to_image(decoded_rows, width, height)
}


