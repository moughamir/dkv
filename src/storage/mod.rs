//! Storage: normalization, deduplication, and persistence.
//!
//! Providers produce
//! [`CollectedDocs`](crate::providers::types::CollectedDocs); storage
//! normalizes, deduplicates, and persists them into a vault.
//! [`StorageBackend`] keeps storage independent of the backend: `SQLite` is the
//! first backend; the trait exists so tests and future backends (or
//! export/import) do not change the ingest pipeline.

mod checksum;
mod errors;
mod layout;
pub mod metadata;

pub mod ingest;
pub mod sqlite;
pub mod transaction;
pub mod vault;

pub use checksum::{ChecksumKind, content_checksum, content_checksum_file};
pub use errors::{Result, StorageError};
pub use layout::VaultLayout;
pub use metadata::{
    ArtifactRecord, ChecksumRecord, Counts, DocumentRecord, IngestReport, IngestRunRecord,
    PackageKey, PackageRecord, PendingArtifact, PendingBatch, PendingDocument, SourceRecord,
    StatusReport, TagRecord, VerifyReport,
};
pub use vault::Vault;

use std::path::Path;

/// Storage backend abstraction; `SQLite` is the first implementation.
pub trait StorageBackend: Send {
    /// Opens (or creates) the database at `path`. Callers must call
    /// [`StorageBackend::migrate`] after opening before any other method.
    ///
    /// `Self: Sized` — `Vault::open` boxes a concrete backend.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Sqlite`] if the database cannot be opened or
    /// created at `path`.
    fn open(path: &Path) -> Result<Self>
    where
        Self: Sized;

    /// Applies pending schema migrations (idempotent, guarded by
    /// `PRAGMA user_version`).
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Migration`] if a migration fails to apply.
    fn migrate(&mut self) -> Result<()>;

    /// Persists one normalized batch atomically: one transaction; every row
    /// and staged content file is rolled back on any failure. Content files
    /// are staged under `layout.tmp_dir()`, renamed to
    /// `layout.documents_dir()/<sha256>` before commit (skip if already
    /// present). Inserts one `ingest_runs` row.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Sqlite`] if any row insertion fails, or
    /// [`StorageError::Io`] if a content file cannot be staged; the whole
    /// batch is rolled back on failure.
    fn ingest(&mut self, layout: &VaultLayout, batch: &PendingBatch) -> Result<IngestReport>;

    /// Row counts.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Sqlite`] if the count query fails.
    fn counts(&self) -> Result<Counts>;

    /// Total bytes of stored document content (sum over `checksums.size` for
    /// content stored in the vault; never raw source sizes).
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Sqlite`] if the aggregate query fails.
    fn storage_size(&self) -> Result<u64>;

    /// Verifies stored content against the on-disk vault (missing files,
    /// checksum mismatches, orphaned rows, reverse orphans). Read-only.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Sqlite`] if a verification query fails.
    fn verify(&self, layout: &VaultLayout) -> Result<VerifyReport>;
}
