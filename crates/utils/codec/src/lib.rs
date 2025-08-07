use image::Rgb;

pub mod bcn;
mod lzma;
mod seekable_deflate;
mod seekable_lzma;

pub use lzma::*;
pub use seekable_deflate::*;
pub use seekable_lzma::*;

pub fn lerp_u8<const N: u16, const D: u16>(a: u8, b: u8) -> u8 {
    // if N == 0 {
    //     a
    // } else if N == D {
    //     b
    // } else if D == (N << 1) {
    //     (a & b) + ((a ^ b) >> 1)
    // } else {
    //     (((a as u16) * (D - N) + (b as u16) * N) / D) as u8
    // }
    (((a as u16) * (D - N) + (b as u16) * N) / D) as u8
}

pub fn rgb888_lerp<const N: u16, const D: u16>(a: Rgb<u8>, b: Rgb<u8>) -> Rgb<u8> {
    Rgb([
        lerp_u8::<N, D>(a.0[0], b.0[0]),
        lerp_u8::<N, D>(a.0[1], b.0[1]),
        lerp_u8::<N, D>(a.0[2], b.0[2]),
    ])
}

pub fn upscale_lower_u8<const N: u8>(x: u8) -> u8 {
    let shift = 8 - N;
    (x << shift) | (x >> (N - shift))
}

pub fn decode_rgb565(d: u16) -> Rgb<u8> {
    Rgb([
        upscale_lower_u8::<5>((d >> 11 & 0b00011111) as u8),
        upscale_lower_u8::<6>((d >> 5 & 0b00111111) as u8),
        upscale_lower_u8::<5>((d & 0b00011111) as u8),
    ])
}

#[rustfmt::skip]
pub fn encode_rgb565(c: Rgb<u8>) -> u16 {
    (((c.0[0] as u16) & 0b11111000) << 8) |
    (((c.0[1] as u16) & 0b11111100) << 3) |
    (((c.0[2] as u16) & 0b11111000) >> 3)
}

pub fn decode_f16(v: u16) -> f32 {
    // https://stackoverflow.com/questions/36008434/
    let exp: u16 = v >> 10 & 0x1f;
    let mant: u16 = v & 0x3ff;
    let val: f32 = if exp == 0 {
        (mant as f32) * (2.0f32).powi(-24)
    } else if exp != 31 {
        (mant as f32 + 1024f32) * (2.0f32).powi(exp as i32 - 25)
    } else if mant == 0 {
        f32::INFINITY
    } else {
        f32::NAN
    };
    if v & 0x8000 != 0 {
        -val
    } else {
        val
    }
}

#[cfg(test)]
mod test {
    use image::Rgb;

    use crate::{decode_rgb565, encode_rgb565, upscale_lower_u8};

    macro_rules! test_lossless_rgb565_conversion {
        ($color:expr) => {
            assert_eq!($color, decode_rgb565(encode_rgb565($color)));
        };
    }

    #[test]
    fn rgb565() {
        for r in 0..32 {
            for g in 0..64 {
                for b in 0..32 {
                    test_lossless_rgb565_conversion!(Rgb([
                        upscale_lower_u8::<5>(r),
                        upscale_lower_u8::<6>(g),
                        upscale_lower_u8::<5>(b),
                    ]));
                }
            }
        }
    }
}
