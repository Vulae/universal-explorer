
pub mod image;
pub mod source_engine;
pub mod reader;
pub mod texture;
// pub mod virtual_fs;

use std::path::PathBuf;



pub fn filename<P: Into<PathBuf>>(path: P) -> Option<String> {
    let path: PathBuf = path.into();
    path
        .file_name()
        .map(|s| s.to_str().map(|s| s.to_owned()))
        .flatten()
}



#[macro_export]
macro_rules! print_perf {
    ($name:literal, $block:expr) => ({
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
    });
}


