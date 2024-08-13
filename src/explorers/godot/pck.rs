
// https://github.com/Bioruebe/godotdec/blob/master/godotdec/Program.cs

use std::{io::{Read, Seek}, sync::{Arc, Mutex}};
use anyhow::{anyhow, Result};
use bitflags::bitflags;
use regex::Regex;
use crate::util::{file::InnerFile, tree_fs::TreeFs};



bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct GodotPckArchiveFlags: u32 {
        const ENCRYPTED_ARCHIVE = 1 << 0;
    }

    #[derive(Debug, Clone, Copy)]
    struct GodotPckFileFlags: u32 {
        const ENCRYPTED_FILE = 1 << 0;
    }
}



fn fix_path(path: String) -> Result<String> {
    let path = path.trim_end_matches('\0');

    let path_regex = Regex::new(r"^(.+?):\/\/(.+)$")?;

    if let Some(caps) = path_regex.captures(&path) {
        let (base, rest) = (
            caps.get(1).unwrap().as_str(),
            caps.get(2).unwrap().as_str(),
        );

        return Ok(format!("{}/{}", base, rest));
    }

    return Err(anyhow!("Invalid path"));
}



pub struct GodotPck<F: Read + Seek> {
    fs: TreeFs<InnerFile<F>>,
}

impl<F: Read + Seek> GodotPck<F> {
    pub fn load(mut data: F) -> Result<Self> {
        data.rewind()?;
        let mut reader = crate::util::reader::Reader::new_le(&mut data);

        if &reader.read::<[u8; 4]>()? != b"GDPC" {
            return Err(anyhow!("GodotPck identifier doesn't match"));
        }

        let pak_version = reader.read::<i32>()?;
        let _godot_version = reader.read::<[i32; 3]>()?;

        let (flags, files_base_offset) = match pak_version {
            1 => (GodotPckArchiveFlags::empty(), 0),
            2 => (GodotPckArchiveFlags::from_bits_retain(reader.read::<u32>()?), reader.read::<u64>()?),
            _ => return Err(anyhow!("GodotPck version {} not supported.", pak_version)),
        };
        if flags.contains(GodotPckArchiveFlags::ENCRYPTED_ARCHIVE) {
            return Err(anyhow!("GodotPck encrypted archive not supported"));
        }

        reader.skip(16 * 4)?;

        let file_count = reader.read::<i32>()?;

        let mut entries: Vec<(String, u64, u64)> = Vec::new();

        for _ in 0..file_count {
            let path = reader.read_length_string::<i32>()?;
            let offset = reader.read::<u64>()?;
            let length = reader.read::<u64>()?;

            let real_offset = offset + files_base_offset;

            reader.skip(16)?;

            let flags = match pak_version {
                1 => GodotPckFileFlags::empty(),
                2 => GodotPckFileFlags::from_bits_retain(reader.read::<u32>()?),
                _ => unreachable!(),
            };
            if flags.contains(GodotPckFileFlags::ENCRYPTED_FILE) {
                println!("GodotPck encrypted file excluded from archive. \"{}\"", path);
                continue;
            }

            entries.push((path, real_offset, length));
        }

        let file = Arc::new(Mutex::new(data));

        Ok(Self {
            fs: TreeFs::new(
                entries
                    .into_iter()
                    .map(|(path, offset, size)| {
                        Ok((
                            fix_path(path)?,
                            InnerFile::new(Arc::clone(&file), offset, size),
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?
            )?,
        })
    }
}



impl<F: Read + Seek> crate::util::virtual_fs::VirtualFsInner<InnerFile<F>> for GodotPck<F> {
    fn read(&mut self, path: &str) -> Result<crate::util::virtual_fs::VirtualFsInnerEntry<InnerFile<F>>> {
        self.fs.read(path)
    }
}


