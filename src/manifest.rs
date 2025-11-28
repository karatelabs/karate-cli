//! Manifest parsing and management.

use crate::platform::Platform;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default manifest URL.
pub const MANIFEST_URL: &str =
    "https://github.com/karatelabs/karate-cli-manifest/releases/latest/download/manifest.json";

/// Default Karate JAR URL template (used when manifest is unavailable).
pub const DEFAULT_JAR_URL_TEMPLATE: &str =
    "https://github.com/karatelabs/karate/releases/download/v{version}/karate-{version}-all.jar";

/// Default JRE version.
pub const DEFAULT_JRE_VERSION: &str = "17.0.12";

/// Artifact with URL and checksum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// JRE configuration for a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JreConfig {
    pub version: String,
    pub platforms: HashMap<String, Artifact>,
}

/// Plugin configuration in manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestPlugin {
    pub version: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// Channel configuration (stable, beta, nightly).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub version: String,
    pub karate_jar: Artifact,
    pub jre: JreConfig,
    #[serde(default)]
    pub plugins: HashMap<String, ManifestPlugin>,
}

/// Manifest defaults for convention-over-configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManifestDefaults {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub karate_jar_url_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jre_version: Option<String>,
}

/// The full manifest structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub channels: HashMap<String, Channel>,
    #[serde(default)]
    pub defaults: ManifestDefaults,
}

fn default_schema_version() -> u32 {
    1
}

impl Manifest {
    /// Get a channel by name.
    pub fn get_channel(&self, name: &str) -> Option<&Channel> {
        self.channels.get(name)
    }

    /// Get the JRE artifact for a platform.
    pub fn get_jre_artifact(&self, channel: &str, platform: &Platform) -> Option<&Artifact> {
        self.get_channel(channel)
            .and_then(|c| c.jre.platforms.get(&platform.manifest_key()))
    }

    /// Get a plugin from a channel.
    pub fn get_plugin(&self, channel: &str, name: &str) -> Option<&ManifestPlugin> {
        self.get_channel(channel).and_then(|c| c.plugins.get(name))
    }
}

/// Build a Karate JAR URL from template and version.
pub fn build_jar_url(template: &str, version: &str) -> String {
    template.replace("{version}", version)
}

/// Create a minimal default manifest for offline/first-run use.
pub fn create_default_manifest() -> Manifest {
    Manifest {
        schema_version: 1,
        channels: HashMap::new(),
        defaults: ManifestDefaults {
            karate_jar_url_template: Some(DEFAULT_JAR_URL_TEMPLATE.to_string()),
            jre_version: Some(DEFAULT_JRE_VERSION.to_string()),
        },
    }
}

/// Fetch manifest from URL.
pub async fn fetch_manifest(url: &str) -> Result<Manifest> {
    let response = reqwest::get(url)
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

/// Load cached manifest from disk.
pub fn load_cached_manifest(cache_path: &std::path::Path) -> Result<Option<Manifest>> {
    if !cache_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(cache_path)?;
    let manifest: Manifest = serde_json::from_str(&content)?;
    Ok(Some(manifest))
}

/// Save manifest to cache.
pub fn save_manifest_cache(manifest: &Manifest, cache_path: &std::path::Path) -> Result<()> {
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
        let url = build_jar_url(DEFAULT_JAR_URL_TEMPLATE, "2.0.0");
        assert_eq!(
            url,
            "https://github.com/karatelabs/karate/releases/download/v2.0.0/karate-2.0.0-all.jar"
        );
    }
}
