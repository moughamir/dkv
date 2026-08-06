//! Integration tests for Node (`package.json`) and lockfile parsing against
//! the fixtures in `tests/fixtures/`.

use dkv::project::{bun, npm};
use dkv::types::{Dependency, DependencyScope, LockfileKind, PackageManager};

mod common;
use common::fixture;

/// Finds a dependency by name within a vector of dependencies.
fn find_dep<'a>(deps: &'a [Dependency], name: &str) -> &'a Dependency {
    #[allow(clippy::unwrap_used)]
    deps.iter().find(|dep| dep.name == name).unwrap()
}

#[test]
fn single_node_parses_full_metadata() -> dkv::Result<()> {
    let package = npm::load(&fixture("single-node").join("package.json"))?;

    assert_eq!(package.name, "single-node");
    assert_eq!(package.version.as_deref(), Some("0.1.0"));
    assert_eq!(
        package.description.as_deref(),
        Some("A single Node package")
    );
    assert_eq!(package.homepage.as_deref(), Some("https://example.com"));
    assert_eq!(
        package.repository.as_deref(),
        Some("https://github.com/example/single-node")
    );
    assert_eq!(package.license.as_deref(), Some("MIT"));
    assert_eq!(package.package_manager, Some(PackageManager::Npm));
    assert!(package.engines.contains_key("node"));
    Ok(())
}

#[test]
fn single_node_parses_dependency_groups_and_scripts() -> dkv::Result<()> {
    let package = npm::load(&fixture("single-node").join("package.json"))?;

    assert!(matches!(
        find_dep(&package.dependencies, "lodash").scope,
        DependencyScope::Normal
    ));
    assert!(matches!(
        find_dep(&package.dev_dependencies, "typescript").scope,
        DependencyScope::Development
    ));
    assert!(matches!(
        find_dep(&package.peer_dependencies, "react").scope,
        DependencyScope::Peer
    ));
    assert!(matches!(
        find_dep(&package.optional_dependencies, "fsevents").scope,
        DependencyScope::Optional
    ));

    assert!(package.scripts.build.is_some());
    assert!(package.scripts.test.is_some());
    assert!(package.scripts.lint.is_some());
    assert!(package.scripts.dev.is_some());
    assert!(package.scripts.start.is_some());
    Ok(())
}

#[test]
fn single_node_detects_package_lockfile() -> dkv::Result<()> {
    let package = npm::load(&fixture("single-node").join("package.json"))?;

    assert_eq!(package.lockfiles, vec![LockfileKind::PackageLock]);
    Ok(())
}

#[test]
fn single_node_package_lock_parses_empty_packages() -> dkv::Result<()> {
    let lock = fixture("single-node").join("package-lock.json");
    let packages = npm::parse_package_lock(&lock)?;

    assert!(packages.is_empty());
    Ok(())
}

#[test]
fn npm_workspace_resolves_members() -> dkv::Result<()> {
    let package = npm::load(&fixture("npm-workspace").join("package.json"))?;

    assert_eq!(package.package_manager, Some(PackageManager::Npm));
    assert_eq!(package.workspaces.len(), 2);
    assert!(
        package
            .workspaces
            .contains(&fixture("npm-workspace").join("packages/pkg-a/package.json"))
    );
    assert!(
        package
            .workspaces
            .contains(&fixture("npm-workspace").join("packages/pkg-b/package.json"))
    );
    Ok(())
}

#[test]
fn pnpm_workspace_detects_manager_members_and_lockfile() -> dkv::Result<()> {
    let package = npm::load(&fixture("pnpm-workspace").join("package.json"))?;

    assert_eq!(package.package_manager, Some(PackageManager::Pnpm));
    assert!(package.lockfiles.contains(&LockfileKind::PnpmLock));
    assert!(
        package
            .workspaces
            .contains(&fixture("pnpm-workspace").join("packages/foo/package.json"))
    );
    Ok(())
}

#[test]
fn pnpm_lock_parses_locked_packages() -> dkv::Result<()> {
    let lock = fixture("pnpm-workspace").join("pnpm-lock.yaml");
    let packages = npm::parse_pnpm_lock(&lock)?;

    assert!(
        packages
            .iter()
            .any(|pkg| pkg.name == "foo" && pkg.version == "1.0.0")
    );
    Ok(())
}

#[test]
fn bun_workspace_detects_manager_members_and_lockfiles() -> dkv::Result<()> {
    let package = npm::load(&fixture("bun-workspace").join("package.json"))?;

    assert_eq!(package.package_manager, Some(PackageManager::Bun));
    assert!(package.lockfiles.contains(&LockfileKind::BunLock));
    assert!(package.lockfiles.contains(&LockfileKind::BunLockb));
    assert!(
        package
            .workspaces
            .contains(&fixture("bun-workspace").join("packages/bar/package.json"))
    );
    Ok(())
}

#[test]
fn bun_lock_parses_locked_packages() -> dkv::Result<()> {
    let lock = fixture("bun-workspace").join("bun.lock");
    let packages = bun::parse_bun_lock(&lock)?;

    assert!(
        packages
            .iter()
            .any(|pkg| pkg.name == "lodash" && pkg.version == "4.17.0")
    );
    Ok(())
}
