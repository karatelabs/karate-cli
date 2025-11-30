//! Configuration management for Karate CLI.

use crate::platform::KaratePaths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Karate CLI configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Release channel: stable, beta, nightly
    #[serde(default = "default_channel")]
    pub channel: String,

    /// Karate version or "latest"
    #[serde(default = "default_version")]
    pub karate_version: String,

    /// Explicit path to JRE directory. If null, uses ~/.karate/jre/
    /// Can be set by JavaFX installer to point to bundled JRE.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jre_path: Option<String>,

    /// Explicit path to dist directory containing Karate JAR(s).
    /// If null, uses ~/.karate/dist/
    /// Can be set by JavaFX installer to point to bundled location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dist_path: Option<String>,

    /// Additional JVM options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jvm_opts: Option<String>,

    /// Check for updates on run
    #[serde(default = "default_check_updates")]
    pub check_updates: bool,
}

fn default_channel() -> String {
    "stable".to_string()
}

fn default_version() -> String {
    "latest".to_string()
}

fn default_check_updates() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Config {
            channel: default_channel(),
            karate_version: default_version(),
            jre_path: None,
            dist_path: None,
            jvm_opts: None,
            check_updates: default_check_updates(),
        }
    }
}

impl Config {
    /// Load config from a file.
    pub fn load_from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Config::default());
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;

        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config from {}", path.display()))
    }

    /// Save config to a file.
    #[allow(dead_code)]
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;

        Ok(())
    }

    /// Merge another config into this one (other takes precedence).
    pub fn merge(&mut self, other: &Config) {
        if other.channel != default_channel() {
            self.channel = other.channel.clone();
        }
        if other.karate_version != default_version() {
            self.karate_version = other.karate_version.clone();
        }
        if other.jre_path.is_some() {
            self.jre_path = other.jre_path.clone();
        }
        if other.dist_path.is_some() {
            self.dist_path = other.dist_path.clone();
        }
        if other.jvm_opts.is_some() {
            self.jvm_opts = other.jvm_opts.clone();
        }
        if !other.check_updates {
            self.check_updates = false;
        }
    }
}

/// Load and merge all applicable configs.
/// Precedence: project config > global config > defaults
pub fn load_merged_config() -> Result<Config> {
    let paths = KaratePaths::new();

    // Start with defaults
    let mut config = Config::default();

    // Load and merge global config
    let global_config = Config::load_from_file(&paths.global_config)?;
    config.merge(&global_config);

    // Load and merge project config if it exists
    let local_config_path = KaratePaths::local_config();
    if local_config_path.exists() {
        let local_config = Config::load_from_file(&local_config_path)?;
        config.merge(&local_config);
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.channel, "stable");
        assert_eq!(config.karate_version, "latest");
        assert!(config.check_updates);
    }

    #[test]
    fn test_config_merge() {
        let mut base = Config::default();
        let override_config = Config {
            channel: "beta".to_string(),
            karate_version: "2.0.0".to_string(),
            jre_path: Some("/custom/jre".to_string()),
            dist_path: Some("/custom/dist".to_string()),
            jvm_opts: Some("-Xmx1g".to_string()),
            check_updates: false,
        };

        base.merge(&override_config);

        assert_eq!(base.channel, "beta");
        assert_eq!(base.karate_version, "2.0.0");
        assert_eq!(base.jre_path, Some("/custom/jre".to_string()));
        assert_eq!(base.dist_path, Some("/custom/dist".to_string()));
        assert_eq!(base.jvm_opts, Some("-Xmx1g".to_string()));
        assert!(!base.check_updates);
    }
}
