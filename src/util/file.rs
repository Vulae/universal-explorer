
use std::{io::{self, Read, Seek}, path::PathBuf, sync::{Arc, Mutex}};
use anyhow::Result;





pub fn filename<P: Into<PathBuf>>(path: P) -> Option<String> {
    let path: PathBuf = path.into();
    path
        .file_name()
        .map(|s| s.to_str().map(|s| s.to_owned()))
        .flatten()
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

    pub fn size(&mut self) -> Result<FileSize> {
        FileSize::from_file(self)
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



#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct FileSize(u64);

impl FileSize {
    pub fn from_file(mut file: impl Seek) -> Result<Self> {
        let position = file.stream_position()?;
        let size = file.seek(io::SeekFrom::End(0))?;
        file.seek(io::SeekFrom::Start(position))?;
        Ok(Self::from_bytes(size))
    }

    pub fn bytes(&self) -> u64 {
        self.0
    }

    pub fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    pub fn from_kibibytes(kibibytes: u64) -> Self {
        Self(kibibytes * 1024)
    }

    pub fn from_mebibytes(mebibytes: u64) -> Self {
        Self::from_kibibytes(mebibytes * 1024)
    }
}


