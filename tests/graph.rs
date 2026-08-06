//! Integration tests for the package dependency graph.

use std::path::PathBuf;

use dkv::graph;
use dkv::project::detect;
use dkv::types::{
    CargoPackage, CargoTargets, Dependency, DependencyKind, DependencyScope, ProjectManifest,
};

mod common;
use common::fixture;

/// Builds a [`Dependency`] with the given requirement and scope.
fn dependency(
    kind: DependencyKind,
    name: &str,
    requirement: &str,
    scope: DependencyScope,
) -> Dependency {
    Dependency {
        kind,
        name: name.to_string(),
        requirement: requirement.to_string(),
        scope,
    }
}

/// Builds a [`CargoPackage`] carrying only the given dependency list.
fn cargo_package(name: &str, version: &str, dependencies: Vec<Dependency>) -> CargoPackage {
    CargoPackage {
        name: name.to_string(),
        version: Some(version.to_string()),
        edition: None,
        license: None,
        description: None,
        repository: None,
        homepage: None,
        authors: Vec::new(),
        keywords: Vec::new(),
        categories: Vec::new(),
        manifest_path: PathBuf::from(format!("{name}/Cargo.toml")),
        workspace: false,
        workspace_root: None,
        workspace_info: None,
        dependencies,
        dev_dependencies: Vec::new(),
        build_dependencies: Vec::new(),
        target_dependencies: Vec::new(),
        features: Vec::new(),
        default_features: Vec::new(),
        targets: CargoTargets::default(),
    }
}

#[test]
fn build_creates_nodes_and_edges_for_mixed_project() -> dkv::Result<()> {
    let manifest = detect::detect(fixture("mixed"), &[])?;
    let graph = graph::build(&manifest);

    assert!(!graph.packages.is_empty());
    assert!(
        graph
            .packages
            .iter()
            .any(|pkg| pkg.id.starts_with("cargo:mixed-app")),
        "expected a cargo package for `mixed-app`"
    );
    assert!(
        graph
            .packages
            .iter()
            .any(|pkg| pkg.id.starts_with("npm:mixed-app")),
        "expected an npm package for `mixed-app`"
    );

    #[allow(clippy::unwrap_used)]
    let cargo_id = graph
        .packages
        .iter()
        .find(|pkg| pkg.id.starts_with("cargo:mixed-app"))
        .unwrap()
        .id
        .clone();

    let cargo_edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.from == cargo_id)
        .collect();
    assert!(
        cargo_edges.len() >= 3,
        "expected one edge per declared cargo dependency, got {}",
        cargo_edges.len()
    );
    Ok(())
}

#[test]
fn transitive_dependencies_follow_resolved_edges_and_exclude_start() {
    let manifest = ProjectManifest {
        root: PathBuf::from("."),
        cargo_packages: vec![
            cargo_package(
                "a",
                "0.1.0",
                vec![dependency(
                    DependencyKind::Cargo,
                    "b",
                    "*",
                    DependencyScope::Normal,
                )],
            ),
            cargo_package(
                "b",
                "0.1.0",
                vec![dependency(
                    DependencyKind::Cargo,
                    "c",
                    "*",
                    DependencyScope::Normal,
                )],
            ),
            cargo_package("c", "0.1.0", Vec::new()),
        ],
        ..Default::default()
    };

    let graph = graph::build(&manifest);
    let result = graph.transitive_dependencies("cargo:a@0.1.0");

    assert_eq!(result, vec!["cargo:b@0.1.0", "cargo:c@0.1.0"]);
    assert!(!result.contains(&"cargo:a@0.1.0".to_string()));
}

#[test]
fn external_dependencies_yield_unresolved_edges() {
    let manifest = ProjectManifest {
        root: PathBuf::from("."),
        cargo_packages: vec![cargo_package(
            "app",
            "1.0.0",
            vec![dependency(
                DependencyKind::Cargo,
                "serde",
                "1",
                DependencyScope::Normal,
            )],
        )],
        ..Default::default()
    };

    let graph = graph::build(&manifest);

    assert_eq!(graph.edges.len(), 1);
    let edge = &graph.edges[0];
    assert_eq!(edge.to, "cargo:serde");
    assert!(!edge.resolved);
    assert_eq!(edge.requirement, "1");
}
