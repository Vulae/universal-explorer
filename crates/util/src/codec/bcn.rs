// NOTE: https://learn.microsoft.com/en-us/windows/win32/direct3d10/d3d10-graphics-programming-guide-resources-block-compression

use image::{Pixel, Rgba, RgbaImage};

use super::{decode_rgb565, lerp_u8, rgb888_lerp, upscale_lower_u8};

fn decode_bc1_block<const A: bool>(data: &[u8; 8]) -> [Rgba<u8>; 16] {
    let q0 = u16::from_le_bytes(data[0..=1].try_into().unwrap());
    let q1 = u16::from_le_bytes(data[2..=3].try_into().unwrap());
    let rgb0 = decode_rgb565(q0);
    let rgb1 = decode_rgb565(q1);

    let palette: [Rgba<u8>; 4] = if q0 > q1 {
        [
            rgb0.to_rgba(),
            rgb1.to_rgba(),
            rgb888_lerp::<1, 3>(rgb0, rgb1).to_rgba(),
            rgb888_lerp::<2, 3>(rgb0, rgb1).to_rgba(),
        ]
    } else {
        [
            rgb0.to_rgba(),
            rgb1.to_rgba(),
            rgb888_lerp::<1, 2>(rgb0, rgb1).to_rgba(),
            Rgba([0, 0, 0, if A { 0 } else { 255 }]),
        ]
    };

    let indices = u32::from_le_bytes(data[4..=7].try_into().unwrap());

    std::array::from_fn(|i| {
        let index = (indices >> (i << 1)) & 0b11;
        palette[index as usize]
    })
}

fn decode_bc2_alpha_block(data: &[u8; 8]) -> [u8; 16] {
    let alphas = u64::from_le_bytes(*data);
    std::array::from_fn(|i| upscale_lower_u8::<4>(((alphas >> (i << 2)) & 0b1111) as u8))
}

fn decode_bc3_alpha_block(data: &[u8; 8]) -> [u8; 16] {
    let a0 = data[0];
    let a1 = data[1];

    let palette: [u8; 8] = if a0 > a1 {
        [
            a0,
            a1,
            lerp_u8::<1, 7>(a0, a1),
            lerp_u8::<2, 7>(a0, a1),
            lerp_u8::<3, 7>(a0, a1),
            lerp_u8::<4, 7>(a0, a1),
            lerp_u8::<5, 7>(a0, a1),
            lerp_u8::<6, 7>(a0, a1),
        ]
    } else {
        [
            a0,
            a1,
            lerp_u8::<1, 5>(a0, a1),
            lerp_u8::<2, 5>(a0, a1),
            lerp_u8::<3, 5>(a0, a1),
            lerp_u8::<4, 5>(a0, a1),
            0,
            255,
        ]
    };

    let packed_indices = u64::from_le_bytes(*data) >> 16;
    std::array::from_fn(|i| palette[((packed_indices >> (i * 3)) & 0b111) as usize])
}

pub fn decode_bc1<const A: bool>(data: &[u8], width: u32, height: u32) -> RgbaImage {
    let num_blocks_x = width.div_ceil(4);
    let num_blocks_y = height.div_ceil(4);

    let mut img = RgbaImage::new(width, height);

    (0..num_blocks_y).for_each(|block_y| {
        (0..num_blocks_x).for_each(|block_x| {
            let data_offset =
                ((block_x as usize) + (block_y as usize) * (num_blocks_x as usize)) << 3;
            let decoded =
                decode_bc1_block::<A>(&data[data_offset..(data_offset + 8)].try_into().unwrap());
            for dy in 0..4 {
                for dx in 0..4 {
                    if let Some(imgcol) =
                        img.get_pixel_mut_checked(block_x * 4 + dx, block_y * 4 + dy)
                    {
                        *imgcol = decoded[(dx as usize) + (dy as usize) * 4];
                    }
                }
            }
        });
    });

    img
}

pub fn decode_bc2(data: &[u8], width: u32, height: u32) -> RgbaImage {
    let num_blocks_x = width.div_ceil(4);
    let num_blocks_y = height.div_ceil(4);

    let mut img = RgbaImage::new(width, height);

    (0..num_blocks_y).for_each(|block_y| {
        (0..num_blocks_x).for_each(|block_x| {
            let data_offset =
                ((block_x as usize) + (block_y as usize) * (num_blocks_x as usize)) << 4;
            let alpha_decoded =
                decode_bc2_alpha_block(&data[data_offset..(data_offset + 8)].try_into().unwrap());
            let color_decoded = decode_bc1_block::<false>(
                &data[(data_offset + 8)..(data_offset + 16)]
                    .try_into()
                    .unwrap(),
            );
            for dy in 0..4 {
                for dx in 0..4 {
                    if let Some(imgcol) =
                        img.get_pixel_mut_checked(block_x * 4 + dx, block_y * 4 + dy)
                    {
                        *imgcol = color_decoded[(dx as usize) + (dy as usize) * 4];
                        imgcol.0[3] = alpha_decoded[(dx as usize) + (dy as usize) * 4];
                    }
                }
            }
        });
    });

    img
}

pub fn decode_bc3(data: &[u8], width: u32, height: u32) -> RgbaImage {
    let num_blocks_x = width.div_ceil(4);
    let num_blocks_y = height.div_ceil(4);

    let mut img = RgbaImage::new(width, height);

    (0..num_blocks_y).for_each(|block_y| {
        (0..num_blocks_x).for_each(|block_x| {
            let data_offset =
                ((block_x as usize) + (block_y as usize) * (num_blocks_x as usize)) << 4;
            let alpha_decoded =
                decode_bc3_alpha_block(&data[data_offset..(data_offset + 8)].try_into().unwrap());
            let color_decoded = decode_bc1_block::<false>(
                &data[(data_offset + 8)..(data_offset + 16)]
                    .try_into()
                    .unwrap(),
            );
            for dy in 0..4 {
                for dx in 0..4 {
                    if let Some(imgcol) =
                        img.get_pixel_mut_checked(block_x * 4 + dx, block_y * 4 + dy)
                    {
                        *imgcol = color_decoded[(dx as usize) + (dy as usize) * 4];
                        imgcol.0[3] = alpha_decoded[(dx as usize) + (dy as usize) * 4];
                    }
                }
            }
        });
    });

    img
}
