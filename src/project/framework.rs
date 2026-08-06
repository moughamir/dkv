//! Framework detection from manifests and configuration files.

use std::{collections::BTreeSet, path::PathBuf};

use crate::types::{CargoPackage, Framework, NodePackage};

/// Known framework configuration file names used as detection signals.
const CONFIG_FILES: &[&str] = &[
    "next.config.js",
    "next.config.mjs",
    "next.config.ts",
    "astro.config.js",
    "astro.config.mjs",
    "astro.config.ts",
    "nuxt.config.js",
    "nuxt.config.mjs",
    "nuxt.config.ts",
    "angular.json",
    "svelte.config.js",
    "svelte.config.mjs",
    "svelte.config.ts",
];

/// Returns `true` if the given file name is a known framework configuration
/// file.
#[must_use]
pub fn is_config_file(file_name: &str) -> bool {
    CONFIG_FILES.contains(&file_name)
}

/// Detects frameworks from parsed packages and configuration files.
///
/// Cargo frameworks are matched on normal and build dependencies; Node
/// frameworks on regular and dev dependencies; configuration files act as an
/// additional signal.
///
/// The exhaustive match arms covering every framework mapping keep this
/// function over clippy's default line budget.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn detect(
    cargo_packages: &[CargoPackage],
    node_packages: &[NodePackage],
    config_files: &[PathBuf],
) -> BTreeSet<Framework> {
    let mut frameworks = BTreeSet::new();

    for package in cargo_packages {
        for dependency in package
            .dependencies
            .iter()
            .chain(&package.build_dependencies)
        {
            match dependency.name.as_str() {
                "tauri" | "tauri-build" => {
                    frameworks.insert(Framework::Tauri);
                }
                "axum" => {
                    frameworks.insert(Framework::Axum);
                }
                "actix-web" => {
                    frameworks.insert(Framework::Actix);
                }
                "rocket" => {
                    frameworks.insert(Framework::Rocket);
                }
                "leptos" => {
                    frameworks.insert(Framework::Leptos);
                }
                "yew" => {
                    frameworks.insert(Framework::Yew);
                }
                "dioxus" => {
                    frameworks.insert(Framework::Dioxus);
                }
                "bevy" => {
                    frameworks.insert(Framework::Bevy);
                }
                _ => {}
            }
        }
    }

    for package in node_packages {
        for dependency in package.dependencies.iter().chain(&package.dev_dependencies) {
            match dependency.name.as_str() {
                "react" | "react-dom" => {
                    frameworks.insert(Framework::React);
                }
                "vue" => {
                    frameworks.insert(Framework::Vue);
                }
                "solid-js" => {
                    frameworks.insert(Framework::Solid);
                }
                "@angular/core" => {
                    frameworks.insert(Framework::Angular);
                }
                "svelte" => {
                    frameworks.insert(Framework::Svelte);
                }
                "@sveltejs/kit" => {
                    frameworks.insert(Framework::SvelteKit);
                }
                "next" => {
                    frameworks.insert(Framework::Next);
                }
                "nuxt" => {
                    frameworks.insert(Framework::Nuxt);
                }
                "astro" => {
                    frameworks.insert(Framework::Astro);
                }
                "@remix-run/react" | "@remix-run/node" => {
                    frameworks.insert(Framework::Remix);
                }
                "express" => {
                    frameworks.insert(Framework::Express);
                }
                "fastify" => {
                    frameworks.insert(Framework::Fastify);
                }
                "@nestjs/core" => {
                    frameworks.insert(Framework::NestJs);
                }
                "hono" => {
                    frameworks.insert(Framework::Hono);
                }
                "@tauri-apps/api" | "@tauri-apps/cli" => {
                    frameworks.insert(Framework::Tauri);
                }
                _ => {}
            }
        }
    }

    for path in config_files {
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            match name {
                "next.config.js" | "next.config.mjs" | "next.config.ts" => {
                    frameworks.insert(Framework::Next);
                }
                "astro.config.js" | "astro.config.mjs" | "astro.config.ts" => {
                    frameworks.insert(Framework::Astro);
                }
                "nuxt.config.js" | "nuxt.config.mjs" | "nuxt.config.ts" => {
                    frameworks.insert(Framework::Nuxt);
                }
                "angular.json" => {
                    frameworks.insert(Framework::Angular);
                }
                "svelte.config.js" | "svelte.config.mjs" | "svelte.config.ts" => {
                    frameworks.insert(Framework::Svelte);
                }
                _ => {}
            }
        }
    }

    frameworks
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use super::*;
    use crate::types::{CargoTargets, Dependency, DependencyKind, DependencyScope, NodeScripts};

    /// Builds a [`Dependency`] with a placeholder requirement.
    fn dependency(kind: DependencyKind, name: &str, scope: DependencyScope) -> Dependency {
        Dependency {
            kind,
            name: name.to_string(),
            requirement: "*".to_string(),
            scope,
        }
    }

    /// Builds a [`CargoPackage`] carrying only the given dependency lists.
    fn cargo_package(
        name: &str,
        dependencies: Vec<Dependency>,
        dev_dependencies: Vec<Dependency>,
        build_dependencies: Vec<Dependency>,
    ) -> CargoPackage {
        CargoPackage {
            name: name.to_string(),
            version: None,
            edition: None,
            license: None,
            description: None,
            repository: None,
            homepage: None,
            authors: Vec::new(),
            keywords: Vec::new(),
            categories: Vec::new(),
            manifest_path: PathBuf::from("Cargo.toml"),
            workspace: false,
            workspace_root: None,
            workspace_info: None,
            dependencies,
            dev_dependencies,
            build_dependencies,
            target_dependencies: Vec::new(),
            features: Vec::new(),
            default_features: Vec::new(),
            targets: CargoTargets::default(),
        }
    }

    /// Builds a [`NodePackage`] carrying only the given dependency lists.
    fn node_package(
        name: &str,
        dependencies: Vec<Dependency>,
        dev_dependencies: Vec<Dependency>,
    ) -> NodePackage {
        NodePackage {
            name: name.to_string(),
            version: None,
            description: None,
            homepage: None,
            repository: None,
            license: None,
            manifest_path: PathBuf::from("package.json"),
            package_manager: None,
            lockfiles: Vec::new(),
            dependencies,
            dev_dependencies,
            peer_dependencies: Vec::new(),
            optional_dependencies: Vec::new(),
            scripts: NodeScripts::default(),
            engines: BTreeMap::new(),
            workspaces: Vec::new(),
        }
    }

    #[test]
    fn cargo_dependencies_detect_axum_and_tauri() {
        let package = cargo_package(
            "api",
            vec![
                dependency(DependencyKind::Cargo, "axum", DependencyScope::Normal),
                dependency(DependencyKind::Cargo, "tauri", DependencyScope::Normal),
            ],
            Vec::new(),
            Vec::new(),
        );

        let frameworks = detect(&[package], &[], &[]);

        assert!(frameworks.contains(&Framework::Axum));
        assert!(frameworks.contains(&Framework::Tauri));
    }

    #[test]
    fn node_dependencies_detect_react_sveltekit_and_nestjs() {
        let package = node_package(
            "web",
            vec![
                dependency(DependencyKind::Npm, "react", DependencyScope::Normal),
                dependency(
                    DependencyKind::Npm,
                    "@sveltejs/kit",
                    DependencyScope::Normal,
                ),
            ],
            vec![dependency(
                DependencyKind::Npm,
                "@nestjs/core",
                DependencyScope::Development,
            )],
        );

        let frameworks = detect(&[], &[package], &[]);

        assert!(frameworks.contains(&Framework::React));
        assert!(frameworks.contains(&Framework::SvelteKit));
        assert!(frameworks.contains(&Framework::NestJs));
    }

    #[test]
    fn config_file_detects_next() {
        let frameworks = detect(&[], &[], &[PathBuf::from("/app/next.config.js")]);

        assert!(frameworks.contains(&Framework::Next));
    }

    #[test]
    fn cargo_dev_dependencies_do_not_detect_frameworks() {
        let package = cargo_package(
            "game",
            Vec::new(),
            vec![dependency(
                DependencyKind::Cargo,
                "bevy",
                DependencyScope::Development,
            )],
            Vec::new(),
        );

        let frameworks = detect(&[package], &[], &[]);

        assert!(!frameworks.contains(&Framework::Bevy));
    }
}
