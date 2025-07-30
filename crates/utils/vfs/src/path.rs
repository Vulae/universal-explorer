use std::fmt::Display;

/// A path inside a filesystem from the root.
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct VirtualFileSystemPath(String);

impl Display for VirtualFileSystemPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for VirtualFileSystemPath {
    fn from(value: &str) -> Self {
        Self::new(value.to_owned())
    }
}

impl From<String> for VirtualFileSystemPath {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&VirtualFileSystemPath> for VirtualFileSystemPath {
    fn from(value: &VirtualFileSystemPath) -> Self {
        value.clone()
    }
}

impl VirtualFileSystemPath {
    fn new(string: String) -> Self {
        let mut path = Self(string);
        path.fix();
        path
    }

    pub fn fix(&mut self) {
        self.0 = self.0.replace('\\', "/");
        self.0 = self.0.trim_start_matches('/').to_owned();
    }

    pub fn to_str(&self) -> &str {
        &self.0
    }

    pub fn is_root(&self) -> bool {
        self.0 == "/" || self.0.is_empty()
    }

    pub fn is_directory(&self) -> bool {
        self.0.ends_with('/') || self.0.is_empty()
    }

    pub fn is_file(&self) -> bool {
        !self.is_directory()
    }

    pub fn segments(&self) -> impl DoubleEndedIterator<Item = &str> {
        self.0.split('/')
    }

    pub fn name(&self) -> Option<&str> {
        let mut iter = self.segments();
        if self.is_directory() {
            iter.next_back();
        }
        iter.next_back()
    }

    pub fn extension(&self) -> Option<&str> {
        let name = self.name()?;
        let dot_index = name
            .char_indices()
            .rev()
            .find_map(|(i, c)| (c == '.').then_some(i))?;
        Some(&name[(dot_index + 1)..])
    }

    pub fn push(&mut self, segment: &str) {
        if !self.0.ends_with('/') {
            self.0.push('/');
        }
        self.0.push_str(segment);
        self.fix();
    }

    pub fn parent(&self) -> Option<VirtualFileSystemPath> {
        if self.is_directory() {
            let mut segments = self.segments().collect::<Vec<_>>();
            segments.pop();
            if segments.is_empty() {
                return None;
            }
            segments.pop();
            Some(format!("{}/", segments.join("/")).into())
        } else {
            let mut segments = self.segments().collect::<Vec<_>>();
            segments.pop();
            (!segments.is_empty()).then_some(format!("{}/", segments.join("/")).into())
        }
    }
}