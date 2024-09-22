#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

use std::path::PathBuf;

use anyhow::Result;
use app::run_app;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(index = 1)]
    open: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    run_app(&cli.open)?;

    Ok(())
}
