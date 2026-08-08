//! The vault: layout plus backend behind one handle.
//!
//! [`crate::storage::VaultLayout`] describes the directory layout;
//! `vault` binds a layout to a concrete
//! [`crate::storage::StorageBackend`] (`SQLite`) behind a single handle.
//! Implemented in a later milestone.
