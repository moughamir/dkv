use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub vault: PathBuf,
    pub editor: String,
    pub search: SearchEngine,
    pub zeal: bool,
    pub manpages: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vault: PathBuf::from("~/Documents/DevKnowledgeVault"),
            editor: std::env::var("EDITOR").unwrap_or_else(|_| "nano".into()),
            search: SearchEngine::Sqlite,
            zeal: true,
            manpages: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchEngine {
    Sqlite,
    Ripgrep,
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::Sqlite
    }
}
