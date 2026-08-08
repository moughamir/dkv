//! Ingestion pipeline: normalize, deduplicate, checksum, persist.
//!
//! Converts provider [`CollectedDocs`](crate::providers::types::CollectedDocs)
//! into [`crate::storage::PendingBatch`]es, reusing provider checksums and
//! computing content checksums with
//! [`crate::storage::content_checksum`]. Implemented in a later milestone.
