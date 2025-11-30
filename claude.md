# Claude Code Instructions for karate-cli

## Critical Rules

1. **NEVER delete `~/.karate` folder** - It contains `uuid.txt` and `karate.lic` for license management. Only delete specific subdirectories like `~/.karate/jre/*` when needed.

2. **Use `./home/.karate` for development testing** - The git-ignored `./home` folder simulates a realistic project directory with `src/`, feature files, and a `.karate` subfolder. Set `KARATE_HOME=./home/.karate` when testing.

## Project Context

This is a Rust CLI launcher for Karate testing framework. It:
- Downloads and manages JRE from Eclipse JustJ
- Downloads Karate JAR from GitHub releases
- Delegates runtime commands (run, mock, mcp) to the JAR via JVM

## Key Architecture

- **Rust-native commands**: setup, upgrade, config, jre, ext, doctor, version
- **JAR-delegated commands**: Everything else passes through to JVM
- **JustJ pattern**: Uses `justj.manifest` for dynamic JRE resolution (same as Red Hat vscode-java)

## Two-Level Path Resolution

Resources (dist, jre, ext) are resolved with local override:
1. If `.karate/{resource}/` exists in cwd → use local
2. Otherwise → use global (`KARATE_HOME` or `~/.karate`)

This allows projects to override specific resources (e.g., local extensions) while falling back to global for others (e.g., shared JRE).

## Environment Variables

- `KARATE_HOME` - Override default `~/.karate` global home location
- `NO_COLOR` - Disable colored output

## Testing

```bash
# Use local home for development (overrides global entirely)
KARATE_HOME=./home/.karate cargo run -- setup
KARATE_HOME=./home/.karate cargo run -- doctor
KARATE_HOME=./home/.karate cargo run -- run --help

# Test with local .karate override
mkdir -p .karate/ext
cargo run -- doctor  # Shows local override active
```
