
// Alot of code stolen from: https://github.com/UniversalGameExtraction/texture2ddecoder
// TODO: Make decoding faster.

use image::RgbaImage;



type RGB888 = (u8, u8, u8);
type RGBA8888 = (u8, u8, u8, u8);

fn decode_rgb565(d: u16) -> RGB888 {
    (
        (d >> 8 & 0xf8) as u8 | (d >> 13) as u8,
        (d >> 3 & 0xfc) as u8 | (d >> 9 & 3) as u8,
        (d << 3) as u8 | (d >> 2 & 7) as u8,
    )
}

fn rgb888_lerp_13(rgb0: RGB888, rgb1: RGB888) -> RGB888 {
    (
        (((rgb0.0 as u16) * 2 + (rgb1.0 as u16)) / 3) as u8,
        (((rgb0.1 as u16) * 2 + (rgb1.1 as u16)) / 3) as u8,
        (((rgb0.2 as u16) * 2 + (rgb1.2 as u16)) / 3) as u8,
    )
}

fn rgb888_lerp_12(rgb0: RGB888, rgb1: RGB888) -> RGB888 {
    (
        (((rgb0.0 as u16) + (rgb1.0 as u16)) / 2) as u8,
        (((rgb0.1 as u16) + (rgb1.1 as u16)) / 2) as u8,
        (((rgb0.2 as u16) + (rgb1.2 as u16)) / 2) as u8,
    )
}

fn rgb888_to_rgba8888(rgb: RGB888) -> RGBA8888 {
    (rgb.0, rgb.1, rgb.2, 255)
}



fn decode_bc1_block(data: &[u8], out: &mut [u8], extra_color: RGBA8888) {
    let q0 = u16::from_le_bytes([data[0], data[1]]);
    let q1 = u16::from_le_bytes([data[2], data[3]]);
    let rgb0 = decode_rgb565(q0);
    let rgb1 = decode_rgb565(q1);

    let mut palette: [RGBA8888; 4] = [
        rgb888_to_rgba8888(rgb0),
        rgb888_to_rgba8888(rgb1),
        RGBA8888::default(),
        RGBA8888::default(),
    ];
    if q0 > q1 {
        palette[2] = rgb888_to_rgba8888(rgb888_lerp_13(rgb0, rgb1));
        palette[3] = rgb888_to_rgba8888(rgb888_lerp_13(rgb1, rgb0));
    } else {
        palette[2] = rgb888_to_rgba8888(rgb888_lerp_12(rgb0, rgb1));
        palette[3] = extra_color;
    }

    let indices: u32 = u32::from_le_bytes([ data[4], data[5], data[6], data[7] ]);
    for pi in 0..16 {
        let index = (indices >> (pi << 1)) & 0b11;
        let color = palette[index as usize];
        out[pi*4 + 0] = color.0;
        out[pi*4 + 1] = color.1;
        out[pi*4 + 2] = color.2;
        out[pi*4 + 3] = color.3;
    }
}

fn decode_bc3_alpha_block(data: &[u8], out: &mut [u8]) {
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

    let indices: u64 = u64::from_le_bytes([ 0, 0, data[2], data[3], data[4], data[5], data[6], data[7] ]);
    for pi in 0..16 {
        let index = (indices >> (pi * 3)) & 0b111;
        let alpha = palette[index as usize];
        out[pi*4 + 3] = alpha;
    }
}



fn copy_block_buffer(block_x: usize, block_y: usize, width: usize, height: usize, block_width: usize, block_height: usize, buffer: &[u8], image: &mut [u8]) {
    let x = block_x * block_width;
    let y = block_y * block_height;
    for dy in 0..block_height {
        for dx in 0..block_width {
            let x = x + dx;
            let y = y + dy;
            if x >= width || y >= height {
                continue;
            }
            let buffer_index = (dx + dy * block_width) * 4;
            let image_index = (x + y * width) * 4;
            image[image_index..(image_index + 4)].copy_from_slice(&buffer[buffer_index..(buffer_index + 4)]);
        }
    }
}

fn blocks<F>(data: &[u8], out: &mut [u8], width: usize, height: usize, block_width: usize, block_height: usize, block_size: usize, callback: F)
where 
    F: Fn(&[u8], &mut [u8])
{
    let num_blocks_x = width.div_ceil(block_width);
    let num_blocks_y = height.div_ceil(block_height);

    let mut data_offset: usize = 0;

    let mut decoded = vec![0u8; block_width * block_height * 4];
    for by in 0..num_blocks_y {
        for bx in 0..num_blocks_x {
            callback(&data[data_offset..(data_offset + block_size)], &mut decoded);
            copy_block_buffer(bx, by, width, height, block_width, block_height, &decoded, out);
            data_offset += block_size;
        }
    }
}





pub fn decode_bc1(data: &[u8], width: u32, height: u32, extra_color: RGBA8888) -> RgbaImage {
    let mut out = vec![0u8; (width as usize) * (height as usize) * 4];

    blocks(data, &mut out, width as usize, height as usize, 4, 4, 8, |data, out| {
        decode_bc1_block(data, out, extra_color);
    });

    RgbaImage::from_raw(width, height, out).unwrap()
}

pub fn decode_bc3(data: &[u8], width: u32, height: u32) -> RgbaImage {
    let mut out = vec![0u8; (width as usize) * (height as usize) * 4];

    blocks(data, &mut out, width as usize, height as usize, 4, 4, 16, |data, out| {
        decode_bc1_block(&data[8..], out, (0, 0, 0, 255));
        decode_bc3_alpha_block(data, out);
    });

    RgbaImage::from_raw(width, height, out).unwrap()
}


