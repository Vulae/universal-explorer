use super::reader::CompressionMethod;
use std::io::{Read, Seek};
use util::{
    SeekableDeflateDecoder, VirtualFileSystemError, VirtualFileSystemFileSliced,
    VirtualFileSystemFileTrait,
};

#[derive(Debug)]
enum ZipFileEnum {
    Store(VirtualFileSystemFileSliced),
    Deflate(DeflateFile),
}

impl ZipFileEnum {
    fn new(
        slice: VirtualFileSystemFileSliced,
        compression_method: CompressionMethod,
        uncompressed_size: u64,
    ) -> Self {
        match compression_method {
            CompressionMethod::Store => Self::Store(slice),
            CompressionMethod::Delfated => {
                Self::Deflate(DeflateFile::new(slice, uncompressed_size))
            }
            CompressionMethod::LZMA => todo!("ZIP LZMA not supported"),
        }
    }
}

impl Seek for ZipFileEnum {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match self {
            Self::Store(inner) => inner.seek(pos),
            Self::Deflate(inner) => inner.seek(pos),
        }
    }
}

impl Read for ZipFileEnum {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Store(inner) => inner.read(buf),
            Self::Deflate(inner) => inner.read(buf),
        }
    }
}

impl VirtualFileSystemFileTrait for ZipFileEnum {
    fn try_clone_inner(
        &self,
    ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError> {
        match self {
            Self::Store(inner) => inner.try_clone_inner(),
            Self::Deflate(inner) => inner.try_clone_inner(),
        }
    }
}

#[derive(Debug)]
pub struct ZipFile {
    inner: ZipFileEnum,
}

impl ZipFile {
    pub fn new(
        slice: VirtualFileSystemFileSliced,
        compression_method: CompressionMethod,
        uncompressed_size: u64,
    ) -> Self {
        Self {
            inner: ZipFileEnum::new(slice, compression_method, uncompressed_size),
        }
    }
}

impl Seek for ZipFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl Read for ZipFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl VirtualFileSystemFileTrait for ZipFile {
    fn try_clone_inner(
        &self,
    ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError> {
        self.inner.try_clone_inner()
    }
}

#[derive(Debug)]
struct DeflateFile {
    decoder: SeekableDeflateDecoder<VirtualFileSystemFileSliced>,
    uncompressed_size: u64,
}

impl DeflateFile {
    fn new(inner: VirtualFileSystemFileSliced, uncompressed_size: u64) -> Self {
        Self {
            decoder: SeekableDeflateDecoder::new(inner, uncompressed_size),
            uncompressed_size,
        }
    }
}

impl Seek for DeflateFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.decoder.seek(pos)
    }
}

impl Read for DeflateFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.decoder.read(buf)
    }
}

impl VirtualFileSystemFileTrait for DeflateFile {
    fn try_clone_inner(
        &self,
    ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError> {
        Ok(Box::new(Self::new(
            self.decoder.get_ref().try_clone()?,
            self.uncompressed_size,
        )))
    }
}
