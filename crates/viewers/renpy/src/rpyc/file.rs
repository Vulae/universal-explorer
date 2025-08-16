use std::io::{Read, Seek};
use util_general::ReadExt as _;

use thiserror::Error;
use util_python::pickle;
use util_vfs::VirtualFileSystemPath;

#[derive(Debug, Error)]
pub enum RenPyCompiledScriptFileError {
    #[error("Invalid identifier")]
    InvalidIdentifier,
    #[error(transparent)]
    PickleError(#[from] pickle::PickleError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

pub fn is_rpyc_file(path: &VirtualFileSystemPath) -> bool {
    path.is_file()
        && path
            .name()
            .map(|name| name.ends_with(".rpyc"))
            .unwrap_or(false)
}

const IDENTIFIER: [u8; 10] = *b"RENPY RPC2";

#[derive(Debug, Clone)]
pub enum RenPyCompiledScriptSlot {
    Original,
    StaticTransform,
    Unknown(u32),
}

#[derive(Debug, Clone)]
pub struct RenPyCompiledScriptChunk {
    slot: RenPyCompiledScriptSlot,
    offset: u32,
    length: u32,
}

impl RenPyCompiledScriptChunk {
    pub fn slot(&self) -> &RenPyCompiledScriptSlot {
        &self.slot
    }

    pub fn read_raw<R: Read + Seek>(
        &self,
        mut reader: R,
    ) -> Result<Box<[u8]>, RenPyCompiledScriptFileError> {
        reader.seek(std::io::SeekFrom::Start(self.offset as u64))?;
        Ok(reader.read_var(self.length as usize)?)
    }

    pub fn read_pickle<R: Read + Seek>(
        &self,
        reader: R,
    ) -> Result<pickle::Value, RenPyCompiledScriptFileError> {
        let raw = self.read_raw(reader)?;
        Ok(pickle::Value::from_binary(std::io::Cursor::new(raw), true)?)
    }
}

#[derive(Debug)]
pub struct RenPyCompiledScriptFile<R: Read + Seek> {
    reader: R,
    chunks: Box<[RenPyCompiledScriptChunk]>,
}

impl<R: Read + Seek> RenPyCompiledScriptFile<R> {
    pub fn load(mut reader: R) -> Result<Self, RenPyCompiledScriptFileError> {
        reader.rewind()?;

        if reader.read_const()? != IDENTIFIER {
            return Err(RenPyCompiledScriptFileError::InvalidIdentifier);
        }

        let mut chunks = Vec::new();

        loop {
            let slot = u32::from_le_bytes(reader.read_const()?);
            let offset = u32::from_le_bytes(reader.read_const()?);
            let length = u32::from_le_bytes(reader.read_const()?);
            chunks.push(RenPyCompiledScriptChunk {
                slot: match slot {
                    0 => break,
                    1 => RenPyCompiledScriptSlot::Original,
                    2 => RenPyCompiledScriptSlot::StaticTransform,
                    slot => RenPyCompiledScriptSlot::Unknown(slot),
                },
                offset,
                length,
            });
        }

        Ok(Self {
            reader,
            chunks: chunks.into_boxed_slice(),
        })
    }

    pub fn reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    pub fn chunks(&self) -> Box<[RenPyCompiledScriptChunk]> {
        self.chunks.clone()
    }
}
