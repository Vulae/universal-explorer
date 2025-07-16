use std::{
    fmt::{Debug, Display},
    io::{Read, Seek},
};

use thiserror::Error;

mod os_fs;
mod util;
pub use os_fs::*;
pub use util::*;

#[derive(Debug, Error)]
pub enum VirtualFileSystemError {
    #[error("Directory \"{0}\" doesn't exist.")]
    DirectoryDoesntExist(VirtualFileSystemPath),
    #[error("File \"{0}\" doesn't exist.")]
    FileDoesntExist(VirtualFileSystemPath),
    #[error("File cannot be cloned")]
    FileCannotBeCloned,
    /// Error caused by external implementation.
    #[error(transparent)]
    ExternalError(#[from] anyhow::Error),
}

/// A path inside a filesystem from the root.
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct VirtualFileSystemPath(String);

impl Display for VirtualFileSystemPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for VirtualFileSystemPath {
    fn from(value: &str) -> Self {
        Self::new(value.to_owned())
    }
}

impl From<String> for VirtualFileSystemPath {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&VirtualFileSystemPath> for VirtualFileSystemPath {
    fn from(value: &VirtualFileSystemPath) -> Self {
        value.clone()
    }
}

impl VirtualFileSystemPath {
    fn new(string: String) -> Self {
        let mut path = Self(string);
        path.fix();
        path
    }

    pub fn fix(&mut self) {
        self.0 = self.0.replace('\\', "/");
        self.0 = self.0.trim_start_matches('/').to_owned();
    }

    pub fn to_str(&self) -> &str {
        &self.0
    }

    pub fn is_root(&self) -> bool {
        self.0 == "/" || self.0.is_empty()
    }

    pub fn is_directory(&self) -> bool {
        self.0.ends_with('/') || self.0.is_empty()
    }

    pub fn is_file(&self) -> bool {
        !self.is_directory()
    }

    pub fn segments(&self) -> impl DoubleEndedIterator<Item = &str> {
        self.0.split('/')
    }

    pub fn name(&self) -> Option<&str> {
        let mut iter = self.segments();
        if self.is_directory() {
            iter.next_back();
        }
        iter.next_back()
    }

    pub fn extension(&self) -> Option<&str> {
        let name = self.name()?;
        let dot_index = name
            .char_indices()
            .rev()
            .find_map(|(i, c)| (c == '.').then_some(i))?;
        Some(&name[(dot_index + 1)..])
    }

    pub fn push(&mut self, segment: &str) {
        if !self.0.ends_with('/') {
            self.0.push('/');
        }
        self.0.push_str(segment);
        self.fix();
    }

    pub fn parent(&self) -> Option<VirtualFileSystemPath> {
        if self.is_directory() {
            let mut segments = self.segments().collect::<Vec<_>>();
            segments.pop();
            if segments.is_empty() {
                return None;
            }
            segments.pop();
            Some(format!("{}/", segments.join("/")).into())
        } else {
            let mut segments = self.segments().collect::<Vec<_>>();
            segments.pop();
            (!segments.is_empty()).then_some(format!("{}/", segments.join("/")).into())
        }
    }
}

pub trait VirtualFileSystemFileTrait: Read + Seek {
    /// Tries to clone self.
    /// Unlike std::fs::File::try_clone, the instances do not share the same handle/are not linked.
    ///
    /// Cloned file read position should be at the start of the file.
    fn try_clone_inner(
        &self,
    ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError>;
}

pub struct VirtualFileSystemFile {
    path: VirtualFileSystemPath,
    inner: Box<dyn VirtualFileSystemFileTrait>,
}

impl Debug for VirtualFileSystemFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualFileSystemFile")
            .field("path", &self.path)
            .field(
                "inner",
                &(&*self.inner as *const dyn VirtualFileSystemFileTrait),
            )
            .finish()
    }
}

impl Read for VirtualFileSystemFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Seek for VirtualFileSystemFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl VirtualFileSystemFile {
    pub fn __debug_new(inner: Box<dyn VirtualFileSystemFileTrait>) -> Self {
        Self {
            path: "".into(),
            inner,
        }
    }

    pub fn path(&self) -> &VirtualFileSystemPath {
        &self.path
    }

    pub fn try_clone(&self) -> Result<Self, VirtualFileSystemError> {
        Ok(Self {
            path: self.path.clone(),
            inner: self.inner.try_clone_inner()?,
        })
    }
}

/// Read-only virtual filesystem
pub trait VirtualFileSystemTrait {
    fn read_directory_inner(
        &mut self,
        path: VirtualFileSystemPath,
    ) -> Result<Box<[VirtualFileSystemPath]>, VirtualFileSystemError>;

    fn open_file_inner(
        &mut self,
        path: VirtualFileSystemPath,
    ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError>;
}

pub struct VirtualFileSystem {
    inner: Box<dyn VirtualFileSystemTrait>,
}

impl Debug for VirtualFileSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualFileSystem")
            .field(
                "inner",
                &(&*self.inner as *const dyn VirtualFileSystemTrait),
            )
            .finish()
    }
}

impl VirtualFileSystem {
    pub fn new(inner: Box<dyn VirtualFileSystemTrait>) -> Self {
        Self { inner }
    }

    pub fn read_directory<P: Into<VirtualFileSystemPath>>(
        &mut self,
        path: P,
    ) -> Result<Box<[VirtualFileSystemPath]>, VirtualFileSystemError> {
        self.inner.read_directory_inner(path.into())
    }

    pub fn open_file<P: Into<VirtualFileSystemPath>>(
        &mut self,
        path: P,
    ) -> Result<VirtualFileSystemFile, VirtualFileSystemError> {
        let path = path.into();
        Ok(VirtualFileSystemFile {
            path: path.clone(),
            inner: self.inner.open_file_inner(path)?,
        })
    }

    pub fn iter_entries<P: Into<VirtualFileSystemPath>>(
        &mut self,
        path: P,
    ) -> impl Iterator<Item = Result<VirtualFileSystemPath, VirtualFileSystemError>> {
        let mut entries: Vec<VirtualFileSystemPath> = vec![path.into()];

        std::iter::from_fn(move || {
            entries.pop().map(|entry| {
                if entry.is_directory() {
                    let mut subentries = self.read_directory(&entry)?;
                    subentries.sort();
                    entries.append(&mut subentries.into_vec());
                }
                Ok(entry)
            })
        })
    }

    pub fn debug_print<P: Into<VirtualFileSystemPath>, F: Fn(&VirtualFileSystemPath) -> bool>(
        &mut self,
        path: P,
        filter: F,
    ) -> Result<(), VirtualFileSystemError> {
        const DEBUG_MAX_DEPTH: u64 = 20;
        const DEBUG_MAX_SEARCH_ENTRIES: u64 = 1000;

        let mut entries = vec![(path.into(), 0)];
        let mut num_entries = 0;

        'outer: while let Some((entry, depth)) = entries.pop() {
            if !filter(&entry) {
                continue;
            }

            if entry.is_directory() {
                if depth >= DEBUG_MAX_DEPTH {
                    log::debug!("VirtualFileSystemExt::debug_print stopped, exceeded max depth");
                    break 'outer;
                }

                let mut subentries = self.read_directory(&entry)?;
                subentries.sort();

                for entry in subentries {
                    entries.push((entry, depth + 1));
                }
            } else if entry.is_file() {
                num_entries += 1;
                if num_entries > DEBUG_MAX_SEARCH_ENTRIES {
                    log::debug!("VirtualFileSystemExt::debug_print stopped, exceeded max entries");
                    break 'outer;
                }
            }

            println!(
                "{}{}{}",
                "┃ ".repeat((depth as usize).saturating_sub(1)),
                if depth > 0 { "┣━" } else { "" },
                entry.name().unwrap_or("")
            );
        }

        Ok(())
    }
}
