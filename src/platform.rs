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
        Self::resolve(Self::resolve_global_home(), std::env::current_dir().ok())
    }

    fn resolve(home: PathBuf, cwd: Option<PathBuf>) -> Self {
        let local = cwd.and_then(|dir| Self::resolve_local(&dir));

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

    /// Check for local .karate directory in the given working directory.
    fn resolve_local(cwd: &std::path::Path) -> Option<PathBuf> {
        let local = cwd.join(".karate");

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

    /// Get the project-local config path (.karate/karate-cli.json in cwd).
    pub fn local_config() -> PathBuf {
        std::env::current_dir()
            .expect("Could not determine current directory")
            .join(".karate")
            .join("karate-cli.json")
    }

    /// Ensure all directories exist (creates in resolved locations).
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dist)?;
        std::fs::create_dir_all(&self.jre)?;
        std::fs::create_dir_all(&self.ext)?;
        std::fs::create_dir_all(&self.cache)?;
        Ok(())
    }

    /// Resources resolved from the local `.karate/` instead of the global home.
    /// `ext` is not listed: local extensions are added to the global ones, not swapped in.
    pub fn local_overrides(&self) -> Vec<&'static str> {
        let Some(local) = &self.local else {
            return Vec::new();
        };
        if same_dir(local, &self.home) {
            return Vec::new();
        }
        [("dist", &self.dist), ("jre", &self.jre)]
            .into_iter()
            .filter(|(name, path)| **path == local.join(name))
            .map(|(name, _)| name)
            .collect()
    }

    /// One line naming the local overrides, for commands that install, update or resolve
    /// components. `None` when everything resolves from the global home.
    pub fn local_override_notice(&self) -> Option<String> {
        let overrides = self.local_overrides();
        if overrides.is_empty() {
            return None;
        }
        let names: Vec<String> = overrides.iter().map(|r| format!(".karate/{r}")).collect();
        let (verb, pronoun) = if names.len() == 1 {
            ("overrides", "it")
        } else {
            ("override", "them")
        };
        Some(format!(
            "{} in this directory {verb} the global home {}; remove {pronoun} or run from another directory to use the global one",
            names.join(" and "),
            self.home.display(),
        ))
    }

    /// Print `local_override_notice` to stderr, so it never mixes into delegated output.
    pub fn print_local_override_notice(&self) {
        if let Some(notice) = self.local_override_notice() {
            eprintln!("{} {notice}", console::style("note:").yellow().bold());
        }
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

/// The version in a `karate-<version>.jar` file name; `None` for any other file, robot JARs included.
pub fn karate_jar_version(path: &std::path::Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let version = name.strip_prefix("karate-")?.strip_suffix(".jar")?;
    (!name.contains("robot")).then(|| version.to_string())
}

/// The highest-versioned Karate JAR in `dist`, as `(version, path)`.
pub fn highest_karate_jar(dist: &std::path::Path) -> Option<(String, PathBuf)> {
    std::fs::read_dir(dist)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter_map(|p| karate_jar_version(&p).map(|v| (v, p)))
        .max_by(|(a, _), (b, _)| compare_versions(a, b))
}

/// Compares dotted versions numerically (2.1.10 > 2.1.4); a release outranks its
/// qualified pre-release (2.1.5 > 2.1.5.RC1).
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let split = |v: &str| v.split(['.', '-']).map(str::to_string).collect::<Vec<_>>();
    let (a, b) = (split(a), split(b));
    for i in 0..a.len().max(b.len()) {
        let order = match (a.get(i), b.get(i)) {
            (Some(x), Some(y)) => match (x.parse::<u64>(), y.parse::<u64>()) {
                (Ok(x), Ok(y)) => x.cmp(&y),
                (Ok(_), Err(_)) => Ordering::Greater,
                (Err(_), Ok(_)) => Ordering::Less,
                (Err(_), Err(_)) => x.cmp(y),
            },
            (Some(x), None) if x.parse::<u64>().is_err() => Ordering::Less,
            (None, Some(y)) if y.parse::<u64>().is_err() => Ordering::Greater,
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    Ordering::Equal
}

fn same_dir(a: &std::path::Path, b: &std::path::Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(local_entries: &[&str]) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("global");
        let cwd = root.path().join("project");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(cwd.join(".karate")).unwrap();
        for entry in local_entries {
            let path = cwd.join(".karate").join(entry);
            if entry.contains('.') {
                std::fs::write(path, b"x").unwrap();
            } else {
                std::fs::create_dir_all(path).unwrap();
            }
        }
        (root, home, cwd)
    }

    #[test]
    fn versions_compare_numerically_and_releases_outrank_pre_releases() {
        use std::cmp::Ordering::*;
        assert_eq!(compare_versions("2.1.10", "2.1.4"), Greater);
        assert_eq!(compare_versions("2.1.5", "2.1.5.RC1"), Greater);
        assert_eq!(compare_versions("2.1.5.RC1", "2.1.4"), Greater);
        assert_eq!(compare_versions("1.5.2", "1.5.2"), Equal);
    }

    #[test]
    fn licence_only_local_folder_overrides_nothing() {
        let (_root, home, cwd) = project(&["karate.lic"]);
        let paths = KaratePaths::resolve(home.clone(), Some(cwd));
        assert_eq!(paths.dist, home.join("dist"));
        assert_eq!(paths.jre, home.join("jre"));
        assert!(paths.local_overrides().is_empty());
        assert_eq!(paths.local_override_notice(), None);
    }

    #[test]
    fn local_dist_is_named_in_the_notice() {
        let (_root, home, cwd) = project(&["dist", "ext", "karate.lic"]);
        let paths = KaratePaths::resolve(home.clone(), Some(cwd.clone()));
        assert_eq!(paths.dist, cwd.join(".karate").join("dist"));
        assert_eq!(paths.jre, home.join("jre"));
        assert_eq!(paths.local_overrides(), vec!["dist"]);
        let notice = paths.local_override_notice().unwrap();
        assert!(
            notice.starts_with(".karate/dist in this directory overrides"),
            "{notice}"
        );
        assert!(notice.contains(&home.display().to_string()), "{notice}");
    }

    #[test]
    fn local_dist_and_jre_are_both_named() {
        let (_root, home, cwd) = project(&["dist", "jre"]);
        let paths = KaratePaths::resolve(home, Some(cwd));
        assert_eq!(paths.local_overrides(), vec!["dist", "jre"]);
        assert!(paths
            .local_override_notice()
            .unwrap()
            .contains(".karate/dist and .karate/jre in this directory override"));
    }

    #[test]
    fn karate_home_pointing_at_the_local_folder_is_not_an_override() {
        let (_root, _home, cwd) = project(&["dist", "jre"]);
        let paths = KaratePaths::resolve(cwd.join(".karate"), Some(cwd));
        assert!(paths.local_overrides().is_empty());
    }
}
