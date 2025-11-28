//! Setup command - first-run wizard and targeted setup.

use crate::cli::{SetupArgs, SetupSubcommand};
use crate::download::{download_file, extract_tar_gz, fetch_latest_release, resolve_justj_jre};
use crate::error::ExitCode;
use crate::jre::{find_active_jre, find_system_jre, JreSource, MIN_JAVA_VERSION};
use crate::platform::{KaratePaths, Platform};
use anyhow::Result;
use console::style;
use std::path::PathBuf;

/// Default Java version for Karate (21 required for Karate 1.5.2+)
const DEFAULT_JAVA_VERSION: u8 = MIN_JAVA_VERSION;

pub async fn run(args: SetupArgs) -> Result<ExitCode> {
    match args.subcommand {
        Some(SetupSubcommand::Path(path_args)) => run_setup_path(path_args).await,
        Some(SetupSubcommand::Jre(jre_args)) => run_setup_jre(jre_args).await,
        None => run_setup_wizard(args.yes).await,
    }
}

/// Full setup wizard.
async fn run_setup_wizard(non_interactive: bool) -> Result<ExitCode> {
    let platform = Platform::detect()?;
    let paths = KaratePaths::new();

    println!("{} Karate CLI Setup", style("▶").cyan().bold());
    println!();
    println!(
        "  Platform: {} {}",
        style(format!("{:?}", platform.os)).green(),
        style(format!("{:?}", platform.arch)).green()
    );
    println!("  Home: {}", style(paths.home.display()).dim());
    println!();

    // Ensure directories exist
    paths.ensure_dirs()?;

    // Step 1: Check/Download JRE
    println!("{} Setting up JRE...", style("[1/2]").bold().dim());

    let jre = find_active_jre()?;
    match &jre {
        Some(j) => {
            let source_info = match j.source {
                JreSource::Managed => "managed".to_string(),
                JreSource::JavaHome => "from JAVA_HOME".to_string(),
                JreSource::Path => "from PATH".to_string(),
            };
            println!(
                "  {} JRE available ({}, Java {})",
                style("✓").green(),
                source_info,
                j.major_version.unwrap_or(0)
            );
        }
        None => {
            // Check if there's a system JRE that doesn't meet requirements
            if let Ok(Some(sys_jre)) = find_system_jre() {
                println!(
                    "  {} System Java {} found but requires {}+",
                    style("!").yellow(),
                    sys_jre.major_version.unwrap_or(0),
                    MIN_JAVA_VERSION
                );
            }
            download_jre(&platform, &paths, DEFAULT_JAVA_VERSION).await?;
        }
    }

    // Step 2: Download Karate JAR
    println!();
    println!("{} Setting up Karate JAR...", style("[2/2]").bold().dim());

    let existing_jar = find_karate_jar(&paths.dist);
    if existing_jar.is_some() {
        println!("  {} Karate JAR already installed", style("✓").green());
    } else {
        download_karate_jar(&paths).await?;
    }

    println!();
    println!(
        "{} Setup complete! Run {} to verify.",
        style("✓").green().bold(),
        style("karate doctor").cyan()
    );

    Ok(ExitCode::Success)
}

/// Download and extract JRE using JustJ manifest (same pattern as Red Hat vscode-java)
async fn download_jre(platform: &Platform, paths: &KaratePaths, java_version: u8) -> Result<()> {
    let platform_key = platform.manifest_key();

    println!("  Resolving JRE {} for {}...", java_version, platform_key);

    // Fetch manifest and resolve download URL dynamically
    let jre_info = resolve_justj_jre(java_version, &platform_key).await?;

    println!("  Found: {}", style(&jre_info.version_label).green());
    println!("  {}", style(&jre_info.download_url).dim());

    // Download to temp file
    let archive_name = format!("jre-{}.tar.gz", jre_info.version_label);
    let archive_path = paths.cache.join(&archive_name);

    download_file(&jre_info.download_url, &archive_path, None).await?;

    // Extract - use version_label for directory name (e.g., 21.0.9-macosx-aarch64)
    println!("  Extracting...");
    let jre_dir = paths.jre.join(&jre_info.version_label);
    std::fs::create_dir_all(&jre_dir)?;
    extract_tar_gz(&archive_path, &jre_dir)?;

    // Clean up archive
    let _ = std::fs::remove_file(&archive_path);

    println!("  {} JRE {} installed", style("✓").green(), jre_info.version_label);
    Ok(())
}

/// Download Karate JAR from GitHub releases
async fn download_karate_jar(paths: &KaratePaths) -> Result<()> {
    println!("  Fetching latest release info...");

    let release = fetch_latest_release("karatelabs", "karate").await?;
    let version = release.tag_name.trim_start_matches('v');

    println!("  Latest version: {}", style(version).green());

    // Find the main karate JAR (not robot, not sbom)
    let jar_name = format!("karate-{}.jar", version);
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == jar_name)
        .ok_or_else(|| anyhow::anyhow!("Could not find {} in release assets", jar_name))?;

    println!("  Downloading {}...", jar_name);
    println!("  {}", style(&asset.browser_download_url).dim());

    let dest = paths.dist.join(&jar_name);
    download_file(&asset.browser_download_url, &dest, None).await?;

    println!("  {} Karate JAR installed", style("✓").green());
    Ok(())
}

/// Find existing Karate JAR in dist directory
fn find_karate_jar(dist_dir: &PathBuf) -> Option<PathBuf> {
    if !dist_dir.exists() {
        return None;
    }

    std::fs::read_dir(dist_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.extension().map(|e| e == "jar").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("karate-") && !n.contains("robot"))
                    .unwrap_or(false)
        })
}

/// Setup PATH only.
async fn run_setup_path(args: crate::cli::SetupPathArgs) -> Result<ExitCode> {
    let platform = Platform::detect()?;

    println!("{} Setting up PATH...", style("▶").cyan().bold());

    let bin_dir = args
        .bin_dir
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| platform.os.default_bin_dir());

    println!("  Target: {}", style(bin_dir.display()).green());

    // TODO: Implement symlink creation
    println!("  {} PATH setup not yet implemented", style("!").yellow());

    Ok(ExitCode::Success)
}

/// Setup JRE only.
async fn run_setup_jre(args: crate::cli::SetupJreArgs) -> Result<ExitCode> {
    let platform = Platform::detect()?;
    let paths = KaratePaths::new();

    println!("{} Setting up JRE...", style("▶").cyan().bold());

    paths.ensure_dirs()?;

    let java_version = args
        .version
        .as_ref()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_JAVA_VERSION);

    // Check if we already have a suitable JRE (unless --force is used)
    if !args.force {
        if let Some(jre) = find_active_jre()? {
            let source_info = match jre.source {
                JreSource::Managed => "managed".to_string(),
                JreSource::JavaHome => "from JAVA_HOME".to_string(),
                JreSource::Path => "from PATH".to_string(),
            };
            println!(
                "  {} JRE already available ({}, Java {})",
                style("✓").green(),
                source_info,
                jre.major_version.unwrap_or(0)
            );
            println!();
            println!(
                "  Use {} to force download anyway.",
                style("karate setup jre --force").cyan()
            );
            return Ok(ExitCode::Success);
        }

        // Check if system JRE exists but doesn't meet requirements
        if let Some(sys_jre) = find_system_jre()? {
            if !sys_jre.meets_minimum_version() {
                println!(
                    "  {} System Java {} found but requires {}+",
                    style("!").yellow(),
                    sys_jre.major_version.unwrap_or(0),
                    MIN_JAVA_VERSION
                );
            }
        }
    } else {
        println!("  {} Force mode: downloading JRE", style("!").yellow());
    }

    download_jre(&platform, &paths, java_version).await?;

    Ok(ExitCode::Success)
}
