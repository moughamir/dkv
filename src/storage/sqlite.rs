//! `SQLite` storage backend.
//!
//! Implements [`crate::storage::StorageBackend`] over `SQLite`
//! (`rusqlite`): schema migration via `PRAGMA user_version`, transactional
//! ingest, counts, storage size, and content verification.

#![allow(
    clippy::too_many_lines,
    clippy::needless_raw_string_hashes,
    clippy::uninlined_format_args,
    clippy::assigning_clones,
    clippy::needless_as_bytes,
    clippy::collapsible_if,
    clippy::manual_flatten
)]

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::storage::StorageBackend;
use crate::storage::checksum::{content_checksum, content_checksum_file};
use crate::storage::errors::Result;
use crate::storage::layout::VaultLayout;
use crate::storage::metadata::{Counts, IngestReport, VerifyReport};
use crate::storage::transaction::stage_content;

/// Concrete `SQLite` storage backend.
pub struct SqliteBackend {
    conn: Connection,
}

impl StorageBackend for SqliteBackend {
    fn open(path: &Path) -> Result<Self>
    where
        Self: Sized,
    {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute("PRAGMA foreign_keys = ON;", [])?;
        Ok(Self { conn })
    }

    fn migrate(&mut self) -> Result<()> {
        let version: i32 = self
            .conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))?;

        if version < 1 {
            let tx = self.conn.transaction()?;
            tx.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS packages (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    kind TEXT NOT NULL,
                    name TEXT NOT NULL,
                    version TEXT,
                    root TEXT,
                    first_seen_at INTEGER NOT NULL,
                    last_seen_at INTEGER NOT NULL,
                    UNIQUE(kind, name, version, root)
                );

                CREATE TABLE IF NOT EXISTS sources (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    provider TEXT NOT NULL,
                    source TEXT NOT NULL,
                    UNIQUE(provider, source)
                );

                CREATE TABLE IF NOT EXISTS checksums (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    algorithm TEXT NOT NULL,
                    value TEXT NOT NULL UNIQUE,
                    size INTEGER NOT NULL,
                    ref_count INTEGER NOT NULL DEFAULT 0
                );

                CREATE TABLE IF NOT EXISTS documents (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    package_id INTEGER NOT NULL REFERENCES packages(id),
                    source_id INTEGER NOT NULL REFERENCES sources(id),
                    title TEXT NOT NULL,
                    relative_path TEXT NOT NULL,
                    fs_path TEXT,
                    vault_path TEXT,
                    origin_url TEXT,
                    mime_type TEXT NOT NULL,
                    language TEXT,
                    size INTEGER NOT NULL,
                    metadata_checksum TEXT NOT NULL,
                    content_checksum_id INTEGER REFERENCES checksums(id),
                    content_stored BOOLEAN NOT NULL,
                    modified_at INTEGER NOT NULL,
                    ingested_at INTEGER NOT NULL
                );

                CREATE TABLE IF NOT EXISTS artifacts (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    package_id INTEGER NOT NULL REFERENCES packages(id),
                    name TEXT NOT NULL,
                    source TEXT NOT NULL,
                    path TEXT NOT NULL,
                    fs_path TEXT,
                    mime_type TEXT NOT NULL,
                    size INTEGER NOT NULL,
                    checksum TEXT NOT NULL,
                    modified_at INTEGER NOT NULL,
                    ingested_at INTEGER NOT NULL
                );

                CREATE TABLE IF NOT EXISTS tags (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE
                );

                CREATE TABLE IF NOT EXISTS document_tags (
                    document_id INTEGER NOT NULL REFERENCES documents(id),
                    tag_id INTEGER NOT NULL REFERENCES tags(id),
                    PRIMARY KEY (document_id, tag_id)
                );

                CREATE TABLE IF NOT EXISTS ingest_runs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    started_at INTEGER NOT NULL,
                    finished_at INTEGER NOT NULL,
                    status TEXT NOT NULL,
                    packages INTEGER NOT NULL,
                    documents INTEGER NOT NULL,
                    artifacts INTEGER NOT NULL,
                    error TEXT
                );
                "#,
            )?;
            tx.execute("PRAGMA user_version = 1;", [])?;
            tx.commit()?;
        }

        Ok(())
    }

    fn ingest(
        &mut self,
        layout: &VaultLayout,
        batch: &crate::storage::metadata::PendingBatch,
    ) -> Result<IngestReport> {
        let started_at = current_timestamp();
        let tx = self.conn.transaction()?;

        let mut touched_packages = std::collections::HashSet::new();
        let mut documents_persisted = 0_usize;
        let mut artifacts_persisted = 0_usize;

        // 1. Process documents
        for doc in &batch.documents {
            touched_packages.insert(doc.package.clone());

            // Upsert package
            let package_id = upsert_package(&tx, &doc.package, started_at)?;

            // Upsert source
            let source_id = upsert_source(&tx, &doc.provider, doc.source.as_tag())?;

            // Handle content checksum & file staging if content present
            let mut content_checksum_id = None;
            let mut vault_path = None;
            let mut content_stored = false;

            if let Some(ref content) = doc.content {
                let sha = content_checksum(content.as_bytes());
                let dest = layout.document_file(&sha);
                stage_content(layout.tmp_dir().as_path(), &dest, content.as_bytes())?;

                let size = i64::try_from(content.len()).unwrap_or(0);
                let cs_id = upsert_checksum(&tx, "sha256", &sha, size)?;
                content_checksum_id = Some(cs_id);
                vault_path = Some(layout.logical_vault_path(&sha));
                content_stored = true;
            } else if let Some(ref sha) = doc.content_checksum {
                // If content_checksum is provided without inline content (e.g. file on disk)
                if let Some(ref fs_path) = doc.fs_path {
                    if fs_path.exists() {
                        let dest = layout.document_file(sha);
                        if !dest.exists() {
                            if let Ok(bytes) = std::fs::read(fs_path) {
                                stage_content(layout.tmp_dir().as_path(), &dest, &bytes)?;
                            }
                        }
                    }
                }
                let size = i64::try_from(doc.size).unwrap_or(0);
                let cs_id = upsert_checksum(&tx, "sha256", sha, size)?;
                content_checksum_id = Some(cs_id);
                vault_path = doc.vault_path.clone();
                content_stored = vault_path.is_some();
            }

            let modified_at = doc
                .modified
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .try_into()
                .unwrap_or(0);

            // Insert document (or update if already exists by package + relative_path + metadata_checksum)
            let fs_path_str = doc
                .fs_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());
            let language = doc.language.as_deref();
            let size = i64::try_from(doc.size).unwrap_or(0);

            tx.execute(
                r#"
                INSERT INTO documents (
                    package_id, source_id, title, relative_path, fs_path, vault_path,
                    origin_url, mime_type, language, size, metadata_checksum,
                    content_checksum_id, content_stored, modified_at, ingested_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                "#,
                rusqlite::params![
                    package_id,
                    source_id,
                    doc.title,
                    doc.relative_path.to_string_lossy().to_string(),
                    fs_path_str,
                    vault_path,
                    doc.origin_url,
                    doc.mime_type,
                    language,
                    size,
                    doc.metadata_checksum,
                    content_checksum_id,
                    content_stored,
                    modified_at,
                    started_at,
                ],
            )?;

            let doc_id = tx.last_insert_rowid();

            // Insert tags
            for tag in &doc.tags {
                tx.execute(
                    "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING;",
                    [tag],
                )?;
                let tag_id: i64 =
                    tx.query_row("SELECT id FROM tags WHERE name = ?1;", [tag], |row| {
                        row.get(0)
                    })?;
                tx.execute(
                    "INSERT OR IGNORE INTO document_tags (document_id, tag_id) VALUES (?1, ?2);",
                    [doc_id, tag_id],
                )?;
            }

            documents_persisted += 1;
        }

        // 2. Process artifacts
        for art in &batch.artifacts {
            touched_packages.insert(art.package.clone());
            let package_id = upsert_package(&tx, &art.package, started_at)?;

            let modified_at = art
                .modified
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .try_into()
                .unwrap_or(0);

            let fs_path_str = art
                .fs_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());
            let size = i64::try_from(art.size).unwrap_or(0);

            tx.execute(
                r#"
                INSERT INTO artifacts (
                    package_id, name, source, path, fs_path, mime_type, size,
                    checksum, modified_at, ingested_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                rusqlite::params![
                    package_id,
                    art.name,
                    art.source.as_tag(),
                    art.path.to_string_lossy().to_string(),
                    fs_path_str,
                    art.mime_type,
                    size,
                    art.checksum,
                    modified_at,
                    started_at,
                ],
            )?;

            artifacts_persisted += 1;
        }

        let finished_at = current_timestamp();
        let pkg_count = i64::try_from(touched_packages.len()).unwrap_or(0);
        let doc_count = i64::try_from(documents_persisted).unwrap_or(0);
        let art_count = i64::try_from(artifacts_persisted).unwrap_or(0);

        tx.execute(
            r#"
            INSERT INTO ingest_runs (started_at, finished_at, status, packages, documents, artifacts, error)
            VALUES (?1, ?2, 'ok', ?3, ?4, ?5, NULL)
            "#,
            rusqlite::params![started_at, finished_at, pkg_count, doc_count, art_count],
        )?;

        let run_id = tx.last_insert_rowid();
        tx.commit()?;

        Ok(IngestReport {
            run_id,
            packages: touched_packages.len(),
            documents: documents_persisted,
            artifacts: artifacts_persisted,
        })
    }

    fn counts(&self) -> Result<Counts> {
        let packages: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM packages;", [], |row| row.get(0))?;
        let documents: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM documents;", [], |row| row.get(0))?;
        let artifacts: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM artifacts;", [], |row| row.get(0))?;
        Ok(Counts {
            packages,
            documents,
            artifacts,
        })
    }

    fn storage_size(&self) -> Result<u64> {
        let size: i64 =
            self.conn
                .query_row("SELECT COALESCE(SUM(size), 0) FROM checksums;", [], |row| {
                    row.get(0)
                })?;
        Ok(u64::try_from(size).unwrap_or(0))
    }

    fn verify(&self, layout: &VaultLayout) -> Result<VerifyReport> {
        let mut report = VerifyReport::default();

        // 1. Missing files (original fs_path no longer exists)
        let mut stmt = self.conn.prepare("SELECT fs_path FROM documents WHERE fs_path IS NOT NULL UNION SELECT fs_path FROM artifacts WHERE fs_path IS NOT NULL;")?;
        let paths = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for p_str in paths.flatten() {
            let p = PathBuf::from(p_str);
            if !p.exists() {
                report.missing_files.push(p);
            }
        }

        // 2. Orphaned rows (content_stored documents whose vault file is missing)
        let mut stmt = self.conn.prepare(
            "SELECT vault_path FROM documents WHERE content_stored = 1 AND vault_path IS NOT NULL;",
        )?;
        let vault_paths = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for vp in vault_paths.flatten() {
            let full_path = layout.root().join(&vp);
            if !full_path.exists() {
                report.orphaned_rows.push(vp);
            }
        }

        // 3. Checksum mismatches & Reverse orphans in documents/
        let docs_dir = layout.documents_dir();
        if docs_dir.exists() {
            let entries = std::fs::read_dir(&docs_dir)?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        // Check if file_name matches sha256 of the file content
                        if let Ok(actual_sha) = content_checksum_file(&path)
                            && actual_sha != file_name
                        {
                            report.checksum_mismatches.push(path.clone());
                        }

                        // Check if file is a reverse orphan (no document references it)
                        let logical = format!("documents/{file_name}");
                        let count: i64 = self.conn.query_row(
                            "SELECT COUNT(*) FROM documents WHERE vault_path = ?1;",
                            [&logical],
                            |row| row.get(0),
                        )?;
                        if count == 0 {
                            report.reverse_orphans.push(path);
                        }
                    }
                }
            }
        }

        Ok(report)
    }
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(0)
}

fn upsert_package(
    tx: &rusqlite::Transaction,
    pkg: &crate::storage::metadata::PackageKey,
    now: i64,
) -> Result<i64> {
    let root_str = pkg.root.to_string_lossy().to_string();
    tx.execute(
        r#"
        INSERT INTO packages (kind, name, version, root, first_seen_at, last_seen_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?5)
        ON CONFLICT(kind, name, version, root) DO UPDATE SET last_seen_at = excluded.last_seen_at;
        "#,
        rusqlite::params![pkg.kind, pkg.name, pkg.version, root_str, now],
    )?;

    let id: i64 = tx.query_row(
        r#"
        SELECT id FROM packages
        WHERE kind = ?1 AND name = ?2 AND ((version IS ?3) OR (version = ?3)) AND root = ?4;
        "#,
        rusqlite::params![pkg.kind, pkg.name, pkg.version, root_str],
        |row| row.get(0),
    )?;

    Ok(id)
}

fn upsert_source(tx: &rusqlite::Transaction, provider: &str, source: &str) -> Result<i64> {
    tx.execute(
        r#"
        INSERT INTO sources (provider, source) VALUES (?1, ?2)
        ON CONFLICT(provider, source) DO NOTHING;
        "#,
        [provider, source],
    )?;

    let id: i64 = tx.query_row(
        "SELECT id FROM sources WHERE provider = ?1 AND source = ?2;",
        [provider, source],
        |row| row.get(0),
    )?;

    Ok(id)
}

fn upsert_checksum(
    tx: &rusqlite::Transaction,
    algorithm: &str,
    value: &str,
    size: i64,
) -> Result<i64> {
    tx.execute(
        r#"
        INSERT INTO checksums (algorithm, value, size, ref_count) VALUES (?1, ?2, ?3, 1)
        ON CONFLICT(value) DO UPDATE SET ref_count = ref_count + 1;
        "#,
        rusqlite::params![algorithm, value, size],
    )?;

    let id: i64 = tx.query_row(
        "SELECT id FROM checksums WHERE value = ?1;",
        [value],
        |row| row.get(0),
    )?;

    Ok(id)
}
