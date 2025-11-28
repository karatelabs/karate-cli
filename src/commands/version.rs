//! Version command - show version information.

use crate::cli::VersionArgs;
use crate::error::ExitCode;
use crate::jre::find_active_jre;
use crate::platform::KaratePaths;
use anyhow::Result;
use console::style;
use serde::Serialize;

/// Launcher version (from Cargo.toml)
const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
struct VersionInfo {
    launcher: String,
    karate_jar: Option<String>,
    jre: Option<String>,
    extensions: Vec<String>,
}

pub async fn run(args: VersionArgs) -> Result<ExitCode> {
    let info = build_version_info()?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&info)?);
        return Ok(ExitCode::Success);
    }

    print_version_info(&info);
    Ok(ExitCode::Success)
}

fn build_version_info() -> Result<VersionInfo> {
    let paths = KaratePaths::new();

    // Get Karate JAR version from filename
    let karate_jar = if paths.dist.exists() {
        std::fs::read_dir(&paths.dist)
            .ok()
            .and_then(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.starts_with("karate-") && name.ends_with(".jar") && !name.contains("robot") {
                            // Extract version from karate-X.Y.Z.jar
                            let without_prefix = name.strip_prefix("karate-")?;
                            let without_suffix = without_prefix.strip_suffix(".jar")?;
                            Some(without_suffix.to_string())
                        } else {
                            None
                        }
                    })
                    .max()
            })
    } else {
        None
    };

    // Get JRE version
    let jre = find_active_jre()?.map(|j| j.version);

    // Get extensions
    let extensions = if paths.ext.exists() {
        std::fs::read_dir(&paths.ext)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.ends_with(".jar") {
                            Some(name.strip_suffix(".jar").unwrap_or(&name).to_string())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    Ok(VersionInfo {
        launcher: LAUNCHER_VERSION.to_string(),
        karate_jar,
        jre,
        extensions,
    })
}

fn print_version_info(info: &VersionInfo) {
    println!(
        "{} {}",
        style("Karate CLI Launcher").bold(),
        style(&info.launcher).cyan()
    );
    println!();

    // Karate JAR
    print!("  Karate:   ");
    match &info.karate_jar {
        Some(v) => println!("{}", style(v).green()),
        None => println!("{}", style("not installed").dim()),
    }

    // JRE
    print!("  JRE:      ");
    match &info.jre {
        Some(v) => println!("{}", style(v).green()),
        None => println!("{}", style("not installed").dim()),
    }

    // Extensions
    if !info.extensions.is_empty() {
        println!("  Extensions:");
        for ext in &info.extensions {
            println!("    {} {}", style("•").dim(), ext);
        }
    }
}
