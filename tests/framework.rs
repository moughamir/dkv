//! Integration tests for framework detection, both from parsed packages and
//! from configuration file names.

use std::collections::BTreeMap;
use std::path::PathBuf;

use dkv::project::{cargo, framework};
use dkv::types::{
    CargoPackage, CargoTargets, Dependency, DependencyKind, DependencyScope, Framework,
    NodePackage, NodeScripts,
};

mod common;
use common::fixture;

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
        build_dependencies: Vec::new(),
        target_dependencies: Vec::new(),
        features: Vec::new(),
        default_features: Vec::new(),
        targets: CargoTargets::default(),
    }
}

/// Builds a [`NodePackage`] carrying only the given dependency list.
fn node_package(name: &str, dependencies: Vec<Dependency>) -> NodePackage {
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
        dev_dependencies: Vec::new(),
        peer_dependencies: Vec::new(),
        optional_dependencies: Vec::new(),
        scripts: NodeScripts::default(),
        engines: BTreeMap::new(),
        workspaces: Vec::new(),
    }
}

#[test]
fn cargo_normal_dependencies_detect_axum() {
    let package = cargo_package(
        "api",
        vec![dependency(
            DependencyKind::Cargo,
            "axum",
            DependencyScope::Normal,
        )],
        Vec::new(),
    );

    let frameworks = framework::detect(&[package], &[], &[]);

    assert!(frameworks.contains(&Framework::Axum));
}

#[test]
fn cargo_dev_dependencies_do_not_detect_bevy() {
    let package = cargo_package(
        "game",
        Vec::new(),
        vec![dependency(
            DependencyKind::Cargo,
            "bevy",
            DependencyScope::Development,
        )],
    );

    let frameworks = framework::detect(&[package], &[], &[]);

    assert!(!frameworks.contains(&Framework::Bevy));
}

#[test]
fn node_dependencies_detect_sveltekit() {
    let package = node_package(
        "web",
        vec![dependency(
            DependencyKind::Npm,
            "@sveltejs/kit",
            DependencyScope::Normal,
        )],
    );

    let frameworks = framework::detect(&[], &[package], &[]);

    assert!(frameworks.contains(&Framework::SvelteKit));
}

#[test]
fn node_dependencies_detect_nestjs() {
    let package = node_package(
        "api",
        vec![dependency(
            DependencyKind::Npm,
            "@nestjs/core",
            DependencyScope::Normal,
        )],
    );

    let frameworks = framework::detect(&[], &[package], &[]);

    assert!(frameworks.contains(&Framework::NestJs));
}

#[test]
fn node_dependencies_detect_hono() {
    let package = node_package(
        "edge",
        vec![dependency(
            DependencyKind::Npm,
            "hono",
            DependencyScope::Normal,
        )],
    );

    let frameworks = framework::detect(&[], &[package], &[]);

    assert!(frameworks.contains(&Framework::Hono));
}

#[test]
fn config_files_detect_next() {
    let config = PathBuf::from("/app/next.config.js");

    let frameworks = framework::detect(&[], &[], &[config]);

    assert!(frameworks.contains(&Framework::Next));
}

#[test]
fn is_config_file_recognizes_framework_configs() {
    assert!(framework::is_config_file("astro.config.mjs"));
    assert!(!framework::is_config_file("random.txt"));
}

#[test]
fn fixture_parsed_cargo_package_detects_tauri() -> dkv::Result<()> {
    let package = cargo::load(&fixture("tauri").join("src-tauri/Cargo.toml"))?;

    let frameworks = framework::detect(&[package], &[], &[]);

    assert!(frameworks.contains(&Framework::Tauri));
    Ok(())
}
