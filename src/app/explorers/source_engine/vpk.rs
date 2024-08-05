
use std::{fs::File, io::{Read, Seek}, path::PathBuf};
use crate::{app::{Explorer, SharedAppContext}, util::source_engine::vpk::{VpkArchive, VpkArchiveFiles, VpkFile}};
use anyhow::{anyhow, Result};
use uuid::Uuid;



enum FileTreeNode<F: Read + Seek> {
    Directory(String, bool, Vec<FileTreeNode<F>>),
    File(String, VpkFile<F>),
}

impl<F: Read + Seek> FileTreeNode<F> {
    fn set(&mut self, path: String, file: VpkFile<F>) -> Result<()> {
        let mut components = path.split('/').peekable();

        // Recursively navigate to the correct node
        fn set_recursive<F: Read + Seek>(
            node: &mut FileTreeNode<F>,
            components: &mut std::iter::Peekable<std::str::Split<'_, char>>,
            file: VpkFile<F>,
        ) -> Result<()> {
            if let Some(component) = components.next() {
                match node {
                    FileTreeNode::Directory(_, _, ref mut children) => {
                        if components.peek().is_none() {
                            // We've reached the final component, insert the file here
                            for child in children.iter_mut() {
                                if let FileTreeNode::File(ref name, _) = child {
                                    if name == component {
                                        *child = FileTreeNode::File(component.to_string(), file);
                                        return Ok(());
                                    }
                                }
                            }
                            children.push(FileTreeNode::File(component.to_string(), file));
                            Ok(())
                        } else {
                            // Navigate deeper into the directory tree
                            for child in children.iter_mut() {
                                if let FileTreeNode::Directory(ref name, _, _) = child {
                                    if name == component {
                                        return set_recursive(child, components, file);
                                    }
                                }
                            }
                            // If the directory does not exist, create it
                            let mut new_dir = FileTreeNode::Directory(component.to_string(), false, Vec::new());
                            let result = set_recursive(&mut new_dir, components, file);
                            children.push(new_dir);
                            result
                        }
                    }
                    _ => Err(anyhow!("Path does not match a directory")),
                }
            } else {
                Err(anyhow!("Invalid path"))
            }
        }

        set_recursive(self, &mut components, file)
    }

    pub fn from_vpk(vpk: VpkArchive<F>) -> Result<Self> {
        let mut root = FileTreeNode::Directory("root".to_string(), true, Vec::new());

        for file in vpk.files {
            root.set(file.path(), file)?;
        }

        Ok(root)
    }

    pub fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self {
            FileTreeNode::Directory(self_name, _, _) => {
                match other {
                    FileTreeNode::Directory(other_name, _, _) => self_name.cmp(other_name),
                    FileTreeNode::File(_, _) => std::cmp::Ordering::Less,
                }
            },
            FileTreeNode::File(self_name, _) => {
                match other {
                    FileTreeNode::Directory(_, _, _) => std::cmp::Ordering::Greater,
                    FileTreeNode::File(other_name, _) => self_name.cmp(other_name),
                }
            },
        }
    }

    pub fn sort_by<S>(&mut self, compare: &S)
    where
        S: Fn(&FileTreeNode<F>, &FileTreeNode<F>) -> std::cmp::Ordering,
    {
        if let FileTreeNode::Directory(_, _, entries) = self {
            entries.sort_by(|a, b| compare(a, b));
            entries.iter_mut().for_each(|entry| entry.sort_by(compare));
        }
    }

    pub fn to_sorted_by<S>(mut self, compare: &S) -> Self
    where 
        S: Fn(&FileTreeNode<F>, &FileTreeNode<F>) -> std::cmp::Ordering,
    {
        self.sort_by(compare);
        self
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
            node: FileTreeNode::from_vpk(vpk)?.to_sorted_by(&|a, b| a.cmp(b)),
            uuid: Uuid::now_v7(),
        })
    }

    fn update_node(app_context: &mut SharedAppContext, ui: &mut egui::Ui, node: &mut FileTreeNode<F>) {
        match node {
            FileTreeNode::Directory(name, expanded, entries) => {
                if ui.add(
                    egui::Button::image_and_text(
                        if !*expanded {
                            egui::Image::new(egui::include_image!("../../../../assets/lucide/folder.svg"))
                                .tint(ui.style().visuals.text_color())
                        } else {
                            egui::Image::new(egui::include_image!("../../../../assets/lucide/folder-open.svg"))
                                .tint(ui.style().visuals.text_color())
                        },
                        &*name,
                    )
                ).clicked() {
                    *expanded = !*expanded;
                }
                if *expanded {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            for entry in entries {
                                VpkExplorer::update_node(app_context, ui, entry);
                            }
                        });
                    });
                }
            },
            FileTreeNode::File(name, file) => {
                if ui.add(
                    egui::Button::image_and_text(
                        egui::Image::new(egui::include_image!("../../../../assets/lucide/file.svg"))
                            .tint(ui.style().visuals.text_color()),
                        &*name,
                    )
                ).clicked() {
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

    fn ui(&mut self, ui: &mut egui::Ui) -> Result<()> {
        // TODO: Implement a Windows-like file explorer alongside this one.
        VpkExplorer::update_node(&mut self.app_context.clone(), ui, &mut self.node);
        Ok(())
    }
}


