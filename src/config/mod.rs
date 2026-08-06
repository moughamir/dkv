//! Configuration loading and validation.
use std::{
    fs,
    path::{Path, PathBuf},
};

use directories::BaseDirs;

use crate::{
    error::{DkvError, Result},
    types::Config,
};

const CONFIG_DIR: &str = "dkv";
const CONFIG_FILE: &str = "config.toml";

/// Returns the base configuration directory.
///
/// # Errors
///
/// Returns [`DkvError`] if the user's HOME directory cannot be located.
pub fn config_dir() -> Result<PathBuf> {
    let base = BaseDirs::new()
        .ok_or_else(|| DkvError::Message("Unable to locate HOME directory".into()))?;

    Ok(base.config_dir().join(CONFIG_DIR))
}

/// Returns the full path to the configuration file.
///
/// # Errors
///
/// Returns [`DkvError`] if the config directory cannot be located.
pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(CONFIG_FILE))
}

/// Returns `true` if a configuration file already exists.
///
/// # Errors
///
/// Returns [`DkvError`] if the config directory cannot be located.
pub fn exists() -> Result<bool> {
    Ok(config_path()?.exists())
}

/// Loads the configuration from disk.
///
/// # Errors
///
/// Returns [`DkvError`] if the config directory cannot be located, the file
/// cannot be read, or its contents are not valid TOML.
pub fn load() -> Result<Config> {
    let path = config_path()?;
    let text = fs::read_to_string(path)?;
    Ok(toml::from_str(&text)?)
}

/// Loads the configuration from disk, falling back to [`Config::default`]
/// when no config file exists yet.
///
/// # Errors
///
/// Returns [`DkvError`] if the config directory cannot be located, the file
/// cannot be read, or its contents are not valid TOML.
pub fn load_or_default() -> Result<Config> {
    let path = config_path()?;

    if !path.exists() {
        return Ok(Config::default());
    }

    load()
}

/// Saves the given configuration to disk, creating parent directories as
/// needed.
///
/// # Errors
///
/// Returns [`DkvError`] if the config path cannot be determined, the parent
/// directories cannot be created, the config cannot be serialized to TOML, or
/// the file cannot be written.
pub fn save(config: &Config) -> Result<()> {
    let path = config_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let toml = toml::to_string_pretty(config)?;

    fs::write(path, toml)?;

    Ok(())
}

/// Creates the default configuration file if one does not already exist.
///
/// # Errors
///
/// Returns [`DkvError`] if the config directory cannot be located or the
/// default config cannot be saved.
pub fn init() -> Result<()> {
    if exists()? {
        return Ok(());
    }

    save(&Config::default())
}

/// Creates a directory and all of its missing parent directories.
///
/// # Errors
///
/// Returns [`DkvError`] if the directory cannot be created.
pub fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    Ok(())
}
