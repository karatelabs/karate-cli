//! Config command - view and edit configuration.

use crate::cli::ConfigArgs;
use crate::config::load_merged_config;
use crate::error::ExitCode;
use crate::platform::KaratePaths;
use anyhow::Result;
use console::style;

pub async fn run(args: ConfigArgs) -> Result<ExitCode> {
    if args.show {
        return show_config().await;
    }

    let config_path = if args.local {
        KaratePaths::local_config()
    } else {
        // Default to global, or local if it exists and --global not specified
        let local = KaratePaths::local_config();
        if !args.global && local.exists() {
            local
        } else {
            KaratePaths::new().global_config
        }
    };

    println!("{} Configuration", style("▶").cyan().bold());
    println!();
    println!("  Config file: {}", style(config_path.display()).green());

    if !config_path.exists() {
        println!("  Status: {}", style("does not exist").yellow());
        println!();
        println!("  Run with --show to see resolved defaults, or create the file manually.");
    } else {
        println!("  Status: {}", style("exists").green());
        println!();

        // TODO: Open in $EDITOR or provide simple editing prompts
        println!(
            "  {} Interactive editing not yet implemented",
            style("!").yellow()
        );
        println!("  Edit manually at: {}", config_path.display());
    }

    Ok(ExitCode::Success)
}

/// Show the resolved (merged) configuration.
async fn show_config() -> Result<ExitCode> {
    let config = load_merged_config()?;
    let json = serde_json::to_string_pretty(&config)?;

    println!("{}", json);

    Ok(ExitCode::Success)
}
