//! `SQLite` storage backend.
//!
//! Implements [`crate::storage::StorageBackend`] over `SQLite`
//! (`rusqlite`): schema migration via `PRAGMA user_version`, transactional
//! ingest, counts, storage size, and content verification. Implemented in a
//! later milestone.
