//! Integration tests for Cargo manifest parsing against the fixtures in
//! `tests/fixtures/`.

use std::path::Path;

use dkv::project::cargo;
use dkv::types::{Dependency, DependencyScope};

mod common;
use common::fixture;

/// Finds a dependency by name within a vector of dependencies.
fn find_dep<'a>(deps: &'a [Dependency], name: &str) -> &'a Dependency {
    #[allow(clippy::unwrap_used)]
    deps.iter().find(|dep| dep.name == name).unwrap()
}

#[test]
fn single_cargo_parses_package_metadata() -> dkv::Result<()> {
    let package = cargo::load(&fixture("single-cargo").join("Cargo.toml"))?;

    assert_eq!(package.name, "demo-cli");
    assert_eq!(package.version.as_deref(), Some("0.2.0"));
    assert_eq!(package.edition.as_deref(), Some("2021"));
    assert_eq!(package.license.as_deref(), Some("MIT"));
    assert_eq!(package.description.as_deref(), Some("Demo"));
    assert_eq!(
        package.repository.as_deref(),
        Some("https://github.com/example/demo")
    );
    assert_eq!(package.homepage.as_deref(), Some("https://example.com"));
    assert_eq!(package.authors, vec!["A B <a@example.com>"]);
    assert_eq!(package.keywords, vec!["demo"]);
    assert_eq!(package.categories, vec!["cli"]);
    assert!(!package.workspace);
    assert!(package.workspace_root.is_none());
    assert!(package.workspace_info.is_none());
    Ok(())
}

#[test]
fn single_cargo_parses_dependency_groups() -> dkv::Result<()> {
    let package = cargo::load(&fixture("single-cargo").join("Cargo.toml"))?;

    assert!(matches!(
        find_dep(&package.dependencies, "serde").scope,
        DependencyScope::Normal
    ));
    assert!(matches!(
        find_dep(&package.dependencies, "anyhow").scope,
        DependencyScope::Normal
    ));
    assert!(matches!(
        find_dep(&package.dev_dependencies, "tempfile").scope,
        DependencyScope::Development
    ));
    assert!(matches!(
        find_dep(&package.build_dependencies, "cc").scope,
        DependencyScope::Build
    ));
    Ok(())
}

#[test]
fn single_cargo_parses_features_and_targets() -> dkv::Result<()> {
    let package = cargo::load(&fixture("single-cargo").join("Cargo.toml"))?;

    assert_eq!(package.features, vec!["default", "extra"]);
    assert_eq!(package.default_features, vec!["serde"]);

    let targets = &package.targets;
    assert!(
        targets.bins.iter().any(|target| {
            target.name == "demo-cli" && target.path.as_deref() == Some(Path::new("src/main.rs"))
        }),
        "expected a declared bin target named `demo-cli`"
    );

    #[allow(clippy::unwrap_used)]
    let lib = targets.lib.as_ref().unwrap();
    assert_eq!(lib.name, "demo_cli");
    assert_eq!(lib.path.as_deref(), Some(Path::new("src/lib.rs")));
    Ok(())
}

#[test]
fn workspace_member_inherits_workspace_dependency() -> dkv::Result<()> {
    let member = fixture("cargo-workspace").join("crates/a/Cargo.toml");
    let package = cargo::load(&member)?;

    let serde = find_dep(&package.dependencies, "serde");
    assert!(matches!(serde.scope, DependencyScope::Workspace));
    assert_eq!(serde.requirement, "*");

    assert!(package.workspace);
    assert_eq!(
        package.workspace_root.as_deref(),
        Some(fixture("cargo-workspace").as_path())
    );
    Ok(())
}

#[test]
fn path_dependency_keeps_normal_scope() -> dkv::Result<()> {
    let member = fixture("cargo-workspace").join("crates/b/Cargo.toml");
    let package = cargo::load(&member)?;

    let crate_a = find_dep(&package.dependencies, "crate-a");
    assert!(
        !matches!(crate_a.scope, DependencyScope::Workspace),
        "path dependency without `workspace = true` must stay Normal"
    );
    assert!(matches!(crate_a.scope, DependencyScope::Normal));
    assert_eq!(crate_a.requirement, "0.1");
    Ok(())
}

#[test]
fn workspace_root_lists_all_members() -> dkv::Result<()> {
    let root = fixture("cargo-workspace").join("Cargo.toml");
    let package = cargo::load(&root)?;

    assert!(package.workspace);
    #[allow(clippy::unwrap_used)]
    let info = package.workspace_info.unwrap();

    let expected_members = [
        fixture("cargo-workspace").join("Cargo.toml"),
        fixture("cargo-workspace").join("crates/a/Cargo.toml"),
        fixture("cargo-workspace").join("crates/b/Cargo.toml"),
    ];
    assert_eq!(info.members.len(), 3);
    for member in &expected_members {
        assert!(info.members.contains(member), "missing member {member:?}");
    }

    let serde = find_dep(&info.dependencies, "serde");
    assert!(matches!(serde.scope, DependencyScope::Normal));
    assert_eq!(serde.requirement, "1.0");
    Ok(())
}
