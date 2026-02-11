//! Setup command - first-run wizard and targeted setup.

use crate::cli::SetupArgs;
use crate::config::load_merged_config;
use crate::download::{download_file, extract_tar_gz, resolve_justj_jre};
use crate::error::ExitCode;
use crate::jre::{find_active_jre, find_system_jre, JreSource, MIN_JAVA_VERSION};
use crate::manifest::{fetch_manifest, MANIFEST_URL};
use crate::platform::{KaratePaths, Platform};
use anyhow::Result;
use console::style;
use std::collections::HashSet;
use std::path::PathBuf;

/// Default Java version for Karate (21 required for Karate 1.5.2+)
const DEFAULT_JAVA_VERSION: u8 = MIN_JAVA_VERSION;

/// Valid items for setup
const VALID_ITEMS: &[&str] = &["jar", "jre"];

pub async fn run(args: SetupArgs) -> Result<ExitCode> {
    // Determine which items to install
    let items: HashSet<String> = if args.all {
        // --all installs everything
        VALID_ITEMS.iter().map(|s| s.to_string()).collect()
    } else if let Some(ref item) = args.item {
        // Validate item name
        let item_lower = item.to_lowercase();
        if !VALID_ITEMS.contains(&item_lower.as_str()) {
            eprintln!("{} Unknown item: {}", style("error:").red().bold(), item);
            eprintln!("  Valid items: {}", VALID_ITEMS.join(", "));
            return Ok(ExitCode::ConfigError);
        }
        let mut set = HashSet::new();
        set.insert(item_lower);
        set
    } else {
        // No flags = interactive wizard
        return run_setup_wizard().await;
    };

    // Non-interactive install of specified items
    run_setup_items(
        &items,
        args.force,
        args.java_version,
        args.karate_version,
        args.channel,
    )
    .await
}

/// Non-interactive setup of specified items.
async fn run_setup_items(
    items: &HashSet<String>,
    force: bool,
    java_version: Option<String>,
    version_override: Option<String>,
    channel_override: Option<String>,
) -> Result<ExitCode> {
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
    println!(
        "  Items: {}",
        style(items.iter().cloned().collect::<Vec<_>>().join(", ")).cyan()
    );
    println!();

    paths.ensure_dirs()?;

    let install_jre = items.contains("jre");
    let install_jar = items.contains("jar");
    let total_steps = (install_jre as u8) + (install_jar as u8);
    let mut step = 0;

    // Install JRE if requested
    if install_jre {
        step += 1;
        println!(
            "{} Setting up JRE...",
            style(format!("[{}/{}]", step, total_steps)).bold().dim()
        );

        let java_ver = java_version
            .as_ref()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_JAVA_VERSION);

        if !force {
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
            } else {
                // Check if system JRE exists but doesn't meet requirements
                if let Ok(Some(sys_jre)) = find_system_jre() {
                    if !sys_jre.meets_minimum_version() {
                        println!(
                            "  {} System Java {} found but requires {}+",
                            style("!").yellow(),
                            sys_jre.major_version.unwrap_or(0),
                            MIN_JAVA_VERSION
                        );
                    }
                }
                download_jre(&platform, &paths, java_ver).await?;
            }
        } else {
            println!("  {} Force mode: downloading JRE", style("!").yellow());
            download_jre(&platform, &paths, java_ver).await?;
        }
        println!();
    }

    // Install JAR if requested
    if install_jar {
        step += 1;
        println!(
            "{} Setting up Karate JAR...",
            style(format!("[{}/{}]", step, total_steps)).bold().dim()
        );

        // If a specific version is requested, check for that version; otherwise check for any JAR
        let should_download = if let Some(ref ver) = version_override {
            let target_jar = paths.dist.join(format!("karate-{}.jar", ver));
            if target_jar.exists() && !force {
                println!("  {} Karate {} already installed", style("✓").green(), ver);
                false
            } else {
                true
            }
        } else {
            let existing_jar = find_karate_jar(&paths.dist);
            if existing_jar.is_some() && !force {
                println!("  {} Karate JAR already installed", style("✓").green());
                false
            } else {
                if force {
                    println!("  {} Force mode: re-downloading JAR", style("!").yellow());
                }
                true
            }
        };
        if should_download {
            download_karate_jar(
                &paths,
                version_override.as_deref(),
                channel_override.as_deref(),
            )
            .await?;
        }
        println!();
    }

    println!(
        "{} Setup complete! Run {} to verify.",
        style("✓").green().bold(),
        style("karate doctor").cyan()
    );

    Ok(ExitCode::Success)
}

/// Full setup wizard (interactive).
async fn run_setup_wizard() -> Result<ExitCode> {
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
        download_karate_jar(&paths, None, None).await?;
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

    println!(
        "  {} JRE {} installed",
        style("✓").green(),
        jre_info.version_label
    );
    Ok(())
}

/// Download Karate JAR using manifest from karate.sh
async fn download_karate_jar(
    paths: &KaratePaths,
    version_override: Option<&str>,
    channel_override: Option<&str>,
) -> Result<()> {
    // Load config to get channel and version preferences
    let config = load_merged_config()?;
    let channel = channel_override.unwrap_or(&config.channel);

    println!("  Fetching release manifest from karate.sh...");

    let manifest = fetch_manifest().await.map_err(|e| {
        anyhow::anyhow!(
            "Failed to fetch manifest from {}: {}\n\n\
            Check your network connection or try again later.",
            MANIFEST_URL,
            e
        )
    })?;

    // Determine version: CLI flag → config pin → latest from channel
    let version = if let Some(v) = version_override {
        println!("  Requested version: {}", style(v).cyan());
        v.to_string()
    } else if config.karate_version != "latest" {
        // User pinned a specific version in config
        println!(
            "  Using pinned version: {}",
            style(&config.karate_version).cyan()
        );
        config.karate_version.clone()
    } else {
        // Get latest from configured channel
        manifest
            .get_latest_version("karate", channel)
            .map(|s| s.to_string())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No '{}' karate version found in manifest.\n\
                    Available channels: stable, beta\n\
                    Set channel with: karate config --global",
                    channel
                )
            })?
    };

    if channel != "stable" {
        println!("  Channel: {}", style(channel).yellow());
    }
    println!("  Version: {}", style(&version).green());

    let (url, sha256) = manifest
        .get_jar_download("karate", &version)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No download URL found for karate {} in manifest.\n\
                Check available versions at: {}",
                version,
                MANIFEST_URL
            )
        })?;

    let jar_name = format!("karate-{}.jar", version);
    println!("  Downloading {}...", jar_name);
    println!("  {}", style(url).dim());

    let dest = paths.dist.join(&jar_name);
    download_file(url, &dest, Some(sha256)).await?;

    // Cache the manifest for future use
    let cache_path = paths.cache.join("manifest.json");
    if let Err(e) = crate::manifest::save_manifest_cache(&manifest, &cache_path) {
        eprintln!("  {} Failed to cache manifest: {}", style("!").yellow(), e);
    }

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
