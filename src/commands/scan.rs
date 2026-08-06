use std::{collections::BTreeSet, path::Path, time::Duration};

use owo_colors::OwoColorize;

use crate::{
    config,
    error::Result,
    project::detect,
    types::{Framework, Language, PackageManager},
};

/// Scans a project directory and prints its detected languages, frameworks,
/// and package manifests.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] if the given path cannot be canonicalized
/// or the configuration cannot be loaded.
#[allow(clippy::too_many_lines)]
pub fn run(path: &Path) -> Result<()> {
    let config = config::load_or_default()?;
    let project = detect::detect(path, &config.exclusions)?;

    println!("{}", "Project".green().bold());
    println!("{}", "─".repeat(7));
    println!("Root: {}", project.root.display());

    let workspace = project
        .cargo_workspace
        .as_ref()
        .map(|ws| ws.root.display().to_string())
        .or_else(|| {
            project
                .node_workspace
                .as_ref()
                .map(|ws| ws.root.display().to_string())
        })
        .unwrap_or_else(|| "(none)".to_string());
    println!("Workspace: {workspace}");

    let managers: BTreeSet<PackageManager> = project
        .node_packages
        .iter()
        .filter_map(|pkg| pkg.package_manager)
        .collect();
    let managers = if managers.is_empty() {
        "(none)".to_string()
    } else {
        managers
            .iter()
            .map(|pm| package_manager_name(*pm))
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!("Package Manager(s): {managers}");

    println!();

    println!("{}", "Languages".green().bold());
    println!("{}", "─".repeat(8));
    if project.languages.is_empty() {
        println!("(none)");
    } else {
        for lang in &project.languages {
            println!("{}", language_name(lang.clone()));
        }
    }

    println!();

    println!("{}", "Frameworks".green().bold());
    println!("{}", "─".repeat(10));
    if project.frameworks.is_empty() {
        println!("(none)");
    } else {
        for fw in &project.frameworks {
            println!("{}", framework_name(fw.clone()));
        }
    }

    println!();

    println!("{}", "Cargo Packages".green().bold());
    println!("{}", "─".repeat(14));
    if project.cargo_packages.is_empty() {
        println!("(none)");
    } else {
        for (i, pkg) in project.cargo_packages.iter().enumerate() {
            if i > 0 {
                println!();
            }
            let features = if pkg.features.is_empty() {
                "(none)".to_string()
            } else {
                pkg.features.join(", ")
            };
            let workspace = pkg.workspace_root.as_ref().map_or_else(
                || "(standalone)".to_string(),
                |root| root.display().to_string(),
            );
            println!("crate: {}", pkg.name);
            println!("version: {}", pkg.version.as_deref().unwrap_or("(unknown)"));
            println!("edition: {}", pkg.edition.as_deref().unwrap_or("(unknown)"));
            println!("features: {features}");
            println!("workspace: {workspace}");
        }
    }

    println!();

    println!("{}", "Node Packages".green().bold());
    println!("{}", "─".repeat(13));
    if project.node_packages.is_empty() {
        println!("(none)");
    } else {
        for (i, pkg) in project.node_packages.iter().enumerate() {
            if i > 0 {
                println!();
            }
            let manager = pkg
                .package_manager
                .map_or("(unknown)", package_manager_name);
            let scripts = [
                pkg.scripts.build.is_some().then_some("build"),
                pkg.scripts.test.is_some().then_some("test"),
                pkg.scripts.lint.is_some().then_some("lint"),
                pkg.scripts.dev.is_some().then_some("dev"),
                pkg.scripts.start.is_some().then_some("start"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
            let scripts = if scripts.is_empty() {
                "(none)".to_string()
            } else {
                scripts.join(", ")
            };
            let dependencies: Vec<&str> = pkg
                .dependencies
                .iter()
                .chain(&pkg.dev_dependencies)
                .chain(&pkg.peer_dependencies)
                .chain(&pkg.optional_dependencies)
                .map(|dep| dep.name.as_str())
                .collect();
            let dependencies = if dependencies.is_empty() {
                "(none)".to_string()
            } else {
                dependencies.join(", ")
            };
            println!("package: {}", pkg.name);
            println!("version: {}", pkg.version.as_deref().unwrap_or("(unknown)"));
            println!("manager: {manager}");
            println!("scripts: {scripts}");
            println!("dependencies: {dependencies}");
        }
    }

    println!();

    println!("{}", "Dependency Summary".green().bold());
    println!("{}", "─".repeat(18));
    let workspace_members = project
        .cargo_workspace
        .as_ref()
        .map_or(0, |ws| ws.members.len())
        + project
            .node_workspace
            .as_ref()
            .map_or(0, |ws| ws.members.len());
    println!("Rust crates: {}", project.cargo_packages.len());
    println!("Node packages: {}", project.node_packages.len());
    println!("Workspace members: {workspace_members}");

    println!();

    println!("{}", "Statistics".green().bold());
    println!("{}", "─".repeat(11));
    println!(
        "Directories scanned: {}",
        project.statistics.directories_scanned
    );
    println!("Files scanned: {}", project.statistics.files_scanned);
    println!(
        "Ignored directories: {}",
        project.statistics.ignored_directories
    );
    println!(
        "Elapsed time: {}",
        format_duration(project.statistics.elapsed)
    );

    Ok(())
}

/// Returns the display name for a [`Language`].
#[allow(clippy::needless_pass_by_value)]
const fn language_name(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "Rust",
        Language::TypeScript => "TypeScript",
        Language::JavaScript => "JavaScript",
        Language::Python => "Python",
        Language::Go => "Go",
        Language::C => "C",
        Language::Cpp => "C++",
    }
}

/// Returns the display name for a [`Framework`].
#[allow(clippy::needless_pass_by_value)]
const fn framework_name(fw: Framework) -> &'static str {
    match fw {
        Framework::Tauri => "Tauri",
        Framework::Svelte => "Svelte",
        Framework::SvelteKit => "SvelteKit",
        Framework::React => "React",
        Framework::Vue => "Vue",
        Framework::Solid => "Solid",
        Framework::Angular => "Angular",
        Framework::Axum => "Axum",
        Framework::Actix => "Actix",
        Framework::Next => "Next.js",
        Framework::Nuxt => "Nuxt",
        Framework::Remix => "Remix",
        Framework::Astro => "Astro",
        Framework::Express => "Express",
        Framework::Fastify => "Fastify",
        Framework::NestJs => "NestJS",
        Framework::Hono => "Hono",
        Framework::Rocket => "Rocket",
        Framework::Leptos => "Leptos",
        Framework::Yew => "Yew",
        Framework::Dioxus => "Dioxus",
        Framework::Bevy => "Bevy",
    }
}

/// Returns the display name for a [`PackageManager`].
const fn package_manager_name(pm: PackageManager) -> &'static str {
    match pm {
        PackageManager::Npm => "npm",
        PackageManager::Pnpm => "pnpm",
        PackageManager::Bun => "bun",
    }
}

/// Formats a [`Duration`] as a millisecond or second string.
fn format_duration(d: Duration) -> String {
    if d.as_millis() < 1000 {
        format!("{}ms", d.as_millis())
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}
