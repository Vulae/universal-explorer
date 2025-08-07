//! https://developer.valvesoftware.com/wiki/BSP_(Source)
//! Only supports reading embedded LUMP_PAKFILE

use std::io::{Read, Seek};

use thiserror::Error;
use util_general::ReadExt as _;
use util_vfs::{
    VirtualFileSystemError, VirtualFileSystemFile, VirtualFileSystemFileSliced,
    VirtualFileSystemFileTrait, VirtualFileSystemPath,
};

#[derive(Debug, Error)]
pub enum BspError {
    #[error("BSP file has invalid identifier")]
    InvalidIdentifier,
    #[error(transparent)]
    VirtualFileSystemError(#[from] VirtualFileSystemError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

const BSP_IDENTIFIER: [u8; 4] = *b"VBSP";

pub fn is_bsp_file(path: &VirtualFileSystemPath) -> bool {
    path.is_file()
        && path
            .name()
            .map(|name| name.ends_with(".bsp"))
            .unwrap_or(false)
}

const NUM_LUMPS: usize = 64;

const LUMP_PAKFILE: usize = 40;

#[derive(Debug)]
#[allow(unused)]
struct BspLump {
    offset: u32,
    length: u32,
    version: i32,
    uncompressed_length: u32,
}

#[derive(Debug)]
pub enum BspLumpReader {
    Uncompressed(VirtualFileSystemFileSliced),
}

impl BspLumpReader {
    fn new_uncompressed(
        file: VirtualFileSystemFile,
        offset: u64,
        length: u64,
    ) -> Result<Self, BspError> {
        Ok(Self::Uncompressed(VirtualFileSystemFileSliced::new(
            file,
            offset,
            offset + length,
        )?))
    }
}

impl Seek for BspLumpReader {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match self {
            BspLumpReader::Uncompressed(inner) => inner.seek(pos),
        }
    }
}

impl Read for BspLumpReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            BspLumpReader::Uncompressed(inner) => inner.read(buf),
        }
    }
}

impl VirtualFileSystemFileTrait for BspLumpReader {
    fn try_clone_inner(
        &self,
    ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError> {
        match self {
            BspLumpReader::Uncompressed(inner) => {
                Ok(Box::new(Self::Uncompressed(inner.try_clone()?)))
            }
        }
    }
}

#[derive(Debug)]
#[allow(unused)]
struct BspReader {
    file: VirtualFileSystemFile,
    version: i32,
    lumps: [Option<BspLump>; NUM_LUMPS],
    map_revision: i32,
}

impl BspReader {
    fn read(mut reader: VirtualFileSystemFile) -> Result<Self, BspError> {
        reader.rewind()?;

        if reader.read_const()? != BSP_IDENTIFIER {
            return Err(BspError::InvalidIdentifier);
        }

        let version = i32::from_le_bytes(reader.read_const()?);

        let mut lumps = std::array::from_fn(|_| None);
        lumps.iter_mut().try_for_each(|lump| {
            let offset = u32::from_le_bytes(reader.read_const()?);
            let length = u32::from_le_bytes(reader.read_const()?);
            let version = i32::from_le_bytes(reader.read_const()?);
            let fourcc = u32::from_le_bytes(reader.read_const::<4>()?);
            if length != 0 {
                *lump = Some(BspLump {
                    offset,
                    length,
                    version,
                    uncompressed_length: fourcc,
                });
            }
            Ok::<(), BspError>(())
        })?;

        let map_revision = i32::from_le_bytes(reader.read_const()?);

        Ok(Self {
            file: reader,
            version,
            lumps,
            map_revision,
        })
    }

    fn open_lump(&self, index: usize) -> Result<Option<BspLumpReader>, BspError> {
        let Some(Some(lump)) = self.lumps.get(index).map(|i| i.as_ref()) else {
            return Ok(None);
        };
        let file = self.file.try_clone()?;
        if lump.uncompressed_length == 0 {
            Ok(Some(BspLumpReader::new_uncompressed(
                file,
                lump.offset as u64,
                lump.length as u64,
            )?))
        } else {
            log::warn!("BSP compressed lump not yet implemented.");
            Ok(None)
        }
    }
}

pub fn bsp_open_pakfile(
    bsp_file: VirtualFileSystemFile,
) -> Result<Option<BspLumpReader>, BspError> {
    let reader = BspReader::read(bsp_file)?;
    let pakfile = reader.open_lump(LUMP_PAKFILE)?;
    Ok(pakfile)
}
