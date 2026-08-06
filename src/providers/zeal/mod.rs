//! Zeal provider.
//!
//! Collects Zeal docset documentation. This skeleton is planned for a later
//! milestone and cannot collect yet.

use crate::error::Result;
use crate::providers::traits::Provider;
use crate::providers::types::{
    CollectedDocs, PackageId, ProviderCapabilities, ProviderMetadata, ProviderStatus,
};

/// Collects Zeal docset documentation.
pub struct ZealProvider;

impl Provider for ZealProvider {
    fn id(&self) -> &'static str {
        "zeal"
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: self.id(),
            name: "Zeal",
            description: "Collects Zeal docset documentation (planned)",
            status: ProviderStatus::Planned,
        }
    }

    fn supports(&self, _package: &PackageId) -> bool {
        // Planned for a later milestone — cannot collect yet.
        false
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            offline: true,
            package_kinds: Vec::new(),
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
