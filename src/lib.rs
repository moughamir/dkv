//! Dev Knowledge Vault (DKV)
//!
//! Library entry point.

pub mod error;

pub use error::{DkvError, Result};

/// Current application version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name.
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
