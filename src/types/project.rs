use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    time::Duration,
};

use crate::types::Dependency;

/// A detected project: its languages, frameworks, package manifests, and
/// workspace information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    /// Canonical root directory of the scanned project.
    pub root: PathBuf,
    /// Detected programming languages.
    pub languages: BTreeSet<Language>,
    /// Detected frameworks.
    pub frameworks: BTreeSet<Framework>,
    /// Every Cargo package manifest found under the root.
    pub cargo_packages: Vec<CargoPackage>,
    /// Every Node package manifest found under the root.
    pub node_packages: Vec<NodePackage>,
    /// The Cargo workspace (if any) declared by a workspace-root manifest.
    pub cargo_workspace: Option<CargoWorkspace>,
    /// The Node workspace (if any) declared by a workspace-root package.
    pub node_workspace: Option<NodeWorkspace>,
    /// Filesystem traversal statistics for the scan.
    pub statistics: ScanStatistics,
}

impl Default for ProjectManifest {
    fn default() -> Self {
        Self {
            root: PathBuf::new(),
            languages: BTreeSet::new(),
            frameworks: BTreeSet::new(),
            cargo_packages: Vec::new(),
            node_packages: Vec::new(),
            cargo_workspace: None,
            node_workspace: None,
            statistics: ScanStatistics::default(),
        }
    }
}

/// A programming language detected in a project.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Language {
    /// Rust.
    Rust,
    /// TypeScript.
    TypeScript,
    /// JavaScript.
    JavaScript,
    /// Python.
    Python,
    /// Go.
    Go,
    /// C.
    C,
    /// C++.
    Cpp,
}

/// A framework detected in a project.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Framework {
    /// Tauri (Rust + web frontend).
    Tauri,
    /// Svelte.
    Svelte,
    /// `SvelteKit`.
    SvelteKit,
    /// React.
    React,
    /// Vue.
    Vue,
    /// Solid.
    Solid,
    /// Angular.
    Angular,
    /// Axum (Rust).
    Axum,
    /// Actix-web (Rust).
    Actix,
    /// Next.js.
    Next,
    /// Nuxt.
    Nuxt,
    /// Remix.
    Remix,
    /// Astro.
    Astro,
    /// Express.
    Express,
    /// Fastify.
    Fastify,
    /// `NestJS`.
    NestJs,
    /// Hono.
    Hono,
    /// Rocket (Rust).
    Rocket,
    /// Leptos (Rust).
    Leptos,
    /// Yew (Rust).
    Yew,
    /// Dioxus (Rust).
    Dioxus,
    /// Bevy (Rust).
    Bevy,
}

/// A Cargo package parsed from a `Cargo.toml` manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoPackage {
    /// Crate name.
    pub name: String,
    /// Package version.
    pub version: Option<String>,
    /// Rust edition.
    pub edition: Option<String>,
    /// License identifier.
    pub license: Option<String>,
    /// Package description.
    pub description: Option<String>,
    /// Repository URL.
    pub repository: Option<String>,
    /// Homepage URL.
    pub homepage: Option<String>,
    /// Package authors.
    pub authors: Vec<String>,
    /// Package keywords.
    pub keywords: Vec<String>,
    /// Package categories.
    pub categories: Vec<String>,
    /// Path to this manifest file.
    pub manifest_path: PathBuf,
    /// Whether this package belongs to a Cargo workspace (as root or member).
    pub workspace: bool,
    /// Directory of the workspace root this package belongs to, if any.
    pub workspace_root: Option<PathBuf>,
    /// Workspace data, present only when this manifest declares `[workspace]`.
    pub workspace_info: Option<CargoWorkspace>,
    /// Normal (runtime) dependencies.
    pub dependencies: Vec<Dependency>,
    /// Dev-dependencies.
    pub dev_dependencies: Vec<Dependency>,
    /// Build-dependencies.
    pub build_dependencies: Vec<Dependency>,
    /// Target-specific dependencies (from `[target.*.dependencies]`).
    pub target_dependencies: Vec<Dependency>,
    /// Declared feature names.
    pub features: Vec<String>,
    /// Members of the `default` feature.
    pub default_features: Vec<String>,
    /// Declared and conventionally inferred targets.
    pub targets: CargoTargets,
}

/// A Cargo workspace as declared by a workspace-root `Cargo.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoWorkspace {
    /// Directory containing the workspace-root manifest.
    pub root: PathBuf,
    /// Resolved `Cargo.toml` paths of workspace members.
    pub members: Vec<PathBuf>,
    /// Resolved `Cargo.toml` paths of default members.
    pub default_members: Vec<PathBuf>,
    /// Dependencies shared via `workspace.dependencies`.
    pub dependencies: Vec<Dependency>,
}

/// Targets (lib, binaries, examples, benches, tests) of a Cargo package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoTargets {
    /// The library target, if any.
    pub lib: Option<CargoTarget>,
    /// Binary targets.
    pub bins: Vec<CargoTarget>,
    /// Example targets.
    pub examples: Vec<CargoTarget>,
    /// Benchmark targets.
    pub benches: Vec<CargoTarget>,
    /// Test targets.
    pub tests: Vec<CargoTarget>,
}

#[allow(clippy::derivable_impls)]
impl Default for CargoTargets {
    fn default() -> Self {
        Self {
            lib: None,
            bins: Vec::new(),
            examples: Vec::new(),
            benches: Vec::new(),
            tests: Vec::new(),
        }
    }
}

/// A single Cargo target (lib, bin, example, bench, or test).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoTarget {
    /// Target name.
    pub name: String,
    /// Path to the target's source file or directory.
    pub path: Option<PathBuf>,
}

/// A Node package parsed from a `package.json` manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePackage {
    /// Package name.
    pub name: String,
    /// Package version.
    pub version: Option<String>,
    /// Package description.
    pub description: Option<String>,
    /// Homepage URL.
    pub homepage: Option<String>,
    /// Repository URL.
    pub repository: Option<String>,
    /// License identifier.
    pub license: Option<String>,
    /// Path to this manifest file.
    pub manifest_path: PathBuf,
    /// Detected package manager (npm, pnpm, or Bun).
    pub package_manager: Option<PackageManager>,
    /// Lockfiles detected in the package directory.
    pub lockfiles: Vec<LockfileKind>,
    /// Runtime dependencies.
    pub dependencies: Vec<Dependency>,
    /// Dev-dependencies.
    pub dev_dependencies: Vec<Dependency>,
    /// Peer-dependencies.
    pub peer_dependencies: Vec<Dependency>,
    /// Optional-dependencies.
    pub optional_dependencies: Vec<Dependency>,
    /// Scripts of interest (build, test, lint, dev, start).
    pub scripts: NodeScripts,
    /// Engine constraints (e.g. `node`, `npm`).
    pub engines: BTreeMap<String, String>,
    /// Resolved `package.json` paths of workspace members, when this package
    /// is a workspace root; empty otherwise.
    pub workspaces: Vec<PathBuf>,
}

/// A detected Node package manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PackageManager {
    /// npm.
    Npm,
    /// pnpm.
    Pnpm,
    /// Bun.
    Bun,
}

/// A detected lockfile kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LockfileKind {
    /// `package-lock.json`.
    PackageLock,
    /// `bun.lock`.
    BunLock,
    /// `bun.lockb` (binary; detect only).
    BunLockb,
    /// `pnpm-lock.yaml`.
    PnpmLock,
}

/// Scripts of interest from a `package.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeScripts {
    /// The `build` script.
    pub build: Option<String>,
    /// The `test` script.
    pub test: Option<String>,
    /// The `lint` script.
    pub lint: Option<String>,
    /// The `dev` script.
    pub dev: Option<String>,
    /// The `start` script.
    pub start: Option<String>,
}

#[allow(clippy::derivable_impls)]
impl Default for NodeScripts {
    fn default() -> Self {
        Self {
            build: None,
            test: None,
            lint: None,
            dev: None,
            start: None,
        }
    }
}

/// A Node workspace as declared by a workspace-root `package.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeWorkspace {
    /// Directory containing the workspace-root package.json.
    pub root: PathBuf,
    /// Resolved `package.json` paths of workspace members.
    pub members: Vec<PathBuf>,
    /// The package manager in use, if detected.
    pub package_manager: Option<PackageManager>,
}

/// Filesystem traversal statistics collected during a scan.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScanStatistics {
    /// Number of directories visited.
    pub directories_scanned: usize,
    /// Number of files visited.
    pub files_scanned: usize,
    /// Number of directories pruned because they were ignored.
    pub ignored_directories: usize,
    /// Wall-clock time taken by the scan.
    pub elapsed: Duration,
}

impl Default for ScanStatistics {
    fn default() -> Self {
        Self {
            directories_scanned: 0,
            files_scanned: 0,
            ignored_directories: 0,
            elapsed: Duration::ZERO,
        }
    }
}
