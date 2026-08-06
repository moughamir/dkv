use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use crate::{
    error::Result,
    project::{cargo, framework, npm},
    scanner::Scanner,
    types::{Language, NodeWorkspace, ProjectManifest},
};

/// Detects languages, frameworks, and package manifests within a project
/// directory.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] if the root path cannot be canonicalized.
pub fn detect(root: impl AsRef<Path>, config_exclusions: &[String]) -> Result<ProjectManifest> {
    let root = root.as_ref().canonicalize()?;

    let mut manifest = ProjectManifest {
        root: root.clone(),
        ..Default::default()
    };

    let mut scanner = Scanner::new(config_exclusions.to_vec());
    scanner.load_ignore_files(&root)?;

    let start = Instant::now();
    let mut config_files: Vec<PathBuf> = Vec::new();

    for entry in scanner.walk(&root)? {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let file_name = path.file_name().and_then(|name| name.to_str());

        match file_name {
            Some("Cargo.toml") => {
                if let Ok(package) = cargo::load(path) {
                    manifest.languages.insert(Language::Rust);

                    if let Some(workspace_info) = package.workspace_info.clone() {
                        manifest.cargo_workspace.get_or_insert(workspace_info);
                    }

                    if !package.name.is_empty() {
                        manifest.cargo_packages.push(package);
                    }
                }
            }

            Some("package.json") => {
                if let Ok(package) = npm::load(path) {
                    if path.with_file_name("tsconfig.json").exists() {
                        manifest.languages.insert(Language::TypeScript);
                    } else {
                        manifest.languages.insert(Language::JavaScript);
                    }

                    if !package.workspaces.is_empty() {
                        manifest.node_workspace = Some(NodeWorkspace {
                            root: path.parent().map_or_else(PathBuf::new, Path::to_path_buf),
                            members: package.workspaces.clone(),
                            package_manager: package.package_manager,
                        });
                    }
                    manifest.node_packages.push(package);
                }
            }

            Some(name) if framework::is_config_file(name) => {
                config_files.push(path.to_path_buf());
            }

            _ => {}
        }
    }

    manifest.frameworks = framework::detect(
        &manifest.cargo_packages,
        &manifest.node_packages,
        &config_files,
    );

    let mut statistics = scanner.statistics();
    statistics.elapsed = start.elapsed();
    manifest.statistics = statistics;

    Ok(manifest)
}
