//! Offline dependency graph built from local manifests only.

use std::{collections::BTreeSet, path::PathBuf};

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use crate::types::{Dependency, DependencyKind, DependencyScope, ProjectManifest};

/// A package node in the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPackage {
    /// Unique package id (`<kind>:<name>[@<version>]`).
    pub id: String,
    /// Package name.
    pub name: String,
    /// Package version.
    pub version: Option<String>,
    /// Package kind (Cargo or Npm).
    pub kind: DependencyKind,
    /// Path to the package manifest.
    pub manifest_path: PathBuf,
    /// Whether the package belongs to a workspace.
    pub in_workspace: bool,
    /// Workspace root directory, when the package is in a workspace.
    pub workspace_root: Option<PathBuf>,
}

/// A directed dependency edge between two graph nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Id of the depending package.
    pub from: String,
    /// Id of the dependency (a local package id, or `<kind>:<name>` when
    /// unresolved).
    pub to: String,
    /// Version requirement as declared in the manifest.
    pub requirement: String,
    /// Dependency scope (normal, development, build, optional, peer, workspace).
    pub scope: DependencyScope,
    /// Whether `to` resolves to a package found locally in the project.
    pub resolved: bool,
}

/// A project's dependency graph: every local package and its dependency edges.
///
/// Transitive dependencies are obtained by following `resolved` edges between
/// local nodes; edges to external packages carry only their declared
/// requirement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageGraph {
    /// All local packages (nodes).
    pub packages: Vec<GraphPackage>,
    /// All dependency edges.
    pub edges: Vec<GraphEdge>,
}

/// Builds a dependency graph from a project manifest.
///
/// Only local manifests are used; nothing is resolved from the network.
/// Edges to packages not found locally are marked `resolved: false` and point
/// at a synthetic id (`<kind>:<name>`).
#[must_use]
pub fn build(manifest: &ProjectManifest) -> PackageGraph {
    let mut graph = PackageGraph::default();

    for pkg in &manifest.cargo_packages {
        graph.packages.push(GraphPackage {
            id: package_id(&DependencyKind::Cargo, &pkg.name, pkg.version.as_deref()),
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            kind: DependencyKind::Cargo,
            manifest_path: pkg.manifest_path.clone(),
            in_workspace: pkg.workspace,
            workspace_root: pkg.workspace_root.clone(),
        });
    }

    for pkg in &manifest.node_packages {
        graph.packages.push(GraphPackage {
            id: package_id(&DependencyKind::Npm, &pkg.name, pkg.version.as_deref()),
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            kind: DependencyKind::Npm,
            manifest_path: pkg.manifest_path.clone(),
            in_workspace: !pkg.workspaces.is_empty(),
            workspace_root: None,
        });
    }

    let node_offset = manifest.cargo_packages.len();
    for (i, pkg) in manifest.cargo_packages.iter().enumerate() {
        let from = graph.packages[i].id.clone();
        for dep in pkg
            .dependencies
            .iter()
            .chain(&pkg.dev_dependencies)
            .chain(&pkg.build_dependencies)
            .chain(&pkg.target_dependencies)
        {
            push_edge(&mut graph, &from, dep);
        }
    }
    for (i, pkg) in manifest.node_packages.iter().enumerate() {
        let from = graph.packages[node_offset + i].id.clone();
        for dep in pkg
            .dependencies
            .iter()
            .chain(&pkg.dev_dependencies)
            .chain(&pkg.peer_dependencies)
            .chain(&pkg.optional_dependencies)
        {
            push_edge(&mut graph, &from, dep);
        }
    }

    graph
}

impl PackageGraph {
    /// Returns the ids of all local packages reachable from `id` via resolved
    /// edges, excluding `id` itself (breadth-first traversal).
    #[must_use]
    pub fn transitive_dependencies(&self, id: &str) -> Vec<String> {
        let mut visited = BTreeSet::new();
        let mut queue = Vec::new();

        for edge in &self.edges {
            if edge.from == id && edge.resolved && visited.insert(edge.to.clone()) {
                queue.push(edge.to.clone());
            }
        }

        let mut index = 0;
        while index < queue.len() {
            let current = queue[index].clone();
            index += 1;
            for edge in &self.edges {
                if edge.from == current && edge.resolved && visited.insert(edge.to.clone()) {
                    queue.push(edge.to.clone());
                }
            }
        }

        visited.remove(id);
        visited.into_iter().collect()
    }
}

/// Lowercase label used to prefix package and synthetic ids.
const fn kind_label(kind: &DependencyKind) -> &'static str {
    match *kind {
        DependencyKind::Cargo => "cargo",
        DependencyKind::Npm => "npm",
    }
}

/// Builds the graph id for a package: `<kind>:<name>` plus `@<version>` when a
/// version is present.
fn package_id(kind: &DependencyKind, name: &str, version: Option<&str>) -> String {
    let label = kind_label(kind);
    let base = format!("{label}:{name}");
    match version {
        Some(v) => format!("{base}@{v}"),
        None => base,
    }
}

/// Resolves a declared dependency against the local package set.
///
/// Returns the target id and whether it resolved to a local package. When no
/// local package matches, the target is the synthetic id `<kind>:<name>`.
fn resolve(
    packages: &[GraphPackage],
    name: &str,
    kind: &DependencyKind,
    requirement: &str,
) -> (String, bool) {
    let candidates: Vec<&GraphPackage> = packages
        .iter()
        .filter(|pkg| pkg.kind == *kind && pkg.name == name)
        .collect();

    let label = kind_label(kind);
    let synthetic = format!("{label}:{name}");

    if candidates.is_empty() {
        return (synthetic, false);
    }

    if let Ok(req) = VersionReq::parse(requirement)
        && let Some(matched) = candidates.iter().find(|candidate| {
            candidate
                .version
                .as_deref()
                .and_then(|v| Version::parse(v).ok())
                .is_some_and(|version| req.matches(&version))
        })
    {
        return (matched.id.clone(), true);
    }

    candidates
        .first()
        .map_or((synthetic, false), |candidate| (candidate.id.clone(), true))
}

/// Records one edge for a dependency of the package with id `from`.
fn push_edge(graph: &mut PackageGraph, from: &str, dep: &Dependency) {
    let (to, resolved) = resolve(&graph.packages, &dep.name, &dep.kind, &dep.requirement);
    graph.edges.push(GraphEdge {
        from: from.to_owned(),
        to,
        requirement: dep.requirement.clone(),
        scope: dep.scope.clone(),
        resolved,
    });
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::types::{CargoPackage, CargoTargets, NodePackage, NodeScripts};

    fn dep(
        kind: DependencyKind,
        name: &str,
        requirement: &str,
        scope: DependencyScope,
    ) -> Dependency {
        Dependency {
            kind,
            name: name.to_owned(),
            requirement: requirement.to_owned(),
            scope,
        }
    }

    fn cargo_package(name: &str, version: Option<&str>, deps: Vec<Dependency>) -> CargoPackage {
        CargoPackage {
            name: name.to_owned(),
            version: version.map(str::to_owned),
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
            dependencies: deps,
            dev_dependencies: Vec::new(),
            build_dependencies: Vec::new(),
            target_dependencies: Vec::new(),
            features: Vec::new(),
            default_features: Vec::new(),
            targets: CargoTargets::default(),
        }
    }

    fn node_package(name: &str, version: Option<&str>, deps: Vec<Dependency>) -> NodePackage {
        NodePackage {
            name: name.to_owned(),
            version: version.map(str::to_owned),
            description: None,
            homepage: None,
            repository: None,
            license: None,
            manifest_path: PathBuf::from(format!("{name}/package.json")),
            package_manager: None,
            lockfiles: Vec::new(),
            dependencies: deps,
            dev_dependencies: Vec::new(),
            peer_dependencies: Vec::new(),
            optional_dependencies: Vec::new(),
            scripts: NodeScripts::default(),
            engines: BTreeMap::new(),
            workspaces: Vec::new(),
        }
    }

    #[test]
    fn build_creates_one_node_per_local_package() {
        let manifest = ProjectManifest {
            root: PathBuf::from("."),
            cargo_packages: vec![
                cargo_package("app", Some("1.0.0"), Vec::new()),
                cargo_package("lib-a", Some("0.2.0"), Vec::new()),
            ],
            node_packages: vec![node_package("web", Some("3.0.0"), Vec::new())],
            ..Default::default()
        };

        let graph = build(&manifest);

        assert_eq!(graph.packages.len(), 3);
        let ids: Vec<&str> = graph.packages.iter().map(|pkg| pkg.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["cargo:app@1.0.0", "cargo:lib-a@0.2.0", "npm:web@3.0.0"]
        );
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn build_marks_local_edges_resolved_and_external_edges_unresolved() {
        let manifest = ProjectManifest {
            root: PathBuf::from("."),
            cargo_packages: vec![
                cargo_package(
                    "app",
                    Some("1.0.0"),
                    vec![
                        dep(
                            DependencyKind::Cargo,
                            "lib-a",
                            "0.2",
                            DependencyScope::Normal,
                        ),
                        dep(DependencyKind::Cargo, "serde", "1", DependencyScope::Normal),
                    ],
                ),
                cargo_package("lib-a", Some("0.2.0"), Vec::new()),
            ],
            ..Default::default()
        };

        let graph = build(&manifest);

        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].from, "cargo:app@1.0.0");
        assert_eq!(graph.edges[0].to, "cargo:lib-a@0.2.0");
        assert!(graph.edges[0].resolved);
        assert_eq!(graph.edges[1].to, "cargo:serde");
        assert!(!graph.edges[1].resolved);
    }

    #[test]
    fn build_prefers_candidate_matching_version_requirement() {
        let manifest = ProjectManifest {
            root: PathBuf::from("."),
            cargo_packages: vec![
                cargo_package(
                    "app",
                    Some("1.0.0"),
                    vec![dep(
                        DependencyKind::Cargo,
                        "lib-a",
                        "^0.3",
                        DependencyScope::Normal,
                    )],
                ),
                cargo_package("lib-a", Some("0.2.0"), Vec::new()),
                cargo_package("lib-a", Some("0.3.1"), Vec::new()),
            ],
            ..Default::default()
        };

        let graph = build(&manifest);

        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].to, "cargo:lib-a@0.3.1");
        assert!(graph.edges[0].resolved);
    }

    #[test]
    fn build_preserves_dependency_scopes() {
        let manifest = ProjectManifest {
            root: PathBuf::from("."),
            cargo_packages: vec![CargoPackage {
                dependencies: vec![dep(
                    DependencyKind::Cargo,
                    "crate-a",
                    "1",
                    DependencyScope::Normal,
                )],
                dev_dependencies: vec![dep(
                    DependencyKind::Cargo,
                    "crate-b",
                    "1",
                    DependencyScope::Development,
                )],
                build_dependencies: vec![dep(
                    DependencyKind::Cargo,
                    "crate-c",
                    "1",
                    DependencyScope::Build,
                )],
                target_dependencies: vec![dep(
                    DependencyKind::Cargo,
                    "crate-d",
                    "1",
                    DependencyScope::Workspace,
                )],
                ..cargo_package("app", Some("1.0.0"), Vec::new())
            }],
            node_packages: vec![NodePackage {
                dependencies: vec![dep(
                    DependencyKind::Npm,
                    "pkg-a",
                    "1",
                    DependencyScope::Normal,
                )],
                dev_dependencies: vec![dep(
                    DependencyKind::Npm,
                    "pkg-b",
                    "1",
                    DependencyScope::Development,
                )],
                peer_dependencies: vec![dep(
                    DependencyKind::Npm,
                    "pkg-c",
                    "1",
                    DependencyScope::Peer,
                )],
                optional_dependencies: vec![dep(
                    DependencyKind::Npm,
                    "pkg-d",
                    "1",
                    DependencyScope::Optional,
                )],
                ..node_package("web", Some("1.0.0"), Vec::new())
            }],
            ..Default::default()
        };

        let graph = build(&manifest);

        let scopes: Vec<&DependencyScope> = graph.edges.iter().map(|edge| &edge.scope).collect();
        assert_eq!(
            scopes,
            vec![
                &DependencyScope::Normal,
                &DependencyScope::Development,
                &DependencyScope::Build,
                &DependencyScope::Workspace,
                &DependencyScope::Normal,
                &DependencyScope::Development,
                &DependencyScope::Peer,
                &DependencyScope::Optional,
            ]
        );
    }

    #[test]
    fn transitive_dependencies_follows_resolved_edges_and_excludes_start() {
        let manifest = ProjectManifest {
            root: PathBuf::from("."),
            cargo_packages: vec![
                cargo_package(
                    "a",
                    Some("1.0.0"),
                    vec![dep(
                        DependencyKind::Cargo,
                        "b",
                        "1",
                        DependencyScope::Normal,
                    )],
                ),
                cargo_package(
                    "b",
                    Some("1.0.0"),
                    vec![dep(
                        DependencyKind::Cargo,
                        "c",
                        "1",
                        DependencyScope::Normal,
                    )],
                ),
                cargo_package(
                    "c",
                    Some("1.0.0"),
                    vec![dep(
                        DependencyKind::Cargo,
                        "d",
                        "1",
                        DependencyScope::Normal,
                    )],
                ),
                cargo_package("d", Some("1.0.0"), Vec::new()),
            ],
            ..Default::default()
        };

        let graph = build(&manifest);
        let result = graph.transitive_dependencies("cargo:a@1.0.0");

        assert_eq!(
            result,
            vec!["cargo:b@1.0.0", "cargo:c@1.0.0", "cargo:d@1.0.0"]
        );
    }
}
