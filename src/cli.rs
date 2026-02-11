//! CLI argument parsing using clap derive macros.

use clap::{Args, Parser, Subcommand};

/// Karate CLI - setup and launcher for the Karate automation framework
#[derive(Parser, Debug)]
#[command(name = "karate")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Disable colored output
    #[arg(long, global = true, env = "NO_COLOR")]
    pub no_color: bool,

    /// Additional classpath entries (JAR files or directories) appended to the JVM classpath.
    /// Can be specified multiple times. Only applies to JAR-delegated commands.
    #[arg(long = "cp", global = true, num_args = 1)]
    pub extra_classpath: Vec<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// First-run setup wizard
    Setup(SetupArgs),

    /// Update Karate JAR and JRE to latest versions
    Update(UpdateArgs),

    /// View or edit configuration
    Config(ConfigArgs),

    /// JRE management
    Jre(JreArgs),

    /// Extensions and plugins
    Ext(PluginArgs),

    /// System diagnostics
    Doctor(DoctorArgs),

    /// Show version information
    Version(VersionArgs),

    /// Pass-through to Karate JAR (run, mock, mcp, init, etc.)
    #[command(external_subcommand)]
    External(Vec<String>),
}

// ============================================================================
// Setup command
// ============================================================================

#[derive(Args, Debug)]
pub struct SetupArgs {
    /// Install all components (JAR + JRE) non-interactively
    #[arg(long, conflicts_with = "item")]
    pub all: bool,

    /// Install specific item: jar, jre
    #[arg(long)]
    pub item: Option<String>,

    /// Force download even if components are already installed
    #[arg(long, short)]
    pub force: bool,

    /// Specific Java major version to install (e.g., 17, 21)
    #[arg(long = "java-version")]
    pub java_version: Option<String>,

    /// Karate JAR version to install (e.g., 1.5.2, 2.0.0)
    #[arg(long = "karate-version")]
    pub karate_version: Option<String>,

    /// Release channel: stable or beta (overrides config)
    #[arg(long)]
    pub channel: Option<String>,
}

// ============================================================================
// Update command
// ============================================================================

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Update all components non-interactively
    #[arg(long, conflicts_with = "item")]
    pub all: bool,

    /// Update specific item: jar, jre
    #[arg(long)]
    pub item: Option<String>,

    /// Release channel: stable or beta (overrides config)
    #[arg(long)]
    pub channel: Option<String>,
}

// ============================================================================
// Config command
// ============================================================================

#[derive(Args, Debug)]
pub struct ConfigArgs {
    /// Edit global config (~/.karate/karate-cli.json)
    #[arg(long, conflicts_with = "local")]
    pub global: bool,

    /// Edit project config (./.karate/karate.json)
    #[arg(long, conflicts_with = "global")]
    pub local: bool,

    /// Print resolved (merged) config as JSON
    #[arg(long)]
    pub show: bool,
}

// ============================================================================
// JRE command
// ============================================================================

#[derive(Args, Debug)]
pub struct JreArgs {
    #[command(subcommand)]
    pub subcommand: JreSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum JreSubcommand {
    /// List installed JRE versions
    List,

    /// Check JRE health and compatibility
    Doctor,
}

// ============================================================================
// Plugin command
// ============================================================================

#[derive(Args, Debug)]
pub struct PluginArgs {
    #[command(subcommand)]
    pub subcommand: PluginSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum PluginSubcommand {
    /// Install a plugin
    Install(PluginInstallArgs),

    /// Remove a plugin
    Remove(PluginRemoveArgs),

    /// List installed plugins
    List,
}

#[derive(Args, Debug)]
pub struct PluginInstallArgs {
    /// Plugin name with optional version (e.g., xplorer@1.3.0)
    pub name: String,
}

#[derive(Args, Debug)]
pub struct PluginRemoveArgs {
    /// Plugin name to remove
    pub name: String,
}

// ============================================================================
// Doctor command
// ============================================================================

#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

// ============================================================================
// Version command
// ============================================================================

#[derive(Args, Debug)]
pub struct VersionArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}
