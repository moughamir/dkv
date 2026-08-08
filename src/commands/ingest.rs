//! Ingest command implementation.

use std::path::Path;

use owo_colors::OwoColorize;

use crate::{
    config,
    error::Result,
    project::detect,
    providers::{
        registry::Registry,
        types::{PackageId, PackageKind},
    },
    storage::Vault,
};

/// Runs project scanning, provider collection, and vault ingestion.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] if project detection, provider collection,
/// or vault persistence fails.
pub fn run(path: &Path) -> Result<()> {
    let config = config::load_or_default()?;
    let project = detect::detect(path, &config.exclusions)?;

    let mut vault = Vault::open(&config.vault)?;
    let registry = Registry::builtin();

    let mut total_packages = 0;
    let mut total_documents = 0;
    let mut total_artifacts = 0;

    let mut all_collected = Vec::new();

    for pkg in &project.cargo_packages {
        let mut pkg_id = PackageId::new(
            PackageKind::Cargo,
            pkg.name.clone(),
            pkg.workspace_root.as_ref().unwrap_or(&project.root),
        );
        if let Some(ref ver) = pkg.version {
            pkg_id = pkg_id.with_version(ver.clone());
        }
        let collected = registry.collect_all(&pkg_id)?;
        all_collected.extend(collected);
    }

    for pkg in &project.node_packages {
        let mut pkg_id = PackageId::new(PackageKind::Node, pkg.name.clone(), &project.root);
        if let Some(ref ver) = pkg.version {
            pkg_id = pkg_id.with_version(ver.clone());
        }
        let collected = registry.collect_all(&pkg_id)?;
        all_collected.extend(collected);
    }

    if project.cargo_packages.is_empty() && project.node_packages.is_empty() {
        let name = project
            .root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project");
        let pkg_id = PackageId::new(PackageKind::Local, name, &project.root);
        let collected = registry.collect_all(&pkg_id)?;
        all_collected.extend(collected);
    }

    // Convert collected docs and adjust package keys with proper kind/root/version
    let batch = crate::storage::ingest::prepare_batch(&all_collected);

    // Refine package keys in batch based on detected project
    let mut refined_documents = batch.documents;
    for doc in &mut refined_documents {
        doc.package.root.clone_from(&project.root);
    }
    let mut refined_artifacts = batch.artifacts;
    for art in &mut refined_artifacts {
        art.package.root.clone_from(&project.root);
    }

    let refined_batch = crate::storage::metadata::PendingBatch {
        documents: refined_documents,
        artifacts: refined_artifacts,
    };

    let report = vault.ingest(&refined_batch)?;
    total_packages += report.packages;
    total_documents += report.documents;
    total_artifacts += report.artifacts;

    println!("{}", "Ingest Complete".green().bold());
    println!("{}", "─".repeat(15));
    println!("Vault: {}", config.vault.display());
    println!("Run ID: {}", report.run_id);
    println!("Packages touched: {total_packages}");
    println!("Documents persisted: {total_documents}");
    println!("Artifacts persisted: {total_artifacts}");

    Ok(())
}
