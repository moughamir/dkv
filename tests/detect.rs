//! Integration tests for whole-project detection against the fixtures in
//! `tests/fixtures/`.

use std::time::Duration;

use dkv::project::detect;
use dkv::types::{Framework, Language, PackageManager};

mod common;
use common::fixture;

#[test]
fn detects_cargo_workspace() -> dkv::Result<()> {
    let manifest = detect::detect(fixture("cargo-workspace"), &[])?;

    assert!(manifest.languages.contains(&Language::Rust));
    assert!(manifest.node_packages.is_empty());

    #[allow(clippy::unwrap_used)]
    let workspace = manifest.cargo_workspace.unwrap();
    assert_eq!(workspace.members.len(), 3);
    Ok(())
}

#[test]
fn detects_npm_workspace() -> dkv::Result<()> {
    let manifest = detect::detect(fixture("npm-workspace"), &[])?;

    #[allow(clippy::unwrap_used)]
    let workspace = manifest.node_workspace.unwrap();
    assert_eq!(workspace.members.len(), 2);
    assert_eq!(workspace.package_manager, Some(PackageManager::Npm));
    assert!(manifest.node_packages.len() >= 3);
    Ok(())
}

#[test]
fn detects_pnpm_workspace() -> dkv::Result<()> {
    let manifest = detect::detect(fixture("pnpm-workspace"), &[])?;

    #[allow(clippy::unwrap_used)]
    let workspace = manifest.node_workspace.unwrap();
    assert_eq!(workspace.package_manager, Some(PackageManager::Pnpm));
    Ok(())
}

#[test]
fn detects_bun_workspace() -> dkv::Result<()> {
    let manifest = detect::detect(fixture("bun-workspace"), &[])?;

    #[allow(clippy::unwrap_used)]
    let workspace = manifest.node_workspace.unwrap();
    assert_eq!(workspace.package_manager, Some(PackageManager::Bun));
    Ok(())
}

#[test]
fn detects_tauri() -> dkv::Result<()> {
    let manifest = detect::detect(fixture("tauri"), &[])?;

    assert!(manifest.frameworks.contains(&Framework::Tauri));
    assert!(manifest.languages.contains(&Language::Rust));
    Ok(())
}

#[test]
fn detects_sveltekit() -> dkv::Result<()> {
    let manifest = detect::detect(fixture("sveltekit"), &[])?;

    assert!(manifest.frameworks.contains(&Framework::SvelteKit));
    assert!(manifest.frameworks.contains(&Framework::Svelte));
    Ok(())
}

#[test]
fn detects_mixed_project_and_default_ignores() -> dkv::Result<()> {
    let manifest = detect::detect(fixture("mixed"), &[])?;

    assert!(manifest.languages.contains(&Language::Rust));
    assert!(manifest.languages.contains(&Language::TypeScript));
    assert!(manifest.frameworks.contains(&Framework::Axum));
    assert!(manifest.frameworks.contains(&Framework::React));
    assert!(manifest.frameworks.contains(&Framework::Express));

    assert!(
        manifest.statistics.ignored_directories >= 2,
        "expected `dist` and `node_modules` to be pruned"
    );
    assert!(manifest.statistics.files_scanned > 0);
    assert!(manifest.statistics.elapsed > Duration::ZERO);
    Ok(())
}
