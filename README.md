# Karate CLI

Command-line launcher for the [Karate](https://github.com/karatelabs/karate) testing framework. Downloads and manages JRE and Karate JAR automatically.

> Easy to remember domain: **[karate.sh](https://karate.sh)**

## Install

**macOS / Linux:**
```bash
curl -fsSL https://karate.sh/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://karate.sh/install.ps1 | iex
```

**One-shot install** (downloads CLI + JRE + Karate JAR immediately):
```bash
# macOS / Linux
curl -fsSL https://karate.sh/install.sh | sh -s -- --all

# Windows (PowerShell)
iex "& { $(irm https://karate.sh/install.ps1) } -All"
```

## Quick Start

```bash
# First-time setup (downloads JRE and Karate JAR)
karate setup

# Run your tests
karate test.feature

# Check system status
karate doctor
```

## Commands

| Command | Description |
|---------|-------------|
| `karate setup` | Interactive setup wizard |
| `karate setup --all` | Non-interactive setup (JAR + JRE) |
| `karate setup --item jar` | Install JAR only (use system JRE) |
| `karate setup --item jre` | Install JRE only |
| `karate update` | Check for and install updates |
| `karate update --all` | Update all components non-interactively |
| `karate doctor` | System diagnostics |
| `karate doctor --json` | JSON output (for CI/scripts) |
| `karate version` | Show version info |

To update the CLI itself, re-run the install command.

All other commands pass through to Karate:

```bash
karate test.feature           # Run tests
karate -t @smoke test.feature # Run with tags
karate mock server.js         # Start mock server
```

## Configuration

Global config: `~/.karate/karate-cli.json`
Project config: `.karate/karate.json`

```json
{
  "channel": "stable",
  "karate_version": "latest",
  "jvm_opts": "-Xmx512m"
}
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `KARATE_HOME` | Override global home (default: `~/.karate`) |
| `JAVA_HOME` | System Java installation (used if Java 21+) |
| `NO_COLOR` | Disable colored output |

## Requirements

- **Java 21+** (managed automatically, or uses system Java if available)
- macOS (Intel/Apple Silicon), Linux (x64/ARM64), Windows (x64)

## Documentation

- [Architecture & Spec](docs/spec.md)
- [Contributing](CONTRIBUTING.md)

## License

MIT
