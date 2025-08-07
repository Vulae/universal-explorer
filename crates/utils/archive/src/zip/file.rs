use util_codec::{
    LzmaDecodeOptionSize, LzmaDecodeOptions, SeekableDeflateDecoder, SeekableLZMADecoder,
};
use util_vfs::{VirtualFileSystemError, VirtualFileSystemFileSliced, VirtualFileSystemFileTrait};

use super::reader::CompressionMethod;
use std::io::{Read, Seek};

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum ZipFileEnum {
    Store(VirtualFileSystemFileSliced),
    Deflate(DeflateFile),
    #[allow(clippy::upper_case_acronyms)]
    LZMA(LZMAFile),
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
            CompressionMethod::LZMA => Self::LZMA(LZMAFile::new(slice, uncompressed_size)),
        }
    }
}

impl Seek for ZipFileEnum {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match self {
            Self::Store(inner) => inner.seek(pos),
            Self::Deflate(inner) => inner.seek(pos),
            Self::LZMA(inner) => inner.seek(pos),
        }
    }
}

impl Read for ZipFileEnum {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Store(inner) => inner.read(buf),
            Self::Deflate(inner) => inner.read(buf),
            Self::LZMA(inner) => inner.read(buf),
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
            Self::LZMA(inner) => inner.try_clone_inner(),
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

#[derive(Debug)]
struct LZMAFile {
    decoder: SeekableLZMADecoder<VirtualFileSystemFileSliced>,
    uncompressed_size: u64,
}

impl LZMAFile {
    fn new(inner: VirtualFileSystemFileSliced, uncompressed_size: u64) -> Self {
        Self {
            decoder: SeekableLZMADecoder::new(
                inner,
                uncompressed_size,
                LzmaDecodeOptions {
                    // This seems default? But there is a flag for this.
                    // (See section 4.4.4 in APPNOTE)
                    size: LzmaDecodeOptionSize::User(uncompressed_size),
                    zip_format: true,
                },
            ),
            uncompressed_size,
        }
    }
}

impl Seek for LZMAFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.decoder.seek(pos)
    }
}

impl Read for LZMAFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.decoder.read(buf)
    }
}

impl VirtualFileSystemFileTrait for LZMAFile {
    fn try_clone_inner(
        &self,
    ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError> {
        Ok(Box::new(Self::new(
            self.decoder.get_ref().try_clone()?,
            self.uncompressed_size,
        )))
    }
}
