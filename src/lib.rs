//! Dev Knowledge Vault (DKV)
//!
//! Library entry point.

pub mod commands;
pub mod config;
pub mod error;
pub mod project;
pub mod scanner;
pub mod types;

pub use error::{DkvError, Result};

/// Current application version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name.
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
