//! Ingestion pipeline: normalize, deduplicate, checksum, persist.
//!
//! Converts provider [`CollectedDocs`]
//! into [`PendingBatch`]es, reusing provider checksums and
//! computing content checksums with
//! [`crate::storage::content_checksum`].

use crate::providers::types::CollectedDocs;
use crate::storage::checksum::content_checksum;
use crate::storage::metadata::{PendingArtifact, PendingBatch, PendingDocument};

/// Normalizes and deduplicates a slice of [`CollectedDocs`] into a [`PendingBatch`].
#[must_use]
pub fn prepare_batch(collected_list: &[CollectedDocs]) -> PendingBatch {
    let mut documents = Vec::new();
    let mut artifacts = Vec::new();

    let mut seen_docs = std::collections::HashSet::new();

    for collected in collected_list {
        for doc in &collected.documents {
            let content_checksum = doc.content.as_ref().map(|c| content_checksum(c.as_bytes()));
            let vault_path = content_checksum
                .as_ref()
                .map(|sha| format!("documents/{sha}"));

            // Deduplication key: (package name, relative_path, metadata_checksum)
            let dedup_key = (doc.package.clone(), doc.path.clone(), doc.checksum.clone());
            if !seen_docs.insert(dedup_key) {
                continue;
            }

            let package = crate::storage::metadata::PackageKey {
                kind: "cargo".to_string(), // Default or inferred, will be refined in project ingest
                name: doc.package.clone(),
                version: None,
                root: std::path::PathBuf::from("."),
            };

            documents.push(PendingDocument {
                package,
                provider: collected.provider.clone(),
                source: doc.source,
                title: doc.title.clone(),
                relative_path: doc.path.clone(),
                fs_path: None,
                mime_type: doc.mime_type.clone(),
                language: doc.language.clone(),
                size: doc.size,
                metadata_checksum: doc.checksum.clone(),
                modified: doc.modified,
                tags: doc.tags.clone(),
                content: doc.content.clone(),
                content_checksum,
                vault_path,
                origin_url: None,
            });
        }

        for asset in &collected.assets {
            let package = crate::storage::metadata::PackageKey {
                kind: "cargo".to_string(),
                name: asset.package.clone(),
                version: None,
                root: std::path::PathBuf::from("."),
            };

            artifacts.push(PendingArtifact {
                package,
                provider: collected.provider.clone(),
                name: asset.name.clone(),
                source: asset.source,
                path: asset.path.clone(),
                fs_path: None,
                mime_type: asset.mime_type.clone(),
                size: asset.size,
                checksum: asset.checksum.clone(),
                modified: asset.modified,
                tags: asset.tags.clone(),
            });
        }
    }

    PendingBatch {
        documents,
        artifacts,
    }
}
