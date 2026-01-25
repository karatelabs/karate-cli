//! Doctor command - full system diagnostics.

use crate::cli::DoctorArgs;
use crate::error::ExitCode;
use crate::jre::{find_active_jre, find_system_jre, MIN_JAVA_VERSION};
use crate::platform::{KaratePaths, Platform};
use anyhow::Result;
use console::style;
use serde::Serialize;

#[derive(Serialize)]
struct DoctorReport {
    platform: PlatformInfo,
    karate_home: String,
    local_override: Option<String>,
    jre: Option<JreInfo>,
    system_jre: SystemJreInfo,
    karate_jar: Option<JarInfo>,
    extensions: Vec<String>,
    config: ConfigInfo,
}

#[derive(Serialize)]
struct PlatformInfo {
    os: String,
    arch: String,
    key: String,
}

#[derive(Serialize)]
struct JreInfo {
    version: String,
    path: String,
    executable: String,
    valid: bool,
    source: String,
    major_version: Option<u8>,
}

#[derive(Serialize)]
struct SystemJreInfo {
    available: bool,
    version: Option<String>,
    major_version: Option<u8>,
    source: Option<String>,
    path: Option<String>,
    meets_minimum: bool,
}

#[derive(Serialize)]
struct JarInfo {
    path: String,
    filename: String,
}

#[derive(Serialize)]
struct ConfigInfo {
    global_exists: bool,
    global_path: String,
    local_exists: bool,
    local_path: String,
}

pub async fn run(args: DoctorArgs) -> Result<ExitCode> {
    let report = build_report()?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(ExitCode::Success);
    }

    print_report(&report);
    Ok(ExitCode::Success)
}

fn build_report() -> Result<DoctorReport> {
    let platform = Platform::detect()?;
    let paths = KaratePaths::new();

    // JRE info (active JRE - could be managed or system)
    let jre = find_active_jre()?.map(|j| JreInfo {
        version: j.version.clone(),
        path: j.path.to_string_lossy().to_string(),
        executable: j.java_executable.to_string_lossy().to_string(),
        valid: j.is_valid(),
        source: j.source.to_string(),
        major_version: j.major_version,
    });

    // System JRE info (always check, for diagnostics)
    let system_jre = match find_system_jre()? {
        Some(j) => SystemJreInfo {
            available: true,
            version: Some(j.version.clone()),
            major_version: j.major_version,
            source: Some(j.source.to_string()),
            path: Some(j.path.to_string_lossy().to_string()),
            meets_minimum: j.meets_minimum_version(),
        },
        None => SystemJreInfo {
            available: false,
            version: None,
            major_version: None,
            source: None,
            path: None,
            meets_minimum: false,
        },
    };

    // Karate JAR info
    let karate_jar = find_karate_jar(&paths);

    // Extensions (from both global and local ext directories)
    let extensions: Vec<String> = paths
        .all_ext_dirs()
        .iter()
        .flat_map(|dir| list_jars(dir))
        .collect();

    // Config info
    let local_config_path = KaratePaths::local_config();
    let config = ConfigInfo {
        global_exists: paths.global_config.exists(),
        global_path: paths.global_config.to_string_lossy().to_string(),
        local_exists: local_config_path.exists(),
        local_path: local_config_path.to_string_lossy().to_string(),
    };

    Ok(DoctorReport {
        platform: PlatformInfo {
            os: format!("{:?}", platform.os),
            arch: format!("{:?}", platform.arch),
            key: platform.manifest_key(),
        },
        karate_home: paths.home.to_string_lossy().to_string(),
        local_override: paths
            .local
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        jre,
        system_jre,
        karate_jar,
        extensions,
        config,
    })
}

fn find_karate_jar(paths: &KaratePaths) -> Option<JarInfo> {
    if !paths.dist.exists() {
        return None;
    }

    std::fs::read_dir(&paths.dist)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.extension().map(|ext| ext == "jar").unwrap_or(false)
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("karate-") && !n.contains("robot"))
                    .unwrap_or(false)
        })
        .max_by_key(|e| e.file_name())
        .map(|e| JarInfo {
            path: e.path().to_string_lossy().to_string(),
            filename: e.file_name().to_string_lossy().to_string(),
        })
}

fn list_jars(dir: &std::path::Path) -> Vec<String> {
    if !dir.exists() {
        return Vec::new();
    }

    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "jar")
                        .unwrap_or(false)
                })
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn print_report(report: &DoctorReport) {
    println!("{} Karate CLI Diagnostics", style("▶").cyan().bold());
    println!();

    // Platform
    println!("{}", style("Platform").bold().underlined());
    println!("  OS:   {}", style(&report.platform.os).green());
    println!("  Arch: {}", style(&report.platform.arch).green());
    println!("  Key:  {}", style(&report.platform.key).dim());
    println!();

    // Karate Home
    println!("{}", style("Karate Home").bold().underlined());
    println!("  Global: {}", report.karate_home);
    if let Some(local) = &report.local_override {
        println!("  Local:  {} {}", local, style("(active)").cyan());
    }
    println!();

    // JRE (Active)
    println!("{}", style("JRE (Active)").bold().underlined());
    match &report.jre {
        Some(jre) => {
            let status = if jre.valid {
                style("✓").green()
            } else {
                style("✗").red()
            };
            println!(
                "  Status:     {} {}",
                status,
                if jre.valid { "OK" } else { "Invalid" }
            );
            println!("  Source:     {}", style(&jre.source).cyan());
            println!("  Version:    {}", style(&jre.version).green());
            if let Some(major) = jre.major_version {
                println!("  Java:       {}", style(format!("Java {}", major)).green());
            }
            println!("  Path:       {}", jre.path);
            println!("  Executable: {}", style(&jre.executable).dim());
        }
        None => {
            println!("  Status: {} Not available", style("✗").red());
            println!("  Run {} to install", style("karate setup").cyan());
        }
    }
    println!();

    // System JRE
    println!("{}", style("System JRE").bold().underlined());
    if report.system_jre.available {
        let version = report.system_jre.version.as_deref().unwrap_or("unknown");
        let source = report.system_jre.source.as_deref().unwrap_or("unknown");
        let path = report.system_jre.path.as_deref().unwrap_or("unknown");

        if report.system_jre.meets_minimum {
            println!(
                "  Status:  {} Java {} (meets minimum)",
                style("✓").green(),
                report.system_jre.major_version.unwrap_or(0)
            );
        } else {
            println!(
                "  Status:  {} Java {} (requires {}+)",
                style("!").yellow(),
                report.system_jre.major_version.unwrap_or(0),
                MIN_JAVA_VERSION
            );
        }
        println!("  Source:  {}", style(source).cyan());
        println!("  Version: {}", version);
        println!("  Path:    {}", style(path).dim());
    } else {
        println!("  Status: {} Not found", style("-").dim());
        println!("  {}", style("No JAVA_HOME or java on PATH").dim());
    }
    println!();

    // Karate JAR
    println!("{}", style("Karate JAR").bold().underlined());
    match &report.karate_jar {
        Some(jar) => {
            println!("  Status: {} Installed", style("✓").green());
            println!("  File:   {}", style(&jar.filename).green());
            println!("  Path:   {}", style(&jar.path).dim());
        }
        None => {
            println!("  Status: {} Not installed", style("✗").red());
            println!("  Run {} to install", style("karate setup").cyan());
        }
    }
    println!();

    // Extensions
    println!("{}", style("Extensions (ext/)").bold().underlined());
    if report.extensions.is_empty() {
        println!("  {}", style("None").dim());
    } else {
        for ext in &report.extensions {
            println!("  {} {}", style("•").cyan(), ext);
        }
    }
    println!();

    // Config
    println!("{}", style("Configuration").bold().underlined());
    if report.config.global_exists {
        println!("  Global: {} {}", style("✓").green(), report.config.global_path);
    } else {
        println!(
            "  Global: {}",
            style(format!("(none) create with: karate config --global")).dim()
        );
    }
    if report.config.local_exists {
        println!("  Local:  {} {}", style("✓").green(), report.config.local_path);
    } else {
        println!(
            "  Local:  {}",
            style(format!("(none) create with: karate config --local")).dim()
        );
    }
}
