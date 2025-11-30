//! JRE command - JRE inspection and management.

use crate::cli::{JreArgs, JreSubcommand};
use crate::error::ExitCode;
use crate::jre::{find_active_jre, list_installed_jres};
use crate::platform::Platform;
use anyhow::Result;
use console::style;

pub async fn run(args: JreArgs) -> Result<ExitCode> {
    match args.subcommand {
        JreSubcommand::List => run_list().await,
        JreSubcommand::Doctor => run_doctor().await,
    }
}

/// List installed JREs.
async fn run_list() -> Result<ExitCode> {
    let platform = Platform::detect()?;
    let jres = list_installed_jres()?;

    println!("{} Installed JREs", style("▶").cyan().bold());
    println!();

    if jres.is_empty() {
        println!("  No JREs installed.");
        println!();
        println!("  Run {} to install a JRE.", style("karate setup").cyan());
        return Ok(ExitCode::Success);
    }

    let active_jre = find_active_jre()?;

    for jre in &jres {
        let is_active = active_jre
            .as_ref()
            .map(|a| a.path == jre.path)
            .unwrap_or(false);

        let marker = if is_active {
            style("*").green().bold()
        } else {
            style(" ").dim()
        };

        let status = if jre.is_valid() {
            style("✓").green()
        } else {
            style("✗").red()
        };

        println!(
            "  {} {} {} ({})",
            marker,
            status,
            style(&jre.version).bold(),
            jre.platform
        );
        println!("      {}", style(jre.path.display()).dim());
    }

    println!();
    println!(
        "  Current platform: {}",
        style(platform.manifest_key()).cyan()
    );

    Ok(ExitCode::Success)
}

/// Check JRE health.
async fn run_doctor() -> Result<ExitCode> {
    println!("{} JRE Health Check", style("▶").cyan().bold());
    println!();

    let platform = Platform::detect()?;
    println!("  Platform: {}", style(platform.manifest_key()).green());

    match find_active_jre()? {
        Some(jre) => {
            println!("  Status: {}", style("OK").green().bold());
            println!();
            println!("  Version: {}", style(&jre.version).bold());
            println!("  Path: {}", jre.path.display());
            println!("  Executable: {}", jre.java_executable.display());

            // Try to get actual Java version
            match jre.check_version() {
                Ok(version_string) => {
                    println!();
                    println!("  Java version output:");
                    println!("    {}", style(version_string).dim());
                }
                Err(e) => {
                    println!();
                    println!(
                        "  {} Failed to get Java version: {}",
                        style("!").yellow(),
                        e
                    );
                }
            }

            Ok(ExitCode::Success)
        }
        None => {
            println!(
                "  Status: {} No working JRE found",
                style("ERROR").red().bold()
            );
            println!();
            println!("  Run {} to install a JRE.", style("karate setup").cyan());

            Ok(ExitCode::JreError)
        }
    }
}
