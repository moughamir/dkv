//! The common [`Provider`] trait implemented by every knowledge provider.

use crate::error::Result;
use crate::providers::types::{CollectedDocs, PackageId, ProviderCapabilities, ProviderMetadata};

/// Common interface implemented by every knowledge provider.
pub trait Provider {
    /// Stable unique identifier (e.g. "cargo").
    fn id(&self) -> &'static str;

    /// Display metadata for registry/CLI listing.
    fn metadata(&self) -> ProviderMetadata;

    /// Whether this provider can process the package.
    fn supports(&self, package: &PackageId) -> bool;

    /// Declared capabilities.
    fn capabilities(&self) -> ProviderCapabilities;

    /// Collect knowledge documents for the package.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::DkvError`] when the package root cannot be read.
    fn collect(&self, package: &PackageId) -> Result<CollectedDocs>;
}
