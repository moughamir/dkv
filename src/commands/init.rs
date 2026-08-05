use owo_colors::OwoColorize;

use crate::{config, error::Result};

pub fn run() -> Result<()> {
    println!("{}", "Initializing Dev Knowledge Vault".bold());

    config::init()?;

    let path = config::config_path()?;

    println!("{} {}", "Created".green().bold(), path.display());

    Ok(())
}
