//! GitHub provider.
//!
//! Collects repository documentation from GitHub. This skeleton is planned for
//! a later milestone and cannot collect yet.

use crate::error::Result;
use crate::providers::traits::Provider;
use crate::providers::types::{
    CollectedDocs, PackageId, PackageKind, ProviderCapabilities, ProviderMetadata, ProviderStatus,
};

/// Collects repository documentation from GitHub.
pub struct GithubProvider;

impl Provider for GithubProvider {
    fn id(&self) -> &'static str {
        "github"
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: self.id(),
            name: "GitHub",
            description: "Collects repository documentation from GitHub (planned)",
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
            package_kinds: vec![PackageKind::Other],
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
