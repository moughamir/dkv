//! Man provider.
//!
//! Collects man page documentation. This skeleton is planned for a later
//! milestone and cannot collect yet.

use crate::error::Result;
use crate::providers::traits::Provider;
use crate::providers::types::{
    CollectedDocs, PackageId, ProviderCapabilities, ProviderMetadata, ProviderStatus,
};

/// Collects man page documentation.
pub struct ManProvider;

impl Provider for ManProvider {
    fn id(&self) -> &'static str {
        "man"
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: self.id(),
            name: "Man",
            description: "Collects man page documentation (planned)",
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
