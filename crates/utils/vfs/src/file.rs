use std::{fmt::Debug, io::{Read, Seek}};

use crate::{VirtualFileSystemError, VirtualFileSystemPath};

pub trait VirtualFileSystemFileTrait: Read + Seek + Send + Sync {
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
    pub(crate) fn new(path: VirtualFileSystemPath, inner: Box<dyn VirtualFileSystemFileTrait>) -> Self {
        Self { path, inner }
    }

    pub fn __debug_new_from_trait(inner: Box<dyn VirtualFileSystemFileTrait>) -> Self {
        Self {
            path: "/debug".into(),
            inner,
        }
    }

    pub fn __debug_new_from_bytes(slice: Box<[u8]>) -> Self {
        let cursor = std::io::Cursor::new(slice);

        #[allow(non_local_definitions)]
        impl VirtualFileSystemFileTrait for std::io::Cursor<Box<[u8]>> {
            fn try_clone_inner(
                &self,
            ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError> {
                Ok(Box::new(std::io::Cursor::new(self.get_ref().clone())))
            }
        }

        Self {
            path: "/debug".into(),
            inner: Box::new(cursor),
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
