//! Update command - check for and install updates to Karate JAR and JRE.

use crate::cli::UpdateArgs;
use crate::commands::version::LAUNCHER_VERSION;
use crate::config::load_merged_config;
use crate::download::{download_file, extract_tar_gz, extract_zip, resolve_justj_jre};
use crate::error::ExitCode;
use crate::jre::{install_jre_archive, MIN_JAVA_VERSION};
use crate::manifest::{fetch_manifest, ReleasesManifest};
use crate::platform::{highest_karate_jar, karate_jar_version, KaratePaths, Os, Platform};
use anyhow::{Context, Result};
use console::style;
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Valid items for update
const VALID_ITEMS: &[&str] = &["jar", "jre", "cli"];

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
    let check_cli = items.contains("cli");
    if check_jar || check_jre {
        paths.print_local_override_notice();
    }

    // Clean up leftover .old binary from previous self-update (Windows)
    cleanup_old_binary();

    let mut jar_status: Option<ComponentStatus> = None;
    let mut jre_status: Option<ComponentStatus> = None;
    let mut cli_status: Option<ComponentStatus> = None;

    // Load config for channel preference (command line overrides config)
    let config = load_merged_config()?;
    let channel = args.channel.as_deref().unwrap_or(&config.channel);

    // Fetch manifest once for JAR and CLI checks
    let manifest = if check_jar || check_cli {
        Some(fetch_manifest().await?)
    } else {
        None
    };

    // A pinned karate_version is the exact JAR `run` uses (spec: Version Pinning), so
    // update leaves the JARs alone rather than fetch one that would never run.
    let pinned = (config.karate_version != "latest").then_some(config.karate_version.as_str());
    if let (true, Some(pin)) = (check_jar, pinned) {
        let hint = if paths.dist.join(format!("karate-{pin}.jar")).exists() {
            "set karate_version to \"latest\" to follow the channel".to_string()
        } else {
            format!("not installed: karate setup --item jar --karate-version {pin}")
        };
        println!(
            "  {} JAR: pinned to {} in config, not updated ({})",
            style("-").dim(),
            pin,
            hint
        );
    }

    // Check JAR status
    if check_jar && pinned.is_none() {
        let installed = highest_karate_jar(&paths.dist).map(|(version, _)| version);
        let latest = manifest
            .as_ref()
            .and_then(|m| m.get_latest_version("karate", channel))
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No '{}' karate version found in manifest", channel))?;

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

    // Check CLI status
    if check_cli {
        let installed = LAUNCHER_VERSION.to_string();
        let latest = manifest
            .as_ref()
            .and_then(|m| m.get_latest_version("karate-cli", channel))
            .map(|s| s.to_string())
            .ok_or_else(|| {
                anyhow::anyhow!("No '{}' karate-cli version found in manifest", channel)
            })?;

        let has_update = installed != latest;

        cli_status = Some(ComponentStatus {
            installed_version: Some(installed),
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

    if let Some(ref status) = cli_status {
        if status.has_update {
            any_updates = true;
            println!(
                "  {} CLI: {} → {} available",
                style("↑").cyan(),
                status.installed_version.as_ref().unwrap(),
                style(&status.latest_version).green()
            );
        } else {
            println!(
                "  {} CLI: {} (up to date)",
                style("✓").green(),
                status.installed_version.as_ref().unwrap()
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
        + jre_status.as_ref().map(|s| s.has_update as u8).unwrap_or(0)
        + cli_status.as_ref().map(|s| s.has_update as u8).unwrap_or(0);

    // Update JAR
    if let Some(ref status) = jar_status {
        if status.has_update {
            step += 1;
            println!(
                "{} Updating JAR to {}...",
                style(format!("[{}/{}]", step, total_steps)).bold().dim(),
                status.latest_version
            );
            update_karate_jar(&paths, channel)
                .await
                .context("JAR not updated; the installed JARs are unchanged")?;
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
            update_jre(&platform, &paths)
                .await
                .context("JRE not updated; the installed JRE is unchanged")?;
        }
    }

    // Update CLI (last — if binary replacement disrupts process, JAR/JRE are already done)
    if let Some(ref status) = cli_status {
        if status.has_update {
            step += 1;
            println!(
                "{} Updating CLI to {}...",
                style(format!("[{}/{}]", step, total_steps)).bold().dim(),
                status.latest_version
            );
            update_cli_binary(
                &paths,
                &platform,
                &status.latest_version,
                manifest.as_ref().unwrap(),
            )
            .await
            .context("CLI not updated; the installed binary is unchanged")?;
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

/// Download and update Karate JAR using manifest from karate.sh
async fn update_karate_jar(paths: &KaratePaths, channel: &str) -> Result<()> {
    let manifest = fetch_manifest().await?;

    let version = manifest
        .get_latest_version("karate", channel)
        .ok_or_else(|| anyhow::anyhow!("No '{}' karate version found in manifest", channel))?;

    let (url, sha256) = manifest
        .get_jar_download("karate", version)
        .ok_or_else(|| anyhow::anyhow!("No download URL found for karate {}", version))?;

    println!("  Downloading karate-{}.jar...", version);
    replace_karate_jar(&paths.dist, version, url, sha256).await?;

    println!("  {} JAR updated to {}", style("✓").green(), version);
    Ok(())
}

/// Install `karate-<version>.jar` into `dist`, then remove the JARs it supersedes. A failed
/// download or checksum mismatch leaves `dist` as it was.
async fn replace_karate_jar(dist: &Path, version: &str, url: &str, sha256: &str) -> Result<()> {
    let dest = dist.join(format!("karate-{version}.jar"));
    download_file(url, &dest, Some(sha256)).await?;
    for entry in std::fs::read_dir(dist)? {
        let path = entry?.path();
        if path != dest && karate_jar_version(&path).is_some() {
            let _ = std::fs::remove_file(&path);
        }
    }
    Ok(())
}

/// Clean up leftover .old binary from a previous self-update (Windows compatibility).
/// On Windows, a running binary cannot be deleted, so cleanup is deferred to next run.
fn cleanup_old_binary() {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Ok(canonical) = current_exe.canonicalize() {
            let old_path = canonical.with_extension("old");
            if old_path.exists() {
                let _ = std::fs::remove_file(&old_path);
            }
        }
    }
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

    println!("  Extracting...");
    let installed = install_jre_archive(&archive_path, &paths.jre, &jre_info.version_label, true);
    let _ = std::fs::remove_file(&archive_path);
    installed?;

    println!(
        "  {} JRE updated to {}",
        style("✓").green(),
        jre_info.version_label
    );
    Ok(())
}

/// Download and replace the CLI binary with a new version
async fn update_cli_binary(
    paths: &KaratePaths,
    platform: &Platform,
    version: &str,
    manifest: &ReleasesManifest,
) -> Result<()> {
    let artifact = manifest
        .get_platform_download("karate-cli", version, platform)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No CLI binary found for platform '{}' in version {}",
                platform.manifest_key(),
                version
            )
        })?;

    let url = &artifact.url;
    let sha256 = &artifact.sha256;

    // Determine archive extension based on platform
    let ext = if platform.os == Os::Windows {
        "zip"
    } else {
        "tar.gz"
    };
    let archive_name = format!("karate-cli-{}.{}", version, ext);
    let archive_path = paths.cache.join(&archive_name);

    println!("  Downloading CLI {}...", version);
    download_file(url, &archive_path, Some(sha256)).await?;

    // Extract to temp directory
    let extract_dir = paths.cache.join(format!("karate-cli-{}-extract", version));
    if extract_dir.exists() {
        std::fs::remove_dir_all(&extract_dir)?;
    }

    println!("  Extracting...");
    if platform.os == Os::Windows {
        extract_zip(&archive_path, &extract_dir)?;
    } else {
        extract_tar_gz(&archive_path, &extract_dir)?;
    }

    // Find the binary in extracted dir
    let binary_name = if platform.os == Os::Windows {
        "karate.exe"
    } else {
        "karate"
    };
    let new_binary = find_binary_in_dir(&extract_dir, binary_name)?;

    // Get current executable path
    let current_exe = std::env::current_exe()
        .context("Failed to determine current executable path")?
        .canonicalize()
        .context("Failed to resolve current executable path")?;

    let backup_path = current_exe.with_extension("old");

    // Remove any existing backup
    let _ = std::fs::remove_file(&backup_path);

    // Rename current → .old
    std::fs::rename(&current_exe, &backup_path).with_context(|| {
        format!(
            "Failed to back up current binary {} → {}",
            current_exe.display(),
            backup_path.display()
        )
    })?;

    // Move new binary → current location (fall back to copy for cross-filesystem)
    let result = std::fs::rename(&new_binary, &current_exe)
        .or_else(|_| std::fs::copy(&new_binary, &current_exe).map(|_| ()));

    if let Err(e) = result {
        // Restore backup on failure
        eprintln!("  {} Failed to place new binary: {}", style("!").red(), e);
        let _ = std::fs::rename(&backup_path, &current_exe);
        return Err(e.into());
    }

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&current_exe, std::fs::Permissions::from_mode(0o755))?;
    }

    // Cleanup
    let _ = std::fs::remove_file(&archive_path);
    let _ = std::fs::remove_dir_all(&extract_dir);
    // On Unix we can remove the backup immediately; on Windows the running binary is locked
    #[cfg(unix)]
    {
        let _ = std::fs::remove_file(&backup_path);
    }

    println!("  {} CLI updated to {}", style("✓").green(), version);
    Ok(())
}

/// Find the karate binary in an extracted directory (top-level or one level deep)
fn find_binary_in_dir(dir: &std::path::Path, binary_name: &str) -> Result<PathBuf> {
    // Check top level
    let top_level = dir.join(binary_name);
    if top_level.exists() {
        return Ok(top_level);
    }

    // Check one level deep
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let nested = path.join(binary_name);
                if nested.exists() {
                    return Ok(nested);
                }
            }
        }
    }

    anyhow::bail!(
        "Could not find '{}' in extracted archive at {}",
        binary_name,
        dir.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_jar_version_is_the_highest() {
        let dist = tempfile::tempdir().unwrap();
        for jar in [
            "karate-2.1.4.jar",
            "karate-2.1.10.jar",
            "karate-robot-9.9.9.jar",
        ] {
            std::fs::write(dist.path().join(jar), b"jar").unwrap();
        }
        let (version, _) = highest_karate_jar(dist.path()).unwrap();
        assert_eq!(version, "2.1.10");
    }

    /// Serves `body` to every request on a local port; returns the URL.
    async fn serve(body: &'static [u8]) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/karate.jar", listener.local_addr().unwrap());
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut request = [0u8; 1024];
                let _ = socket.read(&mut request).await;
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = socket.write_all(head.as_bytes()).await;
                let _ = socket.write_all(body).await;
            }
        });
        url
    }

    fn dist_with_old_jar() -> tempfile::TempDir {
        let dist = tempfile::tempdir().unwrap();
        std::fs::write(dist.path().join("karate-2.1.3.jar"), b"old").unwrap();
        dist
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        names
    }

    #[tokio::test]
    async fn checksum_mismatch_leaves_the_installed_jar() {
        let dist = dist_with_old_jar();
        let url = serve(b"new").await;
        let err = replace_karate_jar(dist.path(), "2.1.4", &url, &"0".repeat(64))
            .await
            .unwrap_err();
        assert_eq!(crate::error::exit_code_for(&err), ExitCode::NetworkError);
        assert_eq!(names(dist.path()), vec!["karate-2.1.3.jar"]);
    }

    #[tokio::test]
    async fn unreachable_download_leaves_the_installed_jar() {
        let dist = dist_with_old_jar();
        let err = replace_karate_jar(dist.path(), "2.1.4", "http://127.0.0.1:9/k.jar", "00")
            .await
            .unwrap_err();
        assert_eq!(crate::error::exit_code_for(&err), ExitCode::NetworkError);
        assert_eq!(names(dist.path()), vec!["karate-2.1.3.jar"]);
    }

    #[tokio::test]
    async fn verified_jar_replaces_the_old_one() {
        let dist = dist_with_old_jar();
        let url = serve(b"new").await;
        let sha = hex::encode(<sha2::Sha256 as sha2::Digest>::digest(b"new"));
        replace_karate_jar(dist.path(), "2.1.4", &url, &sha)
            .await
            .unwrap();
        assert_eq!(names(dist.path()), vec!["karate-2.1.4.jar"]);
    }
}
