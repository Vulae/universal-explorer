
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
    pub fn string(&self) -> String {
        self.0.clone()
    }

    pub fn str(&self) -> &str {
        &self.0
    }

    pub fn new<S: Into<String>>(s: S) -> Self {
        Self(s.into())
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



pub enum VirtualFsEntry<F: Read + Seek, I: VirtualFsInner<F>> {
    File(VirtualFs<F, I>, FullPath, F),
    Directory(VirtualFs<F, I>, FullPath, Vec<FullPath>),
}

impl<F: Read + Seek, I: VirtualFsInner<F>> VirtualFsEntry<F, I> {
    pub fn path(&self) -> FullPath {
        match self {
            VirtualFsEntry::File(_, path, _) => path.clone(),
            VirtualFsEntry::Directory(_, path, _) => path.clone(),
        }
    }

    pub fn fs(&self) -> VirtualFs<F, I> {
        match self {
            VirtualFsEntry::File(fs, _, _) => fs.clone(),
            VirtualFsEntry::Directory(fs, _, _) => fs.clone(),
        }
    }

    pub fn is_file(&self) -> bool {
        match self {
            VirtualFsEntry::File(_, _, _) => true,
            VirtualFsEntry::Directory(_, _, _) => false,
        }
    }

    pub fn is_directory(&self) -> bool {
        match self {
            VirtualFsEntry::File(_, _, _) => false,
            VirtualFsEntry::Directory(_, _, _) => true,
        }
    }

    pub fn as_file(&mut self) -> Option<&mut F> {
        match self {
            VirtualFsEntry::File(_, _, file) => Some(file),
            VirtualFsEntry::Directory(_, _, _) => None,
        }
    }

    pub fn as_directory(&self) -> Option<Vec<FullPath>> {
        match self {
            VirtualFsEntry::File(_, _, _) => None,
            VirtualFsEntry::Directory(_, _, entries) => Some(entries.clone()),
        }
    }

    pub fn children(&mut self) -> Result<Vec<VirtualFsEntry<F, I>>> {
        match self {
            VirtualFsEntry::File(_, _, _) => Err(anyhow!("Cannot get children on file entry")),
            VirtualFsEntry::Directory(fs, _, entries) => Ok(
                entries
                    .iter()
                    .map(|entry| fs.read(entry.string()))
                    .collect::<Result<Vec<_>>>()?
            ),
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
            Ok(VirtualFsInnerEntry::File(file)) => Ok(VirtualFsEntry::File(
                self.clone(),
                FullPath::new(path),
                file
            )),
            Ok(VirtualFsInnerEntry::Directory(entries)) => Ok(VirtualFsEntry::Directory(
                self.clone(),
                FullPath::new(path.clone()),
                entries.into_iter()
                    .map(|name| FullPath::new_path_filename(path.clone(), name.clone()))
                    .collect::<Vec<_>>()
            )),
            Err(err) => Err(err),
        }
    }
}

impl<F: Read + Seek, I: VirtualFsInner<F>> Clone for VirtualFs<F, I> {
    fn clone(&self) -> Self {
        VirtualFs(self.0.clone(), self.1.clone())
    }
}


