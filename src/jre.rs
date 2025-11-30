//! JRE management.

use crate::platform::{KaratePaths, Os, Platform};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Minimum Java major version required for Karate 1.5.2+
pub const MIN_JAVA_VERSION: u8 = 21;

/// Source of the JRE (for diagnostics)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JreSource {
    /// Managed JRE in ~/.karate/jre or .karate/jre
    Managed,
    /// System JRE from JAVA_HOME
    JavaHome,
    /// System JRE from PATH
    Path,
}

impl std::fmt::Display for JreSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JreSource::Managed => write!(f, "managed"),
            JreSource::JavaHome => write!(f, "JAVA_HOME"),
            JreSource::Path => write!(f, "PATH"),
        }
    }
}

/// Information about an installed JRE.
#[derive(Debug, Clone)]
pub struct InstalledJre {
    pub version: String,
    pub platform: String,
    pub path: PathBuf,
    pub java_executable: PathBuf,
    pub source: JreSource,
    /// Java major version (e.g., 21 for Java 21.0.9)
    pub major_version: Option<u8>,
}

impl InstalledJre {
    /// Check if this JRE is valid and working.
    pub fn is_valid(&self) -> bool {
        self.java_executable.exists() && self.check_version().is_ok()
    }

    /// Get the Java version string.
    pub fn check_version(&self) -> Result<String> {
        let output = Command::new(&self.java_executable)
            .arg("-version")
            .output()
            .with_context(|| "Failed to run java -version")?;

        // Java prints version to stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(stderr.lines().next().unwrap_or("unknown").to_string())
    }

    /// Check if this JRE meets minimum version requirements.
    pub fn meets_minimum_version(&self) -> bool {
        self.major_version
            .map(|v| v >= MIN_JAVA_VERSION)
            .unwrap_or(false)
    }
}

/// Find the active JRE for the current platform.
///
/// Resolution order:
/// 1. Managed JRE in local .karate/jre (if exists)
/// 2. Managed JRE in global ~/.karate/jre
/// 3. System JRE from JAVA_HOME (if version >= 21)
/// 4. System JRE from PATH (if version >= 21)
pub fn find_active_jre() -> Result<Option<InstalledJre>> {
    let platform = Platform::detect()?;

    // 1 & 2: Check managed JREs (local override handled by KaratePaths)
    let jres = list_installed_jres()?;
    for jre in jres {
        if jre.platform == platform.manifest_key() && jre.is_valid() {
            return Ok(Some(jre));
        }
    }

    // 3 & 4: Fall back to system JRE
    if let Some(system_jre) = find_system_jre()? {
        if system_jre.meets_minimum_version() {
            return Ok(Some(system_jre));
        }
    }

    Ok(None)
}

/// Find system JRE from JAVA_HOME or PATH.
pub fn find_system_jre() -> Result<Option<InstalledJre>> {
    let platform = Platform::detect()?;

    // Try JAVA_HOME first
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let java_home = PathBuf::from(java_home);
        if let Some(jre) = check_java_home(&java_home, &platform) {
            return Ok(Some(jre));
        }
    }

    // Try java on PATH
    if let Some(jre) = check_java_on_path(&platform) {
        return Ok(Some(jre));
    }

    Ok(None)
}

/// Check if JAVA_HOME contains a valid Java installation.
fn check_java_home(java_home: &Path, platform: &Platform) -> Option<InstalledJre> {
    let java_name = platform.os.java_executable();
    let java_executable = java_home.join("bin").join(java_name);

    if !java_executable.exists() {
        return None;
    }

    let (version_string, major_version) = parse_java_version(&java_executable)?;

    Some(InstalledJre {
        version: version_string,
        platform: platform.manifest_key(),
        path: java_home.to_path_buf(),
        java_executable,
        source: JreSource::JavaHome,
        major_version: Some(major_version),
    })
}

/// Check if java is available on PATH.
fn check_java_on_path(platform: &Platform) -> Option<InstalledJre> {
    let java_name = platform.os.java_executable();

    // Use `which` on Unix or `where` on Windows to find java
    let java_executable = find_executable_on_path(java_name)?;

    let (version_string, major_version) = parse_java_version(&java_executable)?;

    // Try to determine JAVA_HOME from executable path (go up from bin/)
    let java_home = java_executable
        .parent() // bin/
        .and_then(|p| p.parent()) // JAVA_HOME
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| java_executable.parent().unwrap().to_path_buf());

    Some(InstalledJre {
        version: version_string,
        platform: platform.manifest_key(),
        path: java_home,
        java_executable,
        source: JreSource::Path,
        major_version: Some(major_version),
    })
}

/// Find executable on PATH.
fn find_executable_on_path(name: &str) -> Option<PathBuf> {
    #[cfg(unix)]
    {
        Command::new("which")
            .arg(name)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
    }

    #[cfg(windows)]
    {
        Command::new("where")
            .arg(name)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .next()
                    .map(|s| PathBuf::from(s.trim()))
            })
    }
}

/// Parse Java version from java -version output.
/// Returns (full version string, major version number).
fn parse_java_version(java_executable: &PathBuf) -> Option<(String, u8)> {
    let output = Command::new(java_executable)
        .arg("-version")
        .output()
        .ok()?;

    // Java prints version to stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    let first_line = stderr.lines().next()?;

    // Extract version string from first line
    // Examples:
    //   openjdk version "21.0.1" 2023-10-17
    //   java version "1.8.0_301"
    let version = extract_version_from_line(first_line)?;
    let major = parse_major_version(&version)?;

    Some((version, major))
}

/// Extract version string from java -version output line.
fn extract_version_from_line(line: &str) -> Option<String> {
    // Find quoted version string
    let start = line.find('"')? + 1;
    let end = line[start..].find('"')? + start;
    Some(line[start..end].to_string())
}

/// Parse major version from version string.
/// "21.0.1" -> 21
/// "1.8.0_301" -> 8 (legacy format)
fn parse_major_version(version: &str) -> Option<u8> {
    let first_part = version.split('.').next()?;
    let major: u8 = first_part.parse().ok()?;

    // Legacy format: 1.8 means Java 8
    if major == 1 {
        version.split('.').nth(1)?.parse().ok()
    } else {
        Some(major)
    }
}

/// List all installed (managed) JREs.
pub fn list_installed_jres() -> Result<Vec<InstalledJre>> {
    let paths = KaratePaths::new();
    let platform = Platform::detect()?;

    let mut jres = Vec::new();

    if !paths.jre.exists() {
        return Ok(jres);
    }

    for entry in std::fs::read_dir(&paths.jre)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();

        // Parse directory name: version-platform (e.g., 17.0.12-macos-aarch64)
        if let Some((version, platform_str)) = parse_jre_dir_name(dir_name) {
            let java_path = find_java_executable(&path, &platform.os);

            if let Some(java_executable) = java_path {
                // Parse major version from directory name
                let major_version = parse_major_version(&version);

                jres.push(InstalledJre {
                    version,
                    platform: platform_str,
                    path: path.clone(),
                    java_executable,
                    source: JreSource::Managed,
                    major_version,
                });
            }
        }
    }

    Ok(jres)
}

/// Parse JRE directory name into (version, platform).
fn parse_jre_dir_name(name: &str) -> Option<(String, String)> {
    // Format: version-os-arch (e.g., 21.0.9-macosx-aarch64)
    // Version contains dots, platform contains dashes
    let parts: Vec<&str> = name.splitn(2, '-').collect();
    if parts.len() == 2 {
        // Normalize JustJ platform names to our internal format
        let platform = normalize_platform(parts[1]);
        Some((parts[0].to_string(), platform))
    } else {
        None
    }
}

/// Normalize JustJ platform names to our internal format.
/// JustJ uses: macosx-aarch64, macosx-x86_64, linux-x86_64, win32-x86_64
/// We use: macos-aarch64, macos-x64, linux-x64, linux-aarch64, windows-x64
fn normalize_platform(justj_platform: &str) -> String {
    match justj_platform {
        "macosx-aarch64" => "macos-aarch64".to_string(),
        "macosx-x86_64" => "macos-x64".to_string(),
        "linux-x86_64" => "linux-x64".to_string(),
        "linux-aarch64" => "linux-aarch64".to_string(),
        "win32-x86_64" => "windows-x64".to_string(),
        other => other.to_string(),
    }
}

/// Find the java executable within a JRE directory.
fn find_java_executable(jre_dir: &PathBuf, os: &Os) -> Option<PathBuf> {
    let java_name = os.java_executable();

    // Try common JRE layouts
    let candidates = [
        jre_dir.join("bin").join(java_name),
        jre_dir.join("Contents/Home/bin").join(java_name), // macOS bundle
        jre_dir.join("jre/bin").join(java_name),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Some(candidate.clone());
        }
    }

    // Search recursively for java executable
    if let Ok(entries) = walkdir(jre_dir, java_name) {
        return entries.into_iter().next();
    }

    None
}

/// Simple recursive search for a file.
fn walkdir(dir: &PathBuf, target: &str) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();

    fn search(dir: &PathBuf, target: &str, results: &mut Vec<PathBuf>, depth: usize) -> Result<()> {
        if depth > 5 {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                search(&path, target, results, depth + 1)?;
            } else if path.file_name().and_then(|n| n.to_str()) == Some(target) {
                results.push(path);
            }
        }

        Ok(())
    }

    search(dir, target, &mut results, 0)?;
    Ok(results)
}

/// Get the JRE directory name for a version and platform.
#[allow(dead_code)]
pub fn jre_dir_name(version: &str, platform: &Platform) -> String {
    format!("{}-{}", version, platform.manifest_key())
}
