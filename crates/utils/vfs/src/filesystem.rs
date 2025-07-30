use std::{sync::{Arc, Mutex}, fmt::Debug};

use crate::{VirtualFileSystemError, VirtualFileSystemFile, VirtualFileSystemFileTrait, VirtualFileSystemPath};


/// Read-only virtual filesystem
pub trait VirtualFileSystemTrait: Send + Sync {
    fn read_directory_inner(
        &self,
        path: VirtualFileSystemPath,
    ) -> Result<Box<[VirtualFileSystemPath]>, VirtualFileSystemError>;

    fn open_file_inner(
        &self,
        path: VirtualFileSystemPath,
    ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError>;
}

#[derive(Clone)]
pub struct VirtualFileSystem {
    inner: Arc<Mutex<Box<dyn VirtualFileSystemTrait>>>,
}

impl Debug for VirtualFileSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualFileSystem").finish()
    }
}

unsafe impl Send for VirtualFileSystem {}
unsafe impl Sync for VirtualFileSystem {}

impl VirtualFileSystem {
    pub fn new(inner: Box<dyn VirtualFileSystemTrait>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn read_directory<P: Into<VirtualFileSystemPath>>(
        &self,
        path: P,
    ) -> Result<Box<[VirtualFileSystemPath]>, VirtualFileSystemError> {
        self.inner.lock().unwrap().read_directory_inner(path.into())
    }

    pub fn open_file<P: Into<VirtualFileSystemPath>>(
        &self,
        path: P,
    ) -> Result<VirtualFileSystemFile, VirtualFileSystemError> {
        let path = path.into();
        Ok(VirtualFileSystemFile::new(
            path.clone(),
            self.inner.lock().unwrap().open_file_inner(path)?,
        ))
    }

    pub fn iter_entries<P: Into<VirtualFileSystemPath>>(
        &self,
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
        &self,
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
