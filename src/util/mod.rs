
pub mod image;
pub mod source_engine;
pub mod reader;
pub mod texture;
// pub mod virtual_fs;



pub trait TryClone {
    fn try_clone(&self) -> anyhow::Result<Self> where Self: Sized;
}

impl TryClone for std::fs::File {
    fn try_clone(&self) -> anyhow::Result<Self> {
        Ok(self.try_clone()?)
    }
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


