
use std::io::{Read, Seek};
use crate::util::virtual_fs::{VirtualFs, VirtualFsEntry, VirtualFsInner};



pub fn render_dropdown_fs<F: Read + Seek, I: VirtualFsInner<F>, C>(ui: &mut egui::Ui, fs: &mut VirtualFs<F, I>, root_name: &str, mut open: C)
where 
    C: FnMut(VirtualFsEntry<F, I>),
{
    
    fn render_entry<F: Read + Seek, I: VirtualFsInner<F>>(ui: &mut egui::Ui, entry: &mut VirtualFsEntry<F, I>, root_name: &str) -> Vec<VirtualFsEntry<F, I>> {

        let mut opened = Vec::new();

        match entry {
            VirtualFsEntry::File(file) => {
                if ui.button(file.path().name().unwrap()).clicked() {
                    opened.push(file.clone().as_entry());
                }
            },
            VirtualFsEntry::Directory(directory) => {
                let name = directory.path().name().unwrap_or(root_name);
                let mut directory = directory.clone();
                let entries_iter = directory.entries();
                ui.menu_button(name, |ui| {
                    for entry in entries_iter {
                        opened.append(&mut render_entry(ui, &mut entry.unwrap(), root_name));
                    }
                });
            },
        }

        opened
    }

    let opened = render_entry(ui, &mut fs.root().unwrap().as_entry(), root_name);

    for entry in opened {
        open(entry);
    }
}


