
pub mod image;
pub mod source_engine;
pub mod reader;
pub mod texture;
// pub mod virtual_fs;



pub trait TryClone {
    fn try_clone(&self) -> anyhow::Result<Self> where Self: Sized;
}

impl TryClone for std::fs::File {
    fn try_clone(&self) -> anyhow::Result<Self> {
        Ok(self.try_clone()?)
    }
}


