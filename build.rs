use std::{env, io};

fn main() -> io::Result<()> {
    if env::var_os("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        winresource::WindowsResource::new()
            .set("FileDescription", "Universal Explorer")
            .set("ProductName", "Universal Explorer")
            .set("InternalName", "universal-explorer")
            .set("Comments", "https://github.com/Vulae/universal-explorer")
            .set_icon("app/assets/icon.ico")
            .set_language(0x0009)
            .compile()?;
    }

    Ok(())
}
