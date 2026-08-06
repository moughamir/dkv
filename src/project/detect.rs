use std::path::Path;

use crate::{
    error::Result,
    project::{cargo, npm},
    scanner::Scanner,
    types::{Framework, Language, ProjectManifest},
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

    for entry in scanner.walk(&root)? {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        match path.file_name().and_then(|n| n.to_str()) {
            Some("Cargo.toml") => {
                if let Ok(pkg) = cargo::load(path) {
                    manifest.languages.insert(Language::Rust);
                    manifest.cargo_packages.push(pkg);
                }
            }

            Some("package.json") => {
                if let Ok(pkg) = npm::load(path) {
                    if path.with_file_name("tsconfig.json").exists() {
                        manifest.languages.insert(Language::TypeScript);
                    } else {
                        manifest.languages.insert(Language::JavaScript);
                    }

                    let deps = pkg.dependencies.iter().chain(pkg.dev_dependencies.iter());

                    for dep in deps {
                        match dep.name.as_str() {
                            "@tauri-apps/api" | "@tauri-apps/cli" => {
                                manifest.frameworks.insert(Framework::Tauri);
                            }

                            "@sveltejs/kit" => {
                                manifest.frameworks.insert(Framework::SvelteKit);
                            }

                            "svelte" => {
                                manifest.frameworks.insert(Framework::Svelte);
                            }

                            "react" => {
                                manifest.frameworks.insert(Framework::React);
                            }

                            "vue" => {
                                manifest.frameworks.insert(Framework::Vue);
                            }

                            "solid-js" => {
                                manifest.frameworks.insert(Framework::Solid);
                            }

                            "@angular/core" => {
                                manifest.frameworks.insert(Framework::Angular);
                            }

                            _ => {}
                        }
                    }

                    manifest.node_packages.push(pkg);
                }
            }

            _ => {}
        }
    }

    Ok(manifest)
}
