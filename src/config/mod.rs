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

pub fn config_dir() -> Result<PathBuf> {
    let base = BaseDirs::new()
        .ok_or_else(|| DkvError::Message("Unable to locate HOME directory".into()))?;

    Ok(base.config_dir().join(CONFIG_DIR))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(CONFIG_FILE))
}

pub fn exists() -> Result<bool> {
    Ok(config_path()?.exists())
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    let text = fs::read_to_string(path)?;
    Ok(toml::from_str(&text)?)
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let toml = toml::to_string_pretty(config)?;

    fs::write(path, toml)?;

    Ok(())
}

pub fn init() -> Result<()> {
    if exists()? {
        return Ok(());
    }

    save(&Config::default())
}

pub fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    Ok(())
}
