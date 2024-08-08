
pub mod image;
pub mod reader;
pub mod texture;
pub mod egui;
pub mod virtual_fs;
pub mod pickle;

use std::{io::{self, Read, Seek}, num::ParseIntError, path::PathBuf, sync::{Arc, Mutex}};



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



pub struct InnerFile<F: Read + Seek> {
    file: Arc<Mutex<F>>,
    offset: u64,
    size: u64,
    pointer: u64,
}

impl<F: Read + Seek> InnerFile<F> {
    pub fn new(file: Arc<Mutex<F>>, offset: u64, size: u64) -> Self {
        Self { file, offset, size, pointer: 0 }
    }
}

impl<F: Read + Seek> Read for InnerFile<F> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut file = self.file.lock().unwrap();
        file.seek(io::SeekFrom::Start(self.offset + self.pointer))?;

        let max_bytes = (self.size - self.pointer).min(buf.len() as u64) as usize;
        let bytes_read = file.read(&mut buf[..max_bytes])?;
        self.pointer += bytes_read as u64;

        Ok(bytes_read)
    }
}

impl<F: Read + Seek> Seek for InnerFile<F> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let new_pointer = (match pos {
            io::SeekFrom::Start(offset) => Some(offset),
            io::SeekFrom::End(offset) => self.size.checked_add_signed(offset),
            io::SeekFrom::Current(offset) => self.pointer.checked_add_signed(offset),
        }).ok_or(std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek u64 overflow"))?;

        if new_pointer > self.size {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "seek out of bounds"));
        }

        self.pointer = new_pointer;
        Ok(self.pointer)
    }
}

impl<F: Read + Seek> Clone for InnerFile<F> {
    fn clone(&self) -> Self {
        Self { file: self.file.clone(), offset: self.offset, size: self.size, pointer: self.pointer }
    }
}



pub fn decode_hex(hex_string: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..hex_string.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex_string[i..(i + 2)], 16)
        })
        .collect()
}


