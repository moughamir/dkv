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
    pub exclusions: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vault: PathBuf::from("~/Documents/DevKnowledgeVault"),
            editor: std::env::var("EDITOR").unwrap_or_else(|_| "nano".into()),
            search: SearchEngine::Sqlite,
            zeal: true,
            manpages: true,
            exclusions: vec![
                ".git".into(),
                "target".into(),
                "node_modules".into(),
                "dist".into(),
                "build".into(),
                "coverage".into(),
                ".cache".into(),
                ".next".into(),
                ".nuxt".into(),
                ".svelte-kit".into(),
                "vendor".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchEngine {
    #[default]
    Sqlite,
    Ripgrep,
}
