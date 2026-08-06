//! npm provider.
//!
//! Collects npm package documentation. This skeleton is planned for a later
//! milestone and cannot collect yet.

use crate::error::Result;
use crate::providers::traits::Provider;
use crate::providers::types::{
    CollectedDocs, PackageId, PackageKind, ProviderCapabilities, ProviderMetadata, ProviderStatus,
};

/// Collects npm package documentation.
pub struct NpmProvider;

impl Provider for NpmProvider {
    fn id(&self) -> &'static str {
        "npm"
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: self.id(),
            name: "npm",
            description: "Collects npm package documentation (planned)",
            status: ProviderStatus::Planned,
        }
    }

    fn supports(&self, _package: &PackageId) -> bool {
        // Planned for a later milestone — cannot collect yet.
        false
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            offline: false,
            package_kinds: vec![PackageKind::Node],
            sources: Vec::new(),
            reads_content: false,
        }
    }

    fn collect(&self, _package: &PackageId) -> Result<CollectedDocs> {
        Ok(CollectedDocs {
            provider: self.id().to_string(),
            ..CollectedDocs::default()
        })
    }
}
