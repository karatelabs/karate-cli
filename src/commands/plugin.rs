//! Plugin command - manage Karate extensions.
//!
//! Note: For v1, plugins are simply JAR files dropped in ~/.karate/ext/
//! This command provides info about the ext/ folder.

use crate::cli::{PluginArgs, PluginSubcommand};
use crate::error::ExitCode;
use crate::platform::KaratePaths;
use anyhow::Result;
use console::style;

pub async fn run(args: PluginArgs) -> Result<ExitCode> {
    match args.subcommand {
        PluginSubcommand::Install(_) => run_install_info().await,
        PluginSubcommand::Remove(_) => run_remove_info().await,
        PluginSubcommand::List => run_list().await,
    }
}

/// Show info about how to install extensions
async fn run_install_info() -> Result<ExitCode> {
    let paths = KaratePaths::new();

    println!("{} Installing Extensions", style("▶").cyan().bold());
    println!();
    println!("  To add extensions, simply drop JAR files into:");
    println!("  {}", style(paths.ext.display()).green());
    println!();
    println!("  All JARs in this folder are automatically added to the classpath.");
    println!();
    println!(
        "  Tip: Run {} to verify extensions are detected.",
        style("karate doctor").cyan()
    );

    Ok(ExitCode::Success)
}

/// Show info about how to remove extensions
async fn run_remove_info() -> Result<ExitCode> {
    let paths = KaratePaths::new();

    println!("{} Removing Extensions", style("▶").cyan().bold());
    println!();
    println!("  To remove an extension, delete the JAR file from:");
    println!("  {}", style(paths.ext.display()).green());

    Ok(ExitCode::Success)
}

/// List installed extensions
async fn run_list() -> Result<ExitCode> {
    let paths = KaratePaths::new();

    println!("{} Extensions", style("▶").cyan().bold());
    println!();
    println!("  Location: {}", style(paths.ext.display()).dim());
    println!();

    if !paths.ext.exists() {
        println!("  {}", style("No extensions installed").dim());
        println!();
        println!(
            "  Drop JAR files into the ext/ folder to add extensions."
        );
        return Ok(ExitCode::Success);
    }

    let jars: Vec<_> = std::fs::read_dir(&paths.ext)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "jar")
                .unwrap_or(false)
        })
        .collect();

    if jars.is_empty() {
        println!("  {}", style("No extensions installed").dim());
        println!();
        println!("  Drop JAR files into the ext/ folder to add extensions.");
    } else {
        for entry in jars {
            let name = entry.file_name().to_string_lossy().to_string();
            println!("  {} {}", style("•").cyan(), name);
        }
    }

    Ok(ExitCode::Success)
}
