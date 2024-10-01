use std::io::{Read, Seek};

use anyhow::{anyhow, Result};

#[derive(Debug)]
pub enum RenPyScriptSlot {
    Original,
    StaticTransform,
    Unknown(u32),
}

impl RenPyScriptSlot {
    fn slot_value(&self) -> u32 {
        match self {
            RenPyScriptSlot::Original => 1,
            RenPyScriptSlot::StaticTransform => 2,
            RenPyScriptSlot::Unknown(slot) => *slot,
        }
    }
}

#[derive(Debug)]
pub struct RenPyScriptChunk {
    pub slot: RenPyScriptSlot,
    pub data: Box<[u8]>,
}

impl RenPyScriptChunk {
    pub fn decompile(&self) -> Result<String> {
        let pickle =
            util::pickle::pickle::Value::from_binary(&mut std::io::Cursor::new(&self.data), true)?;
        Ok(format!("{:?}", pickle))
    }
}

pub struct RenPyScriptReader<R: Read + Seek> {
    data: R,
    chunks: Vec<(u32, u32, u32)>,
}

impl<R: Read + Seek> RenPyScriptReader<R> {
    pub fn new(mut data: R) -> Result<Self> {
        data.rewind()?;
        let mut reader = util::reader::Reader::new_le(&mut data);

        if reader.read_string(10)? != "RENPY RPC2" {
            return Err(anyhow!("Invalid Magic"));
        }

        let mut chunks: Vec<(u32, u32, u32)> = Vec::new();
        loop {
            let slot = reader.read::<u32>()?;
            let offset = reader.read::<u32>()?;
            let length = reader.read::<u32>()?;

            if slot == 0 {
                break;
            }

            chunks.push((slot, offset, length));
        }

        Ok(Self { data, chunks })
    }

    pub fn chunks(&self) -> &[(u32, u32, u32)] {
        &self.chunks
    }

    pub fn read_script_chunk(&mut self, slot: RenPyScriptSlot) -> Result<Option<RenPyScriptChunk>> {
        let Some(chunk) = self.chunks.iter().find(|c| c.0 == slot.slot_value()) else {
            return Ok(None);
        };
        self.data.seek(std::io::SeekFrom::Start(chunk.1 as u64))?;
        let mut data = vec![0u8; chunk.2 as usize];
        self.data.read_exact(&mut data)?;
        Ok(Some(RenPyScriptChunk {
            slot,
            data: data.into_boxed_slice(),
        }))
    }
}
