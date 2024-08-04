
use std::{collections::HashMap, fs::File, io::{Read, Seek}, path::PathBuf};
use crate::{app::{Explorer, SharedAppContext}, util::source_engine::vpk::{VpkArchive, VpkArchiveFiles, VpkFile}};
use anyhow::{anyhow, Result};
use uuid::Uuid;



enum FileTreeNode<F: Read + Seek> {
    Directory(String, bool, HashMap<String, FileTreeNode<F>>),
    File(String, VpkFile<F>),
}

impl<F: Read + Seek> FileTreeNode<F> {
    pub fn from_vpk(vpk: VpkArchive<F>) -> Result<Self> {
        let mut root = HashMap::new();

        for file in vpk.files {
            let path = file.path();
            let parts = path.split("/").collect::<Vec<&str>>();
            let mut current = &mut root;

            for part in parts.iter().take(parts.len() - 1) {
                current = match current.entry(part.to_string()).or_insert_with(|| FileTreeNode::Directory(part.to_string(), false, HashMap::new())) {
                    FileTreeNode::Directory(_, _, ref mut entries) => entries,
                    FileTreeNode::File(_, _) => return Err(anyhow!("Failed to construct file tree. Expected a directory, but found file.")),
                }
            }

            let file_name = parts.last().unwrap().to_string();
            current.insert(file_name.clone(), FileTreeNode::File(file_name, file));
        }

        Ok(FileTreeNode::Directory("root".to_string(), true, root))
    }
}



pub struct VpkExplorer<F: Read + Seek> {
    app_context: SharedAppContext,
    name: Option<String>,
    node: FileTreeNode<F>,
    uuid: Uuid,
}

impl<F: Read + Seek> VpkExplorer<F> {
    pub fn new(app_context: SharedAppContext, vpk: VpkArchive<F>, name: Option<String>) -> Result<VpkExplorer<F>> {
        Ok(VpkExplorer {
            app_context,
            name,
            node: FileTreeNode::from_vpk(vpk)?,
            uuid: Uuid::now_v7(),
        })
    }

    fn update_node(app_context: &mut SharedAppContext, ui: &mut egui::Ui, node: &mut FileTreeNode<F>) {
        match node {
            FileTreeNode::Directory(name, expanded, entries) => {
                if ui.button(&*name).clicked() {
                    *expanded = !*expanded;
                }
                if *expanded {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            for (_name, entry) in entries {
                                VpkExplorer::update_node(app_context, ui, entry);
                            }
                        });
                    });
                }
            },
            FileTreeNode::File(name, file) => {
                if ui.button(&*name).clicked() {
                    file.rewind().expect("VTF failed to reset file stream position.");
                    app_context.open_file(file.clone(), Some(file.path())).expect("Failed to open VTF file");
                }
            },
        }
    }
}

impl VpkExplorer<File> {
    pub fn open<P: Into<PathBuf>>(app_context: SharedAppContext, path: P) -> Result<VpkExplorer<File>> {
        let path: PathBuf = path.into();
        let (archive_name, vpk_archive_files) = VpkArchiveFiles::locate(&path)?;
        let vpk = VpkArchive::<File>::open(vpk_archive_files)?;
        VpkExplorer::new(
            app_context,
            vpk,
            crate::util::filename(&archive_name),
        )
    }
}

impl<F: Read + Seek> Explorer for VpkExplorer<F> {
    fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn name(&mut self) -> String {
        self.name.clone().unwrap_or("VPK Archive".to_owned())
    }

    fn update(&mut self, ui: &mut egui::Ui) -> Result<()> {
        VpkExplorer::update_node(&mut self.app_context.clone(), ui, &mut self.node);
        Ok(())
    }
}


