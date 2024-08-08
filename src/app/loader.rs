
use std::{fs::File, io::{Read, Seek}, path::{Path, PathBuf}};
use super::{Explorer, SharedAppContext};
use anyhow::{anyhow, Result};
use crate::app::explorers;



pub fn open_file<F: Read + Seek>(_app_context: SharedAppContext, mut file: F, filename: Option<String>) -> Result<Option<Box<dyn Explorer>>> {
    // FIXME: Do not clone filename.
    if let Ok(explorer) = explorers::image::ImageExplorer::file(&mut file, filename.clone()) {
        return Ok(Some(Box::new(explorer)));
    }
    if let Ok(explorer) = explorers::source_engine::vtf::VtfExplorer::file(&mut file, filename.clone()) {
        return Ok(Some(Box::new(explorer)));
    }
    if let Ok(explorer) = explorers::text::TextExplorer::file(&mut file, filename.clone()) {
        return Ok(Some(Box::new(explorer)));
    }

    Ok(None)
}

pub fn open<P: AsRef<Path>>(app_context: SharedAppContext, path: P) -> Result<Option<Box<dyn Explorer>>> {
    let path: PathBuf = path.as_ref().into();

    if !path.try_exists()? {
        return Err(anyhow!("Failed to open path."));
    }

    if path.is_file() {
        if let Ok(explorer) = explorers::source_engine::vpk::VpkExplorer::open(app_context.clone(), &path) {
            return Ok(Some(Box::new(explorer)));
        }
        if let Ok(explorer) = explorers::renpy::rpa::RenPyArchiveExplorer::open(app_context.clone(), &path) {
            return Ok(Some(Box::new(explorer)));
        }

        open_file(
            app_context,
            File::open(&path)?,
            crate::util::filename(&path),
        )?;
    }

    Ok(None)
}


