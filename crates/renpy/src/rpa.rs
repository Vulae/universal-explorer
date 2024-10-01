use anyhow::{anyhow, Result};
use itertools::Itertools;
use std::{
    collections::HashMap,
    io::{Read, Seek},
    sync::{Arc, Mutex},
};
use util::{file_utils::InnerFile, tree_fs::TreeFs};

pub struct RenPyArchive<F: Read + Seek> {
    fs: TreeFs<InnerFile<F>>,
}

impl<F: Read + Seek> RenPyArchive<F> {
    pub fn new(entries: Vec<(String, InnerFile<F>)>) -> Result<Self> {
        Ok(RenPyArchive {
            fs: TreeFs::new(entries)?,
        })
    }

    pub fn load(mut file: F) -> Result<Self> {
        let mut reader = crate::util::reader::Reader::new_le(&mut file);

        let header = reader.read_string(34)?;
        if !header.ends_with('\n') {
            return Err(anyhow!("RenPy .rpa invalid header"));
        }

        let (identifier, offset, xor): (&str, &str, &str) = header
            .trim()
            .split(' ')
            .collect_tuple()
            .ok_or(anyhow!("RenPy .rpa invalid header"))?;

        if identifier != "RPA-3.0" {
            return Err(anyhow!("RenPy .rpa invalid header"));
        }

        let offset = u64::from_be_bytes({
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&crate::util::decode_hex(offset)?);
            arr
        });
        let xor = u32::from_be_bytes({
            let mut arr = [0u8; 4];
            arr.copy_from_slice(&crate::util::decode_hex(xor)?);
            arr
        }) as u64;

        reader.seek(std::io::SeekFrom::Start(offset))?;
        let bytes_remaining = reader.bytes_remaining()?;
        let encoded = reader.read_buf(bytes_remaining as usize)?;
        let pickle = util::pickle::pickle::Value::from_binary(std::io::Cursor::new(encoded), true)?;

        type Entries = HashMap<String, Vec<(u64, u64, serde_json::Value)>>;
        let json = pickle.to_json();
        let entries: Entries = serde_json::from_value(json)?;
        let entries: Entries = entries
            .into_iter()
            .map(|(path, chunks)| {
                (
                    path,
                    chunks
                        .into_iter()
                        .map(|(offset, size, extra)| (offset ^ xor, size ^ xor, extra))
                        .collect(),
                )
            })
            .collect();

        let archive_file = Arc::new(Mutex::new(file));
        let mut files = Vec::new();

        for (path, chunks) in entries {
            if chunks.len() == 0 {
                return Err(anyhow!(
                    "RenPy archive file \"{}\" has no data chunks!",
                    path
                ));
            }
            if chunks.len() > 1 {
                eprintln!("RenPy archive file \"{}\" with more than 1 chunk has been excluded from final file list.", path);
                continue;
            }

            files.push((
                path,
                InnerFile::new(Arc::clone(&archive_file), chunks[0].0, chunks[0].1),
            ));
        }

        RenPyArchive::new(files)
    }
}

impl<F: Read + Seek> crate::util::virtual_fs::VirtualFsInner<InnerFile<F>> for RenPyArchive<F> {
    fn read(
        &mut self,
        path: &str,
    ) -> Result<crate::util::virtual_fs::VirtualFsInnerEntry<InnerFile<F>>> {
        self.fs.read(path)
    }
}
