//! Error types for the storage layer.

/// Error type for the storage layer.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    #[error("database migration failed: {0}")]
    Migration(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid vault layout: {0}")]
    InvalidLayout(String),

    #[error("{0}")]
    Message(String),
}

/// Result alias for the storage layer.
pub type Result<T> = std::result::Result<T, StorageError>;
