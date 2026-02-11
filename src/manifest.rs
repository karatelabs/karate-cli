//! Manifest parsing and management for karate.sh releases.json

use crate::platform::Platform;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manifest URL at karate.sh
pub const MANIFEST_URL: &str = "https://karate.sh/manifest.json";

/// Fallback: Default Karate JAR URL template (used when manifest is unavailable).
#[allow(dead_code)]
pub const DEFAULT_JAR_URL_TEMPLATE: &str =
    "https://github.com/karatelabs/karate/releases/download/v{version}/karate-{version}.jar";

/// Platform-specific artifact (for CLI binaries)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformArtifact {
    pub url: String,
    pub sha256: String,
}

/// A specific version of an artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactVersion {
    pub channels: Vec<String>,
    pub released_at: String,
    /// Direct URL for platform-independent artifacts (JARs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// SHA256 for platform-independent artifacts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Platform-specific downloads (for CLI binaries)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub platforms: HashMap<String, PlatformArtifact>,
    /// Dependencies on other artifacts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
}

/// An artifact in the manifest (karate-cli, karate, extensions, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(default, rename = "type")]
    pub artifact_type: Option<String>,
    pub versions: HashMap<String, ArtifactVersion>,
}

/// The full releases.json manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleasesManifest {
    pub schema_version: u32,
    pub generated_at: String,
    pub artifacts: HashMap<String, Artifact>,
    pub channel_defaults: HashMap<String, HashMap<String, String>>,
}

impl ReleasesManifest {
    /// Get an artifact by ID
    pub fn get_artifact(&self, id: &str) -> Option<&Artifact> {
        self.artifacts.get(id)
    }

    /// Get the default version for an artifact in a channel
    pub fn get_default_version(&self, artifact_id: &str, channel: &str) -> Option<&String> {
        self.channel_defaults
            .get(channel)
            .and_then(|defaults| defaults.get(artifact_id))
    }

    /// Get artifact version info
    pub fn get_version(&self, artifact_id: &str, version: &str) -> Option<&ArtifactVersion> {
        self.get_artifact(artifact_id)
            .and_then(|a| a.versions.get(version))
    }

    /// Get the latest version for an artifact in a channel (from channel_defaults)
    pub fn get_latest_version(&self, artifact_id: &str, channel: &str) -> Option<&str> {
        self.get_default_version(artifact_id, channel)
            .map(|s| s.as_str())
    }

    /// Get download info for a platform-independent artifact (like karate JAR)
    pub fn get_jar_download(&self, artifact_id: &str, version: &str) -> Option<(&str, &str)> {
        self.get_version(artifact_id, version)
            .and_then(|v| match (&v.url, &v.sha256) {
                (Some(url), Some(sha)) => Some((url.as_str(), sha.as_str())),
                _ => None,
            })
    }

    /// Get download info for a platform-specific artifact (like karate-cli binary)
    pub fn get_platform_download(
        &self,
        artifact_id: &str,
        version: &str,
        platform: &Platform,
    ) -> Option<&PlatformArtifact> {
        self.get_version(artifact_id, version)
            .and_then(|v| v.platforms.get(&platform.manifest_key()))
    }

    /// Get all versions of an artifact in a specific channel
    #[allow(dead_code)]
    pub fn get_versions_in_channel(&self, artifact_id: &str, channel: &str) -> Vec<&str> {
        self.get_artifact(artifact_id)
            .map(|a| {
                a.versions
                    .iter()
                    .filter(|(_, v)| v.channels.contains(&channel.to_string()))
                    .map(|(version, _)| version.as_str())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Fetch manifest from karate.sh (or KARATE_MANIFEST_URL override for testing)
pub async fn fetch_manifest() -> Result<ReleasesManifest> {
    let url = std::env::var("KARATE_MANIFEST_URL").unwrap_or_else(|_| MANIFEST_URL.to_string());
    fetch_manifest_from_url(&url).await
}

/// Fetch manifest from a specific URL
pub async fn fetch_manifest_from_url(url: &str) -> Result<ReleasesManifest> {
    let client = reqwest::Client::builder()
        .user_agent("karate-cli")
        .build()?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch manifest from {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch manifest: HTTP {}",
            response.status().as_u16()
        );
    }

    response
        .json()
        .await
        .with_context(|| "Failed to parse manifest JSON")
}

/// Build a Karate JAR URL from template and version (fallback when manifest unavailable).
#[allow(dead_code)]
pub fn build_jar_url(template: &str, version: &str) -> String {
    template.replace("{version}", version)
}

/// Load cached manifest from disk.
#[allow(dead_code)]
pub fn load_cached_manifest(cache_path: &std::path::Path) -> Result<Option<ReleasesManifest>> {
    if !cache_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(cache_path)?;
    let manifest: ReleasesManifest = serde_json::from_str(&content)?;
    Ok(Some(manifest))
}

/// Save manifest to cache.
pub fn save_manifest_cache(
    manifest: &ReleasesManifest,
    cache_path: &std::path::Path,
) -> Result<()> {
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(manifest)?;
    std::fs::write(cache_path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_jar_url() {
        let url = build_jar_url(DEFAULT_JAR_URL_TEMPLATE, "1.5.2");
        assert_eq!(
            url,
            "https://github.com/karatelabs/karate/releases/download/v1.5.2/karate-1.5.2.jar"
        );
    }

    #[test]
    fn test_parse_manifest() {
        let json = r#"{
            "schema_version": 1,
            "generated_at": "2025-02-05T00:00:00Z",
            "artifacts": {
                "karate": {
                    "description": "Karate Core",
                    "versions": {
                        "1.5.2": {
                            "channels": ["stable"],
                            "released_at": "2025-01-01T00:00:00Z",
                            "url": "https://example.com/karate-1.5.2.jar",
                            "sha256": "abc123"
                        }
                    }
                }
            },
            "channel_defaults": {
                "stable": {
                    "karate": "1.5.2"
                }
            }
        }"#;

        let manifest: ReleasesManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(
            manifest.get_latest_version("karate", "stable"),
            Some("1.5.2")
        );

        let (url, sha) = manifest.get_jar_download("karate", "1.5.2").unwrap();
        assert_eq!(url, "https://example.com/karate-1.5.2.jar");
        assert_eq!(sha, "abc123");
    }
}
