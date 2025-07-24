use std::{collections::HashMap, io::Seek};

use thiserror::Error;
use util::{
    python::pickle, ReadExt, TreeNode, VirtualFileSystemError, VirtualFileSystemFile,
    VirtualFileSystemFileSliced, VirtualFileSystemFileTrait, VirtualFileSystemPath,
    VirtualFileSystemTrait,
};

#[derive(Debug, Error)]
pub enum RenPyArchiveError {
    #[error("File is not a renpy archive")]
    NotRenPyArchive,
    #[error("Renpy archive version \"{0}\" not supported")]
    NotSupportedVersion(String),
    #[error(transparent)]
    PickleError(#[from] pickle::PickleError),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

pub fn is_rpa_file(path: &VirtualFileSystemPath) -> bool {
    path.is_file()
        && path
            .name()
            .map(|name| name.ends_with(".rpa"))
            .unwrap_or(false)
}

#[derive(Debug)]
struct RenPyArchiveEntry {
    _fullpath: String,
    offset: u64,
    length: u64,
}

#[derive(Debug)]
pub struct RenPyArchive {
    file: VirtualFileSystemFile,
    tree: TreeNode<usize>,
    entries: Vec<RenPyArchiveEntry>,
}

impl RenPyArchive {
    pub fn new(mut file: VirtualFileSystemFile) -> Result<Self, RenPyArchiveError> {
        let header = String::from_utf8(file.read_const::<34>()?.to_vec())
            .map_err(|_| RenPyArchiveError::NotRenPyArchive)?;

        if !header.ends_with('\n') {
            return Err(RenPyArchiveError::NotRenPyArchive);
        }
        let header = &header[..(header.len() - 1)];

        let [identifier, offset, xor]: [&str; 3] = header
            .split(' ')
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| RenPyArchiveError::NotRenPyArchive)?;

        let [ident_name, ident_ver]: [&str; 2] = identifier
            .split('-')
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| RenPyArchiveError::NotRenPyArchive)?;

        if ident_name != "RPA" {
            return Err(RenPyArchiveError::NotRenPyArchive);
        }

        if ident_ver != "3.0" {
            return Err(RenPyArchiveError::NotSupportedVersion(ident_ver.to_owned()));
        }

        let offset = u64::from_str_radix(offset, 16)?;
        #[allow(unused)]
        let xor = u32::from_str_radix(xor, 16)? as u64;

        file.seek(std::io::SeekFrom::Start(offset))?;

        type Entries = HashMap<String, Vec<(u64, u64)>>;
        let mut read_entries: Entries =
            pickle::from_pickle(pickle::Value::from_binary(&mut file, true)?)?;
        read_entries.iter_mut().for_each(|(_key, chunks)| {
            chunks.iter_mut().for_each(|(offset, length)| {
                *offset ^= xor;
                *length ^= xor;
            });
        });

        let mut tree = TreeNode::default();
        let mut entries = Vec::new();

        read_entries.into_iter().for_each(|(fullpath, chunks)| {
            let [(offset, length)] = &chunks[..] else {
                if chunks.is_empty() {
                    log::warn!("RenPyArchive file with 0 chunks skipped");
                } else {
                    log::warn!("RenPyArchive file with more than 1 chunks not supported");
                }
                return;
            };
            if tree.insert(&fullpath, entries.len()) {
                entries.push(RenPyArchiveEntry {
                    _fullpath: fullpath,
                    offset: *offset,
                    length: *length,
                });
            } else {
                log::warn!("RenPyArchive failed to insert into tree \"{fullpath}\"");
            }
        });

        Ok(Self {
            file,
            tree,
            entries,
        })
    }
}

impl VirtualFileSystemTrait for RenPyArchive {
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

        Ok(Box::new(
            VirtualFileSystemFileSliced::new(
                self.file.try_clone()?,
                entry.offset,
                entry.offset + entry.length,
            )
            .map_err(anyhow::Error::from)?,
        ))
    }
}
