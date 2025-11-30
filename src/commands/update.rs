//! Update command - check for and install updates to Karate JAR and JRE.

use crate::cli::UpdateArgs;
use crate::download::{download_file, extract_tar_gz, fetch_latest_release, resolve_justj_jre};
use crate::error::ExitCode;
use crate::jre::MIN_JAVA_VERSION;
use crate::platform::{KaratePaths, Platform};
use anyhow::Result;
use console::style;
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::PathBuf;

/// Valid items for update
const VALID_ITEMS: &[&str] = &["jar", "jre"];

/// Info about an installed component and its update status
#[derive(Debug)]
struct ComponentStatus {
    installed_version: Option<String>,
    latest_version: String,
    has_update: bool,
}

pub async fn run(args: UpdateArgs) -> Result<ExitCode> {
    let platform = Platform::detect()?;
    let paths = KaratePaths::new();

    // Determine which items to check
    let items: HashSet<String> = if args.all {
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
        // No flags = interactive mode, check all items
        VALID_ITEMS.iter().map(|s| s.to_string()).collect()
    };

    let interactive = !args.all && args.item.is_none();

    println!("{} Checking for updates...", style("▶").cyan().bold());
    println!();

    paths.ensure_dirs()?;

    let check_jar = items.contains("jar");
    let check_jre = items.contains("jre");

    let mut jar_status: Option<ComponentStatus> = None;
    let mut jre_status: Option<ComponentStatus> = None;

    // Check JAR status
    if check_jar {
        let installed = get_installed_jar_version(&paths.dist);
        let latest_release = fetch_latest_release("karatelabs", "karate").await?;
        let latest = latest_release.tag_name.trim_start_matches('v').to_string();

        let has_update = match &installed {
            Some(v) => v != &latest,
            None => true,
        };

        jar_status = Some(ComponentStatus {
            installed_version: installed,
            latest_version: latest,
            has_update,
        });
    }

    // Check JRE status
    if check_jre {
        let installed = get_installed_jre_version(&paths.jre);
        let platform_key = platform.manifest_key();
        let jre_info = resolve_justj_jre(MIN_JAVA_VERSION, &platform_key).await?;

        // Extract just the version part (e.g., "21.0.9" from "21.0.9-macosx-aarch64")
        let latest = jre_info
            .version_label
            .split('-')
            .next()
            .unwrap_or(&jre_info.version_label)
            .to_string();

        let has_update = match &installed {
            Some(v) => {
                // Compare just the version part
                let installed_ver = v.split('-').next().unwrap_or(v);
                installed_ver != latest
            }
            None => true,
        };

        jre_status = Some(ComponentStatus {
            installed_version: installed,
            latest_version: latest,
            has_update,
        });
    }

    // Display status
    let mut any_updates = false;

    if let Some(ref status) = jar_status {
        if status.has_update {
            any_updates = true;
            match &status.installed_version {
                Some(v) => println!(
                    "  {} JAR: {} → {} available",
                    style("↑").cyan(),
                    v,
                    style(&status.latest_version).green()
                ),
                None => println!(
                    "  {} JAR: not installed → {} available",
                    style("↑").cyan(),
                    style(&status.latest_version).green()
                ),
            }
        } else {
            println!(
                "  {} JAR: {} (up to date)",
                style("✓").green(),
                status.installed_version.as_ref().unwrap()
            );
        }
    }

    if let Some(ref status) = jre_status {
        if status.has_update {
            any_updates = true;
            match &status.installed_version {
                Some(v) => {
                    let installed_ver = v.split('-').next().unwrap_or(v);
                    println!(
                        "  {} JRE: {} → {} available",
                        style("↑").cyan(),
                        installed_ver,
                        style(&status.latest_version).green()
                    )
                }
                None => println!(
                    "  {} JRE: not installed → {} available",
                    style("↑").cyan(),
                    style(&status.latest_version).green()
                ),
            }
        } else if let Some(v) = &status.installed_version {
            let installed_ver = v.split('-').next().unwrap_or(v);
            println!(
                "  {} JRE: {} (up to date)",
                style("✓").green(),
                installed_ver
            );
        }
    }

    println!();

    if !any_updates {
        println!(
            "{} All components are up to date!",
            style("✓").green().bold()
        );
        return Ok(ExitCode::Success);
    }

    // Interactive confirmation
    if interactive {
        print!("Update available components? [Y/n] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if !input.is_empty() && input != "y" && input != "yes" {
            println!("Update cancelled.");
            return Ok(ExitCode::Success);
        }
        println!();
    }

    // Perform updates
    let mut step = 0;
    let total_steps = jar_status.as_ref().map(|s| s.has_update as u8).unwrap_or(0)
        + jre_status.as_ref().map(|s| s.has_update as u8).unwrap_or(0);

    // Update JAR
    if let Some(ref status) = jar_status {
        if status.has_update {
            step += 1;
            println!(
                "{} Updating JAR to {}...",
                style(format!("[{}/{}]", step, total_steps)).bold().dim(),
                status.latest_version
            );
            update_karate_jar(&paths).await?;
        }
    }

    // Update JRE
    if let Some(ref status) = jre_status {
        if status.has_update {
            step += 1;
            println!(
                "{} Updating JRE to {}...",
                style(format!("[{}/{}]", step, total_steps)).bold().dim(),
                status.latest_version
            );
            update_jre(&platform, &paths).await?;
        }
    }

    println!();
    println!(
        "{} Update complete! Run {} to verify.",
        style("✓").green().bold(),
        style("karate doctor").cyan()
    );

    Ok(ExitCode::Success)
}

/// Get the installed JAR version from the dist directory
fn get_installed_jar_version(dist_dir: &PathBuf) -> Option<String> {
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
        .and_then(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix("karate-"))
                .map(|s| s.to_string())
        })
}

/// Get the installed JRE version from the jre directory
fn get_installed_jre_version(jre_dir: &PathBuf) -> Option<String> {
    if !jre_dir.exists() {
        return None;
    }

    // Find the first directory that looks like a version
    std::fs::read_dir(jre_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .and_then(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
}

/// Download and update Karate JAR
async fn update_karate_jar(paths: &KaratePaths) -> Result<()> {
    let release = fetch_latest_release("karatelabs", "karate").await?;
    let version = release.tag_name.trim_start_matches('v');

    // Find the main karate JAR
    let jar_name = format!("karate-{}.jar", version);
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == jar_name)
        .ok_or_else(|| anyhow::anyhow!("Could not find {} in release assets", jar_name))?;

    println!("  Downloading {}...", jar_name);

    // Remove old JAR(s) first
    if paths.dist.exists() {
        for entry in std::fs::read_dir(&paths.dist)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "jar").unwrap_or(false) {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("karate-") && !name.contains("robot") {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }

    let dest = paths.dist.join(&jar_name);
    download_file(&asset.browser_download_url, &dest, None).await?;

    println!("  {} JAR updated to {}", style("✓").green(), version);
    Ok(())
}

/// Download and update JRE
async fn update_jre(platform: &Platform, paths: &KaratePaths) -> Result<()> {
    let platform_key = platform.manifest_key();
    let jre_info = resolve_justj_jre(MIN_JAVA_VERSION, &platform_key).await?;

    println!("  Downloading JRE {}...", jre_info.version_label);

    // Download to temp file
    let archive_name = format!("jre-{}.tar.gz", jre_info.version_label);
    let archive_path = paths.cache.join(&archive_name);

    download_file(&jre_info.download_url, &archive_path, None).await?;

    // Remove old JRE directories
    if paths.jre.exists() {
        for entry in std::fs::read_dir(&paths.jre)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let _ = std::fs::remove_dir_all(&path);
            }
        }
    }

    // Extract
    println!("  Extracting...");
    let jre_dir = paths.jre.join(&jre_info.version_label);
    std::fs::create_dir_all(&jre_dir)?;
    extract_tar_gz(&archive_path, &jre_dir)?;

    // Clean up archive
    let _ = std::fs::remove_file(&archive_path);

    println!(
        "  {} JRE updated to {}",
        style("✓").green(),
        jre_info.version_label
    );
    Ok(())
}
