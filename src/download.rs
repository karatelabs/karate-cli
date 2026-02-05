//! File downloading with progress and checksum verification.

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::io::AsyncWriteExt;

/// GitHub release info (kept as fallback if manifest unavailable)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub assets: Vec<GitHubAsset>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
}

/// Fetch latest release info from GitHub (fallback if manifest unavailable)
#[allow(dead_code)]
pub async fn fetch_latest_release(owner: &str, repo: &str) -> Result<GitHubRelease> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        owner, repo
    );

    let client = reqwest::Client::builder()
        .user_agent("karate-cli")
        .build()?;

    let response = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch release info from {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch release: HTTP {}",
            response.status().as_u16()
        );
    }

    response
        .json()
        .await
        .with_context(|| "Failed to parse release JSON")
}

/// JustJ JRE download info resolved from manifest
#[derive(Debug)]
pub struct JustJInfo {
    pub download_url: String,
    pub version_label: String,
}

/// Map our platform key to JustJ platform suffix
fn to_justj_platform(platform: &str) -> &str {
    match platform {
        "macos-aarch64" => "macosx-aarch64",
        "macos-x64" => "macosx-x86_64",
        "linux-x64" => "linux-x86_64",
        "linux-aarch64" => "linux-aarch64",
        "windows-x64" => "win32-x86_64",
        _ => platform,
    }
}

/// Fetch JustJ manifest and resolve download URL for a platform.
/// This follows the same pattern as Red Hat's vscode-java extension.
pub async fn resolve_justj_jre(java_version: u8, platform: &str) -> Result<JustJInfo> {
    let justj_platform = to_justj_platform(platform);
    let manifest_url = format!(
        "https://download.eclipse.org/justj/jres/{}/downloads/latest/justj.manifest",
        java_version
    );

    let client = reqwest::Client::builder()
        .user_agent("karate-cli")
        .build()?;

    let response = client
        .get(&manifest_url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch JustJ manifest from {}", manifest_url))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch JustJ manifest: HTTP {}\n\n\
            This could mean Java {} is not available from JustJ.\n\
            Check available versions at: https://download.eclipse.org/justj/jres/",
            response.status().as_u16(),
            java_version
        );
    }

    let manifest = response.text().await?;

    // Find the full.stripped JRE for our platform
    // Pattern: org.eclipse.justj.openjdk.hotspot.jre.full.stripped-{version}-{platform}.tar.gz
    let jre_entry = manifest
        .lines()
        .find(|line| {
            line.contains("org.eclipse.justj.openjdk.hotspot.jre.full.stripped")
                && line.contains(justj_platform)
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "JustJ does not provide JRE {} for platform '{}'\n\n\
                Check supported platforms at: {}\n\n\
                Workaround: Set 'jre_path' in config to use a manually installed JRE:\n  \
                karate config --global",
                java_version,
                platform,
                manifest_url
            )
        })?;

    // Entry format: ../20251104_1502/org.eclipse.justj...tar.gz
    // We need the filename part after the last /
    let filename = jre_entry.rsplit('/').next().unwrap_or(jre_entry);

    let download_url = format!(
        "https://download.eclipse.org/justj/jres/{}/downloads/latest/{}",
        java_version, jre_entry
    );

    // Extract version label from filename
    // e.g., org.eclipse.justj.openjdk.hotspot.jre.full.stripped-21.0.9-macosx-aarch64.tar.gz
    // -> 21.0.9-macosx-aarch64
    let version_label = filename
        .strip_prefix("org.eclipse.justj.openjdk.hotspot.jre.full.stripped-")
        .and_then(|s| s.strip_suffix(".tar.gz"))
        .unwrap_or(filename)
        .to_string();

    Ok(JustJInfo {
        download_url,
        version_label,
    })
}

/// Download a file with progress indication.
pub async fn download_file(url: &str, dest: &Path, expected_sha256: Option<&str>) -> Result<()> {
    let client = reqwest::Client::new();

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to start download from {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!("Download failed: HTTP {}", response.status().as_u16());
    }

    let total_size = response.content_length();

    // Set up progress bar
    let pb = if let Some(size) = total_size {
        let pb = ProgressBar::new(size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {bytes}")
                .unwrap(),
        );
        pb
    };

    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Download to a temp file first
    let temp_path = dest.with_extension("tmp");
    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .with_context(|| format!("Failed to create file {}", temp_path.display()))?;

    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.with_context(|| "Failed to read download chunk")?;
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        pb.inc(chunk.len() as u64);
    }

    file.flush().await?;
    drop(file);

    pb.finish_with_message("Download complete");

    // Verify checksum if provided
    if let Some(expected) = expected_sha256 {
        let actual = hex::encode(hasher.finalize());
        if actual != expected.to_lowercase() {
            // Clean up temp file
            let _ = std::fs::remove_file(&temp_path);
            anyhow::bail!(
                "Checksum mismatch for {}: expected {}, got {}",
                dest.display(),
                expected,
                actual
            );
        }
    }

    // Move temp file to final destination
    std::fs::rename(&temp_path, dest).with_context(|| {
        format!(
            "Failed to move {} to {}",
            temp_path.display(),
            dest.display()
        )
    })?;

    Ok(())
}

/// Calculate SHA256 of a file.
#[allow(dead_code)]
pub fn calculate_sha256(path: &Path) -> Result<String> {
    let content = std::fs::read(path)?;
    let hash = Sha256::digest(&content);
    Ok(hex::encode(hash))
}

/// Extract a tar.gz archive.
pub fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let file =
        std::fs::File::open(archive_path).with_context(|| "Failed to open archive for reading")?;

    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    std::fs::create_dir_all(dest_dir)?;
    archive
        .unpack(dest_dir)
        .with_context(|| "Failed to extract tar.gz archive")?;

    Ok(())
}

/// Extract a zip archive.
#[allow(dead_code)]
pub fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let file =
        std::fs::File::open(archive_path).with_context(|| "Failed to open archive for reading")?;

    let mut archive = zip::ZipArchive::new(file)?;

    std::fs::create_dir_all(dest_dir)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = dest_dir.join(file.mangled_name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        // Set permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
            }
        }
    }

    Ok(())
}
