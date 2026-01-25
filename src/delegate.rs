//! JAR delegation - pass commands through to the Karate JAR via JVM.

use crate::config::load_merged_config;
use crate::error::{ExitCode, KarateError};
use crate::jre::find_active_jre;
use crate::platform::KaratePaths;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run a delegated command through the JVM.
pub async fn run(args: Vec<String>) -> Result<ExitCode> {
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

    let jar_path = find_karate_jar(&dist_dir)?;

    // Build classpath
    let classpath = build_classpath(&paths, &jar_path)?;

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
fn find_karate_jar(dist_dir: &Path) -> Result<PathBuf> {
    if !dist_dir.exists() {
        return Err(KarateError::NotBootstrapped.into());
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
fn build_classpath(paths: &KaratePaths, jar_path: &Path) -> Result<String> {
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

    // Join with platform-appropriate separator
    let separator = if cfg!(windows) { ";" } else { ":" };
    Ok(classpath_parts.join(separator))
}
