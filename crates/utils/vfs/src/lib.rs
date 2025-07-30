use std::{
    fmt::{Debug},
};

use thiserror::Error;

mod path;
mod file;
mod filesystem;
mod util;
mod os_fs;

pub use path::*;
pub use file::*;
pub use filesystem::*;
pub use util::*;
pub use os_fs::*;

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
