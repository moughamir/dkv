//! Deterministic content hashing for stored documents.
//!
//! Content checksums are computed here as `SHA-256` lowercase hex. Metadata
//! checksums are never recomputed in this module: they are reused from the
//! provider-supplied FNV-1a fingerprint
//! ([`KnowledgeDocument::checksum`](crate::providers::types::KnowledgeDocument)),
//! per the milestone spec ("reuse existing checksums").

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::storage::errors::Result;

/// Which kind of checksum a value represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumKind {
    /// Metadata fingerprint (FNV-1a), provider-supplied and reused.
    Metadata,
    /// `SHA-256` of the document content.
    Content,
}

/// `SHA-256` lowercase hex checksum of `bytes`.
///
/// Used for document content checksums, e.g. `content_checksum(b"")` is the
/// well-known empty-input digest. Metadata checksums are *not* computed here;
/// they are reused from the provider-supplied FNV-1a fingerprint
/// ([`KnowledgeDocument::checksum`](crate::providers::types::KnowledgeDocument)).
#[must_use]
pub fn content_checksum(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_encode(&hasher.finalize())
}

/// `SHA-256` lowercase hex checksum of the file at `path`, streamed in chunks.
///
/// The file is read through a buffered reader in fixed-size chunks so that
/// arbitrarily large files are never held entirely in memory.
///
/// # Errors
///
/// Returns [`crate::storage::StorageError::Io`] if the file cannot be
/// opened or read.
pub fn content_checksum_file(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_encode(&hasher.finalize()))
}

/// Encodes raw digest bytes as lowercase hex.
///
/// `sha2`'s digest output type does not implement `LowerHex` in the resolved
/// `generic-array` version, so the encoding is done explicitly here.
#[must_use]
fn hex_encode(bytes: &[u8]) -> String {
    const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn content_checksum_matches_known_sha256_vectors() {
        assert_eq!(
            content_checksum(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            content_checksum(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn content_checksum_file_matches_content_checksum() {
        #[allow(clippy::unwrap_used)]
        let dir = tempdir().unwrap();
        let path = dir.path().join("sample.txt");
        #[allow(clippy::unwrap_used)]
        fs::write(&path, b"hello vault").unwrap();
        let expected = content_checksum(b"hello vault");
        #[allow(clippy::unwrap_used)]
        let actual = content_checksum_file(&path).unwrap();
        assert_eq!(actual, expected);
    }
}
