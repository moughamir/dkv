//! npm (`package.json`) manifest parsing and lockfile handling.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{
    error::Result,
    types::{
        Dependency, DependencyKind, DependencyScope, LockedPackage, LockfileKind, NodePackage,
        NodeScripts, PackageManager,
    },
};

/// Raw `package.json` structure (relevant subset).
#[derive(Debug, Deserialize)]
struct PackageJson {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    homepage: Option<String>,
    repository: Option<RepositorySpec>,
    license: Option<String>,

    #[serde(default)]
    dependencies: BTreeMap<String, String>,

    #[serde(rename = "devDependencies", default)]
    dev_dependencies: BTreeMap<String, String>,

    #[serde(rename = "peerDependencies", default)]
    peer_dependencies: BTreeMap<String, String>,

    #[serde(rename = "optionalDependencies", default)]
    optional_dependencies: BTreeMap<String, String>,

    #[serde(default)]
    scripts: BTreeMap<String, String>,

    #[serde(default)]
    engines: BTreeMap<String, String>,

    #[serde(rename = "packageManager")]
    package_manager: Option<String>,

    #[serde(default)]
    workspaces: Option<WorkspacesField>,
}

/// The `repository` field, either a URL string or a detailed object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RepositorySpec {
    /// Plain repository URL string.
    String(String),
    /// Detailed repository object.
    Detailed { url: Option<String> },
}

/// The `workspaces` field, either a list of globs or a detailed object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum WorkspacesField {
    /// List of workspace globs.
    List(Vec<String>),
    /// Detailed workspaces object.
    Detailed { packages: Vec<String> },
}

/// Loads an npm package manifest.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] if the manifest cannot be read or contains
/// invalid JSON.
pub fn load(path: &Path) -> Result<NodePackage> {
    let text = fs::read_to_string(path)?;
    let package: PackageJson = serde_json::from_str(&text)?;

    let dir = path.parent().unwrap_or(path);

    let scripts = NodeScripts {
        build: package.scripts.get("build").cloned(),
        test: package.scripts.get("test").cloned(),
        lint: package.scripts.get("lint").cloned(),
        dev: package.scripts.get("dev").cloned(),
        start: package.scripts.get("start").cloned(),
    };

    let repository = match package.repository {
        Some(RepositorySpec::String(repository)) => Some(repository),
        Some(RepositorySpec::Detailed { url }) => url,
        None => None,
    };

    Ok(NodePackage {
        name: package.name.unwrap_or_default(),
        version: package.version,
        description: package.description,
        homepage: package.homepage,
        repository,
        license: package.license,
        manifest_path: path.to_path_buf(),
        package_manager: detect_manager(package.package_manager.as_deref(), dir),
        lockfiles: detect_lockfiles(dir),
        dependencies: to_dependencies(package.dependencies, DependencyScope::Normal),
        dev_dependencies: to_dependencies(package.dev_dependencies, DependencyScope::Development),
        peer_dependencies: to_dependencies(package.peer_dependencies, DependencyScope::Peer),
        optional_dependencies: to_dependencies(
            package.optional_dependencies,
            DependencyScope::Optional,
        ),
        scripts,
        engines: package.engines,
        workspaces: resolve_workspaces(&package.workspaces, dir),
    })
}

/// Detects the package manager from the `packageManager` field or, failing
/// that, from lockfile and workspace-file presence.
#[must_use]
fn detect_manager(pm_field: Option<&str>, dir: &Path) -> Option<PackageManager> {
    if let Some(field) = pm_field {
        let name = field.split_once('@').map_or(field, |(name, _)| name);
        return match name {
            "npm" => Some(PackageManager::Npm),
            "pnpm" => Some(PackageManager::Pnpm),
            "bun" => Some(PackageManager::Bun),
            _ => None,
        };
    }
    if dir.join("bun.lock").is_file() || dir.join("bun.lockb").is_file() {
        return Some(PackageManager::Bun);
    }
    if dir.join("pnpm-workspace.yaml").is_file() || dir.join("pnpm-lock.yaml").is_file() {
        return Some(PackageManager::Pnpm);
    }
    if dir.join("package-lock.json").is_file() {
        return Some(PackageManager::Npm);
    }
    None
}

/// Detects all lockfiles present in `dir`.
#[must_use]
fn detect_lockfiles(dir: &Path) -> Vec<LockfileKind> {
    let mut lockfiles = Vec::new();
    if dir.join("package-lock.json").is_file() {
        lockfiles.push(LockfileKind::PackageLock);
    }
    if dir.join("bun.lock").is_file() {
        lockfiles.push(LockfileKind::BunLock);
    }
    if dir.join("bun.lockb").is_file() {
        lockfiles.push(LockfileKind::BunLockb);
    }
    if dir.join("pnpm-lock.yaml").is_file() {
        lockfiles.push(LockfileKind::PnpmLock);
    }
    lockfiles
}

/// Resolves workspace glob entries to the `package.json` paths of their members.
///
/// The `&Option<WorkspacesField>` signature is kept (rather than the more
/// idiomatic `Option<&WorkspacesField>`) for consistency with the module's
/// pinned contract.
#[must_use]
#[allow(clippy::ref_option)]
fn resolve_workspaces(field: &Option<WorkspacesField>, dir: &Path) -> Vec<PathBuf> {
    let Some(workspaces) = field else {
        return Vec::new();
    };
    let patterns = match workspaces {
        WorkspacesField::List(patterns) => patterns,
        WorkspacesField::Detailed { packages } => packages,
    };
    let mut resolved = Vec::new();
    for raw_pattern in patterns {
        let pattern = raw_pattern.trim_end_matches('/');
        if let Some(prefix) = pattern.strip_suffix("/*") {
            let Some(entries) = fs::read_dir(dir.join(prefix)).ok() else {
                continue;
            };
            for entry in entries.flatten() {
                let member_manifest = entry.path().join("package.json");
                if member_manifest.is_file() {
                    resolved.push(member_manifest);
                }
            }
        } else {
            let member_manifest = dir.join(pattern).join("package.json");
            if member_manifest.is_file() {
                resolved.push(member_manifest);
            }
        }
    }
    resolved.sort();
    resolved.dedup();
    resolved
}

/// Converts a dependency map into `Dependency` entries with the given scope.
///
/// Requirements prefixed with `workspace:` are given the
/// [`DependencyScope::Workspace`] scope.
#[must_use]
#[allow(clippy::needless_pass_by_value)]
fn to_dependencies(map: BTreeMap<String, String>, scope: DependencyScope) -> Vec<Dependency> {
    let mut dependencies = Vec::with_capacity(map.len());
    for (name, requirement) in map {
        let effective_scope = if requirement.starts_with("workspace:") {
            DependencyScope::Workspace
        } else {
            scope.clone()
        };
        dependencies.push(Dependency {
            kind: DependencyKind::Npm,
            name,
            requirement,
            scope: effective_scope,
        });
    }
    dependencies
}

/// Extracts the package name from a `package-lock.json` `node_modules/...` key.
///
/// Scoped packages (`@scope/name`) consume the first two path segments;
/// unscoped packages the first one. Returns `None` when the key does not
/// start with `node_modules/`.
#[must_use]
fn package_name_from_lock_key(key: &str) -> Option<String> {
    let name = key.strip_prefix("node_modules/")?;
    let mut segments = name.split('/');
    let first = segments.next()?;
    if first.starts_with('@') {
        let scope_name = segments.next()?;
        Some(format!("{first}/{scope_name}"))
    } else {
        Some(first.to_string())
    }
}

/// Splits a lockfile key of the form `name@version` (or `@scope/name@version`)
/// into its name and version components.
///
/// Returns `None` when the key carries no `@version` suffix.
#[must_use]
pub(crate) fn split_name_version(key: &str) -> Option<(String, String)> {
    let idx = key.rfind('@')?;
    if idx == 0 {
        return None;
    }
    let name = &key[..idx];
    let version = &key[idx + 1..];
    if name.is_empty() || version.is_empty() {
        return None;
    }
    Some((name.to_string(), version.to_string()))
}

/// Parses a `package-lock.json` into the packages it locks.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] if the file cannot be read or is invalid JSON.
pub fn parse_package_lock(path: &Path) -> Result<Vec<LockedPackage>> {
    let text = fs::read_to_string(path)?;
    let lock: PackageLock = serde_json::from_str(&text)?;
    let mut packages = Vec::new();
    for (key, entry) in lock.packages {
        let Some(name) = package_name_from_lock_key(&key) else {
            continue;
        };
        packages.push(LockedPackage {
            name,
            version: entry.version.unwrap_or_default(),
        });
    }
    Ok(packages)
}

/// Raw `package-lock.json` structure (relevant subset).
#[derive(Debug, Deserialize)]
struct PackageLock {
    #[serde(default)]
    packages: BTreeMap<String, PackageLockEntry>,
}

/// A single entry in a `package-lock.json` `packages` map.
#[derive(Debug, Deserialize)]
struct PackageLockEntry {
    version: Option<String>,
}

/// Parses a `pnpm-lock.yaml` into the packages it locks.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] if the file cannot be read or is invalid YAML.
pub fn parse_pnpm_lock(path: &Path) -> Result<Vec<LockedPackage>> {
    let text = fs::read_to_string(path)?;
    let lock: PnpmLock = serde_yaml::from_str(&text)?;
    let mut packages = Vec::new();
    for (key, entry) in lock.packages {
        let Some((name, key_version)) = split_name_version(&key) else {
            continue;
        };
        let version = entry.version.unwrap_or(key_version);
        packages.push(LockedPackage { name, version });
    }
    Ok(packages)
}

/// Raw `pnpm-lock.yaml` structure (relevant subset).
#[derive(Debug, Deserialize)]
struct PnpmLock {
    #[serde(default)]
    packages: BTreeMap<String, PnpmPackage>,
}

/// A single entry in a `pnpm-lock.yaml` `packages` map.
#[derive(Debug, Deserialize)]
struct PnpmPackage {
    version: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, path::PathBuf};

    use tempfile::tempdir;

    use super::{detect_manager, load, parse_package_lock, parse_pnpm_lock, resolve_workspaces};
    use crate::types::{DependencyScope, LockedPackage, LockfileKind, PackageManager};

    /// Writes `contents` to `dir.join(relative)`, creating parent directories
    /// as needed, and returns the resulting path.
    fn write(dir: &Path, relative: &str, contents: &str) -> PathBuf {
        let path = dir.join(relative);
        let parent = path.parent().unwrap_or(&path);
        if !parent.as_os_str().is_empty() {
            #[allow(clippy::unwrap_used)]
            fs::create_dir_all(parent).unwrap();
        }
        #[allow(clippy::unwrap_used)]
        fs::write(&path, contents).unwrap();
        path
    }

    /// Writes `contents` to `path`.
    fn write_file(path: &Path, contents: &str) {
        #[allow(clippy::unwrap_used)]
        fs::write(path, contents).unwrap();
    }

    /// Writes a full-featured `package.json` to `dir` and loads it.
    fn load_full_package(dir: &Path) -> crate::types::NodePackage {
        let manifest = write(
            dir,
            "package.json",
            r#"{
                "name": "test-pkg",
                "version": "1.0.0",
                "description": "A test package",
                "homepage": "https://example.com",
                "repository": "https://github.com/example/test-pkg",
                "license": "MIT",
                "packageManager": "pnpm@9.0.0",
                "engines": { "node": ">=18" },
                "scripts": {
                    "build": "tsc",
                    "test": "vitest",
                    "lint": "eslint",
                    "dev": "vite",
                    "start": "node server.js"
                },
                "dependencies": {
                    "react": "^18.0.0",
                    "@my/lib": "workspace:*"
                },
                "devDependencies": { "typescript": "^5.0.0" },
                "peerDependencies": { "react-dom": "^18.0.0" },
                "optionalDependencies": { "fsevents": "^2.3.0" }
            }"#,
        );
        #[allow(clippy::unwrap_used)]
        load(&manifest).unwrap()
    }

    /// Flattens a dependency list into `(name, requirement, scope)` triples.
    fn scope_tuples(
        dependencies: &[crate::types::Dependency],
    ) -> Vec<(String, String, DependencyScope)> {
        dependencies
            .iter()
            .map(|d| (d.name.clone(), d.requirement.clone(), d.scope.clone()))
            .collect()
    }

    #[test]
    fn load_populates_metadata_and_scripts() {
        #[allow(clippy::unwrap_used)]
        let dir = tempdir().unwrap();
        let package = load_full_package(dir.path());

        assert_eq!(package.name, "test-pkg");
        assert_eq!(package.version.as_deref(), Some("1.0.0"));
        assert_eq!(package.description.as_deref(), Some("A test package"));
        assert_eq!(package.homepage.as_deref(), Some("https://example.com"));
        assert_eq!(
            package.repository.as_deref(),
            Some("https://github.com/example/test-pkg")
        );
        assert_eq!(package.license.as_deref(), Some("MIT"));
        assert_eq!(package.package_manager, Some(PackageManager::Pnpm));
        assert_eq!(package.lockfiles, Vec::<LockfileKind>::new());
        assert_eq!(package.scripts.build.as_deref(), Some("tsc"));
        assert_eq!(package.scripts.test.as_deref(), Some("vitest"));
        assert_eq!(package.scripts.lint.as_deref(), Some("eslint"));
        assert_eq!(package.scripts.dev.as_deref(), Some("vite"));
        assert_eq!(package.scripts.start.as_deref(), Some("node server.js"));
        assert_eq!(
            package.engines.get("node").map(String::as_str),
            Some(">=18")
        );
    }

    #[test]
    fn load_maps_dependency_groups_and_scopes() {
        #[allow(clippy::unwrap_used)]
        let dir = tempdir().unwrap();
        let package = load_full_package(dir.path());

        assert_eq!(
            scope_tuples(&package.dependencies),
            vec![
                (
                    "@my/lib".to_string(),
                    "workspace:*".to_string(),
                    DependencyScope::Workspace,
                ),
                (
                    "react".to_string(),
                    "^18.0.0".to_string(),
                    DependencyScope::Normal
                ),
            ]
        );
        assert_eq!(
            scope_tuples(&package.dev_dependencies),
            vec![(
                "typescript".to_string(),
                "^5.0.0".to_string(),
                DependencyScope::Development,
            )]
        );
        assert_eq!(
            scope_tuples(&package.peer_dependencies),
            vec![(
                "react-dom".to_string(),
                "^18.0.0".to_string(),
                DependencyScope::Peer,
            )]
        );
        assert_eq!(
            scope_tuples(&package.optional_dependencies),
            vec![(
                "fsevents".to_string(),
                "^2.3.0".to_string(),
                DependencyScope::Optional,
            )]
        );
    }

    #[test]
    fn load_reads_detailed_repository_object() {
        #[allow(clippy::unwrap_used)]
        let dir = tempdir().unwrap();
        let manifest = write(
            dir.path(),
            "package.json",
            r#"{
                "name": "obj-repo",
                "repository": { "type": "git", "url": "git+https://github.com/a/b.git" }
            }"#,
        );
        #[allow(clippy::unwrap_used)]
        let package = load(&manifest).unwrap();
        assert_eq!(
            package.repository.as_deref(),
            Some("git+https://github.com/a/b.git")
        );
    }

    #[test]
    fn resolves_workspace_globs() {
        #[allow(clippy::unwrap_used)]
        let dir = tempdir().unwrap();
        let root = write(
            dir.path(),
            "package.json",
            r#"{ "workspaces": ["packages/*"] }"#,
        );
        write(dir.path(), "packages/a/package.json", r#"{ "name": "a" }"#);
        write(dir.path(), "packages/b/package.json", r#"{ "name": "b" }"#);
        write(dir.path(), "packages/c/not-a-manifest.txt", "x");
        #[allow(clippy::unwrap_used)]
        let package = load(&root).unwrap();
        assert_eq!(
            package.workspaces,
            vec![
                dir.path().join("packages/a/package.json"),
                dir.path().join("packages/b/package.json"),
            ]
        );
    }

    #[test]
    fn resolves_detailed_workspaces_with_trailing_slash() {
        #[allow(clippy::unwrap_used)]
        let dir = tempdir().unwrap();
        write(dir.path(), "packages/a/package.json", r#"{ "name": "a" }"#);
        let field = Some(super::WorkspacesField::Detailed {
            packages: vec!["packages/a/".to_string()],
        });
        assert_eq!(
            resolve_workspaces(&field, dir.path()),
            vec![dir.path().join("packages/a/package.json")]
        );
    }

    #[test]
    fn detect_manager_prefers_field_then_lockfiles() {
        assert_eq!(
            detect_manager(Some("pnpm@9.0.0"), Path::new("/nonexistent")),
            Some(PackageManager::Pnpm)
        );
        assert_eq!(
            detect_manager(Some("npm@10.0.0"), Path::new("/nonexistent")),
            Some(PackageManager::Npm)
        );
        assert_eq!(
            detect_manager(Some("bun@1.1.0"), Path::new("/nonexistent")),
            Some(PackageManager::Bun)
        );
        assert_eq!(
            detect_manager(Some("yarn@1.0.0"), Path::new("/nonexistent")),
            None
        );

        #[allow(clippy::unwrap_used)]
        let dir = tempdir().unwrap();
        assert_eq!(detect_manager(None, dir.path()), None);

        write_file(&dir.path().join("package-lock.json"), "{}");
        assert_eq!(detect_manager(None, dir.path()), Some(PackageManager::Npm));

        write_file(&dir.path().join("bun.lock"), "{}");
        assert_eq!(detect_manager(None, dir.path()), Some(PackageManager::Bun));

        write_file(&dir.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'");
        assert_eq!(detect_manager(None, dir.path()), Some(PackageManager::Bun));

        #[allow(clippy::unwrap_used)]
        let dir2 = tempdir().unwrap();
        write_file(
            &dir2.path().join("pnpm-workspace.yaml"),
            "packages:\n  - packages/*",
        );
        assert_eq!(
            detect_manager(None, dir2.path()),
            Some(PackageManager::Pnpm)
        );
    }

    #[test]
    fn parse_package_lock_extracts_locked_packages() {
        #[allow(clippy::unwrap_used)]
        let dir = tempdir().unwrap();
        let lock = write(
            dir.path(),
            "package-lock.json",
            r#"{
                "name": "test",
                "version": "1.0.0",
                "lockfileVersion": 3,
                "packages": {
                    "": { "name": "test", "version": "1.0.0" },
                    "node_modules/react": { "version": "18.2.0" },
                    "node_modules/@scope/foo": { "version": "1.0.0" },
                    "node_modules/no-version": {},
                    "packages/member": { "version": "2.0.0" }
                }
            }"#,
        );
        #[allow(clippy::unwrap_used)]
        let packages = parse_package_lock(&lock).unwrap();
        assert_eq!(
            packages,
            vec![
                LockedPackage {
                    name: "@scope/foo".to_string(),
                    version: "1.0.0".to_string()
                },
                LockedPackage {
                    name: "no-version".to_string(),
                    version: String::new()
                },
                LockedPackage {
                    name: "react".to_string(),
                    version: "18.2.0".to_string()
                },
            ]
        );
    }

    #[test]
    fn parse_pnpm_lock_extracts_locked_packages() {
        #[allow(clippy::unwrap_used)]
        let dir = tempdir().unwrap();
        let lock = write(
            dir.path(),
            "pnpm-lock.yaml",
            "lockfileVersion: '9.0'\n\npackages:\n  react@18.2.0:\n    resolution: \
             {integrity: sha512-abc}\n  '@scope/foo@1.0.0':\n    resolution: \
             {integrity: sha512-def}\n  override@1.0.0:\n    version: 2.0.0\n",
        );
        #[allow(clippy::unwrap_used)]
        let packages = parse_pnpm_lock(&lock).unwrap();
        assert_eq!(
            packages,
            vec![
                LockedPackage {
                    name: "@scope/foo".to_string(),
                    version: "1.0.0".to_string()
                },
                LockedPackage {
                    name: "override".to_string(),
                    version: "2.0.0".to_string()
                },
                LockedPackage {
                    name: "react".to_string(),
                    version: "18.2.0".to_string()
                },
            ]
        );
    }
}
