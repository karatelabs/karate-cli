# Karate CLI

Command-line for the [Karate](https://github.com/karatelabs/karate) testing framework. Manages JRE and Karate JAR downloads automatically, so you can run Karate tests without Maven or Gradle.

## Quick Start

```bash
# First-time setup (downloads JRE and Karate JAR)
karate setup

# Run your tests
karate test.feature

# Check system status
karate doctor
```

## Installation

Download the binary for your platform from the releases page, or build from source:

```bash
cargo build --release
```

## Commands

### Launcher Commands (Rust CLI)

| Command | Description |
|---------|-------------|
| `karate setup` | First-run setup wizard - downloads JRE and Karate JAR |
| `karate setup jre` | Install/update JRE only |
| `karate setup jre --force` | Force download JRE even if system Java is available |
| `karate setup path` | Set up PATH/symlinks only |
| `karate upgrade` | Update Karate JAR and JRE to latest versions |
| `karate config` | View or edit configuration |
| `karate jre list` | List installed JRE versions |
| `karate jre doctor` | Check JRE health and compatibility |
| `karate ext list` | List installed extensions |
| `karate ext install <name>` | Install an extension |
| `karate ext remove <name>` | Remove an extension |
| `karate doctor` | Full system diagnostics |
| `karate doctor --json` | Output diagnostics as JSON |
| `karate version` | Show CLI version information |

### Karate Commands (Delegated to JAR)

Any command not listed above is passed through to the Karate JAR:

```bash
karate test.feature           # Run tests
karate -t @smoke test.feature # Run with tags
karate mock server.js         # Start mock server
karate --help                 # Show Karate JAR help
```

Use `--` to explicitly pass arguments to the JAR:

```bash
karate -- --help              # Show JAR help (not CLI help)
```

## Directory Structure

```
~/.karate/
├── dist/                     # Karate JAR files
│   └── karate-1.5.1.jar
├── jre/                      # Managed JRE installations
│   └── 21.0.9-macosx-aarch64/
├── ext/                      # User extension JARs
├── cache/                    # Download cache
└── karate-cli.json           # Global configuration
```

### Project-Local Overrides

Create a `.karate/` folder in your project to override specific resources:

```
my-project/
└── .karate/
    ├── ext/                  # Project-specific extensions
    └── karate.json           # Project-specific config
```

Resources are resolved with local override priority:
1. `.karate/{resource}/` in current directory (if exists)
2. `~/.karate/{resource}/` (global fallback)

## Configuration

Configuration files use JSON format:

**~/.karate/karate-cli.json** (global) or **.karate/karate.json** (project):

```json
{
  "channel": "stable",
  "karate_version": "latest",
  "jre_path": null,
  "dist_path": null,
  "jvm_opts": "-Xmx512m",
  "check_updates": true
}
```

| Field | Description | Default |
|-------|-------------|---------|
| `channel` | Release channel: `stable`, `beta`, `nightly` | `stable` |
| `karate_version` | Specific version or `latest` | `latest` |
| `jre_path` | Custom JRE path (overrides managed JRE) | `null` |
| `dist_path` | Custom JAR directory | `null` |
| `jvm_opts` | Additional JVM options | `null` |
| `check_updates` | Check for updates on run | `true` |

## JRE Resolution

The CLI finds a suitable Java runtime in this order:

1. **Managed JRE** in `.karate/jre/` (local project)
2. **Managed JRE** in `~/.karate/jre/` (global)
3. **JAVA_HOME** environment variable (if Java 21+)
4. **java on PATH** (if Java 21+)

This makes the CLI CI-friendly - it uses system Java when available, or downloads its own if needed.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `KARATE_HOME` | Override the global home directory (default: `~/.karate`) |
| `JAVA_HOME` | System Java installation path |
| `NO_COLOR` | Disable colored output |

## Requirements

- **Java 21+** required for Karate 1.5.2+
- Supported platforms: macOS (Intel/Apple Silicon), Linux (x64/ARM64), Windows (x64)

## License

MIT
