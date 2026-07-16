//! JAR delegation - pass commands through to the Karate JAR via JVM.

use crate::config::load_merged_config;
use crate::error::{ExitCode, KarateError};
use crate::jre::find_active_jre;
use crate::platform::KaratePaths;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run a delegated command through the JVM.
pub async fn run(args: Vec<String>, extra_classpath: &[String]) -> Result<ExitCode> {
    let paths = KaratePaths::new();
    let config = load_merged_config()?;

    // Find JRE - check config override first
    let java_executable = if let Some(jre_path) = &config.jre_path {
        find_java_in_dir(&PathBuf::from(jre_path))?
    } else {
        let jre = find_active_jre()?.ok_or(KarateError::NotBootstrapped)?;
        jre.java_executable
    };

    // Find Karate JAR - check config override first
    let dist_dir = config
        .dist_path
        .map(PathBuf::from)
        .unwrap_or_else(|| paths.dist.clone());

    let jar_path = find_karate_jar(&dist_dir, &config.karate_version)?;

    // Build classpath
    let classpath = build_classpath(&paths, &jar_path, extra_classpath)?;

    // Build JVM command
    let mut cmd = Command::new(&java_executable);

    // Add JVM opts from config
    if let Some(jvm_opts) = &config.jvm_opts {
        for opt in jvm_opts.split_whitespace() {
            cmd.arg(opt);
        }
    }

    // Add classpath
    cmd.arg("-cp").arg(&classpath);

    // Add main class
    cmd.arg("com.intuit.karate.Main");

    // Add user arguments
    cmd.args(&args);

    // Execute and wait
    let status = cmd
        .status()
        .with_context(|| "Failed to execute Karate JAR")?;

    if status.success() {
        Ok(ExitCode::Success)
    } else {
        let code = status.code().unwrap_or(1);
        // Pass through JVM exit code
        std::process::exit(ExitCode::jvm_passthrough(code));
    }
}

/// Find java executable in a JRE directory
fn find_java_in_dir(jre_dir: &Path) -> Result<PathBuf> {
    // Try common locations
    let candidates = [
        jre_dir.join("bin/java"),
        jre_dir.join("bin/java.exe"),
        jre_dir.join("Contents/Home/bin/java"), // macOS bundle
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    anyhow::bail!("Could not find java executable in {}", jre_dir.display())
}

/// Find the Karate JAR to use.
fn find_karate_jar(dist_dir: &Path, karate_version: &str) -> Result<PathBuf> {
    if !dist_dir.exists() {
        return Err(KarateError::NotBootstrapped.into());
    }

    // A pinned karate_version (anything but "latest") selects exactly that jar — newer
    // downloads sitting beside it must not win. Missing pinned jar = a hard, actionable
    // error rather than a silent fallback to some other version.
    if karate_version != "latest" {
        let pinned = dist_dir.join(format!("karate-{karate_version}.jar"));
        if pinned.exists() {
            return Ok(pinned);
        }
        anyhow::bail!(
            "karate_version is pinned to {karate_version} in config, but {} does not exist.\n\
             Install it with: karate setup --item jar --karate-version {karate_version}",
            pinned.display()
        );
    }

    // Find any karate-*.jar in dist (excluding robot JARs)
    let mut jars: Vec<_> = std::fs::read_dir(dist_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|e| e == "jar").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("karate-") && !n.contains("robot"))
                    .unwrap_or(false)
        })
        .collect();

    // Sort by name (version) descending to get latest
    jars.sort();
    jars.reverse();

    jars.into_iter()
        .next()
        .ok_or_else(|| KarateError::NotBootstrapped.into())
}

/// Build the classpath string.
/// Order: karate jar → ext jars (global + local) → extra classpath (--cp flags)
fn build_classpath(
    paths: &KaratePaths,
    jar_path: &Path,
    extra_classpath: &[String],
) -> Result<String> {
    let mut classpath_parts = vec![jar_path.to_string_lossy().to_string()];

    // Add extensions from both global and local ext directories
    for ext_dir in paths.all_ext_dirs() {
        if ext_dir.exists() {
            for entry in std::fs::read_dir(&ext_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "jar").unwrap_or(false) {
                    classpath_parts.push(path.to_string_lossy().to_string());
                }
            }
        }
    }

    // Add extra classpath entries from --cp flags
    for entry in extra_classpath {
        classpath_parts.push(entry.clone());
    }

    // Join with platform-appropriate separator
    let separator = if cfg!(windows) { ";" } else { ":" };
    Ok(classpath_parts.join(separator))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist_with(jars: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for j in jars {
            std::fs::write(dir.path().join(j), b"jar").unwrap();
        }
        dir
    }

    #[test]
    fn latest_picks_newest_jar() {
        let dist = dist_with(&["karate-1.5.2.jar", "karate-2.1.1.jar"]);
        let jar = find_karate_jar(dist.path(), "latest").unwrap();
        assert_eq!(jar.file_name().unwrap(), "karate-2.1.1.jar");
    }

    #[test]
    fn pinned_version_wins_over_newer_download() {
        let dist = dist_with(&["karate-1.5.2.jar", "karate-2.1.1.jar"]);
        let jar = find_karate_jar(dist.path(), "1.5.2").unwrap();
        assert_eq!(jar.file_name().unwrap(), "karate-1.5.2.jar");
    }

    #[test]
    fn missing_pinned_jar_is_an_actionable_error() {
        let dist = dist_with(&["karate-2.1.1.jar"]);
        let err = find_karate_jar(dist.path(), "1.5.2")
            .unwrap_err()
            .to_string();
        assert!(err.contains("pinned to 1.5.2"), "{err}");
        assert!(err.contains("--karate-version 1.5.2"), "{err}");
    }

    #[test]
    fn robot_jars_are_ignored_for_latest() {
        let dist = dist_with(&["karate-1.5.2.jar", "karate-robot-9.9.9.jar"]);
        let jar = find_karate_jar(dist.path(), "latest").unwrap();
        assert_eq!(jar.file_name().unwrap(), "karate-1.5.2.jar");
    }
}
