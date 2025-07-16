pub mod codec;
mod read_ext;
mod vfs;

use std::collections::HashMap;

pub use read_ext::*;
pub use vfs::*;

pub fn index_hashmap_to_vec<T: std::fmt::Debug>(hashmap: HashMap<usize, T>) -> Option<Vec<T>> {
    let mut vec: Vec<Option<T>> = Vec::from_iter((0..hashmap.len()).map(|_| None));
    if !hashmap.into_iter().all(|(i, v)| match vec.get_mut(i) {
        Some(elem @ None) => {
            *elem = Some(v);
            true
        }
        Some(Some(_elem)) => false,
        None => false,
    }) {
        return None;
    }
    vec.into_iter().collect()
}

#[macro_export]
macro_rules! debug_time {
    ($name:literal $inner:block) => {{
        let __debug_time_start = ::std::time::Instant::now();
        let __debug_time_output = { $inner };
        log::debug!(
            "{}: {:?}",
            $name,
            ::std::time::Instant::now().duration_since(__debug_time_start)
        );
        __debug_time_output
    }};
}
