use std::{
    io::{Read, Seek},
    path::{Path, PathBuf},
};

use super::{
    VirtualFileSystemError, VirtualFileSystemFileTrait, VirtualFileSystemPath,
    VirtualFileSystemTrait,
};

#[derive(Debug)]
pub struct OsFs {
    root: PathBuf,
}

impl OsFs {
    pub fn new<P: AsRef<Path>>(root: P) -> std::io::Result<Self> {
        Ok(Self {
            root: root.as_ref().to_path_buf().canonicalize()?,
        })
    }

    pub fn new_root() -> std::io::Result<Self> {
        Self::new("/")
    }
}

#[derive(Debug)]
struct OsFsFile {
    file: std::fs::File,
    real_path: std::path::PathBuf,
}

impl Read for OsFsFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for OsFsFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl VirtualFileSystemFileTrait for OsFsFile {
    fn try_clone_inner(
        &self,
    ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError> {
        Ok(Box::new(Self {
            file: std::fs::File::open(&self.real_path).map_err(anyhow::Error::from)?,
            real_path: self.real_path.clone(),
        }))
    }
}

impl VirtualFileSystemTrait for OsFs {
    fn read_directory_inner(
        &self,
        path: VirtualFileSystemPath,
    ) -> Result<Box<[VirtualFileSystemPath]>, VirtualFileSystemError> {
        let mut directory_path = PathBuf::from(&self.root);
        directory_path.push(path.to_str());

        let mut entries = Vec::new();

        match std::fs::read_dir(directory_path) {
            Ok(entries) => Ok(entries),
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                return Ok(Vec::new().into_boxed_slice())
            }
            Err(err) => Err(err),
        }
        .map_err(anyhow::Error::from)?
        .try_for_each(|entry| {
            let entry = entry?;
            let meta = entry.metadata()?;

            let binding = entry.file_name();
            let name = binding
                .to_str()
                .ok_or(std::io::Error::other("Failed to parse entry name"))?;

            let mut path = path.clone();

            if meta.is_dir() {
                path.push(&format!("{name}/"));
            } else if meta.is_file() {
                path.push(name);
            } else if meta.is_symlink() {
                log::warn!("OsFs {:?} symlink not yet supported", entry.path());
                return Ok(());
            } else {
                log::error!(
                    "OsFs {:?} is not a directory, file, or symlink",
                    entry.path()
                );
                return Ok(());
            }

            entries.push(path);

            Ok::<_, anyhow::Error>(())
        })?;

        Ok(entries.into_boxed_slice())
    }

    fn open_file_inner(
        &self,
        path: VirtualFileSystemPath,
    ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError> {
        let mut file_path = PathBuf::from(&self.root);
        file_path.push(path.to_str());
        Ok(Box::new(OsFsFile {
            file: std::fs::File::open(&file_path).map_err(anyhow::Error::from)?,
            real_path: file_path,
        }))
    }
}
