//! Provider registry and dispatch.

use crate::error::{DkvError, Result};
use crate::providers::cargo::CargoProvider;
use crate::providers::github::GithubProvider;
use crate::providers::local::LocalProvider;
use crate::providers::man::ManProvider;
use crate::providers::npm::NpmProvider;
use crate::providers::traits::Provider;
use crate::providers::types::{CollectedDocs, PackageId};
use crate::providers::zeal::ZealProvider;

/// Holds registered providers and dispatches collection.
#[derive(Default)]
pub struct Registry {
    providers: Vec<Box<dyn Provider>>,
}

impl Registry {
    /// Registry with all built-in providers registered, in this exact order:
    /// cargo, local, github, npm, zeal, man (matches the `dkv providers` CLI output).
    #[must_use]
    pub fn builtin() -> Self {
        let mut registry = Self::default();
        registry.register(Box::new(CargoProvider));
        registry.register(Box::new(LocalProvider));
        registry.register(Box::new(GithubProvider));
        registry.register(Box::new(NpmProvider));
        registry.register(Box::new(ZealProvider));
        registry.register(Box::new(ManProvider));
        registry
    }

    /// Registers a provider. Returns `&mut self` for chaining.
    pub fn register(&mut self, provider: Box<dyn Provider>) -> &mut Self {
        self.providers.push(provider);
        self
    }

    /// All registered providers, in registration order.
    #[must_use]
    pub fn providers(&self) -> &[Box<dyn Provider>] {
        &self.providers
    }

    /// Finds a provider by id.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&dyn Provider> {
        for provider in &self.providers {
            if provider.id() == id {
                return Some(provider.as_ref());
            }
        }
        None
    }

    /// Providers that `supports()` the package (selection).
    #[must_use]
    pub fn for_package(&self, package: &PackageId) -> Vec<&dyn Provider> {
        let mut selected = Vec::new();
        for provider in &self.providers {
            if provider.supports(package) {
                selected.push(provider.as_ref());
            }
        }
        selected
    }

    /// Collects from every provider that supports the package, returning one
    /// `CollectedDocs` per supporting provider (empty if none).
    ///
    /// # Errors
    ///
    /// Returns the first provider error encountered; errors are NOT swallowed.
    pub fn collect_all(&self, package: &PackageId) -> Result<Vec<CollectedDocs>> {
        self.for_package(package)
            .into_iter()
            .map(|provider| provider.collect(package))
            .collect()
    }

    /// Collects from a single provider by id.
    ///
    /// # Errors
    ///
    /// Returns [`DkvError::Message`] if no provider has that id, or the
    /// provider's own error.
    pub fn collect_from(&self, id: &str, package: &PackageId) -> Result<CollectedDocs> {
        let provider = self
            .find(id)
            .ok_or_else(|| DkvError::Message(format!("no provider registered: {id}")))?;
        provider.collect(package)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::types::{KnowledgeSource, PackageKind};

    #[test]
    fn builtin_registers_six_in_order() {
        let registry = Registry::builtin();
        let ids: Vec<&str> = registry
            .providers()
            .iter()
            .map(|provider| provider.id())
            .collect();
        assert_eq!(ids, vec!["cargo", "local", "github", "npm", "zeal", "man"]);
    }

    #[test]
    fn find_returns_provider_and_none_for_unknown() {
        let registry = Registry::builtin();
        assert!(registry.find("cargo").is_some());
        assert!(registry.find("nope").is_none());
    }

    #[test]
    fn register_adds_custom_provider() {
        let mut registry = Registry::default();
        registry.register(Box::new(CargoProvider));
        assert_eq!(registry.providers().len(), 1);
    }

    #[test]
    fn for_package_selects_cargo_only_for_cargo_kind() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let registry = Registry::builtin();

        let cargo_package = PackageId::new(PackageKind::Cargo, "demo", root);
        let providers = registry.for_package(&cargo_package);
        let ids: Vec<&str> = providers.iter().map(|provider| provider.id()).collect();
        assert_eq!(ids, vec!["cargo"]);

        let node_package = PackageId::new(PackageKind::Node, "x", root);
        let providers = registry.for_package(&node_package);
        assert!(providers.is_empty());
    }

    #[test]
    fn collect_all_merges_per_provider() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let package = PackageId::new(PackageKind::Cargo, "demo", dir.path());

        #[allow(clippy::unwrap_used)]
        let results = Registry::builtin().collect_all(&package).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider, "cargo");
        // The cargo provider always emits its metadata document, even for an
        // otherwise empty package.
        assert_eq!(results[0].documents.len(), 1);
        assert_eq!(results[0].documents[0].source, KnowledgeSource::Metadata);
    }
}
