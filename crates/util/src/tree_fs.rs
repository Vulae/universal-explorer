
use std::io::{Read, Seek};
use anyhow::{Result, anyhow};

use crate::virtual_fs;



enum TreeNode<F: Read + Seek + Clone> {
    File(F),
    Directory(Vec<(String, TreeNode<F>)>),
}

impl<F: Read + Seek + Clone> TreeNode<F> {
    fn set(&mut self, path: String, file: F) -> Result<()> {
        let mut components = path.split('/').peekable();

        // Recursively navigate to the correct node
        fn set_recursive<F: Read + Seek + Clone>(
            node: &mut TreeNode<F>,
            components: &mut std::iter::Peekable<std::str::Split<'_, char>>,
            file: F,
        ) -> Result<()> {
            if let Some(component) = components.next() {
                match node {
                    TreeNode::Directory(ref mut children) => {
                        if components.peek().is_none() {
                            // We've reached the final component, insert the file here
                            children.push((component.to_string(), TreeNode::File(file)));
                            Ok(())
                        } else {
                            // Navigate deeper into the directory tree
                            for (name, child) in children.iter_mut() {
                                if let TreeNode::Directory(_) = child {
                                    if name == component {
                                        return set_recursive(child, components, file);
                                    }
                                }
                            }
                            // If the directory does not exist, create it
                            let mut new_dir = TreeNode::Directory(Vec::new());
                            let result = set_recursive(&mut new_dir, components, file);
                            children.push((component.to_string(), new_dir));
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

    fn from_entries(entries: Vec<(String, F)>) -> Result<Self> {
        let mut root = TreeNode::Directory(Vec::new());
        for (path, file) in entries {
            root.set(path, file)?;
        }

        Ok(root)
    }
}



pub struct TreeFs<F: Read + Seek + Clone> {
    node: TreeNode<F>,
}

impl<F: Read + Seek + Clone> TreeFs<F> {
    pub fn new(entries: Vec<(String, F)>) -> Result<Self> {
        Ok(Self { node: TreeNode::from_entries(entries)? })
    }
}

impl<F: Read + Seek + Clone> virtual_fs::VirtualFsInner<F> for TreeFs<F> {
    fn read(&mut self, path: &str) -> Result<virtual_fs::VirtualFsInnerEntry<F>> {
        let mut components = path.split('/');

        let mut current = &self.node;
        while let Some(component) = components.next() {
            if component.is_empty() { continue; }
            if let TreeNode::Directory(entries) = current {
                current = &entries.iter().find(|(name, _)| name == component).unwrap().1;
            } else {
                return Err(anyhow!("Failed to read file from tree"));
            }
        }

        Ok(match current {
            TreeNode::File(file) => virtual_fs::VirtualFsInnerEntry::File(file.clone()),
            TreeNode::Directory(entries) => virtual_fs::VirtualFsInnerEntry::Directory(entries.iter().map(|e| e.0.clone()).collect::<Vec<_>>()),
        })
    }
}


