use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

use crate::types::Dependency;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub root: PathBuf,
    pub languages: BTreeSet<Language>,
    pub frameworks: BTreeSet<Framework>,
    pub cargo_packages: Vec<CargoPackage>,
    pub node_packages: Vec<NodePackage>,
}

impl Default for ProjectManifest {
    fn default() -> Self {
        Self {
            root: PathBuf::new(),
            languages: BTreeSet::new(),
            frameworks: BTreeSet::new(),
            cargo_packages: Vec::new(),
            node_packages: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    C,
    Cpp,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Framework {
    Tauri,
    Svelte,
    SvelteKit,
    React,
    Vue,
    Solid,
    Angular,
    Axum,
    Actix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoPackage {
    pub name: String,
    pub version: Option<String>,
    pub manifest_path: PathBuf,
    pub workspace: bool,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePackage {
    pub name: String,
    pub version: Option<String>,
    pub manifest_path: PathBuf,
    pub dependencies: Vec<Dependency>,
    pub dev_dependencies: Vec<Dependency>,
}
