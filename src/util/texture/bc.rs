
// Alot of code stolen from: https://github.com/UniversalGameExtraction/texture2ddecoder
// TODO: Make decoding faster.
// There's probably alot of tiny optimizations to be made.
// Definitely when loading values from data
// & writing to &mut out
// & decoded_rows_to_image can go faster by coping data by pixel rows.

use image::{Pixel, Rgb, Rgba, RgbaImage};
use rayon::iter::{IntoParallelIterator, ParallelIterator};



fn decode_rgb565(d: u16) -> Rgb<u8> {
    Rgb([
        (d >> 8 & 0xf8) as u8 | (d >> 13) as u8,
        (d >> 3 & 0xfc) as u8 | (d >> 9 & 3) as u8,
        (d << 3) as u8 | (d >> 2 & 7) as u8,
    ])
}

fn rgb888_lerp_13(rgb0: Rgb<u8>, rgb1: Rgb<u8>) -> Rgb<u8> {
    Rgb([
        (((rgb0.0[0] as u16) * 2 + (rgb1.0[0] as u16)) / 3) as u8,
        (((rgb0.0[1] as u16) * 2 + (rgb1.0[1] as u16)) / 3) as u8,
        (((rgb0.0[2] as u16) * 2 + (rgb1.0[2] as u16)) / 3) as u8,
    ])
}

fn rgb888_lerp_12(rgb0: Rgb<u8>, rgb1: Rgb<u8>) -> Rgb<u8> {
    Rgb([
        (((rgb0.0[0] as u16) + (rgb1.0[0] as u16)) / 2) as u8,
        (((rgb0.0[1] as u16) + (rgb1.0[1] as u16)) / 2) as u8,
        (((rgb0.0[2] as u16) + (rgb1.0[2] as u16)) / 2) as u8,
    ])
}



fn decode_bc1_block(data: &[u8], extra_color: &Rgba<u8>, out: &mut [u8], num_blocks_x: u32, block_x: u32) {
    let q0 = u16::from_le_bytes([data[0], data[1]]);
    let q1 = u16::from_le_bytes([data[2], data[3]]);
    let rgb0 = decode_rgb565(q0);
    let rgb1 = decode_rgb565(q1);

    let mut palette: [Rgba<u8>; 4] = [
        rgb0.to_rgba(),
        rgb1.to_rgba(),
        Rgba([ 0, 0, 0, 0 ]),
        Rgba([ 0, 0, 0, 0 ]),
    ];
    if q0 > q1 {
        palette[2] = rgb888_lerp_13(rgb0, rgb1).to_rgba();
        palette[3] = rgb888_lerp_13(rgb1, rgb0).to_rgba();
    } else {
        palette[2] = rgb888_lerp_12(rgb0, rgb1).to_rgba();
        palette[3] = extra_color.clone();
    }

    let indices: u32 = u32::from_le_bytes([ data[4], data[5], data[6], data[7] ]);

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
    let alphas = u64::from_le_bytes([ data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7] ]);
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
    
    let mut palette: [u8; 8] = [ a0, a1, 0, 0, 0, 0, 0, 0 ];
    if a0 > a1 {
        palette[2] = (((a0 as u16) * 6 + (a1 as u16) * 1) / 7) as u8;
        palette[3] = (((a0 as u16) * 5 + (a1 as u16) * 2) / 7) as u8;
        palette[4] = (((a0 as u16) * 4 + (a1 as u16) * 3) / 7) as u8;
        palette[5] = (((a0 as u16) * 3 + (a1 as u16) * 4) / 7) as u8;
        palette[6] = (((a0 as u16) * 2 + (a1 as u16) * 5) / 7) as u8;
        palette[7] = (((a0 as u16) * 1 + (a1 as u16) * 6) / 7) as u8;
    } else {
        palette[2] = (((a0 as u16) * 4 + (a1 as u16) * 1) / 5) as u8;
        palette[3] = (((a0 as u16) * 3 + (a1 as u16) * 2) / 5) as u8;
        palette[4] = (((a0 as u16) * 2 + (a1 as u16) * 3) / 5) as u8;
        palette[5] = (((a0 as u16) * 1 + (a1 as u16) * 4) / 5) as u8;
        palette[6] = 0;
        palette[7] = 255;
    }

    let indices: u64 = u64::from_le_bytes([ data[2], data[3], data[4], data[5], data[6], data[7], 0, 0 ]);
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


