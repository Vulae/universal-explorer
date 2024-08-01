
use std::{fs::{self, File}, io::{Read, Seek}, path::PathBuf};
use anyhow::{anyhow, Result};
use regex::Regex;
use crate::util::TryClone;



pub struct VpkFile<F: Read + Seek + TryClone> {
    archive_file: F,
    path: String,
    offset: u64,
    size: u64,
    preload: Vec<u8>,
    pointer: u64,
}

impl<F: Read + Seek + TryClone> VpkFile<F> {
    pub fn path(&self) -> String {
        self.path.clone()
    }
}

impl<F: Read + Seek + TryClone> Seek for VpkFile<F> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.pointer = match pos {
            std::io::SeekFrom::Start(pointer) => Some(pointer),
            std::io::SeekFrom::End(pointer_end) => self.size.checked_add_signed(pointer_end),
            std::io::SeekFrom::Current(offset) => self.pointer.checked_add_signed(offset),
        }.ok_or(std::io::ErrorKind::Other)?; // FIXME: What error is this supposed to be?
        Ok(self.pointer)
    }
}

impl<F: Read + Seek + TryClone> Read for VpkFile<F> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // TODO: Support preloaded data.
        if !self.preload.is_empty() {
            panic!("VPK file with preload bytes not supported.");
        }
        let offset = self.offset + self.pointer;
        self.pointer += buf.len() as u64;
        self.archive_file.seek(std::io::SeekFrom::Start(offset))?;
        self.archive_file.read(buf)
    }
}

impl<F: Read + Seek + TryClone> TryClone for VpkFile<F> {
    fn try_clone(&self) -> Result<Self> {
        Ok(VpkFile {
            archive_file: self.archive_file.try_clone()?,
            path: self.path.clone(),
            offset: self.offset,
            size: self.size,
            preload: self.preload.clone(),
            pointer: self.pointer,
        })
    }
}



pub struct VpkArchiveFiles<F: Read + Seek + TryClone> {
    pub dir: F,
    pub entries: Vec<F>,
}

impl<F: Read + Seek + TryClone> VpkArchiveFiles<F> {
    pub fn new(dir: F, entries: Vec<F>) -> VpkArchiveFiles<F> {
        VpkArchiveFiles { dir, entries }
    }
}

impl VpkArchiveFiles<File> {
    pub fn locate<P: Into<PathBuf>>(path: P) -> Result<VpkArchiveFiles<File>> {
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
                let open_dir = fs::File::open(dir)?;
                let mut open_entries = Vec::new();
                for entry in entries {
                    open_entries.push(fs::File::open(entry)?);
                }
                return Ok(VpkArchiveFiles::new(open_dir, open_entries));
            }
        }

        Err(anyhow!("Failed to locate VPK archive files."))
    }
}



pub struct VpkArchive<F: Read + Seek + TryClone> {
    pub files: Vec<VpkFile<F>>,
}

impl<F: Read + Seek + TryClone> VpkArchive<F> {
    pub fn new(files: Vec<VpkFile<F>>) -> VpkArchive<F> {
        VpkArchive { files }
    }

    pub fn open<R: Read + Seek + TryClone>(vpk_files: VpkArchiveFiles<R>) -> Result<VpkArchive<R>> {
        let mut reader = crate::util::reader::Reader::new_le(vpk_files.dir.try_clone()?);

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

        let end_of_directory = reader.position() + (tree_size as u64);

        let mut files: Vec<VpkFile<R>> = Vec::new();

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

                    files.push(VpkFile {
                        archive_file: if archive_index == 0x7FFF { vpk_files.dir.try_clone()? } else { vpk_files.entries[archive_index as usize].try_clone()? },
                        path: filename,
                        offset: if archive_index == 0x7FFF { (offset as u64) + end_of_directory } else { offset as u64 },
                        size: size as u64,
                        preload,
                        pointer: 0,
                    });
                }
            }
        }

        Ok(VpkArchive::new(files))
    }
}


