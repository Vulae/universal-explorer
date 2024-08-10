
use std::{fs::{self, File}, io::{Read, Seek}, path::PathBuf, sync::{Arc, Mutex}};
use anyhow::{anyhow, Result};
use regex::Regex;
use crate::util::file::InnerFile;



pub struct VpkFile<F: Read + Seek> {
    inner: InnerFile<F>,
    preload: Vec<u8>,
}

impl<F: Read + Seek> VpkFile<F> {
    pub fn new(file: Arc<Mutex<F>>, offset: u64, size: u64, preload: Vec<u8>) -> Self {
        Self {
            inner: InnerFile::new(file, offset, size),
            preload,
        }
    }
}

impl<F: Read + Seek> Seek for VpkFile<F> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl<F: Read + Seek> Read for VpkFile<F> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // TODO: Support preloaded data.
        if !self.preload.is_empty() {
            panic!("VPK file with preload bytes not supported.");
        }
        self.inner.read(buf)
    }
}

impl<F: Read + Seek> Clone for VpkFile<F> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone(), preload: self.preload.clone() }
    }
}



pub struct VpkArchiveFiles<F: Read + Seek> {
    pub dir: F,
    pub entries: Vec<F>,
}

impl<F: Read + Seek> VpkArchiveFiles<F> {
    pub fn new(dir: F, entries: Vec<F>) -> Self {
        Self { dir, entries }
    }
}

impl VpkArchiveFiles<File> {
    pub fn locate<P: Into<PathBuf>>(path: P) -> Result<(String, Self)> {
        // TODO: Clean up!
        let path: PathBuf = path.into();
        if !path.is_file() || !path.extension().map(|p| p == "vpk").unwrap_or(false) {
            return Err(anyhow!("Invalid .vpk file"));
        }

        let path_filename = path.file_name().unwrap().to_string_lossy().to_string();
        let path_filename_regex = Regex::new(r"(.+?)(?:_dir|_\d+)?.vpk")?;
        
        if let Some(caps) = path_filename_regex.captures(&path_filename) {
            let archive_name = caps.get(1).unwrap().as_str();

            let mut dir: Option<PathBuf> = None;
            let mut entries: Vec<PathBuf> = Vec::new();
            
            for entry in fs::read_dir(path.parent().unwrap())? {
                let entry = entry.unwrap();
                if entry.path().is_dir() { continue; }

                let filename = entry.file_name();
                let filename = filename.to_str().unwrap();
                let filename_regex = Regex::new(r"(.+?)(?:_(dir|\d+))?\.vpk")?;

                if let Some(caps) = filename_regex.captures(filename) {
                    if caps.get(1).unwrap().as_str() != archive_name {
                        continue;
                    }

                    match caps.get(2) {
                        Some(cap) => {
                            if cap.as_str() == "dir" {
                                dir = Some(entry.path());
                            } else {
                                entries.push(entry.path());
                            }
                        },
                        None => {
                            dir = Some(entry.path())
                        }
                    }
                }
            }

            entries.sort();

            if let Some(dir) = dir {
                let open_dir = fs::File::open(&dir)?;
                let mut open_entries = Vec::new();
                for entry in entries {
                    open_entries.push(fs::File::open(entry)?);
                }
                return Ok((dir.to_string_lossy().to_string(), Self::new(open_dir, open_entries)));
            }
        }

        Err(anyhow!("Failed to locate VPK archive files."))
    }
}



enum VpkNode<F: Read + Seek> {
    File(VpkFile<F>),
    Directory(Vec<(String, VpkNode<F>)>),
}

impl<F: Read + Seek> VpkNode<F> {
    fn set(&mut self, path: String, file: VpkFile<F>) -> Result<()> {
        let mut components = path.split('/').peekable();

        // Recursively navigate to the correct node
        fn set_recursive<F: Read + Seek>(
            node: &mut VpkNode<F>,
            components: &mut std::iter::Peekable<std::str::Split<'_, char>>,
            file: VpkFile<F>,
        ) -> Result<()> {
            if let Some(component) = components.next() {
                match node {
                    VpkNode::Directory(ref mut children) => {
                        if components.peek().is_none() {
                            // We've reached the final component, insert the file here
                            children.push((component.to_string(), VpkNode::File(file)));
                            Ok(())
                        } else {
                            // Navigate deeper into the directory tree
                            for (name, child) in children.iter_mut() {
                                if let VpkNode::Directory(_) = child {
                                    if name == component {
                                        return set_recursive(child, components, file);
                                    }
                                }
                            }
                            // If the directory does not exist, create it
                            let mut new_dir = VpkNode::Directory(Vec::new());
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

    fn from_entries(entries: Vec<(String, VpkFile<F>)>) -> Result<Self> {
        let mut root = VpkNode::Directory(Vec::new());
        for (path, file) in entries {
            root.set(path, file)?;
        }

        Ok(root)
    }
}



pub struct VpkArchive<F: Read + Seek> {
    node: VpkNode<F>,
}

impl<F: Read + Seek> VpkArchive<F> {
    pub fn new(entries: Vec<(String, VpkFile<F>)>) -> Result<Self> {
        Ok(VpkArchive {
            node: VpkNode::from_entries(entries)?,
        })
    }

    pub fn open<R: Read + Seek>(mut vpk_files: VpkArchiveFiles<R>) -> Result<VpkArchive<R>> {
        let mut reader = crate::util::reader::Reader::new_le(&mut vpk_files.dir);

        if &reader.read::<[u8; 4]>()? != b"\x34\x12\xAA\x55" {
            return Err(anyhow!("Invalid .vpk identifier"))
        }
        let version = reader.read::<u32>()?;
        let tree_size = reader.read::<u32>()?;

        match version {
            1 => { },
            2 => { reader.seek(std::io::SeekFrom::Current(16))?; },
            _ => return Err(anyhow!("Unsupported .vpk version.")),
        }

        let end_of_directory = reader.position()? + (tree_size as u64);



        enum ArchiveStoreEntry {
            Dir,
            Entry(u16),
        }

        struct ArchiveStore {
            archive: ArchiveStoreEntry,
            path: String,
            offset: u32,
            size: u32,
            preload: Vec<u8>,
        }

        let mut stores: Vec<ArchiveStore> = Vec::new();

        loop {
            let ext = reader.read_string(None)?;
            if ext.is_empty() { break; }
            loop {
                let path = reader.read_string(None)?;
                if path.is_empty() { break; }
                loop {
                    let name = reader.read_string(None)?;
                    if name.is_empty() { break; }

                    let _crc = reader.read::<u32>()?;
                    let preload_size = reader.read::<u16>()?;
                    let archive_index = reader.read::<u16>()?;
                    let offset = reader.read::<u32>()?;
                    let size = reader.read::<u32>()?;
                    if reader.read::<u16>()? != 0xFFFF {
                        return Err(anyhow!("Malformed .vpk"));
                    }
                    let preload = reader.read_vec::<u8>(preload_size as usize)?;

                    let filename = if path.trim().is_empty() { format!("{}.{}", name, ext) } else { format!("{}/{}.{}", path, name, ext) };

                    stores.push(ArchiveStore {
                        archive: if archive_index == 0x7FFF { ArchiveStoreEntry::Dir } else { ArchiveStoreEntry::Entry(archive_index) },
                        path: filename,
                        offset: if archive_index == 0x7FFF { offset + (end_of_directory as u32) } else { offset },
                        size,
                        preload,
                    });
                }
            }
        }



        let archive_dir = Arc::new(Mutex::new(vpk_files.dir));
        let archive_entries = vpk_files.entries.into_iter().map(|f| Arc::new(Mutex::new(f))).collect::<Vec<_>>();

        let entries = stores.into_iter().map(|s| {
            let archive = match s.archive {
                ArchiveStoreEntry::Dir => Arc::clone(&archive_dir),
                ArchiveStoreEntry::Entry(index) => Arc::clone(&archive_entries[index as usize]),
            };
            (s.path, VpkFile::new(archive, s.offset as u64, s.size as u64, s.preload))
        }).collect::<Vec<_>>();

        Ok(VpkArchive::new(entries)?)
    }
}



impl<F: Read + Seek> crate::util::virtual_fs::VirtualFsInner<VpkFile<F>> for VpkArchive<F> {
    fn read(&mut self, path: &str) -> Result<crate::util::virtual_fs::VirtualFsInnerEntry<VpkFile<F>>> {
        let mut components = path.split('/');

        let mut current = &self.node;
        while let Some(component) = components.next() {
            if component.is_empty() { continue; }
            if let VpkNode::Directory(entries) = current {
                current = &entries.iter().find(|(name, _)| name == component).unwrap().1;
            } else {
                return Err(anyhow!("Failed to read VPK file from tree"));
            }
        }

        Ok(match current {
            VpkNode::File(file) => crate::util::virtual_fs::VirtualFsInnerEntry::File(file.clone()),
            VpkNode::Directory(entries) => crate::util::virtual_fs::VirtualFsInnerEntry::Directory(entries.iter().map(|e| e.0.clone()).collect::<Vec<_>>()),
        })
    }
}


