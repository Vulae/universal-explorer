use std::{io::Read, path::PathBuf};

/// If filename doesn't have extension, it will instead be added to the end.
pub fn change_extension(filename: &str, new_ext: &str) -> String {
    let Some(ext_pos) = filename
        .char_indices()
        .rev()
        .find_map(|(i, c)| (c == '.').then_some(i))
    else {
        return format!("{filename}.{new_ext}");
    };

    let (name, _) = filename.split_at(ext_pos);
    format!("{name}.{new_ext}")
}

pub fn save_image(
    image: &image::DynamicImage,
    name: Option<&str>,
) -> Result<Option<PathBuf>, image::ImageError> {
    let mut dialog = rfd::FileDialog::new().set_title("Save Image");
    if let Some(name) = name {
        dialog = dialog.set_file_name(name);
    }
    let Some(path) = dialog.save_file() else {
        return Ok(None);
    };

    image.save(&path)?;

    log::info!(
        "Saved image of {}x{} to {path:?}",
        image.width(),
        image.height(),
    );

    Ok(Some(path))
}

pub fn save_stream<R: Read>(
    mut reader: R,
    name: Option<&str>,
) -> Result<Option<PathBuf>, std::io::Error> {
    let mut dialog = rfd::FileDialog::new().set_title("Save Data");
    if let Some(name) = name {
        dialog = dialog.set_file_name(name);
    }
    let Some(path) = dialog.save_file() else {
        return Ok(None);
    };

    let mut writer = std::fs::File::options()
        .write(true)
        .create_new(true)
        .open(&path)?;

    let amount = std::io::copy(&mut reader, &mut writer)?;

    log::info!(
        "Saved stream of {:.2?}KiB to {path:?}",
        (amount as f64) / 1024.0,
    );

    Ok(Some(path))
}
