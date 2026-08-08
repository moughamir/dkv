//! Deterministic vault directory layout.
//!
//! The vault root contains eight fixed subdirectories; document content files
//! live under `documents/` named by their `SHA-256` content checksum.

use std::path::{Path, PathBuf};

use crate::storage::errors::{Result, StorageError};

/// Name of the `metadata` subdirectory.
pub const METADATA_DIR: &str = "metadata";
/// Name of the `documents` subdirectory.
pub const DOCUMENTS_DIR: &str = "documents";
/// Name of the `packages` subdirectory.
pub const PACKAGES_DIR: &str = "packages";
/// Name of the `artifacts` subdirectory.
pub const ARTIFACTS_DIR: &str = "artifacts";
/// Name of the `checksums` subdirectory.
pub const CHECKSUMS_DIR: &str = "checksums";
/// Name of the `sqlite` subdirectory.
pub const SQLITE_DIR: &str = "sqlite";
/// Name of the `logs` subdirectory.
pub const LOGS_DIR: &str = "logs";
/// Name of the `tmp` subdirectory.
pub const TMP_DIR: &str = "tmp";

/// Deterministic vault directory layout.
#[derive(Debug, Clone)]
pub struct VaultLayout {
    /// Root directory of the vault.
    pub root: PathBuf,
    metadata: PathBuf,
    documents: PathBuf,
    packages: PathBuf,
    artifacts: PathBuf,
    checksums: PathBuf,
    sqlite: PathBuf,
    logs: PathBuf,
    tmp: PathBuf,
}

impl VaultLayout {
    /// Creates a layout rooted at `root`; no directories are created.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            metadata: root.join(METADATA_DIR),
            documents: root.join(DOCUMENTS_DIR),
            packages: root.join(PACKAGES_DIR),
            artifacts: root.join(ARTIFACTS_DIR),
            checksums: root.join(CHECKSUMS_DIR),
            sqlite: root.join(SQLITE_DIR),
            logs: root.join(LOGS_DIR),
            tmp: root.join(TMP_DIR),
            root,
        }
    }

    /// Returns the vault root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the `metadata` subdirectory path.
    #[must_use]
    pub fn metadata_dir(&self) -> PathBuf {
        self.metadata.clone()
    }

    /// Returns the `documents` subdirectory path.
    #[must_use]
    pub fn documents_dir(&self) -> PathBuf {
        self.documents.clone()
    }

    /// Returns the `packages` subdirectory path.
    #[must_use]
    pub fn packages_dir(&self) -> PathBuf {
        self.packages.clone()
    }

    /// Returns the `artifacts` subdirectory path.
    #[must_use]
    pub fn artifacts_dir(&self) -> PathBuf {
        self.artifacts.clone()
    }

    /// Returns the `checksums` subdirectory path.
    #[must_use]
    pub fn checksums_dir(&self) -> PathBuf {
        self.checksums.clone()
    }

    /// Returns the `sqlite` subdirectory path.
    #[must_use]
    pub fn sqlite_dir(&self) -> PathBuf {
        self.sqlite.clone()
    }

    /// Returns the `logs` subdirectory path.
    #[must_use]
    pub fn logs_dir(&self) -> PathBuf {
        self.logs.clone()
    }

    /// Returns the `tmp` subdirectory path.
    #[must_use]
    pub fn tmp_dir(&self) -> PathBuf {
        self.tmp.clone()
    }

    /// Returns the full path to the `SQLite` database file.
    #[must_use]
    pub fn db_path(&self) -> PathBuf {
        self.sqlite_dir().join("vault.db")
    }

    /// Creates the root directory and all eight subdirectories.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Io`] if any directory cannot be created.
    pub fn ensure(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        for dir in [
            &self.metadata,
            &self.documents,
            &self.packages,
            &self.artifacts,
            &self.checksums,
            &self.sqlite,
            &self.logs,
            &self.tmp,
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    /// Validates that the vault root exists and is a directory.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::InvalidLayout`] when `root` is not an existing
    /// directory.
    pub fn validate(&self) -> Result<()> {
        if !self.root.is_dir() {
            return Err(StorageError::InvalidLayout(format!(
                "vault root is not an existing directory: {}",
                self.root.display()
            )));
        }
        Ok(())
    }

    /// Removes all entries under `tmp_dir()`.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Io`] if an entry cannot be read or removed. A
    /// missing `tmp` directory is tolerated (no-op).
    pub fn clean_tmp(&self) -> Result<()> {
        let entries = match std::fs::read_dir(&self.tmp) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(StorageError::Io(error)),
        };
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else {
                std::fs::remove_file(&path)?;
            }
        }
        Ok(())
    }

    /// Returns the path of the stored content file for a document content
    /// checksum.
    #[must_use]
    pub fn document_file(&self, checksum: &str) -> PathBuf {
        self.documents_dir().join(checksum)
    }

    /// Returns the logical vault path for a document content checksum,
    /// e.g. `documents/<sha256>`.
    #[allow(clippy::unused_self)]
    #[must_use]
    pub fn logical_vault_path(&self, checksum: &str) -> String {
        format!("{DOCUMENTS_DIR}/{checksum}")
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn new_creates_expected_subdir_paths() {
        let layout = VaultLayout::new("/vault/root");
        assert_eq!(layout.root(), Path::new("/vault/root"));
        assert_eq!(layout.metadata_dir(), PathBuf::from("/vault/root/metadata"));
        assert_eq!(
            layout.documents_dir(),
            PathBuf::from("/vault/root/documents")
        );
        assert_eq!(layout.packages_dir(), PathBuf::from("/vault/root/packages"));
        assert_eq!(
            layout.artifacts_dir(),
            PathBuf::from("/vault/root/artifacts")
        );
        assert_eq!(
            layout.checksums_dir(),
            PathBuf::from("/vault/root/checksums")
        );
        assert_eq!(layout.sqlite_dir(), PathBuf::from("/vault/root/sqlite"));
        assert_eq!(layout.logs_dir(), PathBuf::from("/vault/root/logs"));
        assert_eq!(layout.tmp_dir(), PathBuf::from("/vault/root/tmp"));
        assert_eq!(
            layout.db_path(),
            PathBuf::from("/vault/root/sqlite/vault.db")
        );
    }

    #[test]
    fn ensure_creates_root_and_all_subdirs() {
        #[allow(clippy::unwrap_used)]
        let dir = tempdir().unwrap();
        let layout = VaultLayout::new(dir.path().join("vault"));
        #[allow(clippy::unwrap_used)]
        layout.ensure().unwrap();

        assert!(layout.root.is_dir());
        for path in [
            layout.metadata_dir(),
            layout.documents_dir(),
            layout.packages_dir(),
            layout.artifacts_dir(),
            layout.checksums_dir(),
            layout.sqlite_dir(),
            layout.logs_dir(),
            layout.tmp_dir(),
        ] {
            assert!(path.is_dir(), "expected directory {}", path.display());
        }
    }

    #[test]
    fn validate_fails_on_missing_root() {
        #[allow(clippy::unwrap_used)]
        let dir = tempdir().unwrap();
        let layout = VaultLayout::new(dir.path().join("does-not-exist"));
        #[allow(clippy::unwrap_used)]
        let error = layout.validate().unwrap_err();
        assert!(matches!(error, StorageError::InvalidLayout(_)));
    }

    #[test]
    fn clean_tmp_removes_stale_files_and_tolerates_missing_dir() {
        #[allow(clippy::unwrap_used)]
        let dir = tempdir().unwrap();
        let layout = VaultLayout::new(dir.path());
        #[allow(clippy::unwrap_used)]
        layout.ensure().unwrap();

        let stale_file = layout.tmp_dir().join("stale.tmp");
        let stale_dir = layout.tmp_dir().join("stale-dir");
        #[allow(clippy::unwrap_used)]
        fs::write(&stale_file, "stale").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(&stale_dir).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(stale_dir.join("nested.tmp"), "x").unwrap();

        #[allow(clippy::unwrap_used)]
        layout.clean_tmp().unwrap();

        assert!(!stale_file.exists());
        assert!(!stale_dir.exists());

        let isolated = VaultLayout::new(dir.path().join("elsewhere"));
        #[allow(clippy::unwrap_used)]
        isolated.clean_tmp().unwrap();
    }

    #[test]
    fn document_file_and_logical_vault_path_shapes() {
        let layout = VaultLayout::new("/vault");
        assert_eq!(
            layout.document_file("abc123"),
            PathBuf::from("/vault/documents/abc123")
        );
        assert_eq!(layout.logical_vault_path("abc123"), "documents/abc123");
    }
}
