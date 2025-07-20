pub mod codec;
pub mod python;
mod read_ext;
mod vfs;

use std::{collections::HashMap, fmt::Debug};

pub use read_ext::*;
pub use vfs::*;

pub fn index_hashmap_to_vec<T: std::fmt::Debug>(hashmap: HashMap<usize, T>) -> Option<Vec<T>> {
    let mut vec: Vec<Option<T>> = Vec::from_iter((0..hashmap.len()).map(|_| None));
    if !hashmap.into_iter().all(|(i, v)| match vec.get_mut(i) {
        Some(elem @ None) => {
            *elem = Some(v);
            true
        }
        Some(Some(_elem)) => false,
        None => false,
    }) {
        return None;
    }
    vec.into_iter().collect()
}

#[macro_export]
macro_rules! debug_time {
    ($name:literal $inner:block) => {{
        let __debug_time_start = ::std::time::Instant::now();
        let __debug_time_output = { $inner };
        log::debug!(
            "{}: {:?}",
            $name,
            ::std::time::Instant::now().duration_since(__debug_time_start)
        );
        __debug_time_output
    }};
}

pub enum TreeNode<T> {
    Branch(HashMap<String, TreeNode<T>>),
    Leaf(T),
}

impl<T: Debug> Debug for TreeNode<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Branch(branch) => f.debug_tuple("Branch").field(branch).finish(),
            Self::Leaf(leaf) => f.debug_tuple("Leaf").field(leaf).finish(),
        }
    }
}

impl<T> Default for TreeNode<T> {
    fn default() -> Self {
        Self::Branch(HashMap::new())
    }
}

impl<T> TreeNode<T> {
    pub fn insert(&mut self, path: &str, value: T) -> bool {
        let TreeNode::Branch(branch) = self else {
            return false;
        };

        if let Some(path_dir_root) = path.find('/') {
            let (root_dir, rest) = path.split_at(path_dir_root);
            branch
                .entry(root_dir.to_owned())
                .or_default()
                .insert(&rest[1..], value)
        } else {
            if branch.contains_key(path) {
                return false;
            }
            if branch
                .insert(path.to_owned(), TreeNode::Leaf(value))
                .is_some()
            {
                unreachable!();
            }
            true
        }
    }

    pub fn get(&self, path: &str) -> Option<&TreeNode<T>> {
        match self {
            TreeNode::Leaf(_) => (!path.is_empty()).then_some(self),
            TreeNode::Branch(branch) => {
                if path.is_empty() {
                    return Some(self);
                }
                if let Some(path_dir_root) = path.find('/') {
                    let (current_dir, rest) = path.split_at(path_dir_root);
                    branch.get(current_dir)?.get(&rest[1..])
                } else {
                    branch.get(path)
                }
            }
        }
    }
}
