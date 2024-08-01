
// Yeah I have absolutely no idea on a good way to implement this that actually works.



// use std::{cell::RefCell, io::{Read, Seek}, rc::Rc};



// pub enum VirtualFsItem<F: VirtualFile> {
//     None,
//     File(F),
//     Directory(Vec<VirtualPath<F>>),
// }



// #[derive(Clone)]
// pub struct VirtualPath<F: VirtualFile>(VirtualFs<F>, String);

// impl<F: VirtualFile> VirtualPath<F> {
//     pub fn to_str(&self) -> &str { &self.1 }
//     pub fn fs(&self) -> VirtualFs<F> { &self.0.clone() }
//     pub fn push(&mut self, path: &str) { unimplemented!() }
//     pub fn parent(&self) -> Option<VirtualPath<F>> { unimplemented!() }
//     pub fn get(&self) -> VirtualFsItem<F> { unimplemented!() }
// }



// pub trait VirtualFile: Read + Seek + Clone {
//     fn path(&self) -> VirtualPath<Self>;
// }



// pub trait VirtualFs<F: VirtualFile>: Clone + Sized {
//     fn root(&self) -> VirtualPath<F> {
//         VirtualPath(self.clone(), "/".to_string())
//     }
//     fn path(&self, path: String) -> VirtualPath<F> {
//         let mut p = self.root();
//         p.push(&path);
//         p
//     }
// }



use anyhow::Result;

pub enum VirtualEntry<'a, F> {
    File((&'a str, &'a mut F)),
    Directory((&'a str, &'a mut dyn VirtualDirectory<F>)),
}

pub trait VirtualDirectory<F> {
    fn read_entries(&mut self) -> Result<Vec<VirtualEntry<F>>>;
}


