/// https://developer.valvesoftware.com/wiki/VPK_(file_format)
use std::{collections::HashMap, io::Seek};

use log::debug;
use thiserror::Error;
use util::{
    index_hashmap_to_vec, ReadExt as _, TreeNode, VirtualFileSystem, VirtualFileSystemError,
    VirtualFileSystemFile, VirtualFileSystemFileSliced, VirtualFileSystemFileTrait,
    VirtualFileSystemPath, VirtualFileSystemTrait,
};

#[derive(Debug, Error)]
pub enum VPKError {
    #[error("VPK file has invalid identifier")]
    InvalidIdentifier,
    #[error("VPK file has invalid version \"{0}\"")]
    InvalidVersion(u32),
    #[error("VPK file entry has invalid terminator")]
    InvalidTerminator,
    #[error("VPK could not locate archive files: {0}")]
    CouldNotLocateArchiveFiles(&'static str),
    #[error("VPK invalid VPKArchiveFiles")]
    InvalidArchiveFiles,
    #[error("VPK file entry has invalid archive file")]
    FileInvalidArchive,
    #[error(transparent)]
    VirtualFileSystemError(#[from] VirtualFileSystemError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

pub fn is_vpk_file(path: &VirtualFileSystemPath) -> bool {
    path.is_file()
        && path
            .name()
            .map(|name| name.ends_with(".vpk"))
            .unwrap_or(false)
}

const VPK_IDENTIFIER: [u8; 4] = [0x34, 0x12, 0xAA, 0x55];
const VPK_DIR_INDEX: u16 = 0x7FFF;

#[derive(Debug)]
pub struct VPKArchiveFiles {
    dir: VirtualFileSystemFile,
    archives: Vec<VirtualFileSystemFile>,
}

impl VPKArchiveFiles {
    pub fn new(dir: VirtualFileSystemFile, archives: Vec<VirtualFileSystemFile>) -> Self {
        Self { dir, archives }
    }

    pub fn locate_archives<P: Into<VirtualFileSystemPath>>(
        fs: &mut VirtualFileSystem,
        file: P,
    ) -> Result<Self, VPKError> {
        let file: VirtualFileSystemPath = file.into();
        if !file.is_file() {
            todo!();
        }
        let filename = file.name().unwrap();
        if !filename.ends_with(".vpk") {
            todo!();
        }
        let vpk_name = regex::Regex::new(r"^(.+)_(?:dir|\d\d\d)\.vpk$")
            .unwrap()
            .captures(filename)
            .ok_or(VPKError::CouldNotLocateArchiveFiles("File is not VPK"))?
            .get(1)
            .ok_or(VPKError::CouldNotLocateArchiveFiles("File is not VPK"))?
            .as_str();

        let mut dir = None;
        let mut archives = HashMap::new();

        #[allow(unused_must_use)]
        fs.read_directory(file.parent().unwrap())?
            .into_iter()
            .filter_map(|path| {
                let captures = regex::Regex::new(r"^(.+)_(dir|\d\d\d)\.vpk$")
                    .unwrap()
                    .captures(path.name()?)?;
                let name = captures.get(1)?;
                let index_str = captures.get(2)?;
                if name.as_str() != vpk_name {
                    return None::<()>;
                }

                match index_str.as_str() {
                    "dir" => {
                        dir = Some(path);
                    }
                    s if s.parse::<u16>().is_ok() => {
                        archives.insert(s.parse::<u16>().unwrap() as usize, path);
                    }
                    _ => {}
                }

                None
            })
            .collect::<()>();

        Ok(Self {
            dir: fs.open_file(dir.ok_or(VPKError::CouldNotLocateArchiveFiles(
                "Could not find _DIR.vpk",
            ))?)?,
            archives: index_hashmap_to_vec(archives)
                .ok_or(VPKError::CouldNotLocateArchiveFiles(
                    "Failed to construct all _XXX.vpk indexes",
                ))?
                .into_iter()
                .map(|path| fs.open_file(path))
                .collect::<Result<_, _>>()?,
        })
    }

    fn validate(&mut self, max_index: Option<u16>) -> Result<(), VPKError> {
        // TODO: Check filenames
        if let Some(max_index) = max_index {
            if self.archives.is_empty() || (self.archives.len() - 1) != (max_index as usize) {
                return Err(VPKError::InvalidArchiveFiles);
            }
        }
        Ok(())
    }

    pub fn load(self) -> Result<VPK, VPKError> {
        VPK::new(self)
    }
}

#[derive(Debug)]
struct VPKEntry {
    _fullpath: String,
    archive_index: u16,
    offset: u32,
    length: u32,
}

#[derive(Debug)]
pub struct VPK {
    archive_files: VPKArchiveFiles,
    dir_entries_offset: u64,
    tree: TreeNode<usize>,
    entries: Vec<VPKEntry>,
}

impl VPK {
    pub fn new(mut archive_files: VPKArchiveFiles) -> Result<Self, VPKError> {
        let dir = &mut archive_files.dir;
        dir.rewind()?;

        if dir.read_const()? != VPK_IDENTIFIER {
            return Err(VPKError::InvalidIdentifier);
        }

        let version = u32::from_le_bytes(dir.read_const()?);
        if version != 1 && version != 2 {
            return Err(VPKError::InvalidVersion(version));
        }
        let tree_size = u32::from_le_bytes(dir.read_const()?);
        if version == 2 {
            dir.seek_relative(16)?;
        }

        let mut tree_data = std::io::Cursor::new(dir.read_var(tree_size as usize)?);

        let mut tree = TreeNode::default();
        let mut entries = Vec::new();

        fn nonempty_string(string: String) -> Option<String> {
            match string.len() {
                0 => None,
                _ => Some(string),
            }
        }

        let mut max_archive_index: Option<u16> = None;

        while let Some(extension) = nonempty_string(tree_data.read_null_terminated_string()?) {
            while let Some(path) = nonempty_string(tree_data.read_null_terminated_string()?) {
                while let Some(filename) = nonempty_string(tree_data.read_null_terminated_string()?)
                {
                    let _crc = u32::from_le_bytes(tree_data.read_const()?);
                    let preload_bytes = u16::from_le_bytes(tree_data.read_const()?);
                    let archive_index = u16::from_le_bytes(tree_data.read_const()?);
                    let entry_offset = u32::from_le_bytes(tree_data.read_const()?);
                    let entry_length = u32::from_le_bytes(tree_data.read_const()?);
                    if tree_data.read_const()? != [0xFF, 0xFF] {
                        return Err(VPKError::InvalidTerminator);
                    }

                    if archive_index != VPK_DIR_INDEX {
                        let max_archive_index = max_archive_index.get_or_insert_default();
                        *max_archive_index = u16::max(*max_archive_index, archive_index);
                    }

                    let fullpath = format!("{path}/{filename}.{extension}");

                    if preload_bytes != 0 {
                        log::warn!(
                            "VPK entry with preload bytes not supported, skipped \"{fullpath}\""
                        );
                        continue;
                    }

                    if tree.insert(&fullpath, entries.len()) {
                        entries.push(VPKEntry {
                            _fullpath: fullpath,
                            archive_index,
                            offset: entry_offset,
                            length: entry_length,
                        });
                    } else {
                        log::warn!("VPK failed to insert into tree \"{fullpath}\"");
                    }
                }
            }
        }

        archive_files.validate(max_archive_index)?;

        debug!("Opened VPK: \"{}\"", archive_files.dir.path());

        Ok(Self {
            dir_entries_offset: match version {
                1 => 12 + (tree_size as u64),
                2 => 28 + (tree_size as u64),
                _ => unreachable!(),
            },
            archive_files,
            tree,
            entries,
        })
    }
}

impl VirtualFileSystemTrait for VPK {
    fn read_directory_inner(
        &self,
        path: VirtualFileSystemPath,
    ) -> Result<Box<[VirtualFileSystemPath]>, VirtualFileSystemError> {
        let Some(TreeNode::Branch(entries)) = self.tree.get(path.to_str()) else {
            return Err(VirtualFileSystemError::DirectoryDoesntExist(path));
        };
        Ok(entries
            .iter()
            .map(|(name, entry)| {
                let name = match entry {
                    TreeNode::Branch(_) => &format!("{name}/"),
                    TreeNode::Leaf(_) => name,
                };
                let mut path = path.clone();
                path.push(name);
                path
            })
            .collect())
    }

    fn open_file_inner(
        &self,
        path: VirtualFileSystemPath,
    ) -> Result<Box<dyn VirtualFileSystemFileTrait>, VirtualFileSystemError> {
        let Some(TreeNode::Leaf(index)) = self.tree.get(path.to_str()) else {
            return Err(VirtualFileSystemError::FileDoesntExist(path));
        };
        let entry = self.entries.get(*index).unwrap();

        let (archive, offset) = if entry.archive_index == VPK_DIR_INDEX {
            (
                &self.archive_files.dir,
                entry.offset as u64 + self.dir_entries_offset,
            )
        } else {
            (
                self.archive_files
                    .archives
                    .get(entry.archive_index as usize)
                    .ok_or(VPKError::FileInvalidArchive)
                    .map_err(anyhow::Error::from)?,
                entry.offset as u64,
            )
        };

        Ok(Box::new(
            VirtualFileSystemFileSliced::new(
                archive.try_clone()?,
                offset,
                offset + entry.length as u64,
            )
            .map_err(anyhow::Error::from)?,
        ))
    }
}
