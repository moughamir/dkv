use std::{collections::BTreeMap, fs, path::Path};

use serde::Deserialize;

use crate::{
    error::Result,
    types::{CargoPackage, Dependency, DependencyKind, DependencyScope},
};

#[derive(Debug, Deserialize)]
struct CargoToml {
    package: Option<Package>,
    workspace: Option<Workspace>,

    #[serde(default)]
    dependencies: BTreeMap<String, DependencySpec>,

    #[serde(rename = "dev-dependencies", default)]
    dev_dependencies: BTreeMap<String, DependencySpec>,

    #[serde(rename = "build-dependencies", default)]
    build_dependencies: BTreeMap<String, DependencySpec>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct Workspace {}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DependencySpec {
    Version(String),
    Detailed(BTreeMap<String, toml::Value>),
}

fn version(spec: &DependencySpec) -> String {
    match spec {
        DependencySpec::Version(v) => v.clone(),

        DependencySpec::Detailed(map) => map
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("*")
            .to_string(),
    }
}

/// Loads a Cargo package manifest.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] if the manifest cannot be read or contains invalid
/// TOML.
pub fn load(path: &Path) -> Result<CargoPackage> {
    let text = fs::read_to_string(path)?;

    let manifest: CargoToml = toml::from_str(&text)?;

    let mut deps = Vec::new();

    for (name, spec) in manifest.dependencies {
        deps.push(Dependency {
            kind: DependencyKind::Cargo,
            name,
            requirement: version(&spec),
            scope: DependencyScope::Normal,
        });
    }

    for (name, spec) in manifest.dev_dependencies {
        deps.push(Dependency {
            kind: DependencyKind::Cargo,
            name,
            requirement: version(&spec),
            scope: DependencyScope::Development,
        });
    }

    for (name, spec) in manifest.build_dependencies {
        deps.push(Dependency {
            kind: DependencyKind::Cargo,
            name,
            requirement: version(&spec),
            scope: DependencyScope::Build,
        });
    }

    Ok(CargoPackage {
        name: manifest
            .package
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_default(),

        version: manifest.package.map(|p| p.version),

        manifest_path: path.to_path_buf(),

        workspace: manifest.workspace.is_some(),

        dependencies: deps,
    })
}
