
use std::{collections::HashMap, fmt::Debug};

mod read_ext;

pub use read_ext::*;

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

/// Detects if input sample buffer may be utf-8.
/// Input sample buffer should probably be atleast 32 bytes, or else it will have alot of false
/// positives.
pub fn is_likely_utf8(buf: &[u8]) -> bool {
    // Tries to read as many UTF-8 characters as possible, and if any fails return false.
    // But if we are near the end of the data and it fails, it may just be that character got
    // cut off from the end of the sample buffer.
    let mut read_count = 0;
    for chunk in buf.utf8_chunks() {
        read_count += chunk.valid().len();
        if !chunk.invalid().is_empty() && read_count < buf.len().saturating_sub(4) {
            return false;
        }
    }
    true
}

#[derive(Debug, Clone)]
pub struct LevenshteinDistance {
    pub cost_substitution: usize,
    pub cost_deletion: usize,
    pub cost_insertion: usize,
}

impl LevenshteinDistance {
    pub const fn new(
        cost_substitution: usize,
        cost_deletion: usize,
        cost_insertion: usize,
    ) -> Self {
        Self {
            cost_substitution,
            cost_deletion,
            cost_insertion,
        }
    }

    pub fn distance(&self, a: &str, b: &str) -> usize {
        let a_len = a.chars().count();
        let b_len = b.chars().count();

        if a_len < b_len {
            return self.distance(b, a);
        } else if b_len == 0 {
            return a_len * self.cost_insertion;
        } else if a_len == 0 {
            unreachable!();
        }

        let b_len = b_len + 1;

        let mut cur = vec![0; b_len];

        for (i, ca) in a.char_indices() {
            let mut pre = cur[0];
            cur[0] = i + 1;
            for (j, cb) in b.char_indices() {
                let tmp = cur[j + 1];
                cur[j + 1] = usize::min(
                    // Deletion
                    tmp + self.cost_deletion,
                    usize::min(
                        // Insertion
                        cur[j] + self.cost_insertion,
                        // Matching or subsitution
                        pre + if ca == cb { 0 } else { self.cost_substitution },
                    ),
                );
                pre = tmp;
            }
        }

        cur[b_len - 1]
    }
}
