use std::path::Path;

use owo_colors::OwoColorize;

use crate::{
    error::Result,
    project::detect,
    types::{Framework, Language},
};

/// Scans a project directory and prints its detected languages, frameworks,
/// and package manifests.
///
/// # Errors
///
/// Returns [`DkvError`] if the given path cannot be canonicalized.
pub fn run(path: &Path) -> Result<()> {
    let project = detect::detect(path)?;

    println!("{}", "Project Scan".bold());
    println!("Root: {}", project.root.display());

    println!();

    println!("{}", "Languages".green());

    for lang in &project.languages {
        match lang {
            Language::Rust => println!("  ✓ Rust"),
            Language::TypeScript => println!("  ✓ TypeScript"),
            Language::JavaScript => println!("  ✓ JavaScript"),
            Language::Python => println!("  ✓ Python"),
            Language::Go => println!("  ✓ Go"),
            Language::C => println!("  ✓ C"),
            Language::Cpp => println!("  ✓ C++"),
        }
    }

    println!();

    println!("{}", "Frameworks".green());

    for fw in &project.frameworks {
        match fw {
            Framework::Tauri => println!("  ✓ Tauri"),
            Framework::SvelteKit => println!("  ✓ SvelteKit"),
            Framework::Svelte => println!("  ✓ Svelte"),
            Framework::React => println!("  ✓ React"),
            Framework::Vue => println!("  ✓ Vue"),
            Framework::Solid => println!("  ✓ Solid"),
            Framework::Angular => println!("  ✓ Angular"),
            Framework::Axum => println!("  ✓ Axum"),
            Framework::Actix => println!("  ✓ Actix"),
        }
    }

    println!();

    println!("{}", "Cargo Packages".green());

    for pkg in &project.cargo_packages {
        println!("  {}", pkg.manifest_path.display());
    }

    println!();

    println!("{}", "Node Packages".green());

    for pkg in &project.node_packages {
        println!("  {}", pkg.manifest_path.display());
    }

    Ok(())
}
