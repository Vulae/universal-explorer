
pub const UNIVERSAL_EXPLORER_ICON: egui::ImageSource = egui::include_image!("../../assets/icon.png");
pub const LUCIDE_FOLDER: egui::ImageSource = egui::include_image!("../../assets/lucide/folder.svg");
pub const LUCIDE_FOLDER_OPEN: egui::ImageSource = egui::include_image!("../../assets/lucide/folder-open.svg");
pub const LUCIDE_FILE: egui::ImageSource = egui::include_image!("../../assets/lucide/file.svg");
pub const LUCIDE_FILE_IMAGE: egui::ImageSource = egui::include_image!("../../assets/lucide/file-image.svg");

pub const README: &[u8] = include_bytes!("../../README.md");
pub const LICENSE: &[u8] = include_bytes!("../../LICENSE");
pub const LUCIDE_LICENSE: &[u8] = include_bytes!("../../assets/lucide/LICENSE");




use std::{collections::HashMap, io::Cursor, sync::LazyLock};
use anyhow::anyhow;
use crate::util::virtual_fs::{VirtualFs, VirtualFsInner, VirtualFsInnerEntry};

pub enum AssetsVirtualFsNode {
    File(Vec<u8>),
    Directory(HashMap<String, AssetsVirtualFsNode>),
}

pub struct AssetsVirtualFsInner {
    node: AssetsVirtualFsNode,
}

impl AssetsVirtualFsInner {
    pub fn new(node: AssetsVirtualFsNode) -> Self {
        Self { node }
    }
}

impl VirtualFsInner<Cursor<Vec<u8>>> for AssetsVirtualFsInner {
    fn read(&mut self, path: &str) -> anyhow::Result<VirtualFsInnerEntry<Cursor<Vec<u8>>>> {
        let mut components = path.split('/');

        let mut current = &self.node;
        while let Some(component) = components.next() {
            if component.is_empty() { continue; }
            if let AssetsVirtualFsNode::Directory(entries) = current {
                current = entries.get(component).ok_or(anyhow!("Failed to get entry"))?;
            } else {
                return Err(anyhow!("Failed to get entry"));
            }
        }

        Ok(match current {
            AssetsVirtualFsNode::File(file) => VirtualFsInnerEntry::File(Cursor::new(file.to_vec())),
            AssetsVirtualFsNode::Directory(directory) => VirtualFsInnerEntry::Directory(directory.keys().map(|s| s.to_owned()).collect::<Vec<_>>()),
        })
    }
}

pub static ASSETS_FS: LazyLock<VirtualFs<Cursor<Vec<u8>>, AssetsVirtualFsInner>> = LazyLock::new(|| {

    pub fn image_bytes(image: egui::ImageSource) -> egui::load::Bytes {
        match image {
            egui::ImageSource::Bytes { uri: _, bytes } => bytes,
            _ => unreachable!("image should always be egui::ImageSource::Bytes"),
        }
    }

    VirtualFs::new(AssetsVirtualFsInner::new(AssetsVirtualFsNode::Directory(HashMap::from([
        ("README.md".to_owned(), AssetsVirtualFsNode::File(README.to_vec())),
        ("LICENSE.md".to_owned(), AssetsVirtualFsNode::File(LICENSE.to_vec())),
        ("icon.png".to_owned(), AssetsVirtualFsNode::File(image_bytes(UNIVERSAL_EXPLORER_ICON).to_vec())),
        ("lucide".to_owned(), AssetsVirtualFsNode::Directory(HashMap::from([
            ("LICENSE.md".to_owned(), AssetsVirtualFsNode::File(LUCIDE_LICENSE.to_vec())),
            ("folder.svg".to_owned(), AssetsVirtualFsNode::File(image_bytes(LUCIDE_FOLDER).to_vec())),
            ("folder-open.svg".to_owned(), AssetsVirtualFsNode::File(image_bytes(LUCIDE_FOLDER_OPEN).to_vec())),
            ("file.svg".to_owned(), AssetsVirtualFsNode::File(image_bytes(LUCIDE_FILE).to_vec())),
            ("file-image.svg".to_owned(), AssetsVirtualFsNode::File(image_bytes(LUCIDE_FILE_IMAGE).to_vec())),
        ])))
    ]))))
    
});


