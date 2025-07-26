/// Implementation of the ZIP archive format.
/// https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT
///
/// SUPPORTED COMPRESSION METHODS:
///     Store (Uncompressed)
///     Deflate
///     LZMA
///
/// NOT SUPPORTED:
///     ZIP64
///     Multi-file archives.
///
use std::io::{Read, Seek};

use file::ZipFile;
use reader::{
    CentralDirectoryHeader, EndOfCentralDirectoryRecord, LocalFileHeader, ZipReader as _,
    ZipStruct as _,
};
use thiserror::Error;
use util::{
    ReadExt as _, TreeNode, VirtualFileSystemError, VirtualFileSystemFile,
    VirtualFileSystemFileSliced, VirtualFileSystemFileTrait, VirtualFileSystemPath,
    VirtualFileSystemTrait,
};

mod file;
mod reader;

#[derive(Debug, Error)]
pub enum ZipError {
    #[error("Invalid signature. expected: {expected:?} but got {got:?}")]
    InvalidSignature { expected: [u8; 4], got: [u8; 4] },
    #[error("Could not find the end of central directory record")]
    CouldNotFindEndOfCentralDirectoryRecord,
    #[error("Unsupported compression method: {0}")]
    UnsupportedCompressionMethod(u16),
    #[error("ZIP feature not supported: {0}")]
    NotSupported(&'static str),
    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

const MAYBE_ZIP_EXTS: &[&str] = &[".zip", ".jar", ".apk"];

pub fn is_zip_file(path: &VirtualFileSystemPath) -> bool {
    path.is_file()
        && path
            .name()
            .map(|name| MAYBE_ZIP_EXTS.iter().any(|ext| name.ends_with(ext)))
            .unwrap_or(false)
}

#[derive(Debug)]
pub struct Zip {
    file: VirtualFileSystemFile,
    tree: TreeNode<CentralDirectoryHeader>,
}

impl Zip {
    fn locate_end_of_central_directory_record<R: Read + Seek>(
        mut reader: R,
    ) -> Result<u64, ZipError> {
        let scans: [u64; 2] = [
            22,          // End of central directory with no comment
            0xFFFF + 22, // End of central directory with max comment size
        ];

        let zip_size = reader.seek(std::io::SeekFrom::End(0))?;

        for scan in scans {
            let scan_start =
                reader.seek(std::io::SeekFrom::Start(zip_size.saturating_sub(scan)))?;
            let mut end_data = Vec::new();
            reader.read_to_end(&mut end_data)?;

            if let Some(location) = end_data.windows(4).enumerate().find_map(|(i, window)| {
                (window == EndOfCentralDirectoryRecord::SIGNATURE).then_some(i)
            }) {
                return Ok(scan_start + location as u64);
            }
        }

        Err(ZipError::CouldNotFindEndOfCentralDirectoryRecord)
    }

    pub fn load(mut file: VirtualFileSystemFile) -> Result<Self, ZipError> {
        let end_of_central_directory_record_location =
            Zip::locate_end_of_central_directory_record(&mut file)?;
        file.seek(std::io::SeekFrom::Start(
            end_of_central_directory_record_location,
        ))?;

        let end_of_central_directory_record = file.read_struct::<EndOfCentralDirectoryRecord>()?;
        if end_of_central_directory_record.num_entries
            != end_of_central_directory_record.num_entries_on_this_disk
        {
            return Err(ZipError::NotSupported("Milti-disk ZIP archive"));
        }

        file.seek(std::io::SeekFrom::Start(
            end_of_central_directory_record.central_directory_offset as u64,
        ))?;
        let mut central_directory_reader = std::io::Cursor::new(
            file.read_var(end_of_central_directory_record.central_directory_size as usize)?,
        );
        let mut central_directory_headers = Vec::new();
        while central_directory_reader.position()
            < (central_directory_reader.get_ref().len() as u64)
        {
            let central_directory_header =
                central_directory_reader.read_struct::<CentralDirectoryHeader>()?;
            central_directory_headers.push(central_directory_header);
        }

        let mut tree = TreeNode::default();
        central_directory_headers
            .into_iter()
            .for_each(|central_directory_header| {
                let path: VirtualFileSystemPath = central_directory_header.filename.clone().into();
                if !path.is_file() {
                    return;
                }
                tree.insert(path.to_str(), central_directory_header);
            });

        Ok(Self { file, tree })
    }
}

impl VirtualFileSystemTrait for Zip {
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
        let Some(TreeNode::Leaf(central_directory_header)) = self.tree.get(path.to_str()) else {
            return Err(VirtualFileSystemError::FileDoesntExist(path));
        };

        let mut zip_file = self.file.try_clone()?;

        zip_file
            .seek(std::io::SeekFrom::Start(
                central_directory_header.local_header_offset as u64,
            ))
            .map_err(anyhow::Error::from)?;
        let local_file_header = zip_file
            .read_struct::<LocalFileHeader>()
            .map_err(anyhow::Error::from)?;

        let start_pos = zip_file.stream_position().map_err(anyhow::Error::from)?;
        let slice = VirtualFileSystemFileSliced::new(
            zip_file,
            start_pos,
            // For some reason local_file_header.compressed_size is 0 sometimes,
            // when central_directory_header.compressed_size says otherwise.
            start_pos + central_directory_header.compressed_size as u64,
        )
        .map_err(anyhow::Error::from)?;

        Ok(Box::new(ZipFile::new(
            slice,
            local_file_header.compression_method,
            local_file_header.uncompressed_size as u64,
        )))
    }
}
