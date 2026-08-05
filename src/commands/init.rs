use owo_colors::OwoColorize;

use crate::{config, error::Result};

/// Initializes the Dev Knowledge Vault by creating the default config file.
///
/// # Errors
///
/// Returns [`DkvError`] if the config directory cannot be located, created,
/// or the default config cannot be written.
pub fn run() -> Result<()> {
    println!("{}", "Initializing Dev Knowledge Vault".bold());

    config::init()?;

    let path = config::config_path()?;

    println!("{} {}", "Created".green().bold(), path.display());

    Ok(())
}
