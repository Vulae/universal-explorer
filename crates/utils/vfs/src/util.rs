use std::{
    collections::HashSet,
    io::{Read, Seek},
};

use crate::{VirtualFileSystemError, VirtualFileSystemFile, VirtualFileSystemFileTrait, VirtualFileSystemPath, VirtualFileSystemTrait};

#[derive(Default)]
pub struct MergeFs {
    filesystems: Vec<Box<dyn VirtualFileSystemTrait>>,
}

impl MergeFs {
    pub fn add_fs(&mut self, fs: Box<dyn VirtualFileSystemTrait>) {
        self.filesystems.push(fs);
    }
}

impl VirtualFileSystemTrait for MergeFs {
    fn read_directory_inner(
        &self,
        path: VirtualFileSystemPath,
    ) -> Result<Box<[VirtualFileSystemPath]>, VirtualFileSystemError> {
        let mut has_some_filesystem_with_directory = false;
        let mut entries = HashSet::new();

        self.filesystems.iter().try_for_each(|fs| {
            match fs.read_directory_inner(path.clone()) {
                Ok(fs_entries) => fs_entries.into_iter().for_each(|entry| {
                    has_some_filesystem_with_directory = true;
                    entries.insert(entry);
                }),
                Err(VirtualFileSystemError::DirectoryDoesntExist(_)) => {}
                Err(err) => Err(err)?,
            }
            Ok::<_, VirtualFileSystemError>(())
        })?;

        if has_some_filesystem_with_directory {
            Ok(entries.into_iter().collect())
        } else {
            Err(VirtualFileSystemError::DirectoryDoesntExist(path))
        }
    }

    fn open_file_inner(
        &self,
        path: VirtualFileSystemPath,
    ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError> {
        for fs in self.filesystems.iter() {
            match fs.open_file_inner(path.clone()) {
                Ok(file) => return Ok(file),
                Err(VirtualFileSystemError::FileDoesntExist(_)) => {}
                Err(err) => Err(err)?,
            }
        }

        Err(VirtualFileSystemError::FileDoesntExist(path))
    }
}

#[derive(Debug)]
pub struct VirtualFileSystemFileSliced {
    inner: VirtualFileSystemFile,
    offset: u64,
    start: u64,
    end: u64,
}

impl VirtualFileSystemFileSliced {
    pub fn new(mut inner: VirtualFileSystemFile, start: u64, end: u64) -> std::io::Result<Self> {
        let (start, end) = if start > end {
            (end, start)
        } else {
            (start, end)
        };
        inner.seek(std::io::SeekFrom::Start(start))?;
        Ok(Self {
            inner,
            offset: start,
            start,
            end,
        })
    }

    pub fn try_clone(&self) -> Result<Self, VirtualFileSystemError> {
        Ok(
            Self::new(self.inner.try_clone()?, self.start, self.end)
                .map_err(anyhow::Error::from)?,
        )
    }
}

impl Read for VirtualFileSystemFileSliced {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.offset > self.end {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "End of sliced file",
            ));
        }

        let read_start = self.offset;
        let read_end = u64::min(self.offset + (buf.len() as u64), self.end);
        let read_len = (read_end - read_start) as usize;

        let actual_read_len = self.inner.read(&mut buf[..read_len])?;

        self.offset += actual_read_len as u64;

        Ok(actual_read_len)
    }
}

impl Seek for VirtualFileSystemFileSliced {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.offset = match pos {
            std::io::SeekFrom::Start(offset) => self
                .inner
                .seek(std::io::SeekFrom::Start(self.start + offset))?,
            std::io::SeekFrom::End(offset) => {
                if ((self.end - self.start) as i64) + offset < 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Cannot seek behind the start of the file",
                    ));
                }
                self.inner.seek(std::io::SeekFrom::Start(
                    ((self.end as i64) + offset) as u64,
                ))?
            }
            std::io::SeekFrom::Current(offset) => {
                if ((self.offset - self.start) as i64) + offset < 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Cannot seek behind the start of the file",
                    ));
                }
                self.inner.seek(std::io::SeekFrom::Start(
                    ((self.offset as i64) + offset) as u64,
                ))?
            }
        };
        Ok(self.offset - self.start)
    }
}

impl VirtualFileSystemFileTrait for VirtualFileSystemFileSliced {
    fn try_clone_inner(
        &self,
    ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError> {
        Ok(Box::new(self.try_clone()?))
    }
}

#[cfg(test)]
mod test {
    use std::io::{Seek, SeekFrom};

    use crate::{VirtualFileSystemError, VirtualFileSystemFile, VirtualFileSystemFileTrait};

    use super::VirtualFileSystemFileSliced;

    impl<const N: usize> VirtualFileSystemFileTrait for std::io::Cursor<[u8; N]> {
        fn try_clone_inner(
            &self,
        ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError> {
            Ok(Box::new(std::io::Cursor::new(self.clone().into_inner())))
        }
    }

    #[test]
    fn vfs_file_sliced_seek_test() -> Result<(), anyhow::Error> {
        let testdata: [u8; 1024] = std::array::from_fn(|i| (i % 0xFF) as u8);

        let file =
            VirtualFileSystemFile::__debug_new_from_trait(Box::new(std::io::Cursor::new(testdata)));
        let mut file_slice = VirtualFileSystemFileSliced::new(file.try_clone()?, 10, 20)?;

        assert_eq!(file_slice.seek(SeekFrom::Start(5))?, 5);
        assert_eq!(file_slice.seek(SeekFrom::End(-5))?, 5);
        assert_eq!(file_slice.seek(SeekFrom::Current(-2))?, 3);
        assert_eq!(file_slice.seek(SeekFrom::Current(-3))?, 0);
        assert!(file_slice.seek(SeekFrom::Current(-1)).is_err());
        assert!(file_slice.seek(SeekFrom::End(-11)).is_err());
        assert_eq!(file_slice.seek(SeekFrom::End(-10))?, 0);
        assert_eq!(file_slice.seek(SeekFrom::End(5))?, 15);
        assert_eq!(file_slice.seek(SeekFrom::Start(10))?, 10);

        Ok(())
    }
}
