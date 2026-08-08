//! Vault commands (status and verify) implementation.

use std::path::Path;

use owo_colors::OwoColorize;

use crate::{config, error::Result, storage::Vault};

/// Shows vault status (counts and storage size).
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] if config loading or vault opening/querying fails.
pub fn status(vault_override: Option<&Path>) -> Result<()> {
    let config = config::load_or_default()?;
    let vault_path = vault_override.unwrap_or(&config.vault);
    let vault = Vault::open(vault_path)?;
    let status = vault.status()?;

    println!("{}", "Vault Status".green().bold());
    println!("{}", "─".repeat(12));
    println!("Path: {}", vault_path.display());
    println!("Packages: {}", status.counts.packages);
    println!("Documents: {}", status.counts.documents);
    println!("Artifacts: {}", status.counts.artifacts);
    println!("Storage Size: {} bytes", status.storage_size);

    Ok(())
}

/// Verifies vault integrity (missing files, checksum mismatches, orphaned rows, reverse orphans).
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] if config loading or vault opening/verification fails.
pub fn verify(vault_override: Option<&Path>) -> Result<()> {
    let config = config::load_or_default()?;
    let vault_path = vault_override.unwrap_or(&config.vault);
    let vault = Vault::open(vault_path)?;
    let report = vault.verify()?;

    println!("{}", "Vault Verification".green().bold());
    println!("{}", "─".repeat(18));
    println!("Path: {}", vault_path.display());
    println!("Missing files (source): {}", report.missing_files.len());
    println!("Checksum mismatches: {}", report.checksum_mismatches.len());
    println!("Orphaned rows: {}", report.orphaned_rows.len());
    println!(
        "Reverse orphans (vault files without rows): {}",
        report.reverse_orphans.len()
    );

    let issues = report.issue_count();
    if issues == 0 {
        println!("\n{}", "Vault is valid. No issues found.".green());
    } else {
        println!("\n{} found total issues.", issues.to_string().red().bold());
    }

    Ok(())
}
