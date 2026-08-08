//! Dev Knowledge Vault (DKV)
//!
//! Library entry point.

pub mod commands;
pub mod config;
pub mod error;
pub mod graph;
pub mod project;
pub mod providers;
pub mod scanner;
pub mod storage;
pub mod types;
pub mod util;

pub use error::{DkvError, Result};

/// Current application version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name.
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
