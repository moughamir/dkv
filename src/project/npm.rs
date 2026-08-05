use std::{collections::BTreeMap, fs, path::Path};

use serde::Deserialize;

use crate::{
    error::Result,
    types::{Dependency, DependencyKind, DependencyScope, NodePackage},
};

#[derive(Debug, Deserialize)]
struct PackageJson {
    name: Option<String>,
    version: Option<String>,

    #[serde(default)]
    dependencies: BTreeMap<String, String>,

    #[serde(rename = "devDependencies", default)]
    dev_dependencies: BTreeMap<String, String>,
}

/// Loads an npm package manifest.
///
/// # Errors
///
/// Returns [`DkvError`] if the manifest cannot be read or contains invalid
/// JSON.
pub fn load(path: &Path) -> Result<NodePackage> {
    let text = fs::read_to_string(path)?;

    let package: PackageJson = serde_json::from_str(&text)?;

    let mut dependencies = Vec::new();
    let mut dev_dependencies = Vec::new();

    for (name, version) in package.dependencies {
        dependencies.push(Dependency {
            kind: DependencyKind::Npm,
            name,
            requirement: version,
            scope: DependencyScope::Normal,
        });
    }

    for (name, version) in package.dev_dependencies {
        dev_dependencies.push(Dependency {
            kind: DependencyKind::Npm,
            name,
            requirement: version,
            scope: DependencyScope::Development,
        });
    }

    Ok(NodePackage {
        name: package.name.unwrap_or_default(),
        version: package.version,
        manifest_path: path.to_path_buf(),
        dependencies,
        dev_dependencies,
    })
}
