//! Record, batch, and report models for the storage layer.
//!
//! [`PendingBatch`] is the normalized, deduplicated input to
//! [`crate::storage::StorageBackend::ingest`]; the `*Record` types mirror the
//! normalized `SQLite` schema rows; the report types summarize ingest, status,
//! and verification.

use std::path::PathBuf;
use std::time::SystemTime;

use crate::providers::types::{KnowledgeSource, PackageId, PackageKind};

/// Identity of a package as stored in `packages`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageKey {
    /// Stable kind tag: `"cargo"` | `"node"` | `"local"` | `"other"`.
    pub kind: String,
    /// Package name.
    pub name: String,
    /// Package version when known.
    pub version: Option<String>,
    /// Filesystem root of the package.
    pub root: PathBuf,
}

impl PackageKey {
    /// Builds a key from a provider package id.
    #[must_use]
    pub fn new(package: &PackageId) -> Self {
        Self {
            kind: kind_tag(package.kind).to_string(),
            name: package.name.clone(),
            version: package.version.clone(),
            root: package.root.clone(),
        }
    }
}

/// Maps a [`PackageKind`] to its stable string tag.
#[must_use]
const fn kind_tag(kind: PackageKind) -> &'static str {
    match kind {
        PackageKind::Cargo => "cargo",
        PackageKind::Node => "node",
        PackageKind::Local => "local",
        PackageKind::Other => "other",
    }
}

/// A normalized, deduplicated document ready to be persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDocument {
    /// Owning package identity.
    pub package: PackageKey,
    /// Provider id that produced this document.
    pub provider: String,
    /// Origin classification (`readme`, `docs`, ...).
    pub source: KnowledgeSource,
    /// Human-readable title.
    pub title: String,
    /// Path relative to the package root.
    pub relative_path: PathBuf,
    /// Original source file; `None` for future network docs.
    pub fs_path: Option<PathBuf>,
    /// MIME type of the document.
    pub mime_type: String,
    /// Content language when meaningful (e.g. `"rust"`), else `None`.
    pub language: Option<String>,
    /// Source-file size as reported by the provider.
    pub size: u64,
    /// Provider-supplied FNV-1a fingerprint, reused.
    pub metadata_checksum: String,
    /// Last-modified time of the source file.
    pub modified: SystemTime,
    /// Free-form tags.
    pub tags: Vec<String>,
    /// `None` = metadata-only document.
    pub content: Option<String>,
    /// SHA-256 hex; `Some` iff `content.is_some()`.
    pub content_checksum: Option<String>,
    /// `"documents/<sha256>"`; `Some` iff `content.is_some()`.
    pub vault_path: Option<String>,
    /// Origin URL; `None` for now.
    pub origin_url: Option<String>,
}

/// A normalized artifact (metadata-only, never copied into the vault).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingArtifact {
    /// Owning package identity.
    pub package: PackageKey,
    /// Provider id that produced this artifact.
    pub provider: String,
    /// Artifact name.
    pub name: String,
    /// Origin classification.
    pub source: KnowledgeSource,
    /// Path relative to the package root.
    pub path: PathBuf,
    /// Original source file, if any.
    pub fs_path: Option<PathBuf>,
    /// MIME type of the artifact.
    pub mime_type: String,
    /// Source-file size as reported by the provider.
    pub size: u64,
    /// Provider-supplied checksum.
    pub checksum: String,
    /// Last-modified time of the source file.
    pub modified: SystemTime,
    /// Free-form tags.
    pub tags: Vec<String>,
}

/// One normalized, deduplicated ingest batch (one batch = one transaction).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PendingBatch {
    /// Documents to persist.
    pub documents: Vec<PendingDocument>,
    /// Artifacts to persist (metadata only).
    pub artifacts: Vec<PendingArtifact>,
}

/// Row in `packages`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRecord {
    /// Row id.
    pub id: i64,
    /// Kind tag (`"cargo"` | `"node"` | `"local"` | `"other"`).
    pub kind: String,
    /// Package name.
    pub name: String,
    /// Package version when known.
    pub version: Option<String>,
    /// Filesystem root, when the package has one on disk.
    pub root: Option<String>,
    /// Unix timestamp of first observation.
    pub first_seen_at: i64,
    /// Unix timestamp of most recent observation.
    pub last_seen_at: i64,
}

/// Row in `sources`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRecord {
    /// Row id.
    pub id: i64,
    /// Provider id.
    pub provider: String,
    /// Source tag (`"readme"`, `"docs"`, ...).
    pub source: String,
}

/// Row in `documents`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentRecord {
    /// Row id.
    pub id: i64,
    /// `packages.id`.
    pub package_id: i64,
    /// `sources.id`.
    pub source_id: i64,
    /// Human-readable title.
    pub title: String,
    /// Path relative to the package root.
    pub relative_path: String,
    /// Original source file, if any.
    pub fs_path: Option<String>,
    /// Logical vault path (`"documents/<sha256>"`) when content is stored.
    pub vault_path: Option<String>,
    /// Origin URL, if any.
    pub origin_url: Option<String>,
    /// MIME type.
    pub mime_type: String,
    /// Content language when meaningful, else `None`.
    pub language: Option<String>,
    /// Source-file size in bytes.
    pub size: i64,
    /// Provider-supplied metadata checksum (reused).
    pub metadata_checksum: String,
    /// `checksums.id` for the stored content, if any.
    pub content_checksum_id: Option<i64>,
    /// Whether content bytes are stored in the vault.
    pub content_stored: bool,
    /// Unix timestamp of the source modification.
    pub modified_at: i64,
    /// Unix timestamp of ingestion.
    pub ingested_at: i64,
}

/// Row in `artifacts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRecord {
    /// Row id.
    pub id: i64,
    /// `packages.id`.
    pub package_id: i64,
    /// Artifact name.
    pub name: String,
    /// Source tag.
    pub source: String,
    /// Path relative to the package root.
    pub path: String,
    /// Original source file, if any.
    pub fs_path: Option<String>,
    /// MIME type.
    pub mime_type: String,
    /// Source-file size in bytes.
    pub size: i64,
    /// Provider-supplied checksum.
    pub checksum: String,
    /// Unix timestamp of the source modification.
    pub modified_at: i64,
    /// Unix timestamp of ingestion.
    pub ingested_at: i64,
}

/// Row in `checksums`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecksumRecord {
    /// Row id.
    pub id: i64,
    /// Algorithm name (e.g. `"sha256"`).
    pub algorithm: String,
    /// Checksum value (lowercase hex).
    pub value: String,
    /// Byte size of the stored content.
    pub size: i64,
    /// Number of documents referencing this checksum.
    pub ref_count: i64,
}

/// Row in `tags`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRecord {
    /// Row id.
    pub id: i64,
    /// Tag name.
    pub name: String,
}

/// Row in `ingest_runs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestRunRecord {
    /// Row id.
    pub id: i64,
    /// Unix timestamp when the run started.
    pub started_at: i64,
    /// Unix timestamp when the run finished.
    pub finished_at: i64,
    /// Run status (e.g. `"ok"` | `"failed"`).
    pub status: String,
    /// Number of packages in the run.
    pub packages: i64,
    /// Number of documents in the run.
    pub documents: i64,
    /// Number of artifacts in the run.
    pub artifacts: i64,
    /// Error message when the run failed.
    pub error: Option<String>,
}

/// Aggregate row counts across the vault.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Counts {
    /// Number of `packages` rows.
    pub packages: i64,
    /// Number of `documents` rows.
    pub documents: i64,
    /// Number of `artifacts` rows.
    pub artifacts: i64,
}

/// Full vault status snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusReport {
    /// Row counts.
    pub counts: Counts,
    /// Total bytes of stored document content in the vault.
    pub storage_size: u64,
}

/// Result of a single ingest run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestReport {
    /// `ingest_runs.id` of the run that produced this report.
    pub run_id: i64,
    /// Number of packages touched by the run.
    pub packages: usize,
    /// Number of documents persisted by the run.
    pub documents: usize,
    /// Number of artifacts persisted by the run.
    pub artifacts: usize,
}

/// Report of content-vs-filesystem consistency checks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VerifyReport {
    /// Rows whose original `fs_path` no longer exists (warning).
    pub missing_files: Vec<PathBuf>,
    /// Stored content files whose SHA-256 != filename (vault file paths).
    pub checksum_mismatches: Vec<PathBuf>,
    /// `content_stored` rows whose vault file is missing (`vault_path` values).
    pub orphaned_rows: Vec<String>,
    /// Files under `documents/` with no matching row.
    pub reverse_orphans: Vec<PathBuf>,
}

impl VerifyReport {
    /// Total number of issues found across all categories.
    #[must_use]
    pub const fn issue_count(&self) -> usize {
        self.missing_files.len()
            + self.checksum_mismatches.len()
            + self.orphaned_rows.len()
            + self.reverse_orphans.len()
    }
}
