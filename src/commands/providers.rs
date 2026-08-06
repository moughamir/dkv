use owo_colors::OwoColorize;

use crate::{
    error::Result,
    providers::{Registry, types::ProviderStatus},
};

/// Lists the registered knowledge providers.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] if provider listing fails (currently never).
pub fn run() -> Result<()> {
    let registry = Registry::builtin();
    println!("{}", "Registered Providers".green().bold());
    println!("{}", "─".repeat(19));
    for provider in registry.providers() {
        let metadata = provider.metadata();
        if metadata.status == ProviderStatus::Planned {
            println!("{} (planned)", metadata.id);
        } else {
            println!("{}", metadata.id);
        }
    }
    Ok(())
}
