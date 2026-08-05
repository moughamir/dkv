mod cli;

use clap::Parser;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::cli::{Cli, Commands};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    init_tracing(cli.verbose);

    match cli.command {
        Commands::Init => {
            println!("Initializing Dev Knowledge Vault...");
        }

        Commands::Sync(args) => {
            println!("Syncing project: {}", args.path.display());
        }

        Commands::Deps(args) => {
            println!("Reading dependencies from {}", args.path.display());
        }

        Commands::Search(args) => {
            println!("Searching: {}", args.query());
        }

        Commands::Open(args) => {
            println!("Opening documentation for '{}'", args.topic);
        }

        Commands::Update(args) => {
            println!("Updating documentation (force={})", args.force);
        }

        Commands::Doctor => {
            println!("Running diagnostics...");
        }

        Commands::Stats => {
            println!("Showing statistics...");
        }
    }

    Ok(())
}

fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => Level::INFO,
        1 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .without_time()
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);
}
