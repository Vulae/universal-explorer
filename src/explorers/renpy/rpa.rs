
use std::{collections::HashMap, io::{Read, Seek}, sync::{Arc, Mutex}};
use anyhow::{anyhow, Result};
use itertools::Itertools;
use crate::util::file::InnerFile;



enum RpaNode<F: Read + Seek> {
    File(InnerFile<F>),
    Directory(Vec<(String, RpaNode<F>)>),
}

impl<F: Read + Seek> RpaNode<F> {
    fn set(&mut self, path: String, file: InnerFile<F>) -> Result<()> {
        let mut components = path.split('/').peekable();

        // Recursively navigate to the correct node
        fn set_recursive<F: Read + Seek>(
            node: &mut RpaNode<F>,
            components: &mut std::iter::Peekable<std::str::Split<'_, char>>,
            file: InnerFile<F>,
        ) -> Result<()> {
            if let Some(component) = components.next() {
                match node {
                    RpaNode::Directory(ref mut children) => {
                        if components.peek().is_none() {
                            // We've reached the final component, insert the file here
                            children.push((component.to_string(), RpaNode::File(file)));
                            Ok(())
                        } else {
                            // Navigate deeper into the directory tree
                            for (name, child) in children.iter_mut() {
                                if let RpaNode::Directory(_) = child {
                                    if name == component {
                                        return set_recursive(child, components, file);
                                    }
                                }
                            }
                            // If the directory does not exist, create it
                            let mut new_dir = RpaNode::Directory(Vec::new());
                            let result = set_recursive(&mut new_dir, components, file);
                            children.push((component.to_string(), new_dir));
                            result
                        }
                    }
                    _ => Err(anyhow!("Path does not match a directory")),
                }
            } else {
                Err(anyhow!("Invalid path"))
            }
        }

        set_recursive(self, &mut components, file)
    }

    fn from_entries(entries: Vec<(String, InnerFile<F>)>) -> Result<Self> {
        let mut root = RpaNode::Directory(Vec::new());
        for (path, file) in entries {
            root.set(path, file)?;
        }

        Ok(root)
    }
}



pub struct RenPyArchive<F: Read + Seek> {
    node: RpaNode<F>,
}

impl<F: Read + Seek> RenPyArchive<F> {
    pub fn new(entries: Vec<(String, InnerFile<F>)>) -> Result<Self> {
        Ok(RenPyArchive {
            node: RpaNode::from_entries(entries)?,
        })
    }

    pub fn load(mut file: F) -> Result<Self> {
        let mut reader = crate::util::reader::Reader::new_le(&mut file);

        let header = reader.read_string(Some(34))?;
        if !header.ends_with('\n') {
            return Err(anyhow!("RenPy .rpa invalid header"));
        }

        let (identifier, offset, xor): (&str, &str, &str) = header.trim().split(' ').collect_tuple().ok_or(anyhow!("RenPy .rpa invalid header"))?;

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
        let pickle = crate::util::pickle::Value::from_binary(std::io::Cursor::new(encoded), true)?;



        type Entries = HashMap<String, Vec<(u64, u64, serde_json::Value)>>;
        let json = pickle.to_json();
        let entries: Entries = serde_json::from_value(json)?;
        let entries: Entries = entries.into_iter().map(|(path, chunks)| {
            (path, chunks.into_iter().map(|(offset, size, extra)| (offset ^ xor, size ^ xor, extra)).collect())
        }).collect();



        let archive_file = Arc::new(Mutex::new(file));
        let mut files = Vec::new();

        for (path, chunks) in entries {
            if chunks.len() == 0 {
                return Err(anyhow!("RenPy archive file \"{}\" has no data chunks!", path));
            }
            if chunks.len() > 1 {
                eprintln!("RenPy archive file \"{}\" with more than 1 chunk has been excluded from final file list.", path);
                continue;
            }

            files.push((path, InnerFile::new(Arc::clone(&archive_file), chunks[0].0, chunks[0].1)));
        }

        RenPyArchive::new(files)
    }
}



impl<F: Read + Seek> crate::util::virtual_fs::VirtualFsInner<InnerFile<F>> for RenPyArchive<F> {
    fn read(&mut self, path: &str) -> Result<crate::util::virtual_fs::VirtualFsInnerEntry<InnerFile<F>>> {
        let mut components = path.split('/');

        let mut current = &self.node;
        while let Some(component) = components.next() {
            if component.is_empty() { continue; }
            if let RpaNode::Directory(entries) = current {
                current = &entries.iter().find(|(name, _)| name == component).unwrap().1;
            } else {
                return Err(anyhow!("Failed to read VPK file from tree"));
            }
        }

        Ok(match current {
            RpaNode::File(file) => crate::util::virtual_fs::VirtualFsInnerEntry::File(file.clone()),
            RpaNode::Directory(entries) => crate::util::virtual_fs::VirtualFsInnerEntry::Directory(entries.iter().map(|e| e.0.clone()).collect::<Vec<_>>()),
        })
    }
}


