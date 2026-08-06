//! Knowledge providers.
//!
//! Providers collect knowledge documents for packages. Only Cargo and Local
//! are functional; the rest are planned skeletons.

pub mod cargo;
pub mod github;
pub mod local;
pub mod man;
pub mod npm;
pub mod registry;
pub mod traits;
pub mod types;
pub mod zeal;

pub use registry::Registry;
pub use traits::Provider;
pub use types::{
    CollectedDocs, KnowledgeAsset, KnowledgeDocument, KnowledgeSource, PackageId, PackageKind,
    ProviderCapabilities, ProviderMetadata, ProviderStatus,
};
