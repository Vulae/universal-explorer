use std::io::{Read, Seek};

use anyhow::Result;
use uuid::Uuid;

use crate::{app::Explorer, explorers::text::TextExplorer};

pub struct RenPyScriptExplorer {
    explorer: TextExplorer,
}

impl RenPyScriptExplorer {
    pub fn file(data: impl Read + Seek, filename: Option<String>) -> Result<Self> {
        let mut reader = renpy::rpyc::RenPyScriptReader::new(data)?;
        let script = reader.read_script_chunk(renpy::rpyc::RenPyScriptSlot::Original)?;
        let string = if let Some(script) = script {
            match script.decompile() {
                Ok(string) => string,
                Err(err) => format!("Error while parsing: {:#?}", err),
            }
        } else {
            "Failed to find script".to_owned()
        };
        Ok(Self {
            explorer: TextExplorer::new(string, filename),
        })
    }
}

impl Explorer for RenPyScriptExplorer {
    fn uuid(&self) -> &Uuid {
        self.explorer.uuid()
    }

    fn title(&self) -> String {
        self.explorer.title()
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        self.explorer.ui(ui);
    }
}
