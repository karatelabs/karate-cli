# Karate CLI Development Tracker

> Last updated: 2026-02-05

## Progress Overview

| Area | Status | Notes |
|------|--------|-------|
| Core CLI | ✅ Complete | Rust binary with clap, all commands defined |
| JRE Management | ✅ Complete | JustJ integration, system JRE fallback |
| Setup Wizard | ✅ Complete | Downloads JRE + JAR automatically |
| JAR Delegation | ✅ Complete | Pass-through to Karate JAR works |
| Doctor Command | ✅ Complete | Full diagnostics with JSON output |
| GitHub Releases | ✅ Complete | v0.1.0 with all platform binaries + checksums |
| karate.sh Site | ✅ Complete | Migrated to Netlify, manifest.json live |
| Central Manifest | ✅ Complete | karate.sh/manifest.json with SHA256 checksums |
| Distribution | 🔄 In Progress | npm, Homebrew, Chocolatey pending |

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
- [x] `karate setup --item jre` with --force option

### Setup & Bootstrap
- [x] `karate setup` - Interactive wizard
- [x] `karate setup --all` - Install JAR + JRE non-interactively
- [x] `karate setup --item jar` - JAR only (use system JRE)
- [x] `karate setup --item jre` - JRE only
- [x] Downloads Karate JAR via karate.sh/manifest.json
- [x] SHA256 checksum verification enforced on downloads

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

#### Update Command ✅
- [x] `karate update` - Check and download updates
  - [x] Detect installed JAR/JRE versions from filenames
  - [x] Compare with latest from GitHub/JustJ
  - [x] Interactive confirmation before downloading
  - [x] --all flag for non-interactive
  - [x] --item flag for targeted updates

#### Config Editing
- [ ] `karate config` - Interactive editing
  - [ ] Open in $EDITOR
  - [ ] Or simple prompts for key values

#### Update Notifications
- [ ] Background update check on delegated commands
- [ ] "Update available" banner
- [ ] Configurable via check_updates setting
- [ ] **Caching**: Store last check timestamp in `~/.karate/cache/update-check.json`
  - [ ] Only ping once per day (24h TTL)
  - [ ] Cache latest version info to avoid repeated API calls
  - [ ] Respect offline mode / network errors gracefully

### Phase 2: Distribution

#### karate.sh Site & Central Manifest ✅
- [x] Shell script for Unix/macOS (install.sh)
  - [x] OS/arch detection (darwin/linux, x64/arm64)
  - [x] Fetches version + URL from karate.sh/manifest.json
  - [x] SHA256 verification from manifest
  - [x] PATH setup instructions
  - [x] --all flag for auto-setup after install
  - [x] --channel flag for stable/beta selection
- [x] PowerShell script for Windows (install.ps1)
  - [x] Same functionality
  - [x] Auto-adds to user PATH
  - [x] -Channel parameter for stable/beta selection
- [x] **Migrated to Netlify** (from AWS Amplify)
  - [x] Landing page with install instructions
  - [x] install.sh and install.ps1 served
  - [x] CORS headers for manifest.json
- [x] **Central manifest at karate.sh/manifest.json**
  - [x] Schema: artifacts, versions, channels, SHA256 checksums
  - [x] Channels: stable, beta
  - [x] CLI fetches from manifest (avoids GitHub API rate limits)
  - [x] Version pinning via config (karate_version)
  - [x] Channel selection via config (channel)
  - [x] Source: [github.com/karatelabs/karate-sh](https://github.com/karatelabs/karate-sh)

**Install URLs:**
- Unix/macOS: `curl -fsSL https://karate.sh/install.sh | sh`
- Windows: `irm https://karate.sh/install.ps1 | iex`

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

#### GitHub Actions ✅
- [x] Build workflow for all platforms
  - [x] macOS arm64
  - [x] macOS x64
  - [x] Linux x64
  - [x] Linux arm64
  - [x] Windows x64
- [x] Release workflow
  - [x] Create GitHub release
  - [x] Upload binaries (tar.gz for Unix, zip for Windows)
  - [x] Generate SHA256 checksums
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
- [ ] **Item version pinning**: `--item jar=1.5.2` or `--item jre=25`
  - [ ] Parse `item=version` syntax in --item flag
  - [ ] JAR version = full semver (e.g., 1.5.2)
  - [ ] JRE version = Java major version (e.g., 21, 25)
  - [ ] Bare item name means "latest" (backwards compatible)
  - [ ] Works for both `setup` and `update` commands

---

## Known Issues / Tech Debt

1. **JRE extraction assumes tar.gz** - Windows may need .zip support
2. **No retry logic for downloads** - Single attempt, fails on network issues
3. **Config file not created by default** - User must create manually or use --show
4. **TODO: consider switching JRE source from Eclipse JustJ to Adoptium Temurin API.** `download.eclipse.org/justj/...` has stalled for 30+ minutes during recent downloads (May 2026). Adoptium ships the same Eclipse OpenJDK build behind a much faster CDN-backed mirror (GitHub Releases via api.adoptium.net), 30-60s vs unresponsive. URL pattern: `https://api.adoptium.net/v3/binary/latest/<jdk-version>/ga/<os>/<arch>/jre/hotspot/normal/eclipse?project=jdk`. Trade-off: loses JustJ's "minimal" / "full.stripped" size variants — Adoptium ships full JRE only (~50MB compressed vs ~30MB for the stripped JustJ). For a CLI bootstrap that runs once-per-machine that size delta is likely acceptable; revisit if it matters. Implementation pattern: use `curl -fSL --retry 5 --connect-timeout 30 --max-time 600` and download-to-file-then-extract (not piped) so transport errors surface as build failures rather than corrupted tarballs.

---

## Testing Notes

### Development Testing

```bash
# Development testing (uses ./home/.karate instead of ~/.karate)
KARATE_HOME=./home/.karate cargo run -- setup
KARATE_HOME=./home/.karate cargo run -- doctor
KARATE_HOME=./home/.karate cargo run -- run --help

# Test with local .karate override
mkdir -p .karate/ext
cargo run -- doctor  # Shows local override active
```

### Netlify Migration Testing (2026-02-05) ✅ COMPLETE

- [x] **Manifest endpoint**: `curl https://karate.sh/manifest.json | jq .schema_version`
- [x] **CORS headers**: `curl -I https://karate.sh/manifest.json` includes `access-control-allow-origin: *`
- [x] **Install scripts**:
  - [x] `curl https://karate.sh/install.sh` returns script (now uses manifest)
  - [x] `curl https://karate.sh/install.ps1` returns script (now uses manifest)
- [x] **Landing page**: `curl -I https://karate.sh/` returns 200
- [x] **CLI setup from manifest**: Downloads JAR with SHA256 verification
- [x] **Channel selection**: `channel: "beta"` downloads karate 2.0.0.RC2
- [x] **Version pinning**: `karate_version: "1.5.2"` works

### Universal Installer Testing (2025-11-30)

Tested platforms via Docker and native:

| Platform | Method | Result |
|----------|--------|--------|
| macOS arm64 | Native | ✅ Full install + setup works |
| Linux arm64 | Docker (Ubuntu) | ✅ Full install + setup works |
| Linux x64 | Docker (Ubuntu) | Expected to work (same code path) |
| Windows x64 | Manual | Needs testing |

```bash
# Test macOS installation
curl -fsSL https://karate.sh/install.sh | sh -s -- --bin-dir /tmp/karate-test
/tmp/karate-test/karate version

# Test Linux installation in Docker
docker run --rm ubuntu:latest bash -c '
  apt-get update && apt-get install -y curl
  curl -fsSL https://karate.sh/install.sh | sh -s -- --bin-dir /tmp/bin --all
  /tmp/bin/karate doctor --json
'
```

---

## Architecture Quick Reference

```
karate (Rust binary)
├── Rust-native commands
│   ├── setup [--all|--item]
│   ├── update [--all|--item]
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
