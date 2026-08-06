//! Integration tests for the `providers` module (Phase 2, milestone M2.2).
//!
//! Exercises the public surface: registry, selection, dispatch, the cargo and
//! local providers end-to-end, metadata generation, and capability/status
//! declarations. Scenarios that unit tests already cover exhaustively are not
//! re-litigated here.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use dkv::providers::cargo::CargoProvider;
use dkv::providers::github::GithubProvider;
use dkv::providers::local::LocalProvider;
use dkv::providers::npm::NpmProvider;
use dkv::providers::types::{
    KnowledgeDocument, KnowledgeSource, PackageId, PackageKind, ProviderStatus, fingerprint, fnv1a,
    mime_type_for,
};
use dkv::providers::{Provider, Registry};

mod common;
use common::fixture;

/// Writes `contents` to `root.join(relative)`, creating parent directories.
fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        #[allow(clippy::unwrap_used)]
        std::fs::create_dir_all(parent).unwrap();
    }
    #[allow(clippy::unwrap_used)]
    std::fs::write(path, contents).unwrap();
}

/// Finds a document by source in a slice.
fn find_by_source(
    documents: &[KnowledgeDocument],
    source: KnowledgeSource,
) -> Option<&KnowledgeDocument> {
    documents.iter().find(|document| document.source == source)
}

// ---------------------------------------------------------------------------
// Provider registry
// ---------------------------------------------------------------------------

#[test]
fn builtin_registers_six_providers_in_order() {
    let registry = Registry::builtin();

    let ids: Vec<&str> = registry
        .providers()
        .iter()
        .map(|provider| provider.id())
        .collect();
    assert_eq!(ids, vec!["cargo", "local", "github", "npm", "zeal", "man"]);

    let statuses: Vec<ProviderStatus> = registry
        .providers()
        .iter()
        .map(|provider| provider.metadata().status)
        .collect();
    assert_eq!(
        statuses,
        vec![
            ProviderStatus::Active,
            ProviderStatus::Active,
            ProviderStatus::Planned,
            ProviderStatus::Planned,
            ProviderStatus::Planned,
            ProviderStatus::Planned,
        ]
    );
}

#[test]
fn find_returns_provider_by_id() {
    let registry = Registry::builtin();

    #[allow(clippy::unwrap_used)]
    let cargo = registry.find("cargo").unwrap();
    assert_eq!(cargo.id(), "cargo");

    assert!(registry.find("nope").is_none());

    #[allow(clippy::unwrap_used)]
    let github = registry.find("github").unwrap();
    assert_eq!(github.metadata().status, ProviderStatus::Planned);
}

#[test]
fn register_adds_a_custom_provider() {
    let mut registry = Registry::default();
    assert!(registry.providers().is_empty());

    registry.register(Box::new(LocalProvider));
    assert_eq!(registry.providers().len(), 1);
    assert_eq!(registry.providers()[0].id(), "local");
}

// ---------------------------------------------------------------------------
// Provider selection and dispatch
// ---------------------------------------------------------------------------

#[test]
fn for_package_selects_supporting_providers() {
    #[allow(clippy::unwrap_used)]
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let registry = Registry::builtin();

    let cargo_package = PackageId::new(PackageKind::Cargo, "demo", root);
    let ids: Vec<&str> = registry
        .for_package(&cargo_package)
        .iter()
        .map(|provider| provider.id())
        .collect();
    assert_eq!(ids, vec!["cargo"]);

    let local_package = PackageId::new(PackageKind::Local, "demo", root);
    let ids: Vec<&str> = registry
        .for_package(&local_package)
        .iter()
        .map(|provider| provider.id())
        .collect();
    assert_eq!(ids, vec!["local"]);

    // npm is planned (`supports` is false), so Node packages select nobody.
    let node_package = PackageId::new(PackageKind::Node, "demo", root);
    assert!(registry.for_package(&node_package).is_empty());
}

#[test]
fn collect_all_dispatches_to_supporting_providers() {
    #[allow(clippy::unwrap_used)]
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let registry = Registry::builtin();

    let cargo_package = PackageId::new(PackageKind::Cargo, "demo", root);
    #[allow(clippy::unwrap_used)]
    let cargo_results = registry.collect_all(&cargo_package).unwrap();
    assert_eq!(cargo_results.len(), 1);
    assert_eq!(cargo_results[0].provider, "cargo");
    assert!(
        cargo_results[0]
            .documents
            .iter()
            .any(|document| document.source == KnowledgeSource::Metadata)
    );

    let local_package = PackageId::new(PackageKind::Local, "demo", root);
    #[allow(clippy::unwrap_used)]
    let local_results = registry.collect_all(&local_package).unwrap();
    assert_eq!(local_results.len(), 1);
    assert_eq!(local_results[0].provider, "local");
    assert!(local_results[0].documents.is_empty());

    let node_package = PackageId::new(PackageKind::Node, "demo", root);
    #[allow(clippy::unwrap_used)]
    let node_results = registry.collect_all(&node_package).unwrap();
    assert!(node_results.is_empty());
}

#[test]
fn collect_from_errors_on_unknown_provider() {
    #[allow(clippy::unwrap_used)]
    let dir = tempfile::tempdir().unwrap();
    let package = PackageId::new(PackageKind::Cargo, "demo", dir.path());

    let registry = Registry::builtin();
    let error = registry.collect_from("nope", &package);
    assert!(error.is_err());
    #[allow(clippy::unwrap_used)]
    let message = error.unwrap_err().to_string();
    assert!(message.contains("no provider registered"));
}

// ---------------------------------------------------------------------------
// Cargo provider (end-to-end)
// ---------------------------------------------------------------------------

#[test]
fn cargo_provider_collects_package_knowledge_end_to_end() {
    #[allow(clippy::unwrap_used)]
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write_file(
        root,
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"2.0.0\"\n",
    );
    write_file(root, "README.md", "# Demo\n");
    write_file(root, "CHANGELOG.md", "## 2.0.0\n");
    write_file(root, "LICENSE", "MIT\n");
    write_file(root, "examples/foo.rs", "fn main() {}\n");
    write_file(root, "src/lib.rs", "//! Crate docs.\n\npub fn f() {}\n");

    #[allow(clippy::unwrap_used)]
    let package = dkv::project::cargo::load(&root.join("Cargo.toml")).unwrap();
    let package_id = PackageId::from_cargo(&package);

    #[allow(clippy::unwrap_used)]
    let collected = CargoProvider.collect(&package_id).unwrap();
    let documents = &collected.documents;

    assert_eq!(collected.provider, "cargo");
    assert_eq!(documents.len(), 6);

    // Documents are deterministic and sorted by path.
    let paths: Vec<String> = documents
        .iter()
        .map(|document| document.path.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        paths,
        vec![
            "CHANGELOG.md",
            "Cargo.toml",
            "LICENSE",
            "README.md",
            "examples/foo.rs",
            "src/lib.rs",
        ]
    );

    #[allow(clippy::unwrap_used)]
    let readme = find_by_source(documents, KnowledgeSource::Readme).unwrap();
    assert_eq!(readme.path, PathBuf::from("README.md"));
    assert_eq!(readme.mime_type, "text/markdown");
    assert!(readme.content.is_none());

    #[allow(clippy::unwrap_used)]
    let changelog = find_by_source(documents, KnowledgeSource::Changelog).unwrap();
    assert_eq!(changelog.path, PathBuf::from("CHANGELOG.md"));

    #[allow(clippy::unwrap_used)]
    let license = find_by_source(documents, KnowledgeSource::License).unwrap();
    assert_eq!(license.path, PathBuf::from("LICENSE"));
    assert_eq!(license.mime_type, "text/plain");
    assert!(license.content.is_none());

    #[allow(clippy::unwrap_used)]
    let example = find_by_source(documents, KnowledgeSource::Examples).unwrap();
    assert_eq!(example.path, PathBuf::from("examples/foo.rs"));
    assert_eq!(example.language.as_deref(), Some("rust"));

    #[allow(clippy::unwrap_used)]
    let lib_docs = find_by_source(documents, KnowledgeSource::Documentation).unwrap();
    assert_eq!(lib_docs.path, PathBuf::from("src/lib.rs"));
    assert_eq!(lib_docs.language.as_deref(), Some("rust"));
    assert_eq!(lib_docs.mime_type, "text/x-rust");
    #[allow(clippy::unwrap_used)]
    let content = lib_docs.content.as_deref().unwrap();
    assert!(content.contains("Crate docs."));

    // The metadata document is always emitted, even when derived from the
    // parsed manifest rather than the manifest file itself.
    #[allow(clippy::unwrap_used)]
    let metadata = find_by_source(documents, KnowledgeSource::Metadata).unwrap();
    assert_eq!(metadata.title, "demo metadata");
    assert_eq!(metadata.path, PathBuf::from("Cargo.toml"));
    assert_eq!(metadata.mime_type, "application/toml");
    assert_eq!(
        metadata.content.as_deref(),
        Some("name: demo\nversion: 2.0.0")
    );

    for document in documents {
        assert_eq!(document.checksum.len(), 16);
        assert!(document.checksum.chars().all(|character| {
            character.is_ascii_hexdigit()
                && (character.is_ascii_lowercase() || character.is_ascii_digit())
        }));
    }
}

#[test]
fn cargo_provider_reuses_scanner_fixture() {
    let fixture_root = fixture("single-cargo");

    #[allow(clippy::unwrap_used)]
    let package = dkv::project::cargo::load(&fixture_root.join("Cargo.toml")).unwrap();
    let package_id = PackageId::from_cargo(&package);

    #[allow(clippy::unwrap_used)]
    let collected = CargoProvider.collect(&package_id).unwrap();
    assert_eq!(collected.provider, "cargo");

    #[allow(clippy::unwrap_used)]
    let metadata = collected
        .documents
        .iter()
        .find(|document| document.source == KnowledgeSource::Metadata)
        .unwrap();
    assert_eq!(metadata.package, package.name);
}

// ---------------------------------------------------------------------------
// Local provider
// ---------------------------------------------------------------------------

#[test]
fn local_provider_collects_supported_documents() {
    #[allow(clippy::unwrap_used)]
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    for name in [
        "a.md",
        "b.adoc",
        "c.txt",
        "d.rst",
        "e.html",
        "f.htm",
        "g.pdf",
        "h.unknown",
    ] {
        write_file(root, name, "content");
    }
    write_file(root, ".hidden/x.md", "x");
    write_file(root, ".git/config", "config");

    let package = PackageId::new(PackageKind::Local, "docs", root);
    #[allow(clippy::unwrap_used)]
    let collected = LocalProvider.collect(&package).unwrap();
    assert_eq!(collected.provider, "local");
    assert_eq!(collected.documents.len(), 7);

    // Exactly the 7 supported files, sorted by relative path; hidden entries
    // and unknown extensions are skipped.
    let paths: Vec<String> = collected
        .documents
        .iter()
        .map(|document| document.path.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        paths,
        vec![
            "a.md", "b.adoc", "c.txt", "d.rst", "e.html", "f.htm", "g.pdf"
        ]
    );

    let mimes: Vec<&str> = collected
        .documents
        .iter()
        .map(|document| document.mime_type.as_str())
        .collect();
    assert_eq!(
        mimes,
        vec![
            "text/markdown",
            "text/asciidoc",
            "text/plain",
            "text/x-rst",
            "text/html",
            "text/html",
            "application/pdf",
        ]
    );

    assert!(
        collected
            .documents
            .iter()
            .all(|document| document.content.is_none())
    );
    assert!(
        collected
            .documents
            .iter()
            .all(|document| document.package == "docs")
    );
    assert!(
        collected
            .documents
            .iter()
            .all(|document| !document.path.is_absolute())
    );
}

#[test]
fn local_provider_empty_directory_returns_no_documents() {
    #[allow(clippy::unwrap_used)]
    let dir = tempfile::tempdir().unwrap();
    let package = PackageId::new(PackageKind::Local, "empty", dir.path());

    #[allow(clippy::unwrap_used)]
    let collected = LocalProvider.collect(&package).unwrap();
    assert_eq!(collected.provider, "local");
    assert!(collected.documents.is_empty());
}

// ---------------------------------------------------------------------------
// Utility functions: fingerprint and mime types
// ---------------------------------------------------------------------------

#[test]
fn fingerprint_is_deterministic_and_sensitive() {
    let path = Path::new("src/lib.rs");
    let time = SystemTime::UNIX_EPOCH;

    let first = fingerprint(path, 42, &time);
    let second = fingerprint(path, 42, &time);
    assert_eq!(first, second);
    assert_eq!(first.len(), 16);

    let different_size = fingerprint(path, 43, &time);
    assert_ne!(first, different_size);

    assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a(b"abc"), fnv1a(b"abc"));
}

#[test]
fn mime_type_for_known_extensions() {
    assert_eq!(
        mime_type_for(Path::new("README.md")),
        Some("text/markdown".to_string())
    );
    assert_eq!(
        mime_type_for(Path::new("guide.markdown")),
        Some("text/markdown".to_string())
    );
    assert_eq!(
        mime_type_for(Path::new("guide.adoc")),
        Some("text/asciidoc".to_string())
    );
    assert_eq!(
        mime_type_for(Path::new("notes.txt")),
        Some("text/plain".to_string())
    );
    assert_eq!(
        mime_type_for(Path::new("index.rst")),
        Some("text/x-rst".to_string())
    );
    assert_eq!(
        mime_type_for(Path::new("page.html")),
        Some("text/html".to_string())
    );
    assert_eq!(
        mime_type_for(Path::new("page.htm")),
        Some("text/html".to_string())
    );
    assert_eq!(
        mime_type_for(Path::new("manual.pdf")),
        Some("application/pdf".to_string())
    );
    assert_eq!(
        mime_type_for(Path::new("lib.rs")),
        Some("text/x-rust".to_string())
    );
    assert_eq!(
        mime_type_for(Path::new("Cargo.toml")),
        Some("application/toml".to_string())
    );

    // Extension matching is case-insensitive.
    assert_eq!(
        mime_type_for(Path::new("README.MD")),
        Some("text/markdown".to_string())
    );

    assert_eq!(mime_type_for(Path::new("image.png")), None);
    assert_eq!(mime_type_for(Path::new("no_extension")), None);
}

// ---------------------------------------------------------------------------
// Capability and status declarations
// ---------------------------------------------------------------------------

#[test]
fn cargo_capabilities_declared() {
    let capabilities = CargoProvider.capabilities();

    assert!(capabilities.offline);
    assert_eq!(capabilities.package_kinds, vec![PackageKind::Cargo]);
    assert_eq!(
        capabilities.sources,
        vec![
            KnowledgeSource::Readme,
            KnowledgeSource::Changelog,
            KnowledgeSource::License,
            KnowledgeSource::Documentation,
            KnowledgeSource::Examples,
            KnowledgeSource::Tests,
            KnowledgeSource::Benches,
            KnowledgeSource::Metadata,
        ]
    );
    assert!(capabilities.reads_content);

    assert_eq!(CargoProvider.metadata().status, ProviderStatus::Active);
}

#[test]
fn local_capabilities_declared() {
    let capabilities = LocalProvider.capabilities();

    assert!(capabilities.offline);
    assert_eq!(capabilities.package_kinds, vec![PackageKind::Local]);
    assert_eq!(capabilities.sources, vec![KnowledgeSource::Documentation]);
    assert!(!capabilities.reads_content);

    assert_eq!(LocalProvider.metadata().status, ProviderStatus::Active);
}

#[test]
fn planned_providers_report_planned_status_and_do_not_support() {
    let npm = NpmProvider;
    assert_eq!(npm.metadata().status, ProviderStatus::Planned);
    assert!(!npm.capabilities().offline);
    assert!(!npm.capabilities().reads_content);

    #[allow(clippy::unwrap_used)]
    let dir = tempfile::tempdir().unwrap();
    let node_package = PackageId::new(PackageKind::Node, "demo", dir.path());
    assert!(!npm.supports(&node_package));

    let github = GithubProvider;
    assert_eq!(github.metadata().status, ProviderStatus::Planned);
    assert!(!github.capabilities().offline);
}
