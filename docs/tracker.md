# Karate CLI Development Tracker

> Last updated: 2025-11-28

## Progress Overview

| Area | Status | Notes |
|------|--------|-------|
| Core CLI | ✅ Complete | Rust binary with clap, all commands defined |
| JRE Management | ✅ Complete | JustJ integration, system JRE fallback |
| Setup Wizard | ✅ Complete | Downloads JRE + JAR automatically |
| JAR Delegation | ✅ Complete | Pass-through to Karate JAR works |
| Doctor Command | ✅ Complete | Full diagnostics with JSON output |
| Distribution | 🔲 Not Started | karate.sh, npm, Homebrew, etc. |

---

## Completed Features

### Core Infrastructure
- [x] Rust project structure with Cargo
- [x] CLI argument parsing with clap derive macros
- [x] Platform detection (OS + arch)
- [x] Two-level path resolution (global ~/.karate + local .karate/)
- [x] Exit codes per spec (0, 1, 2, 3, 4, 100+)
- [x] NO_COLOR support
- [x] KARATE_HOME environment variable override

### JRE Management
- [x] JustJ manifest parsing (same pattern as Red Hat vscode-java)
- [x] Dynamic JRE resolution by Java version + platform
- [x] JRE download and extraction (tar.gz)
- [x] System JRE detection (JAVA_HOME, PATH)
- [x] Minimum version enforcement (Java 21+)
- [x] `karate setup jre` with --force option

### Setup & Bootstrap
- [x] `karate setup` - Full wizard (JRE + JAR)
- [x] `karate setup jre` - JRE only
- [x] Downloads latest Karate JAR from GitHub releases
- [x] SHA256 checksum support infrastructure (not yet enforced)
- [x] Non-interactive mode (--yes flag)

### Diagnostics
- [x] `karate doctor` - Full system diagnostics
- [x] `karate doctor --json` - Machine-readable output
- [x] Shows: platform, JRE (active + system), JAR, extensions, config

### Configuration
- [x] `karate config --show` - Display merged config as JSON
- [x] Config file schema (channel, karate_version, jre_path, dist_path, jvm_opts, check_updates)
- [x] Config loading with defaults
- [x] Global + local config merge

### JAR Delegation
- [x] Pass-through for unknown commands (run, mock, mcp, init, etc.)
- [x] JVM opts from config
- [x] Classpath construction (JAR + ext/*.jar)
- [x] JVM exit code pass-through

### Extensions
- [x] `karate ext list` - List installed extensions
- [x] Auto-discovery of JARs in ext/ folder
- [x] Classpath inclusion for delegated commands

### Version
- [x] `karate version` - Show launcher version
- [x] `karate version --json` - JSON output

---

## In Progress

*None currently*

---

## Pending Features

### Phase 1: MVP Completion

#### Setup Path Command
- [ ] `karate setup path` - Install binary to PATH
  - [ ] Unix: symlink to ~/.local/bin or /usr/local/bin
  - [ ] Windows: copy to %LOCALAPPDATA%\Programs\Karate
  - [ ] --bin-dir override
  - [ ] --modify-shell-profile (Unix)
  - [ ] --add-to-path (Windows registry)

#### Upgrade Command
- [ ] `karate upgrade` - Check and download updates
  - [ ] Fetch manifest for latest version
  - [ ] Download new JAR if available
  - [ ] Download new JRE if available
  - [ ] --version flag for specific version
  - [ ] --yes flag for non-interactive

#### Config Editing
- [ ] `karate config` - Interactive editing
  - [ ] Open in $EDITOR
  - [ ] Or simple prompts for key values

#### Update Notifications
- [ ] Background update check on delegated commands
- [ ] "Update available" banner
- [ ] Configurable via check_updates setting

### Phase 2: Distribution

#### karate.sh Universal Installer
- [ ] Shell script for Unix/macOS
  - [ ] OS/arch detection
  - [ ] Binary download from GitHub releases
  - [ ] SHA256 verification
  - [ ] PATH setup instructions
- [ ] PowerShell script for Windows
  - [ ] Same functionality
  - [ ] Handles execution policies gracefully

#### npm Package (replace karate-npm)
- [ ] New package structure
  - [ ] package.json with bin stubs
  - [ ] postinstall.js to download Rust binary
  - [ ] Platform-specific binary selection
  - [ ] SHA256 verification
- [ ] Unix stub script (bin/karate)
- [ ] Windows stub script (bin/karate.cmd)
- [ ] Publish to npm registry

#### Homebrew
- [ ] Create homebrew-karate tap
- [ ] Formula for karate binary
- [ ] Post-install hook for setup
- [ ] Submit to homebrew-core (later)

#### Chocolatey
- [ ] Create package definition
- [ ] Installer script
- [ ] Submit to community repo

### Phase 3: CI/CD & Polish

#### GitHub Actions
- [ ] Build workflow for all platforms
  - [ ] macOS arm64
  - [ ] macOS x64
  - [ ] Linux x64
  - [ ] Linux arm64
  - [ ] Windows x64
- [ ] Release workflow
  - [ ] Create GitHub release
  - [ ] Upload binaries
  - [ ] Generate checksums
- [ ] Test workflow
  - [ ] Unit tests
  - [ ] Integration tests

#### Code Signing
- [ ] macOS notarization
- [ ] Windows code signing

#### Additional Features
- [ ] Shell completions (bash, zsh, fish, PowerShell)
- [ ] Proxy support (HTTP_PROXY, HTTPS_PROXY)
- [ ] Manifest caching with TTL
- [ ] `karate jre list` - List installed JRE versions
- [ ] `karate jre doctor` - JRE health check

### Phase 4: Future Enhancements

- [ ] `karate lock` - Freeze versions in project
- [ ] Docker images
- [ ] `karate init` templates
- [ ] Telemetry (opt-in)
- [ ] Local manifest override for air-gapped networks

---

## Known Issues / Tech Debt

1. **SHA256 verification not enforced** - Infrastructure exists but checksums not validated
2. **JRE extraction assumes tar.gz** - Windows may need .zip support
3. **No retry logic for downloads** - Single attempt, fails on network issues
4. **Config file not created by default** - User must create manually or use --show

---

## Testing Notes

```bash
# Development testing (uses ./home instead of ~/.karate)
KARATE_HOME=./home cargo run -- setup
KARATE_HOME=./home cargo run -- doctor
KARATE_HOME=./home cargo run -- run --help

# Test with local .karate override
mkdir -p .karate/ext
cargo run -- doctor  # Shows local override active
```

---

## Architecture Quick Reference

```
karate (Rust binary)
├── Rust-native commands
│   ├── setup [path|jre]
│   ├── upgrade
│   ├── config
│   ├── jre [list|doctor]
│   ├── ext [install|remove|list]
│   ├── doctor
│   └── version
│
└── JAR-delegated commands
    ├── run
    ├── mock
    ├── mcp
    ├── init
    └── <any other>
```

```
~/.karate/
├── dist/karate-X.X.X.jar
├── jre/21.0.9-macosx-aarch64/
├── ext/*.jar
├── cache/
├── karate-cli.json
├── uuid.txt        # License (NEVER DELETE)
└── karate.lic      # License (NEVER DELETE)
```
