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

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// First-run setup wizard
    Setup(SetupArgs),

    /// Update Karate JAR and JRE to latest version
    Upgrade(UpgradeArgs),

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
    /// Non-interactive mode, accept all defaults
    #[arg(short, long)]
    pub yes: bool,

    #[command(subcommand)]
    pub subcommand: Option<SetupSubcommand>,
}

#[derive(Subcommand, Debug)]
pub enum SetupSubcommand {
    /// Set up PATH/symlinks only
    Path(SetupPathArgs),

    /// Install/update JRE only
    #[command(disable_version_flag = true)]
    Jre(SetupJreArgs),
}

#[derive(Args, Debug)]
pub struct SetupPathArgs {
    /// Directory to install the karate binary
    #[arg(long)]
    pub bin_dir: Option<String>,

    /// Modify shell profile to add to PATH (Unix only)
    #[arg(long)]
    pub modify_shell_profile: bool,

    /// Add to system PATH (Windows only)
    #[arg(long)]
    pub add_to_path: bool,
}

#[derive(Args, Debug)]
pub struct SetupJreArgs {
    /// Specific Java major version to install (e.g., 17, 21)
    #[arg(long = "java-version")]
    pub version: Option<String>,

    /// Force download even if a suitable system JRE is available
    #[arg(long, short)]
    pub force: bool,
}

// ============================================================================
// Upgrade command
// ============================================================================

#[derive(Args, Debug)]
pub struct UpgradeArgs {
    /// Non-interactive mode
    #[arg(short, long)]
    pub yes: bool,

    /// Install specific version instead of latest
    #[arg(long)]
    pub version: Option<String>,
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
