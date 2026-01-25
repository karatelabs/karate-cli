//! Platform detection and OS-specific utilities.

use crate::error::KarateError;
use std::path::PathBuf;

/// Detected platform information.
#[derive(Debug, Clone)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Os {
    MacOS,
    Linux,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Arch {
    X64,
    Aarch64,
}

impl Platform {
    /// Detect the current platform.
    pub fn detect() -> Result<Self, KarateError> {
        let os = Self::detect_os()?;
        let arch = Self::detect_arch()?;
        Ok(Platform { os, arch })
    }

    fn detect_os() -> Result<Os, KarateError> {
        cfg_if::cfg_if! {
            if #[cfg(target_os = "macos")] {
                Ok(Os::MacOS)
            } else if #[cfg(target_os = "linux")] {
                Ok(Os::Linux)
            } else if #[cfg(target_os = "windows")] {
                Ok(Os::Windows)
            } else {
                Err(KarateError::UnsupportedPlatform {
                    os: std::env::consts::OS.to_string(),
                    arch: std::env::consts::ARCH.to_string(),
                })
            }
        }
    }

    fn detect_arch() -> Result<Arch, KarateError> {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "x86_64")] {
                Ok(Arch::X64)
            } else if #[cfg(target_arch = "aarch64")] {
                Ok(Arch::Aarch64)
            } else {
                Err(KarateError::UnsupportedPlatform {
                    os: std::env::consts::OS.to_string(),
                    arch: std::env::consts::ARCH.to_string(),
                })
            }
        }
    }

    /// Get the platform string used in manifest (e.g., "macos-aarch64").
    pub fn manifest_key(&self) -> String {
        let os = match self.os {
            Os::MacOS => "macos",
            Os::Linux => "linux",
            Os::Windows => "windows",
        };
        let arch = match self.arch {
            Arch::X64 => "x64",
            Arch::Aarch64 => "aarch64",
        };
        format!("{}-{}", os, arch)
    }

    /// Get the JRE directory name for this platform.
    #[allow(dead_code)]
    pub fn jre_dir_name(&self, version: &str) -> String {
        format!("{}-{}", version, self.manifest_key())
    }
}

impl Os {
    /// Get the path to the karate home directory (~/.karate).
    #[allow(dead_code)]
    pub fn karate_home(&self) -> PathBuf {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".karate")
    }

    /// Get the default bin directory for CLI installation.
    #[allow(dead_code)]
    pub fn default_bin_dir(&self) -> PathBuf {
        match self {
            Os::MacOS | Os::Linux => dirs::home_dir()
                .expect("Could not determine home directory")
                .join(".local")
                .join("bin"),
            Os::Windows => dirs::data_local_dir()
                .expect("Could not determine local app data directory")
                .join("Programs")
                .join("Karate"),
        }
    }

    /// Get the Java executable name.
    pub fn java_executable(&self) -> &'static str {
        match self {
            Os::Windows => "java.exe",
            _ => "java",
        }
    }
}

/// Get paths to various Karate directories.
///
/// Uses a two-level resolution: local `.karate/` in cwd can override
/// specific directories (dist, jre, ext), falling back to global home
/// for anything not present locally.
pub struct KaratePaths {
    /// The global home directory (KARATE_HOME or ~/.karate)
    pub home: PathBuf,
    /// Local override directory (.karate in cwd), if it exists
    pub local: Option<PathBuf>,
    /// Resolved dist directory (local override or global)
    pub dist: PathBuf,
    /// Resolved jre directory (local override or global)
    pub jre: PathBuf,
    /// Resolved ext directory (local override or global)
    pub ext: PathBuf,
    /// Cache directory (always global)
    pub cache: PathBuf,
    /// Global config file
    pub global_config: PathBuf,
}

impl KaratePaths {
    /// Create paths with two-level resolution:
    /// 1. Check `.karate/` in current directory for local overrides
    /// 2. Fall back to global home (`KARATE_HOME` env var or `~/.karate`)
    ///
    /// For each resource (dist, jre, ext):
    /// - If local `.karate/{resource}/` exists, use it
    /// - Otherwise use global `{home}/{resource}/`
    pub fn new() -> Self {
        let home = Self::resolve_global_home();
        let local = Self::resolve_local();

        // Resolve each path with local override fallback to global
        let dist = Self::resolve_path(&local, &home, "dist");
        let jre = Self::resolve_path(&local, &home, "jre");
        let ext = Self::resolve_path(&local, &home, "ext");

        // Cache and config are always global
        let cache = home.join("cache");
        let global_config = home.join("karate-cli.json");

        KaratePaths {
            home,
            local,
            dist,
            jre,
            ext,
            cache,
            global_config,
        }
    }

    /// Resolve the global Karate home directory.
    /// Priority: KARATE_HOME env var → ~/.karate
    fn resolve_global_home() -> PathBuf {
        if let Ok(karate_home) = std::env::var("KARATE_HOME") {
            return PathBuf::from(karate_home);
        }

        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".karate")
    }

    /// Check for local .karate directory in current working directory.
    fn resolve_local() -> Option<PathBuf> {
        let local = std::env::current_dir().ok()?.join(".karate");

        if local.exists() && local.is_dir() {
            Some(local)
        } else {
            None
        }
    }

    /// Resolve a path with local override fallback to global.
    /// If local/{subdir} exists, use it. Otherwise use global/{subdir}.
    fn resolve_path(local: &Option<PathBuf>, global: &std::path::Path, subdir: &str) -> PathBuf {
        if let Some(local_dir) = local {
            let local_path = local_dir.join(subdir);
            if local_path.exists() {
                return local_path;
            }
        }
        global.join(subdir)
    }

    /// Get the project-local config path (.karate/karate.json in cwd).
    pub fn local_config() -> PathBuf {
        std::env::current_dir()
            .expect("Could not determine current directory")
            .join(".karate")
            .join("karate.json")
    }

    /// Ensure all directories exist (creates in resolved locations).
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dist)?;
        std::fs::create_dir_all(&self.jre)?;
        std::fs::create_dir_all(&self.ext)?;
        std::fs::create_dir_all(&self.cache)?;
        Ok(())
    }

    /// Check if we're using any local overrides.
    #[allow(dead_code)]
    pub fn has_local_overrides(&self) -> bool {
        self.local.is_some()
    }

    /// Get all ext directories to check (global + optional local).
    /// Extensions are composable: both global and local ext jars are loaded.
    pub fn all_ext_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = vec![self.home.join("ext")];
        if let Some(ref local) = self.local {
            let local_ext = local.join("ext");
            if local_ext.exists() {
                dirs.push(local_ext);
            }
        }
        dirs
    }
}

impl Default for KaratePaths {
    fn default() -> Self {
        Self::new()
    }
}
