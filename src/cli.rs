use clap::{ArgAction, Args, Parser, Subcommand};

/// Dev Knowledge Vault
///
/// Offline documentation manager for developers.
#[derive(Debug, Parser)]
#[command(
    name = "dkv",
    author,
    version,
    about = "Dev Knowledge Vault",
    long_about = None,
    propagate_version = true
)]
pub struct Cli {
    /// Increase logging verbosity.
    ///
    /// -v     INFO
    /// -vv    DEBUG
    /// -vvv   TRACE
    #[arg(short, long, action = ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress all output except errors.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize DKV configuration.
    Init,

    /// Synchronize documentation for a project.
    Sync(SyncArgs),

    /// List detected project dependencies.
    Deps(DepsArgs),

    /// Search indexed documentation.
    Search(SearchArgs),

    /// Open documentation.
    Open(OpenArgs),

    /// Update cached documentation.
    Update(UpdateArgs),

    /// Check installation health.
    Doctor,

    /// Show vault statistics.
    Stats,
}

#[derive(Debug, Args)]
pub struct SyncArgs {
    /// Project directory.
    #[arg(default_value = ".")]
    pub path: std::path::PathBuf,
}

#[derive(Debug, Args)]
pub struct DepsArgs {
    /// Project directory.
    #[arg(default_value = ".")]
    pub path: std::path::PathBuf,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search query.
    pub query: Vec<String>,
}

#[derive(Debug, Args)]
pub struct OpenArgs {
    /// Documentation topic.
    pub topic: String,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Force update.
    #[arg(long)]
    pub force: bool,
}

impl SearchArgs {
    #[must_use]
    pub fn query(&self) -> String {
        self.query.join(" ")
    }
}
