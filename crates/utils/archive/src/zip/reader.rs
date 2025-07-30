#![allow(unused)]

use std::io::{Read, Seek};
use util_general::ReadExt as _;

use super::ZipError;

pub trait ZipStruct
where
    Self: Sized,
{
    const SIGNATURE: [u8; 4];

    fn read<R: Read + Seek>(reader: R) -> Result<Self, ZipError>;
}

#[derive(Debug)]
pub struct EndOfCentralDirectoryRecord {
    pub disk: u16,
    pub disk_with_start: u16,
    pub num_entries_on_this_disk: u16,
    pub num_entries: u16,
    pub central_directory_size: u32,
    pub central_directory_offset: u32,
    pub comment: Box<[u8]>,
}

impl ZipStruct for EndOfCentralDirectoryRecord {
    const SIGNATURE: [u8; 4] = *b"PK\x05\x06";

    fn read<R: Read + Seek>(mut reader: R) -> Result<Self, ZipError> {
        Ok(EndOfCentralDirectoryRecord {
            disk: u16::from_le_bytes(reader.read_const()?),
            disk_with_start: u16::from_le_bytes(reader.read_const()?),
            num_entries_on_this_disk: u16::from_le_bytes(reader.read_const()?),
            num_entries: u16::from_le_bytes(reader.read_const()?),
            central_directory_size: u32::from_le_bytes(reader.read_const()?),
            central_directory_offset: u32::from_le_bytes(reader.read_const()?),
            comment: {
                let comment_length = u16::from_le_bytes(reader.read_const()?);
                reader.read_var(comment_length as usize)?
            },
        })
    }
}

#[derive(Debug)]
pub enum CompressionMethod {
    Store,
    Delfated,
    #[allow(clippy::upper_case_acronyms)]
    LZMA,
}

impl TryFrom<u16> for CompressionMethod {
    type Error = ZipError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Store,
            8 => Self::Delfated,
            14 => Self::LZMA,
            _ => return Err(ZipError::UnsupportedCompressionMethod(value)),
        })
    }
}

#[derive(Debug)]
pub struct CentralDirectoryHeader {
    pub version_made: u16,
    pub version_needed: u16,
    pub flags: u16,
    pub compression_method: CompressionMethod,
    pub modification_time: u16,
    pub modification_date: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub filename: String,
    pub extra_field: Box<[u8]>,
    pub comment: Box<[u8]>,
    pub disk_number_start: u16,
    pub internal_attributes: u16,
    pub external_attributes: u32,
    pub local_header_offset: u32,
}

impl ZipStruct for CentralDirectoryHeader {
    const SIGNATURE: [u8; 4] = *b"PK\x01\x02";

    fn read<R: Read + Seek>(mut reader: R) -> Result<Self, ZipError> {
        let version_made = u16::from_le_bytes(reader.read_const()?);
        let version_needed = u16::from_le_bytes(reader.read_const()?);
        let flags = u16::from_le_bytes(reader.read_const()?);
        let compression_method =
            CompressionMethod::try_from(u16::from_le_bytes(reader.read_const()?))?;
        let modification_time = u16::from_le_bytes(reader.read_const()?);
        let modification_date = u16::from_le_bytes(reader.read_const()?);
        let crc32 = u32::from_le_bytes(reader.read_const()?);
        let compressed_size = u32::from_le_bytes(reader.read_const()?);
        let uncompressed_size = u32::from_le_bytes(reader.read_const()?);
        let filename_length = u16::from_le_bytes(reader.read_const()?);
        let extra_field_length = u16::from_le_bytes(reader.read_const()?);
        let comment_length = u16::from_le_bytes(reader.read_const()?);
        let disk_number_start = u16::from_le_bytes(reader.read_const()?);
        let internal_attributes = u16::from_le_bytes(reader.read_const()?);
        let external_attributes = u32::from_le_bytes(reader.read_const()?);
        let local_header_offset = u32::from_le_bytes(reader.read_const()?);
        Ok(Self {
            version_made,
            version_needed,
            flags,
            compression_method,
            modification_time,
            modification_date,
            crc32,
            compressed_size,
            uncompressed_size,
            filename: String::from_utf8(reader.read_var(filename_length as usize)?.into_vec())?,
            extra_field: reader.read_var(extra_field_length as usize)?,
            comment: reader.read_var(comment_length as usize)?,
            disk_number_start,
            internal_attributes,
            external_attributes,
            local_header_offset,
        })
    }
}

#[derive(Debug)]
pub struct LocalFileHeader {
    pub version_needed: u16,
    pub flags: u16,
    pub compression_method: CompressionMethod,
    pub modification_time: u16,
    pub modification_date: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub filename: String,
    pub extra_field: Box<[u8]>,
}

impl ZipStruct for LocalFileHeader {
    const SIGNATURE: [u8; 4] = *b"PK\x03\x04";

    fn read<R: Read + Seek>(mut reader: R) -> Result<Self, ZipError> {
        let version_needed = u16::from_le_bytes(reader.read_const()?);
        let flags = u16::from_le_bytes(reader.read_const()?);
        let compression_method =
            CompressionMethod::try_from(u16::from_le_bytes(reader.read_const()?))?;
        let modification_time = u16::from_le_bytes(reader.read_const()?);
        let modification_date = u16::from_le_bytes(reader.read_const()?);
        let crc32 = u32::from_le_bytes(reader.read_const()?);
        let compressed_size = u32::from_le_bytes(reader.read_const()?);
        let uncompressed_size = u32::from_le_bytes(reader.read_const()?);
        let filename_length = u16::from_le_bytes(reader.read_const()?);
        let extra_field_length = u16::from_le_bytes(reader.read_const()?);
        Ok(Self {
            version_needed,
            flags,
            compression_method,
            modification_time,
            modification_date,
            crc32,
            compressed_size,
            uncompressed_size,
            filename: String::from_utf8(reader.read_var(filename_length as usize)?.into_vec())?,
            extra_field: reader.read_var(extra_field_length as usize)?,
        })
    }
}

pub trait ZipReader: Read + Seek {
    fn read_signature(&mut self, signature: [u8; 4]) -> Result<(), ZipError>;

    fn read_struct<T: ZipStruct>(&mut self) -> Result<T, ZipError> {
        self.read_signature(T::SIGNATURE)?;
        T::read(self)
    }
}

impl<R: Read + Seek> ZipReader for R {
    fn read_signature(&mut self, signature: [u8; 4]) -> Result<(), ZipError> {
        let got = self.read_const::<4>()?;
        if got != signature {
            return Err(ZipError::InvalidSignature {
                expected: signature,
                got,
            });
        }
        Ok(())
    }
}
