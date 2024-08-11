
// I still don't really know if this is a good way to implement a virtual filesystem.
// Hopefully when I realize it's dumb and stupid it isn't too much of a pain refactoring it.
// (Also, it's kinda dumb having to clone Files or Paths alot of the time.)

use std::{io::{Read, Seek}, marker::PhantomData, sync::{Arc, Mutex}};
use anyhow::{anyhow, Result};



pub enum VirtualFsInnerEntry<F: Read + Seek> {
    File(F),
    Directory(Vec<String>),
}

pub trait VirtualFsInner<F: Read + Seek> {
    fn read(&mut self, path: &str) -> Result<VirtualFsInnerEntry<F>>;
}





#[derive(Clone, Hash, PartialEq, Eq)]
pub struct FullPath(String);

impl FullPath {
    pub fn fix(self) -> Self {
        let string = self.0;
        let string = string.trim_start_matches('/');
        let string = string.trim_start_matches('\\');
        let string = string.replace('\\', "/");
        Self(string)
    }

    pub fn string(&self) -> String {
        self.0.clone()
    }

    pub fn str(&self) -> &str {
        &self.0
    }

    pub fn new<S: Into<String>>(s: S) -> Self {
        Self(s.into()).fix()
    }

    pub fn push<S: Into<String>>(&mut self, component: S) {
        if !self.0.ends_with("/") {
            self.0.push('/');
        }
        self.0.push_str(&component.into());
    }

    pub fn new_path_filename<S1: Into<String>, S2: Into<String>>(s1: S1, s2: S2) -> Self {
        let mut path = Self::new(s1);
        path.push(s2);
        path
    }

    pub fn name(&self) -> Option<&str> {
        std::path::Path::new(&self.0).file_name().and_then(|n| n.to_str())
    }

    pub fn segments(&self) -> Vec<&str> {
        self.0.split('/').collect()
    }

    pub fn parent(&self) -> Option<FullPath> {
        let mut parts = self.segments();
        if parts.is_empty() || (parts.len() == 1 && parts[0].is_empty()) {
            return None;
        }
        parts.pop();
        Some(FullPath::new(parts.join("/")))
    }
}

impl core::fmt::Display for FullPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for FullPath {
    fn from(value: String) -> Self {
        FullPath(value)
    }
}

impl Into<String> for FullPath {
    fn into(self) -> String {
        self.0
    }
}

impl From<&str> for FullPath {
    fn from(value: &str) -> Self {
        FullPath(value.to_string())
    }
}

impl<'a> Into<&'a str> for &'a FullPath {
    fn into(self) -> &'a str {
        &self.0
    }
}





pub struct VirtualFsFile<F: Read + Seek, I: VirtualFsInner<F>> {
    fs: VirtualFs<F, I>,
    path: FullPath,
    file: F,
}

impl<F: Read + Seek, I: VirtualFsInner<F>> VirtualFsFile<F, I> {
    pub fn new(fs: VirtualFs<F, I>, path: FullPath, file: F) -> Self { Self { fs, path, file } }
    pub fn as_entry(self) -> VirtualFsEntry<F, I> { VirtualFsEntry::File(self) }
    pub fn path(&self) -> &FullPath { &self.path }
    pub fn fs(&self) -> &VirtualFs<F, I> { &self.fs }
    pub fn fs_mut(&mut self) -> &mut VirtualFs<F, I> { &mut self.fs }
    pub fn size(&mut self) -> Result<u64> {
        let pos = self.stream_position()?;
        let size = self.seek(std::io::SeekFrom::End(0))?;
        self.seek(std::io::SeekFrom::Start(pos))?;
        Ok(size)
    }
    pub fn save<P: AsRef<std::path::Path>>(&mut self, real_path: P) -> Result<()> {
        if let Some(parent) = real_path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(real_path)?;
        self.rewind()?;
        std::io::copy(self, &mut file)?;

        // println!("Save {} to {:?}", self.path(), real_path.as_ref());

        Ok(())
    }
}

impl<F: Read + Seek, I: VirtualFsInner<F>> Read for VirtualFsFile<F, I> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl<F: Read + Seek, I: VirtualFsInner<F>> Seek for VirtualFsFile<F, I> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl<F: Read + Seek, I: VirtualFsInner<F>> Clone for VirtualFsFile<F, I> {
    fn clone(&self) -> Self {
        self.fs.clone().read(self.path.clone()).unwrap().as_file().unwrap()
    }
}



pub struct VirtualFsDirectory<F: Read + Seek, I: VirtualFsInner<F>> {
    fs: VirtualFs<F, I>,
    path: FullPath,
    entries: Vec<FullPath>,
}

impl<F: Read + Seek, I: VirtualFsInner<F>> VirtualFsDirectory<F, I> {
    pub fn new(fs: VirtualFs<F, I>, path: FullPath, entries: Vec<FullPath>) -> Self { Self { fs, path, entries } }
    pub fn as_entry(self) -> VirtualFsEntry<F, I> { VirtualFsEntry::Directory(self) }
    pub fn path(&self) -> &FullPath { &self.path }
    pub fn fs(&self) -> &VirtualFs<F, I> { &self.fs }
    pub fn fs_mut(&mut self) -> &mut VirtualFs<F, I> { &mut self.fs }
    pub fn size(&mut self) -> Result<u64> {
        let mut total = 0;
        for entry in self.entries() {
            let mut entry = entry?;
            total += entry.size()?;
        }
        Ok(total)
    }
    pub fn save<P: AsRef<std::path::Path>>(&self, real_path: P) -> Result<()> {
        for entry in self.entries_recursive() {
            if let Some(mut file) = entry?.as_file() {
                let mut file_path = real_path.as_ref().to_path_buf();
                file_path.push(file.path().str());
                file.save(file_path)?;
            }
        }

        Ok(())
    }

    pub fn entries_paths(&self) -> impl Iterator<Item = &FullPath> { self.entries.iter() }

    pub fn entries(&self) -> impl Iterator<Item = Result<VirtualFsEntry<F, I>>> + '_ {
        let mut fs = self.fs().clone();
        self.entries_paths().map(move |p| fs.read(p.clone()))
    }
    pub fn entries_recursive(&self) -> Box<impl Iterator<Item = Result<VirtualFsEntry<F, I>>> + '_> {
        let fs = self.fs().clone();
        let entries = self.entries.clone();

        Box::new(std::iter::once(Ok(VirtualFsEntry::Directory(self.clone())))
            .chain(entries.into_iter().flat_map(move |path| {
                let mut fs = fs.clone();
                match fs.read(path.clone()) {
                    Ok(VirtualFsEntry::File(file)) => {
                        Box::new(std::iter::once(Ok(VirtualFsEntry::File(file)))) as Box<dyn Iterator<Item = Result<VirtualFsEntry<F, I>>>>
                    },
                    Ok(VirtualFsEntry::Directory(directory)) => {
                        // FIXME: I'm too dumb to figure out lifetimes.
                        // Should not need to collect then back into iter.
                        Box::new(directory.entries_recursive().collect::<Vec<_>>().into_iter()) as Box<dyn Iterator<Item = Result<VirtualFsEntry<F, I>>>>
                    },
                    Err(e) => Box::new(std::iter::once(Err(e))) as Box<dyn Iterator<Item = Result<VirtualFsEntry<F, I>>>>,
                }.collect::<Vec<_>>()
            })))
    }
}

impl<F: Read + Seek, I: VirtualFsInner<F>> Clone for VirtualFsDirectory<F, I> {
    fn clone(&self) -> Self {
        Self { fs: self.fs.clone(), path: self.path.clone(), entries: self.entries.clone() }
    }
}



pub enum VirtualFsEntry<F: Read + Seek, I: VirtualFsInner<F>> {
    File(VirtualFsFile<F, I>),
    Directory(VirtualFsDirectory<F, I>),
}

impl<F: Read + Seek, I: VirtualFsInner<F>> VirtualFsEntry<F, I> {
    pub fn new_file(fs: VirtualFs<F, I>, path: FullPath, file: F) -> Self {
        VirtualFsEntry::File(VirtualFsFile::new(fs, path, file))
    }
    pub fn new_directory(fs: VirtualFs<F, I>, path: FullPath, entries: Vec<FullPath>) -> Self {
        VirtualFsEntry::Directory(VirtualFsDirectory::new(fs, path, entries))
    }

    pub fn path(&self) -> &FullPath {
        match self {
            VirtualFsEntry::File(file) => file.path(),
            VirtualFsEntry::Directory(directory) => directory.path(),
        }
    }
    pub fn fs(&self) -> &VirtualFs<F, I> {
        match self {
            VirtualFsEntry::File(file) => file.fs(),
            VirtualFsEntry::Directory(directory) => directory.fs(),
        }
    }
    pub fn fs_mut(&mut self) -> &mut VirtualFs<F, I> {
        match self {
            VirtualFsEntry::File(file) => file.fs_mut(),
            VirtualFsEntry::Directory(directory) => directory.fs_mut(),
        }
    }
    pub fn size(&mut self) -> Result<u64> {
        match self {
            VirtualFsEntry::File(file) => file.size(),
            VirtualFsEntry::Directory(directory) => directory.size(),
        }
    }

    pub fn as_file(self) -> Option<VirtualFsFile<F, I>> {
        if let VirtualFsEntry::File(file) = self {
            Some(file)
        } else {
            None
        }
    }
    pub fn as_directory(self) -> Option<VirtualFsDirectory<F, I>> {
        if let VirtualFsEntry::Directory(directory) = self {
            Some(directory)
        } else {
            None
        }
    }
}

impl<F: Read + Seek, I: VirtualFsInner<F>> Clone for VirtualFsEntry<F, I> {
    fn clone(&self) -> Self {
        match self {
            Self::File(file) => Self::File(file.clone()),
            Self::Directory(directory) => Self::Directory(directory.clone()),
        }
    }
}





pub struct VirtualFs<F: Read + Seek, I: VirtualFsInner<F>>(Arc<Mutex<I>>, PhantomData<F>);

impl<F: Read + Seek, I: VirtualFsInner<F>> VirtualFs<F, I> {
    pub fn new(fs: I) -> Self {
        Self(Arc::new(Mutex::new(fs)), PhantomData)
    }

    pub fn read<P: Into<FullPath>>(&mut self, path: P) -> Result<VirtualFsEntry<F, I>> {
        let path: FullPath = path.into();
        match self.0.lock().unwrap().read(path.str()) {
            Ok(VirtualFsInnerEntry::File(file)) => Ok(VirtualFsEntry::new_file(self.clone(), FullPath::new(path), file)),
            Ok(VirtualFsInnerEntry::Directory(entries)) => Ok(VirtualFsEntry::new_directory(
                self.clone(),
                FullPath::new(path.clone()),
                entries.into_iter()
                    .map(|name| FullPath::new_path_filename(path.clone(), name.clone()))
                    .collect::<Vec<_>>(),
            )),
            Err(err) => Err(err),
        }
    }

    pub fn root(&mut self) -> Result<VirtualFsDirectory<F, I>> {
        self.read("")?.as_directory().ok_or(anyhow!("VirtualFs root entry is expected to be directory"))
    }
}

impl<F: Read + Seek, I: VirtualFsInner<F>> Clone for VirtualFs<F, I> {
    fn clone(&self) -> Self {
        VirtualFs(self.0.clone(), self.1.clone())
    }
}


