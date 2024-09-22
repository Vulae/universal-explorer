extern crate anyhow;
extern crate flate2;
extern crate image;
extern crate rayon;
extern crate serde;
extern crate serde_json;

pub mod file_utils;
pub mod image_utils;
pub mod pickle;
pub mod reader;
pub mod texture;
pub mod tree_fs;
pub mod virtual_fs;

use std::num::ParseIntError;

#[macro_export]
macro_rules! print_perf {
    ($name:literal, $block:expr) => {{
        #[cfg(debug_assertions)]
        {
            let print_perf_start = std::time::Instant::now();
            let print_perf_result = $block;
            println!("{} {:?}", $name, print_perf_start.elapsed());
            print_perf_result
        }
        #[cfg(not(debug_assertions))]
        {
            $block
        }
    }};
}

pub fn decode_hex(hex_string: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..hex_string.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_string[i..(i + 2)], 16))
        .collect()
}
