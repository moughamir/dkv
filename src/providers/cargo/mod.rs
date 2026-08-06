//! Cargo provider.
//!
//! Collects knowledge documents for local Cargo packages. Purely offline: the
//! conventional top-level README/changelog/license files, the `examples/`,
//! `benches/`, `tests/` and `docs/` trees, module-level docs from `src/lib.rs`,
//! and a metadata summary built from the already-parsed scanner results. No
//! network access, no registry or `docs.rs` lookups.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use walkdir::WalkDir;

use crate::error::{DkvError, Result};
use crate::providers::traits::Provider;
use crate::providers::types::{
    CollectedDocs, KnowledgeDocument, KnowledgeSource, PackageId, PackageKind,
    ProviderCapabilities, ProviderMetadata, ProviderStatus, fingerprint, mime_type_for,
};

/// Collects local Cargo package knowledge.
pub struct CargoProvider;

impl Provider for CargoProvider {
    fn id(&self) -> &'static str {
        "cargo"
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: self.id(),
            name: "Cargo",
            description: "Collects local Cargo package knowledge (README, changelog, license, examples, benches, tests, docs, module docs, metadata)",
            status: ProviderStatus::Active,
        }
    }

    fn supports(&self, package: &PackageId) -> bool {
        package.kind == PackageKind::Cargo
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            offline: true,
            package_kinds: vec![PackageKind::Cargo],
            sources: vec![
                KnowledgeSource::Readme,
                KnowledgeSource::Changelog,
                KnowledgeSource::License,
                KnowledgeSource::Documentation,
                KnowledgeSource::Examples,
                KnowledgeSource::Tests,
                KnowledgeSource::Benches,
                KnowledgeSource::Metadata,
            ],
            reads_content: true,
        }
    }

    fn collect(&self, package: &PackageId) -> Result<CollectedDocs> {
        // 1. Root validation: nothing is collected unless the root exists and
        // is a directory.
        let root_metadata = fs::metadata(&package.root)?;
        if !root_metadata.is_dir() {
            return Err(DkvError::Io(io::Error::new(
                io::ErrorKind::NotADirectory,
                format!(
                    "package root is not a directory: {}",
                    package.root.display()
                ),
            )));
        }

        let mut documents = Vec::new();

        collect_top_level_files(package, &mut documents)?;
        collect_knowledge_directories(package, &mut documents)?;
        collect_lib_module_docs(package, &mut documents)?;
        documents.push(metadata_document(package));

        // 6. Deterministic ordering.
        documents.sort_by_key(|document| document.path.clone());

        Ok(CollectedDocs {
            provider: self.id().to_string(),
            documents,
            assets: Vec::new(),
        })
    }
}

impl PackageId {
    /// Builds a `PackageId` from an already-parsed
    /// [`crate::types::project::CargoPackage`], reusing scanner results (no
    /// filesystem traversal).
    #[must_use]
    pub fn from_cargo(package: &crate::types::project::CargoPackage) -> Self {
        Self {
            kind: PackageKind::Cargo,
            name: package.name.clone(),
            version: package.version.clone(),
            root: package
                .manifest_path
                .parent()
                .map_or_else(PathBuf::new, Path::to_path_buf),
        }
    }
}

/// Collects the conventional top-level knowledge files (README, changelog,
/// license) into `documents`. Files are stat'd only; contents are not read.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] when a file's metadata cannot be read.
fn collect_top_level_files(
    package: &PackageId,
    documents: &mut Vec<KnowledgeDocument>,
) -> Result<()> {
    let candidates: &[(&str, KnowledgeSource)] = &[
        ("README.md", KnowledgeSource::Readme),
        ("README", KnowledgeSource::Readme),
        ("CHANGELOG.md", KnowledgeSource::Changelog),
        ("LICENSE", KnowledgeSource::License),
    ];
    for (file_name, source) in candidates {
        let path = package.root.join(file_name);
        if !exists_as_file(&path) {
            continue;
        }
        documents.push(stat_document(
            &package.name,
            *source,
            &path,
            Path::new(file_name),
            None,
        )?);
    }
    Ok(())
}

/// Recursively collects every regular file under the conventional
/// documentation directories (`examples/`, `benches/`, `tests/`, `docs/`)
/// into `documents`. Directories and symlinks are skipped.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] when a walk or a file stat fails.
fn collect_knowledge_directories(
    package: &PackageId,
    documents: &mut Vec<KnowledgeDocument>,
) -> Result<()> {
    let directories: &[(&str, KnowledgeSource)] = &[
        ("examples", KnowledgeSource::Examples),
        ("benches", KnowledgeSource::Benches),
        ("tests", KnowledgeSource::Tests),
        ("docs", KnowledgeSource::Documentation),
    ];
    for (dir_name, source) in directories {
        collect_dir_documents(&package.root, &package.name, dir_name, *source, documents)?;
    }
    Ok(())
}

/// Extracts module-level docs from `src/lib.rs`, if present, and pushes a
/// document carrying the extracted content (the only content read by this
/// provider).
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] when `src/lib.rs` cannot be read.
fn collect_lib_module_docs(
    package: &PackageId,
    documents: &mut Vec<KnowledgeDocument>,
) -> Result<()> {
    let lib_rs_path = package.root.join("src/lib.rs");
    if !lib_rs_path.is_file() {
        return Ok(());
    }
    let contents = fs::read_to_string(&lib_rs_path)?;
    let module_docs = extract_module_docs(&contents);
    if module_docs.trim().is_empty() {
        return Ok(());
    }

    let metadata = fs::metadata(&lib_rs_path)?;
    let size = metadata.len();
    let modified = metadata.modified()?;
    let relative_path = Path::new("src/lib.rs");
    documents.push(KnowledgeDocument {
        title: "lib module docs".to_string(),
        source: KnowledgeSource::Documentation,
        path: relative_path.to_path_buf(),
        package: package.name.clone(),
        language: Some("rust".to_string()),
        mime_type: "text/x-rust".to_string(),
        size,
        checksum: fingerprint(relative_path, size, &modified),
        modified,
        tags: vec!["docs".to_string()],
        content: Some(module_docs),
    });
    Ok(())
}

/// Builds the package metadata document purely from the in-memory `PackageId`,
/// reusing scanner results without re-parsing the manifest. Always emitted,
/// even for an otherwise empty package.
fn metadata_document(package: &PackageId) -> KnowledgeDocument {
    let content = format!(
        "name: {}\nversion: {}",
        package.name,
        package.version.as_deref().unwrap_or("unknown")
    );
    let size = content.len() as u64;
    let modified = fs::metadata(package.root.join("Cargo.toml"))
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let relative_path = Path::new("Cargo.toml");
    KnowledgeDocument {
        title: format!("{} metadata", package.name),
        source: KnowledgeSource::Metadata,
        path: relative_path.to_path_buf(),
        package: package.name.clone(),
        language: None,
        mime_type: "application/toml".to_string(),
        size,
        checksum: fingerprint(relative_path, size, &modified),
        modified,
        tags: vec!["metadata".to_string(), "cargo".to_string()],
        content: Some(content),
    }
}

/// Whether `path` exists as a regular file (following symlinks).
fn exists_as_file(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// File stem of `path` as an owned string (empty when there is none).
fn file_stem_of(path: &Path) -> String {
    path.file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
}

/// `Some("rust")` for `.rs` files, `None` otherwise.
fn rust_language(path: &Path) -> Option<String> {
    if path.extension().is_some_and(|extension| extension == "rs") {
        Some("rust".to_string())
    } else {
        None
    }
}

/// Builds a metadata-only document for a file, reading only its metadata.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] when the file's metadata cannot be read.
fn stat_document(
    package_name: &str,
    source: KnowledgeSource,
    stat_path: &Path,
    relative_path: &Path,
    language: Option<&str>,
) -> Result<KnowledgeDocument> {
    let metadata = fs::metadata(stat_path)?;
    let size = metadata.len();
    let modified = metadata.modified()?;
    Ok(KnowledgeDocument {
        title: file_stem_of(relative_path),
        source,
        path: relative_path.to_path_buf(),
        package: package_name.to_string(),
        language: language.map(str::to_string),
        mime_type: mime_type_for(relative_path).unwrap_or_else(|| "text/plain".to_string()),
        size,
        checksum: fingerprint(relative_path, size, &modified),
        modified,
        tags: vec![source.as_tag().to_string()],
        content: None,
    })
}

/// Recursively collects every regular file under `root/<dir_name>` into
/// `documents`, skipping directories and symlinks.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] when the walk or a file stat fails.
fn collect_dir_documents(
    root: &Path,
    package_name: &str,
    dir_name: &str,
    source: KnowledgeSource,
    documents: &mut Vec<KnowledgeDocument>,
) -> Result<()> {
    let dir_path = root.join(dir_name);
    if !dir_path.is_dir() {
        return Ok(());
    }
    for entry in WalkDir::new(&dir_path).follow_links(false) {
        let entry = entry.map_err(io::Error::from)?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative_path = entry
            .path()
            .strip_prefix(root)
            .map_or_else(|_| entry.path().to_path_buf(), Path::to_path_buf);
        documents.push(stat_document(
            package_name,
            source,
            entry.path(),
            &relative_path,
            rust_language(entry.path()).as_deref(),
        )?);
    }
    Ok(())
}

/// Appends one line of a `/*!` block, stripping the common leading `*`.
fn push_block_line(out: &mut String, line: &str) {
    let stripped = line.strip_prefix('*').unwrap_or(line);
    let stripped = stripped.strip_prefix(' ').unwrap_or(stripped);
    out.push_str(stripped);
    out.push('\n');
}

/// Extracts the leading module-level doc comments (`//!` lines and `/*! ... */`
/// blocks) from a Rust source file. Returns an empty string when none are
/// present. Line-based: does not attempt full Rust syntax parsing.
fn extract_module_docs(contents: &str) -> String {
    let mut out = String::new();
    let mut in_block = false;
    for line in contents.lines() {
        let trimmed = line.trim_start();
        if in_block {
            if let Some(end) = trimmed.find("*/") {
                push_block_line(&mut out, &trimmed[..end]);
                in_block = false;
            } else {
                push_block_line(&mut out, trimmed);
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("//!") {
            let text = rest.strip_prefix(' ').unwrap_or(rest);
            out.push_str(text);
            out.push('\n');
        } else if let Some(rest) = trimmed.strip_prefix("/*!") {
            if let Some(end) = rest.find("*/") {
                push_block_line(&mut out, &rest[..end]);
            } else {
                push_block_line(&mut out, rest);
                in_block = true;
            }
        } else {
            // First non-doc line ends the module documentation block.
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;

    /// Writes `contents` to `root.join(relative)`, creating parent directories.
    fn write_file(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            #[allow(clippy::unwrap_used)]
            fs::create_dir_all(parent).unwrap();
        }
        #[allow(clippy::unwrap_used)]
        fs::write(path, contents).unwrap();
    }

    /// Collects documents for a Cargo package named `name` rooted at `dir`.
    fn collect_documents(dir: &Path, name: &str) -> CollectedDocs {
        let package = PackageId::new(PackageKind::Cargo, name, dir);
        #[allow(clippy::unwrap_used)]
        CargoProvider.collect(&package).unwrap()
    }

    /// Finds a document by title in a slice.
    fn find_by_title<'a>(
        documents: &'a [KnowledgeDocument],
        title: &str,
    ) -> Option<&'a KnowledgeDocument> {
        documents.iter().find(|document| document.title == title)
    }

    /// Finds a document by source in a slice.
    fn find_by_source(
        documents: &[KnowledgeDocument],
        source: KnowledgeSource,
    ) -> Option<&KnowledgeDocument> {
        documents.iter().find(|document| document.source == source)
    }

    #[test]
    fn collect_finds_readme_changelog_license_and_metadata() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_file(root, "Cargo.toml", "[package]\nname = \"demo\"\n");
        write_file(root, "README.md", "# Demo\n");
        write_file(root, "CHANGELOG.md", "## 0.1.0\n");
        write_file(root, "LICENSE", "MIT\n");

        let collected = collect_documents(root, "demo");
        let documents = &collected.documents;
        assert_eq!(collected.provider, "cargo");
        assert_eq!(documents.len(), 4);

        #[allow(clippy::unwrap_used)]
        let readme = find_by_title(documents, "README").unwrap();
        assert_eq!(readme.source, KnowledgeSource::Readme);
        assert_eq!(readme.mime_type, "text/markdown");
        assert_eq!(readme.path, PathBuf::from("README.md"));
        assert_eq!(readme.package, "demo");
        assert_eq!(readme.language, None);
        assert!(readme.content.is_none());
        assert_eq!(readme.tags, vec!["readme"]);

        #[allow(clippy::unwrap_used)]
        let changelog = find_by_title(documents, "CHANGELOG").unwrap();
        assert_eq!(changelog.source, KnowledgeSource::Changelog);
        assert_eq!(changelog.mime_type, "text/markdown");

        #[allow(clippy::unwrap_used)]
        let license = find_by_title(documents, "LICENSE").unwrap();
        assert_eq!(license.source, KnowledgeSource::License);
        assert_eq!(license.mime_type, "text/plain");

        #[allow(clippy::unwrap_used)]
        let metadata = find_by_source(documents, KnowledgeSource::Metadata).unwrap();
        assert_eq!(
            metadata.content.as_deref(),
            Some("name: demo\nversion: unknown")
        );
        assert_eq!(metadata.tags, vec!["metadata", "cargo"]);
        assert_eq!(metadata.path, PathBuf::from("Cargo.toml"));
        assert_eq!(metadata.mime_type, "application/toml");

        for document in documents {
            assert_eq!(document.checksum.len(), 16);
            assert!(document.checksum.chars().all(|c| {
                c.is_ascii_hexdigit() && (c.is_ascii_lowercase() || c.is_ascii_digit())
            }));
        }
    }

    #[test]
    fn collect_does_not_include_missing_sources() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_file(root, "Cargo.toml", "[package]\nname = \"demo\"\n");

        let collected = collect_documents(root, "demo");
        assert_eq!(collected.documents.len(), 1);
        assert_eq!(collected.documents[0].source, KnowledgeSource::Metadata);
        assert!(collected.assets.is_empty());
    }

    #[test]
    fn collect_walks_examples_benches_tests_docs() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_file(root, "examples/foo.rs", "fn main() {}\n");
        write_file(root, "examples/sub/deep.rs", "fn deep() {}\n");
        write_file(root, "benches/bench.rs", "fn bench() {}\n");
        write_file(root, "tests/integration.rs", "fn test() {}\n");
        write_file(root, "docs/guide.md", "# Guide\n");

        let collected = collect_documents(root, "demo");
        let documents = &collected.documents;

        // 5 walked files plus the always-emitted metadata document.
        assert_eq!(documents.len(), 6);

        let examples: Vec<&KnowledgeDocument> = documents
            .iter()
            .filter(|document| document.source == KnowledgeSource::Examples)
            .collect();
        assert_eq!(examples.len(), 2);
        assert!(
            examples
                .iter()
                .any(|document| document.path.as_path() == Path::new("examples/foo.rs"))
        );
        assert!(
            examples
                .iter()
                .any(|document| document.path.as_path() == Path::new("examples/sub/deep.rs"))
        );
        assert!(
            examples
                .iter()
                .all(|document| document.language.as_deref() == Some("rust"))
        );

        #[allow(clippy::unwrap_used)]
        let bench = documents
            .iter()
            .find(|document| document.source == KnowledgeSource::Benches)
            .unwrap();
        assert_eq!(bench.path, PathBuf::from("benches/bench.rs"));
        assert_eq!(bench.title, "bench");
        assert_eq!(bench.language.as_deref(), Some("rust"));

        #[allow(clippy::unwrap_used)]
        let test_doc = documents
            .iter()
            .find(|document| document.source == KnowledgeSource::Tests)
            .unwrap();
        assert_eq!(test_doc.path, PathBuf::from("tests/integration.rs"));
        assert_eq!(test_doc.title, "integration");

        #[allow(clippy::unwrap_used)]
        let guide = documents
            .iter()
            .find(|document| document.source == KnowledgeSource::Documentation)
            .unwrap();
        assert_eq!(guide.path, PathBuf::from("docs/guide.md"));
        assert_eq!(guide.language, None);
        assert_eq!(guide.mime_type, "text/markdown");

        assert!(find_by_source(documents, KnowledgeSource::Metadata).is_some());
    }

    #[test]
    fn collect_extracts_lib_module_docs() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_file(
            root,
            "src/lib.rs",
            "//! My crate docs.\n//! More docs.\n\npub fn f() {}\n",
        );

        let collected = collect_documents(root, "demo");
        let documents = &collected.documents;

        #[allow(clippy::unwrap_used)]
        let lib_docs = find_by_title(documents, "lib module docs").unwrap();
        assert_eq!(lib_docs.source, KnowledgeSource::Documentation);
        assert_eq!(lib_docs.path, PathBuf::from("src/lib.rs"));
        assert_eq!(lib_docs.language.as_deref(), Some("rust"));
        assert_eq!(lib_docs.mime_type, "text/x-rust");
        assert_eq!(lib_docs.tags, vec!["docs"]);
        assert_eq!(lib_docs.package, "demo");
        #[allow(clippy::unwrap_used)]
        let content = lib_docs.content.as_deref().unwrap();
        assert!(content.contains("My crate docs."));
        assert!(content.contains("More docs."));

        #[allow(clippy::unwrap_used)]
        let metadata = find_by_source(documents, KnowledgeSource::Metadata).unwrap();
        assert_eq!(
            metadata.content.as_deref(),
            Some("name: demo\nversion: unknown")
        );
    }

    #[test]
    fn collect_skips_lib_docs_when_absent() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_file(root, "src/lib.rs", "pub fn f() {}\n");

        let collected = collect_documents(root, "demo");
        assert!(
            !collected
                .documents
                .iter()
                .any(|document| document.title == "lib module docs")
        );
        assert_eq!(collected.documents.len(), 1);
        assert_eq!(collected.documents[0].source, KnowledgeSource::Metadata);
    }

    #[test]
    fn from_cargo_builds_package_id_from_parsed_manifest() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_file(
            root,
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"1.2.3\"\n",
        );

        #[allow(clippy::unwrap_used)]
        let package = crate::project::cargo::load(&root.join("Cargo.toml")).unwrap();
        let package_id = PackageId::from_cargo(&package);

        assert_eq!(package_id.kind, PackageKind::Cargo);
        assert_eq!(package_id.name, "demo");
        assert_eq!(package_id.version.as_deref(), Some("1.2.3"));
        assert_eq!(package_id.root, root.to_path_buf());
        let manifest_path = root.join("Cargo.toml");
        #[allow(clippy::unwrap_used)]
        let manifest_parent = manifest_path.parent().unwrap();
        assert_eq!(package_id.root, manifest_parent);
    }

    #[test]
    fn collect_errors_on_missing_root() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let missing_root = dir.path().join("nope");
        let package = PackageId::new(PackageKind::Cargo, "demo", &missing_root);
        assert!(CargoProvider.collect(&package).is_err());
    }
}
