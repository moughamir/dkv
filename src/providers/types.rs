//! Shared data structures and utilities for knowledge providers.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Kind of package a knowledge provider can operate on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageKind {
    /// Cargo/Rust package.
    Cargo,
    /// Node.js package.
    Node,
    /// Arbitrary local directory (no ecosystem).
    Local,
    /// Anything else.
    Other,
}

/// Identifies a package independently of any provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    pub kind: PackageKind,
    pub name: String,
    pub version: Option<String>,
    /// Filesystem root of the package.
    pub root: PathBuf,
}

impl PackageId {
    /// Creates a new package id without a version.
    #[must_use]
    pub fn new(kind: PackageKind, name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            name: name.into(),
            version: None,
            root: root.into(),
        }
    }

    /// Sets the version (builder-style).
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
}

/// Classification of a knowledge document's origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnowledgeSource {
    Readme,
    Changelog,
    License,
    /// Docs directory, module docs, guides.
    Documentation,
    Examples,
    Tests,
    Benches,
    /// Package manifest/metadata summary.
    Metadata,
    Other,
}

impl KnowledgeSource {
    /// Lowercase tag string used in `tags` (e.g. `Readme` -> "readme",
    /// `Documentation` -> "docs").
    #[must_use]
    pub const fn as_tag(self) -> &'static str {
        match self {
            Self::Readme => "readme",
            Self::Changelog => "changelog",
            Self::License => "license",
            Self::Documentation => "docs",
            Self::Examples => "examples",
            Self::Tests => "tests",
            Self::Benches => "benches",
            Self::Metadata => "metadata",
            Self::Other => "other",
        }
    }
}

/// A text-oriented knowledge document: metadata plus optional extracted content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeDocument {
    pub title: String,
    pub source: KnowledgeSource,
    /// Path relative to the package root.
    pub path: PathBuf,
    /// Owning package name.
    pub package: String,
    /// Content language when meaningful (e.g. "rust"), else `None`.
    pub language: Option<String>,
    pub mime_type: String,
    pub size: u64,
    /// Hex FNV-1a 64-bit fingerprint; see `fingerprint()`.
    pub checksum: String,
    pub modified: SystemTime,
    pub tags: Vec<String>,
    /// Present only when the provider extracted content (e.g. lib.rs module docs).
    pub content: Option<String>,
}

/// A non-text asset collected as metadata only (future: images, binaries, docsets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeAsset {
    pub name: String,
    pub source: KnowledgeSource,
    /// Path relative to the package root.
    pub path: PathBuf,
    pub package: String,
    pub mime_type: String,
    pub size: u64,
    pub checksum: String,
    pub modified: SystemTime,
    pub tags: Vec<String>,
}

/// Result of one provider collection run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CollectedDocs {
    /// Provider id that produced these docs.
    pub provider: String,
    pub documents: Vec<KnowledgeDocument>,
    pub assets: Vec<KnowledgeAsset>,
}

/// What a provider can do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCapabilities {
    /// Works without network access.
    pub offline: bool,
    /// Package kinds the provider can process.
    pub package_kinds: Vec<PackageKind>,
    /// Document sources the provider can produce.
    pub sources: Vec<KnowledgeSource>,
    /// Whether the provider extracts content or is metadata-only.
    pub reads_content: bool,
}

/// Lifecycle status shown by `dkv providers`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderStatus {
    Active,
    Planned,
}

/// Display metadata for the registry/CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderMetadata {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub status: ProviderStatus,
}

/// FNV-1a 64-bit hash (std-only; deterministic across runs).
#[must_use]
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Lowercase hex fingerprint used for `checksum` fields.
///
/// Content-based when content was read; otherwise a metadata fingerprint over
/// the relative path, size, and last-modified nanos (deterministic, no file read).
#[must_use]
pub fn fingerprint(relative_path: &Path, size: u64, modified: &SystemTime) -> String {
    let mut buffer = Vec::with_capacity(relative_path.as_os_str().len() + 16);
    buffer.extend_from_slice(relative_path.as_os_str().as_encoded_bytes());
    buffer.extend_from_slice(&size.to_le_bytes());

    let nanos = modified
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let nanos = u64::try_from(nanos).unwrap_or(u64::MAX);
    buffer.extend_from_slice(&nanos.to_le_bytes());

    format!("{:016x}", fnv1a(&buffer))
}

/// MIME type for a file path by extension, or `None` for unknown.
///
/// Map: md/markdown -> "text/markdown", adoc -> "text/asciidoc", txt -> "text/plain",
/// rst -> "text/x-rst", html/htm -> "text/html", pdf -> "application/pdf", rs -> "text/x-rust",
/// toml -> "application/toml". Extension matching is lowercase and case-insensitive.
#[must_use]
pub fn mime_type_for(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let mime = match extension.as_str() {
        "md" | "markdown" => "text/markdown",
        "adoc" => "text/asciidoc",
        "txt" => "text/plain",
        "rst" => "text/x-rst",
        "html" | "htm" => "text/html",
        "pdf" => "application/pdf",
        "rs" => "text/x-rust",
        "toml" => "application/toml",
        _ => return None,
    };
    Some(mime.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::SystemTime;

    use super::*;

    #[test]
    fn fnv1a_is_stable() {
        let input = b"cargo demo package";
        assert_eq!(fnv1a(input), fnv1a(input));
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
    }

    #[test]
    fn fingerprint_is_stable_and_sensitive() {
        let path = Path::new("src/lib.rs");
        let time = SystemTime::UNIX_EPOCH;

        let first = fingerprint(path, 42, &time);
        let second = fingerprint(path, 42, &time);
        assert_eq!(first, second);
        assert_eq!(first.len(), 16);

        let different_size = fingerprint(path, 43, &time);
        assert_ne!(first, different_size);
    }

    #[test]
    fn mime_type_for_maps_known_extensions() {
        assert_eq!(
            mime_type_for(Path::new("README.md")),
            Some("text/markdown".to_string())
        );
        assert_eq!(
            mime_type_for(Path::new("CHANGELOG.markdown")),
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
        assert_eq!(
            mime_type_for(Path::new("README.MD")),
            Some("text/markdown".to_string())
        );

        assert_eq!(mime_type_for(Path::new("image.png")), None);
        assert_eq!(mime_type_for(Path::new("no_extension")), None);
    }

    #[test]
    fn source_tag_is_lowercase() {
        assert_eq!(KnowledgeSource::Readme.as_tag(), "readme");
        assert_eq!(KnowledgeSource::Documentation.as_tag(), "docs");
        assert_eq!(KnowledgeSource::Metadata.as_tag(), "metadata");
        assert_eq!(KnowledgeSource::Other.as_tag(), "other");
    }
}
