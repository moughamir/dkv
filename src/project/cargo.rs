use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{
    error::Result,
    types::{
        CargoPackage, CargoTarget, CargoTargets, CargoWorkspace, Dependency, DependencyKind,
        DependencyScope,
    },
};

/// A parsed `Cargo.toml` manifest.
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
    #[serde(default)]
    target: BTreeMap<String, TargetSection>,
    #[serde(default)]
    features: BTreeMap<String, Vec<String>>,
    lib: Option<TargetTable>,
    #[serde(default)]
    bin: Vec<TargetTable>,
    #[serde(default)]
    example: Vec<TargetTable>,
    #[serde(default)]
    test: Vec<TargetTable>,
    #[serde(default)]
    bench: Vec<TargetTable>,
}

/// The `[package]` section of a `Cargo.toml` manifest.
#[derive(Debug, Clone, Default, Deserialize)]
struct Package {
    name: Option<String>,
    version: Option<String>,
    edition: Option<String>,
    license: Option<String>,
    description: Option<String>,
    repository: Option<String>,
    homepage: Option<String>,
    workspace: Option<String>,
    #[serde(default)]
    authors: Vec<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    categories: Vec<String>,
}

/// The `[workspace]` section of a `Cargo.toml` manifest.
#[derive(Debug, Deserialize)]
struct Workspace {
    #[serde(default)]
    members: Vec<String>,
    #[serde(default)]
    default_members: Vec<String>,
    #[serde(rename = "dependencies", default)]
    dependencies: BTreeMap<String, DependencySpec>,
}

/// A `[target.<triple>]` section with its dependency tables.
#[derive(Debug, Deserialize)]
struct TargetSection {
    #[serde(default)]
    dependencies: BTreeMap<String, DependencySpec>,
    #[serde(rename = "dev-dependencies", default)]
    dev_dependencies: BTreeMap<String, DependencySpec>,
    #[serde(rename = "build-dependencies", default)]
    build_dependencies: BTreeMap<String, DependencySpec>,
}

/// A declared target table (`[lib]`, `[[bin]]`, `[[example]]`, ...).
#[derive(Debug, Deserialize)]
struct TargetTable {
    name: Option<String>,
    path: Option<String>,
}

/// A dependency specification: either a bare version string or a detailed
/// table with arbitrary keys.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DependencySpec {
    Version(String),
    Detailed(BTreeMap<String, toml::Value>),
}

/// Extracts the version requirement from a dependency specification,
/// defaulting to `"*"` when a detailed spec does not pin one.
fn requirement(spec: &DependencySpec) -> String {
    match spec {
        DependencySpec::Version(version) => version.clone(),
        DependencySpec::Detailed(map) => map
            .get("version")
            .and_then(toml::Value::as_str)
            .unwrap_or("*")
            .to_string(),
    }
}

/// Whether a dependency spec is an inherited workspace dependency
/// (`{ workspace = true }`).
fn is_workspace_spec(spec: &DependencySpec) -> bool {
    matches!(
        spec,
        DependencySpec::Detailed(map)
            if map.get("workspace").and_then(toml::Value::as_bool) == Some(true)
    )
}

/// Builds a Cargo dependency from a manifest entry.
fn build_dependency(name: String, spec: &DependencySpec, scope: &DependencyScope) -> Dependency {
    let resolved_scope: &DependencyScope = if is_workspace_spec(spec) {
        &DependencyScope::Workspace
    } else {
        scope
    };
    Dependency {
        kind: DependencyKind::Cargo,
        name,
        requirement: requirement(spec),
        scope: resolved_scope.clone(),
    }
}

/// Collects a dependency table into a vector of [`Dependency`]s.
fn collect_dependencies(
    specs: &BTreeMap<String, DependencySpec>,
    scope: &DependencyScope,
) -> Vec<Dependency> {
    specs
        .iter()
        .map(|(name, spec)| build_dependency(name.clone(), spec, scope))
        .collect()
}

/// Flattens the dependency tables of every `[target.<triple>]` section into a
/// single vector.
fn collect_target_dependencies(targets: &BTreeMap<String, TargetSection>) -> Vec<Dependency> {
    let mut deps = Vec::new();
    for section in targets.values() {
        deps.extend(collect_dependencies(
            &section.dependencies,
            &DependencyScope::Normal,
        ));
        deps.extend(collect_dependencies(
            &section.dev_dependencies,
            &DependencyScope::Development,
        ));
        deps.extend(collect_dependencies(
            &section.build_dependencies,
            &DependencyScope::Build,
        ));
    }
    deps
}

/// The directory containing a manifest, or the current directory when the
/// manifest has no parent directory.
fn package_dir_of(manifest_path: &Path) -> PathBuf {
    manifest_path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Resolves workspace member entries — explicit directories and `dir/*` globs
/// — into `Cargo.toml` paths rooted at `root`.
fn resolve_members(root: &Path, entries: &[String]) -> Vec<PathBuf> {
    let mut members = Vec::new();
    for entry in entries {
        let trimmed = entry.trim_end_matches('/');
        if let Some(prefix) = trimmed.strip_suffix("/*") {
            if let Ok(dir_entries) = fs::read_dir(root.join(prefix)) {
                for subdir in dir_entries.flatten() {
                    let candidate = subdir.path().join("Cargo.toml");
                    if candidate.is_file() {
                        members.push(candidate);
                    }
                }
            }
        } else {
            let candidate = root.join(trimmed).join("Cargo.toml");
            if candidate.is_file() {
                members.push(candidate);
            }
        }
    }
    members.sort();
    members.dedup();
    members
}

/// Builds workspace data for a manifest that declares `[workspace]`.
fn build_workspace_info(manifest: &CargoToml, manifest_path: &Path) -> Option<CargoWorkspace> {
    let workspace = manifest.workspace.as_ref()?;
    let root = package_dir_of(manifest_path);

    let mut members = resolve_members(&root, &workspace.members);
    if manifest.package.is_some() {
        members.push(manifest_path.to_path_buf());
    }
    members.sort();
    members.dedup();

    let default_members = if workspace.default_members.is_empty() {
        members.clone()
    } else {
        resolve_members(&root, &workspace.default_members)
    };

    let dependencies = workspace
        .dependencies
        .iter()
        .map(|(name, spec)| build_dependency(name.clone(), spec, &DependencyScope::Normal))
        .collect();

    Some(CargoWorkspace {
        root,
        members,
        default_members,
        dependencies,
    })
}

/// Walks the ancestors of a manifest looking for a workspace that lists this
/// manifest as a member.
fn find_workspace_root(manifest_path: &Path) -> Option<PathBuf> {
    let canonical_manifest = fs::canonicalize(manifest_path).ok();
    let start = manifest_path.parent()?;
    for ancestor in start.ancestors().skip(1) {
        let candidate = ancestor.join("Cargo.toml");
        let Ok(text) = fs::read_to_string(candidate) else {
            continue;
        };
        let Ok(ancestor_manifest) = toml::from_str::<CargoToml>(&text) else {
            continue;
        };
        let Some(workspace) = ancestor_manifest.workspace.as_ref() else {
            continue;
        };
        let members = resolve_members(ancestor, &workspace.members);
        let is_member = canonical_manifest.as_ref().is_some_and(|canonical_self| {
            members.iter().any(|member| {
                fs::canonicalize(member)
                    .ok()
                    .is_some_and(|canonical_member| canonical_member == *canonical_self)
            })
        });
        if is_member {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

/// Determines whether a manifest belongs to a Cargo workspace and, if so,
/// which directory is the workspace root.
fn determine_workspace(manifest: &CargoToml, manifest_path: &Path) -> (bool, Option<PathBuf>) {
    if let Some(workspace_path) = manifest.package.as_ref().and_then(|p| p.workspace.as_ref()) {
        let candidate = package_dir_of(manifest_path)
            .join(workspace_path)
            .join("Cargo.toml");
        let canonical = fs::canonicalize(&candidate).unwrap_or(candidate);
        let root_dir = canonical
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        return (true, Some(root_dir));
    }

    if manifest.workspace.is_some() {
        return (true, Some(package_dir_of(manifest_path)));
    }

    let root = find_workspace_root(manifest_path);
    (root.is_some(), root)
}

/// Converts a declared target table into a [`CargoTarget`].
fn target_from_table(table: &TargetTable) -> CargoTarget {
    CargoTarget {
        name: table.name.clone().unwrap_or_default(),
        path: table.path.as_ref().map(PathBuf::from),
    }
}

/// Infers Rust source targets from the top-level `*.rs` files inside a
/// directory (subdirectories are ignored).
fn infer_rs_files(dir: &Path) -> Vec<CargoTarget> {
    let mut targets = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return targets;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_rs_file = path.is_file() && path.extension().is_some_and(|ext| ext == "rs");
        if !is_rs_file {
            continue;
        }
        targets.push(CargoTarget {
            name: path
                .file_stem()
                .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned()),
            path: Some(path),
        });
    }
    targets.sort_by_key(|target| target.name.clone());
    targets
}

/// Collects the declared and conventionally inferred targets of a package.
fn collect_targets(manifest: &CargoToml, package_dir: &Path, package_name: &str) -> CargoTargets {
    let declared_lib = manifest.lib.as_ref().map(target_from_table);

    let lib = match (declared_lib, package_dir.join("src/lib.rs").is_file()) {
        (Some(target), _) => Some(target),
        (None, true) => Some(CargoTarget {
            name: package_name.replace('-', "_"),
            path: Some(PathBuf::from("src/lib.rs")),
        }),
        (None, false) => None,
    };

    let mut bins = manifest
        .bin
        .iter()
        .map(target_from_table)
        .collect::<Vec<_>>();
    if bins.is_empty() {
        if package_dir.join("src/main.rs").is_file() {
            bins.push(CargoTarget {
                name: package_name.replace('-', "_"),
                path: Some(PathBuf::from("src/main.rs")),
            });
        }
        bins.extend(infer_rs_files(&package_dir.join("src/bin")));
    }

    let mut examples = manifest
        .example
        .iter()
        .map(target_from_table)
        .collect::<Vec<_>>();
    if examples.is_empty() {
        examples = infer_rs_files(&package_dir.join("examples"));
    }

    let mut tests = manifest
        .test
        .iter()
        .map(target_from_table)
        .collect::<Vec<_>>();
    if tests.is_empty() {
        tests = infer_rs_files(&package_dir.join("tests"));
    }

    let mut benches = manifest
        .bench
        .iter()
        .map(target_from_table)
        .collect::<Vec<_>>();
    if benches.is_empty() {
        benches = infer_rs_files(&package_dir.join("benches"));
    }

    CargoTargets {
        lib,
        bins,
        examples,
        benches,
        tests,
    }
}

/// Loads a Cargo package manifest and parses its metadata, dependencies,
/// features, targets, and workspace relationships.
///
/// Workspace membership is resolved from `[package].workspace`, a declared
/// `[workspace]` section, or by walking ancestor manifests. Virtual manifests
/// (no `[package]` section) yield an empty package name.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] if the manifest cannot be read or
/// contains invalid TOML.
pub fn load(path: &Path) -> Result<CargoPackage> {
    let text = fs::read_to_string(path)?;

    let manifest: CargoToml = toml::from_str(&text)?;

    let manifest_path = path.to_path_buf();
    let package_dir = package_dir_of(path);
    let package = manifest.package.clone().unwrap_or_default();
    let name = package.name.clone().unwrap_or_default();
    let (workspace, workspace_root) = determine_workspace(&manifest, path);
    let workspace_info = if manifest.workspace.is_some() {
        build_workspace_info(&manifest, path)
    } else {
        None
    };

    Ok(CargoPackage {
        name: name.clone(),
        version: package.version,
        edition: package.edition,
        license: package.license,
        description: package.description,
        repository: package.repository,
        homepage: package.homepage,
        authors: package.authors,
        keywords: package.keywords,
        categories: package.categories,
        manifest_path,
        workspace,
        workspace_root,
        workspace_info,
        dependencies: collect_dependencies(&manifest.dependencies, &DependencyScope::Normal),
        dev_dependencies: collect_dependencies(
            &manifest.dev_dependencies,
            &DependencyScope::Development,
        ),
        build_dependencies: collect_dependencies(
            &manifest.build_dependencies,
            &DependencyScope::Build,
        ),
        target_dependencies: collect_target_dependencies(&manifest.target),
        features: manifest.features.keys().cloned().collect(),
        default_features: manifest
            .features
            .get("default")
            .cloned()
            .unwrap_or_default(),
        targets: collect_targets(&manifest, &package_dir, &name),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes a manifest file into a directory and returns its path.
    fn write_manifest(dir: &Path, name: &str, contents: &str) -> PathBuf {
        let path = dir.join(name);
        #[allow(clippy::unwrap_used)]
        fs::write(&path, contents).unwrap();
        path
    }

    /// Finds a dependency by name within a dependency vector.
    fn find_dep<'a>(deps: &'a [Dependency], name: &str) -> &'a Dependency {
        #[allow(clippy::unwrap_used)]
        deps.iter().find(|dep| dep.name == name).unwrap()
    }

    #[test]
    fn test_load_extracts_full_metadata() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let manifest_path = write_manifest(
            dir.path(),
            "Cargo.toml",
            r#"
[package]
name = "demo"
version = "1.2.3"
edition = "2021"
license = "MIT"
description = "A demo crate"
repository = "https://example.com/repo"
homepage = "https://example.com"
authors = ["Alice <alice@example.com>"]
keywords = ["demo", "example"]
categories = ["development-tools"]

[dependencies]
serde = "1.0"
tokio = { version = "0.3", features = ["rt"] }
shared = { workspace = true }

[dev-dependencies]
tempfile = "3"

[build-dependencies]
cc = "1"

[features]
default = ["std"]
std = []

[target.'cfg(unix)'.dependencies]
libc = "0.2"

[target.'cfg(windows)'.dev-dependencies]
winapi = "0.3"

[[bin]]
name = "cli"
path = "src/bin/cli.rs"

[[example]]
name = "demo_example"

[lib]
name = "demo_lib"
path = "src/lib.rs"
"#,
        );

        #[allow(clippy::unwrap_used)]
        let package = load(&manifest_path).unwrap();

        assert_eq!(package.name, "demo");
        assert_eq!(package.version.as_deref(), Some("1.2.3"));
        assert_eq!(package.edition.as_deref(), Some("2021"));
        assert_eq!(package.license.as_deref(), Some("MIT"));
        assert_eq!(package.description.as_deref(), Some("A demo crate"));
        assert_eq!(
            package.repository.as_deref(),
            Some("https://example.com/repo")
        );
        assert_eq!(package.homepage.as_deref(), Some("https://example.com"));
        assert_eq!(package.authors, vec!["Alice <alice@example.com>"]);
        assert_eq!(package.keywords, vec!["demo", "example"]);
        assert_eq!(package.categories, vec!["development-tools"]);
        assert_eq!(package.manifest_path, manifest_path);
        assert!(!package.workspace);
        assert_eq!(package.workspace_root, None);
        assert!(package.workspace_info.is_none());

        assert_eq!(find_dep(&package.dependencies, "serde").requirement, "1.0");
        assert!(matches!(
            find_dep(&package.dependencies, "serde").scope,
            DependencyScope::Normal
        ));
        assert_eq!(find_dep(&package.dependencies, "tokio").requirement, "0.3");
        assert!(matches!(
            find_dep(&package.dependencies, "shared").scope,
            DependencyScope::Workspace
        ));
        assert!(matches!(
            find_dep(&package.dev_dependencies, "tempfile").scope,
            DependencyScope::Development
        ));
        assert!(matches!(
            find_dep(&package.build_dependencies, "cc").scope,
            DependencyScope::Build
        ));
        assert!(matches!(
            find_dep(&package.target_dependencies, "libc").scope,
            DependencyScope::Normal
        ));
        assert!(matches!(
            find_dep(&package.target_dependencies, "winapi").scope,
            DependencyScope::Development
        ));

        assert_eq!(package.features, vec!["default", "std"]);
        assert_eq!(package.default_features, vec!["std"]);

        let targets = &package.targets;
        #[allow(clippy::unwrap_used)]
        let lib = targets.lib.as_ref().unwrap();
        assert_eq!(lib.name, "demo_lib");
        assert_eq!(lib.path.as_deref(), Some(Path::new("src/lib.rs")));
        assert_eq!(targets.bins.len(), 1);
        assert_eq!(targets.bins[0].name, "cli");
        assert_eq!(targets.examples.len(), 1);
        assert_eq!(targets.examples[0].name, "demo_example");
    }

    #[test]
    fn test_workspace_members_glob_resolution() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("crates/a")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("crates/b")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("crates/c")).unwrap();
        write_manifest(
            &root.join("crates/a"),
            "Cargo.toml",
            "[package]\nname = \"a\"\n",
        );
        write_manifest(
            &root.join("crates/b"),
            "Cargo.toml",
            "[package]\nname = \"b\"\n",
        );
        write_manifest(
            root,
            "Cargo.toml",
            r#"
[workspace]
members = ["crates/*"]
dependencies = { serde = "1.0" }
"#,
        );

        #[allow(clippy::unwrap_used)]
        let package = load(&root.join("Cargo.toml")).unwrap();

        assert_eq!(package.name, "");
        assert!(package.workspace);
        #[allow(clippy::unwrap_used)]
        let info = package.workspace_info.unwrap();
        assert_eq!(info.root, root.to_path_buf());
        assert_eq!(info.members.len(), 2);
        assert_eq!(info.members[0], root.join("crates/a/Cargo.toml"));
        assert_eq!(info.members[1], root.join("crates/b/Cargo.toml"));
        assert_eq!(info.default_members, info.members);
        assert_eq!(info.dependencies.len(), 1);
        assert_eq!(find_dep(&info.dependencies, "serde").requirement, "1.0");
        assert!(matches!(
            find_dep(&info.dependencies, "serde").scope,
            DependencyScope::Normal
        ));
    }

    #[test]
    fn test_workspace_membership_via_ancestor() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let ws_root = dir.path().join("workspace");
        let member_dir = ws_root.join("member");
        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(&member_dir).unwrap();

        let ws_manifest = write_manifest(
            &ws_root,
            "Cargo.toml",
            "[workspace]\nmembers = [\"member\"]\n",
        );
        let member_manifest =
            write_manifest(&member_dir, "Cargo.toml", "[package]\nname = \"member\"\n");

        #[allow(clippy::unwrap_used)]
        let member = load(&member_manifest).unwrap();
        assert!(member.workspace);
        assert_eq!(member.workspace_root.as_deref(), Some(ws_root.as_path()));
        assert!(member.workspace_info.is_none());

        #[allow(clippy::unwrap_used)]
        let root_package = load(&ws_manifest).unwrap();
        assert!(root_package.workspace);
        assert_eq!(
            root_package.workspace_root.as_deref(),
            Some(ws_root.as_path())
        );
        #[allow(clippy::unwrap_used)]
        let info = root_package.workspace_info.unwrap();
        assert_eq!(info.members, vec![member_manifest]);
    }

    #[test]
    fn test_target_inference() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        for sub in ["src/bin", "examples", "examples/nested", "tests", "benches"] {
            #[allow(clippy::unwrap_used)]
            fs::create_dir_all(root.join(sub)).unwrap();
        }
        for file in [
            "src/lib.rs",
            "src/main.rs",
            "src/bin/tool.rs",
            "examples/demo_example.rs",
            "examples/nested/skip_me.rs",
            "tests/integration.rs",
            "benches/benchmark.rs",
        ] {
            #[allow(clippy::unwrap_used)]
            fs::write(root.join(file), "").unwrap();
        }
        let manifest = write_manifest(root, "Cargo.toml", "[package]\nname = \"my-crate\"\n");

        #[allow(clippy::unwrap_used)]
        let package = load(&manifest).unwrap();
        let targets = &package.targets;

        #[allow(clippy::unwrap_used)]
        let lib = targets.lib.as_ref().unwrap();
        assert_eq!(lib.name, "my_crate");
        assert_eq!(lib.path.as_deref(), Some(Path::new("src/lib.rs")));

        assert_eq!(targets.bins.len(), 2);
        assert_eq!(targets.bins[0].name, "my_crate");
        assert_eq!(
            targets.bins[0].path.as_deref(),
            Some(Path::new("src/main.rs"))
        );
        assert_eq!(targets.bins[1].name, "tool");
        assert_eq!(
            targets.bins[1].path.as_deref(),
            Some(root.join("src/bin/tool.rs").as_path())
        );

        assert_eq!(targets.examples.len(), 1);
        assert_eq!(targets.examples[0].name, "demo_example");
        assert_eq!(
            targets.examples[0].path.as_deref(),
            Some(root.join("examples/demo_example.rs").as_path())
        );

        assert_eq!(targets.tests.len(), 1);
        assert_eq!(targets.tests[0].name, "integration");
        assert_eq!(
            targets.tests[0].path.as_deref(),
            Some(root.join("tests/integration.rs").as_path())
        );

        assert_eq!(targets.benches.len(), 1);
        assert_eq!(targets.benches[0].name, "benchmark");
        assert_eq!(
            targets.benches[0].path.as_deref(),
            Some(root.join("benches/benchmark.rs").as_path())
        );
    }

    #[test]
    fn test_workspace_inherited_dependency_scope() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let manifest_path = write_manifest(
            dir.path(),
            "Cargo.toml",
            r#"
[package]
name = "consumer"
version = "0.1.0"

[dependencies]
shared = { workspace = true }
pinned = { version = "2.0", path = "../pinned" }
plain = "1.0"
"#,
        );

        #[allow(clippy::unwrap_used)]
        let package = load(&manifest_path).unwrap();

        let shared = find_dep(&package.dependencies, "shared");
        assert_eq!(shared.requirement, "*");
        assert!(matches!(shared.scope, DependencyScope::Workspace));

        let pinned = find_dep(&package.dependencies, "pinned");
        assert_eq!(pinned.requirement, "2.0");
        assert!(matches!(pinned.scope, DependencyScope::Normal));

        let plain = find_dep(&package.dependencies, "plain");
        assert_eq!(plain.requirement, "1.0");
        assert!(matches!(plain.scope, DependencyScope::Normal));
    }
}
