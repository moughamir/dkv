//! Local provider.
//!
//! Collects documentation files from a local directory, metadata only. Walks
//! the package root with `walkdir`, skipping hidden entries and any file whose
//! extension is not in the supported documentation set.

use std::fs;
use std::io;
use std::path::Path;
use std::time::SystemTime;

use walkdir::WalkDir;

use crate::error::{DkvError, Result};
use crate::providers::traits::Provider;
use crate::providers::types::{
    CollectedDocs, KnowledgeDocument, KnowledgeSource, PackageId, PackageKind,
    ProviderCapabilities, ProviderMetadata, ProviderStatus, fingerprint, mime_type_for,
};

/// Documentation file extensions accepted by the local provider, matched
/// case-insensitively. Everything else is skipped.
const SUPPORTED_EXTENSIONS: &[&str] =
    &["md", "markdown", "adoc", "txt", "rst", "html", "htm", "pdf"];

/// Collects documentation files from a local directory.
pub struct LocalProvider;

impl Provider for LocalProvider {
    fn id(&self) -> &'static str {
        "local"
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: self.id(),
            name: "Local",
            description: "Collects documentation files from a local directory",
            status: ProviderStatus::Active,
        }
    }

    fn supports(&self, package: &PackageId) -> bool {
        package.kind == PackageKind::Local
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            offline: true,
            package_kinds: vec![PackageKind::Local],
            sources: vec![KnowledgeSource::Documentation],
            reads_content: false,
        }
    }

    fn collect(&self, package: &PackageId) -> Result<CollectedDocs> {
        let metadata = fs::metadata(&package.root)?;
        if !metadata.is_dir() {
            return Err(DkvError::Io(io::Error::new(
                io::ErrorKind::NotADirectory,
                format!(
                    "package root is not a directory: {}",
                    package.root.display()
                ),
            )));
        }

        let mut documents = Vec::new();

        for entry in WalkDir::new(&package.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                entry.depth() == 0 || !entry.file_name().to_string_lossy().starts_with('.')
            })
        {
            let entry = entry.map_err(walkdir_error_to_io)?;

            if entry.depth() == 0 {
                continue;
            }
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let relative_path = path.strip_prefix(&package.root).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "walked entry escaped the package root",
                )
            })?;

            if !has_supported_extension(relative_path) {
                continue;
            }

            let entry_metadata = entry.metadata().map_err(walkdir_error_to_io)?;
            let modified = entry_metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let size = entry_metadata.len();

            let title = path.file_stem().map_or_else(
                || {
                    path.file_name().map_or_else(
                        || path.display().to_string(),
                        |name| name.to_string_lossy().into_owned(),
                    )
                },
                |stem| stem.to_string_lossy().into_owned(),
            );

            documents.push(KnowledgeDocument {
                title,
                source: KnowledgeSource::Documentation,
                path: relative_path.to_path_buf(),
                package: package.name.clone(),
                language: None,
                mime_type: mime_type_for(relative_path)
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
                size,
                checksum: fingerprint(relative_path, size, &modified),
                modified,
                tags: vec![KnowledgeSource::Documentation.as_tag().to_string()],
                content: None,
            });
        }

        documents.sort_by_key(|document| document.path.clone());

        Ok(CollectedDocs {
            provider: self.id().to_string(),
            documents,
            assets: Vec::new(),
        })
    }
}

/// Converts a `walkdir::Error` into an `std::io::Error` so traversal failures
/// can propagate through the provider's `Result` with `?`.
fn walkdir_error_to_io(error: walkdir::Error) -> io::Error {
    let message = error.to_string();
    error
        .into_io_error()
        .unwrap_or_else(|| io::Error::other(message))
}

/// Returns `true` when `path` ends in one of [`SUPPORTED_EXTENSIONS`],
/// matched case-insensitively.
fn has_supported_extension(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };
    SUPPORTED_EXTENSIONS
        .iter()
        .any(|supported| extension.eq_ignore_ascii_case(supported))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;

    /// Collects documents for a local package rooted at `dir`, panicking on
    /// collection errors.
    fn collect_documents(dir: &Path) -> Vec<KnowledgeDocument> {
        let provider = LocalProvider;
        let package = PackageId::new(PackageKind::Local, "test-package", dir);
        #[allow(clippy::unwrap_used)]
        provider.collect(&package).unwrap().documents
    }

    #[test]
    fn collect_finds_supported_extensions_and_skips_unknown() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        for name in [
            "a.md",
            "b.markdown",
            "c.adoc",
            "d.txt",
            "e.rst",
            "f.html",
            "g.htm",
            "h.pdf",
            "i.unknown",
            "noext",
        ] {
            #[allow(clippy::unwrap_used)]
            fs::write(root.join(name), "content").unwrap();
        }

        let documents = collect_documents(root);

        assert_eq!(documents.len(), 8);

        let paths: Vec<String> = documents
            .iter()
            .map(|document| document.path.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            paths,
            vec![
                "a.md",
                "b.markdown",
                "c.adoc",
                "d.txt",
                "e.rst",
                "f.html",
                "g.htm",
                "h.pdf"
            ]
        );

        let mimes: Vec<&str> = documents
            .iter()
            .map(|document| document.mime_type.as_str())
            .collect();
        assert_eq!(
            mimes,
            vec![
                "text/markdown",
                "text/markdown",
                "text/asciidoc",
                "text/plain",
                "text/x-rst",
                "text/html",
                "text/html",
                "application/pdf",
            ]
        );

        let titles: Vec<&str> = documents
            .iter()
            .map(|document| document.title.as_str())
            .collect();
        assert_eq!(titles, vec!["a", "b", "c", "d", "e", "f", "g", "h"]);

        assert!(documents.iter().all(|document| document.content.is_none()));
        assert!(
            documents
                .iter()
                .all(|document| document.source == KnowledgeSource::Documentation)
        );
        assert!(
            documents
                .iter()
                .all(|document| document.tags == vec!["docs".to_string()])
        );
        assert!(
            documents
                .iter()
                .all(|document| !document.path.is_absolute())
        );
        assert!(
            documents
                .iter()
                .all(|document| document.package == "test-package")
        );
    }

    #[test]
    fn collect_skips_hidden_entries() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join(".hidden")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join(".hidden/x.md"), "x").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("y.md"), "y").unwrap();

        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join(".git")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join(".git/config"), "config").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("README.md"), "readme").unwrap();

        let paths: Vec<String> = collect_documents(root)
            .into_iter()
            .map(|document| document.path.to_string_lossy().into_owned())
            .collect();
        assert_eq!(paths, vec!["README.md", "y.md"]);
    }

    #[test]
    fn collect_walks_nested_directories() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("src")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("src/guide.md"), "# guide").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("docs/sub")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("docs/sub/deep.adoc"), "= deep").unwrap();

        let paths: Vec<String> = collect_documents(root)
            .into_iter()
            .map(|document| document.path.to_string_lossy().into_owned())
            .collect();
        assert_eq!(paths, vec!["docs/sub/deep.adoc", "src/guide.md"]);
    }

    #[test]
    fn collect_is_case_insensitive_on_extension() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        #[allow(clippy::unwrap_used)]
        fs::write(root.join("UPPER.MD"), "x").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("lower.pdf"), "y").unwrap();

        let paths: Vec<String> = collect_documents(root)
            .into_iter()
            .map(|document| document.path.to_string_lossy().into_owned())
            .collect();
        assert_eq!(paths, vec!["UPPER.MD", "lower.pdf"]);
    }

    #[test]
    fn collect_errors_on_missing_root() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope");

        let provider = LocalProvider;
        let package = PackageId::new(PackageKind::Local, "test-package", missing);
        assert!(provider.collect(&package).is_err());
    }

    #[test]
    fn collect_metadata_is_deterministic() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("docs")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("docs/a.md"), "alpha").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("b.txt"), "beta").unwrap();

        let first_collection = collect_documents(root);
        let second_collection = collect_documents(root);

        assert_eq!(first_collection, second_collection);
        assert_eq!(first_collection.len(), 2);
    }
}
