mod cli;
mod commands;
mod config;
mod delegate;
mod download;
mod error;
mod jre;
mod manifest;
mod platform;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::cli::{Cli, Command};
use crate::error::ExitCode;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let exit_code = run().await;
    std::process::exit(exit_code.into());
}

async fn run() -> ExitCode {
    let cli = Cli::parse();

    // Handle --no-color globally
    if cli.no_color {
        console::set_colors_enabled(false);
    }

    let result = match cli.command {
        // Rust-native commands
        Command::Setup(args) => commands::setup::run(args).await,
        Command::Upgrade(args) => commands::upgrade::run(args).await,
        Command::Config(args) => commands::config::run(args).await,
        Command::Jre(args) => commands::jre::run(args).await,
        Command::Ext(args) => commands::plugin::run(args).await,
        Command::Doctor(args) => commands::doctor::run(args).await,
        Command::Version(args) => commands::version::run(args).await,

        // JAR-delegated commands
        Command::External(args) => delegate::run(args).await,
    };

    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            ExitCode::GeneralError
        }
    }
}
