//! The vault: layout plus backend behind one handle.
//!
//! [`VaultLayout`] describes the directory layout;
//! `vault` binds a layout to a concrete
//! [`crate::storage::StorageBackend`] (`SQLite`) behind a single handle.

use std::path::Path;

use crate::storage::StorageBackend;
use crate::storage::errors::Result;
use crate::storage::layout::VaultLayout;
use crate::storage::metadata::{Counts, IngestReport, PendingBatch, StatusReport, VerifyReport};
use crate::storage::sqlite::SqliteBackend;

/// The Dev Knowledge Vault handle.
pub struct Vault {
    layout: VaultLayout,
    backend: SqliteBackend,
}

impl Vault {
    /// Opens or creates a vault at `root`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::storage::StorageError`] if layout creation, database opening, or migration fails.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let layout = VaultLayout::new(crate::util::path::expand_home(root.as_ref()));
        layout.ensure()?;
        let mut backend = SqliteBackend::open(&layout.db_path())?;
        backend.migrate()?;
        Ok(Self { layout, backend })
    }

    /// Returns the vault layout.
    #[must_use]
    pub const fn layout(&self) -> &VaultLayout {
        &self.layout
    }

    /// Persists a normalized batch transactionally.
    ///
    /// # Errors
    ///
    /// Returns [`crate::storage::StorageError`] on database or I/O failure.
    pub fn ingest(&mut self, batch: &PendingBatch) -> Result<IngestReport> {
        self.backend.ingest(&self.layout, batch)
    }

    /// Returns aggregate counts.
    ///
    /// # Errors
    ///
    /// Returns [`crate::storage::StorageError`] on database failure.
    pub fn counts(&self) -> Result<Counts> {
        self.backend.counts()
    }

    /// Returns total storage size in bytes.
    ///
    /// # Errors
    ///
    /// Returns [`crate::storage::StorageError`] on database failure.
    pub fn storage_size(&self) -> Result<u64> {
        self.backend.storage_size()
    }

    /// Verifies vault integrity.
    ///
    /// # Errors
    ///
    /// Returns [`crate::storage::StorageError`] on database failure.
    pub fn verify(&self) -> Result<VerifyReport> {
        self.backend.verify(&self.layout)
    }

    /// Returns status report.
    ///
    /// # Errors
    ///
    /// Returns [`crate::storage::StorageError`] on database failure.
    pub fn status(&self) -> Result<StatusReport> {
        let counts = self.counts()?;
        let storage_size = self.storage_size()?;
        Ok(StatusReport {
            counts,
            storage_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::types::KnowledgeSource;
    use crate::storage::metadata::{PackageKey, PendingDocument};
    use std::path::PathBuf;
    use std::time::SystemTime;
    use tempfile::tempdir;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn vault_open_ingest_and_status() {
        let dir = tempdir().unwrap();
        let mut vault = Vault::open(dir.path().join("vault")).unwrap();

        let counts = vault.counts().unwrap();
        assert_eq!(counts.packages, 0);

        let batch = PendingBatch {
            documents: vec![PendingDocument {
                package: PackageKey {
                    kind: "cargo".to_string(),
                    name: "test-pkg".to_string(),
                    version: Some("0.1.0".to_string()),
                    root: dir.path().to_path_buf(),
                },
                provider: "cargo".to_string(),
                source: KnowledgeSource::Readme,
                title: "README".to_string(),
                relative_path: PathBuf::from("README.md"),
                fs_path: None,
                mime_type: "text/markdown".to_string(),
                language: Some("markdown".to_string()),
                size: 11,
                metadata_checksum: "1234567890abcdef".to_string(),
                modified: SystemTime::now(),
                tags: vec!["readme".to_string()],
                content: Some("# Test README".to_string()),
                content_checksum: None,
                vault_path: None,
                origin_url: None,
            }],
            artifacts: vec![],
        };

        let report = vault.ingest(&batch).unwrap();
        assert_eq!(report.documents, 1);
        assert_eq!(report.packages, 1);

        let status = vault.status().unwrap();
        assert_eq!(status.counts.documents, 1);
        assert_eq!(status.counts.packages, 1);
        assert!(status.storage_size > 0);

        // Idempotency: ingest again
        let report2 = vault.ingest(&batch).unwrap();
        assert_eq!(report2.documents, 1);
    }
}
