use std::io::{Read, Seek};

use util::VirtualFileSystemFile;

use crate::app::AppEvent;

use super::TabTrait;

const CSQ: char = '\'';
const CBS: char = '\\';
#[rustfmt::skip]
const BYTE_CHAR_MAP: [Option<char>; 256] = [
    None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     ,
    None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     ,
    Some(' '), Some('!'), Some('"'), Some('#'), Some('$'), Some('%'), Some('&'), Some(CSQ), Some('('), Some(')'), Some('*'), Some('+'), Some(','), Some('-'), Some('.'), Some('/'),
    Some('0'), Some('1'), Some('2'), Some('3'), Some('4'), Some('5'), Some('6'), Some('7'), Some('8'), Some('9'), Some(':'), Some(';'), Some('<'), Some('='), Some('>'), Some('?'),
    Some('@'), Some('A'), Some('B'), Some('C'), Some('D'), Some('E'), Some('F'), Some('G'), Some('H'), Some('I'), Some('J'), Some('K'), Some('L'), Some('M'), Some('N'), Some('O'),
    Some('P'), Some('Q'), Some('R'), Some('S'), Some('T'), Some('U'), Some('V'), Some('W'), Some('X'), Some('Y'), Some('Z'), Some('['), Some(CBS), Some(']'), Some('^'), Some('_'),
    Some('`'), Some('a'), Some('b'), Some('c'), Some('d'), Some('e'), Some('f'), Some('g'), Some('h'), Some('i'), Some('j'), Some('k'), Some('l'), Some('m'), Some('n'), Some('o'),
    Some('p'), Some('q'), Some('r'), Some('s'), Some('t'), Some('u'), Some('v'), Some('w'), Some('x'), Some('y'), Some('z'), Some('{'), Some('|'), Some('}'), Some('~'), None     ,
    // TODO: https://www.lookuptables.com/text/extended-ascii-table
    None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     ,
    None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     ,
    None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     ,
    None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     ,
    None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     ,
    None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     ,
    None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     ,
    None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     , None     ,
];

const COLOR_POSITION: egui::Color32 = egui::Color32::WHITE;
const COLOR_BYTE: egui::Color32 = egui::Color32::from_gray(200);
const COLOR_NO_CHAR: egui::Color32 = egui::Color32::from_gray(150);
const COLOR_NO_BYTE: egui::Color32 = egui::Color32::from_gray(100);

#[derive(Debug)]
struct HexTabData {
    position: u64,
    length: u64,
    bytes: Box<[u8]>,
}

#[derive(Debug)]
pub struct HexTab {
    name: String,
    drag_delta: f32,
    file: VirtualFileSystemFile,
    position: u64,
    data: Option<HexTabData>,
}

impl HexTab {
    pub fn new(name: String, file: VirtualFileSystemFile) -> Self {
        Self {
            name,
            drag_delta: 0.0,
            file,
            position: 0,
            data: None,
        }
    }

    // TODO: Read bigger range around the selected range, then slice into that range, to not have
    // to read from the file alot.
    fn update_data(&mut self, len: u64) -> Result<&[u8], std::io::Error> {
        if self
            .data
            .as_ref()
            .map(|data| data.position != self.position || data.length != len)
            .unwrap_or(true)
        {
            log::debug!(
                "Read new data for hex view: {}, POS: {}, LEN: {}",
                self.name,
                self.position,
                len,
            );
            self.file.seek(std::io::SeekFrom::Start(self.position))?;
            let mut bytes = vec![0u8; len as usize];
            let read_len = match self.file.read(&mut bytes) {
                Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => Ok(0),
                v => v,
            }?;
            bytes.truncate(read_len);
            self.data = Some(HexTabData {
                position: self.position,
                length: len,
                bytes: bytes.into_boxed_slice(),
            });
        }

        Ok(&self.data.as_ref().unwrap().bytes)
    }
}

impl TabTrait for HexTab {
    fn name(&self) -> egui::WidgetText {
        self.name.as_str().into()
    }

    fn next_event(&mut self) -> Option<AppEvent> {
        None
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let cols = ((ui.available_width() - 92.0) / 28.0).clamp(4.0, 64.0) as u64;
        let rows = ((ui.available_height() - 18.0) / 18.0).max(8.0) as u64;

        self.position = ((self.drag_delta / 18.0).floor() as u64) * cols;

        let pos = self.position;
        let len = cols * rows;
        let data = match self.update_data(len) {
            Ok(data) => data,
            Err(err) => {
                ui.label(format!("Error: {err}"));
                return;
            }
        };

        let grid_res = egui::Grid::new("Hex Grid")
            .striped(true)
            .spacing([8.0, 0.0])
            .min_col_width(0.0)
            .show(ui, |ui| {
                ui.label("");
                ui.label(
                    egui::WidgetText::from(
                        (0..cols)
                            .map(|col| format!("{col:02X}"))
                            .collect::<Vec<_>>()
                            .join(" "),
                    )
                    .color(COLOR_POSITION)
                    .monospace(),
                );
                ui.end_row();

                for row in 0..rows {
                    ui.label(
                        egui::WidgetText::from(format!("#{:08X}:", pos + row * cols))
                            .color(COLOR_POSITION)
                            .monospace(),
                    );

                    let mut str = String::with_capacity((cols as usize) * 3);
                    for col in 0..cols {
                        if let Some(byte) = data.get((col + row * cols) as usize) {
                            str += &format!("{byte:02X} ");
                        }
                    }
                    ui.label(egui::WidgetText::from(&str).color(COLOR_BYTE).monospace());

                    if ((row as usize) * (cols as usize)) < data.len() {
                        ui.horizontal(|ui| {
                            ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
                            for col in 0..cols {
                                if let Some(byte) = data.get((col + row * cols) as usize) {
                                    ui.label(
                                        if let Some(char) = BYTE_CHAR_MAP[(*byte) as usize] {
                                            egui::WidgetText::from(format!("{char}"))
                                                .color(COLOR_BYTE)
                                        } else {
                                            egui::WidgetText::from(".").color(COLOR_NO_CHAR)
                                        }
                                        .monospace(),
                                    );
                                } else {
                                    ui.label(
                                        egui::WidgetText::from(".")
                                            .color(COLOR_NO_BYTE)
                                            .monospace(),
                                    );
                                }
                            }
                        });
                    }
                    ui.end_row();
                }
            })
            .response;

        let interaction = ui.interact(
            grid_res.rect,
            egui::Id::from(self.name.clone()),
            egui::Sense::all(),
        );
        if interaction.dragged() {
            let delta = interaction.drag_delta();
            self.drag_delta -= delta.y;
            if self.drag_delta < 0.0 {
                self.drag_delta = 0.0;
            }
        }
    }
}
