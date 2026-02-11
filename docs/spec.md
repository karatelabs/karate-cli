# **Karate CLI Launcher – Architecture & Requirements**

## **1. Project Intent**

We are redesigning Karate to be **installable, runnable, and maintainable as a first-class CLI tool**, independent of Maven/Gradle.
The primary motivations:

* **Dead-simple onboarding** for developers and LLM coding agents.
* **Consistent CLI UX** across macOS, Windows, Linux.
* **No need for users to manage Java** — the CLI will bootstrap and manage the JRE/JARs automatically.
* **Lightweight, durable, secure distribution** using a **small Rust binary** as the launcher.
* **Pluggability** for commercial extensions (Xplorer, MCP server, etc.)
* **Optional GUI-driven installation** for paranoid/corporate teams using our JavaFX notarized app.

This document outlines the design and requirements for the first phase of the Karate CLI Launcher.

---

# **2. High-Level Architecture**

## **2.1 Components**

### **A. Rust Launcher (`karate`) — the Core**

A small native binary (5–15 MB) built with Rust:

* Entry point for all user and LLM workflows.
* Detects OS/architecture.
* Reads/manages Karate config.
* Downloads/installs/updates:

  * Karate fat JARs
  * JustJ JRE (per platform)
  * Plugin JARs
* Executes JVM with consistent CLI semantics.
* Manages CLI setup (symlinks, PATH hints).
* Provides machine-readable JSON output modes.

### **B. Karate Runtime Files (~/.karate/)**

Managed by the launcher:

```
~/.karate/
   dist/
      karate-<version>.jar
   jre/
      <version-os-arch>/
   ext/                        # User-provided extension JARs
      custom.jar
   cache/
   karate-cli.json             # user-level config
```

### **C. GitHub Manifest Repository**

A simple online manifest the launcher reads, e.g.:

```
https://karate.sh/manifest.json
```

Contains channel → versions → URLs → checksums.

**Note:** The launcher should work without a manifest using sensible defaults (convention over configuration). Manifest enables customization and version pinning.

### **D. Optional JavaFX Installer (Enterprise Safety Path)**

JavaFX notarized desktop application can:

* Bundle the Rust launcher inside its resources.
* Provide UI to install Karate into project directories.
* Guide users in PATH setup.

---

# **3. Design Principles**

## **3.1 V1 Principles**

1. **Convention over configuration** — works without manifest using sensible defaults
2. **Bundled JRE only** — no system JRE detection complexity for v1
3. **Simple extension model** — `~/.karate/ext/*.jar` added to classpath
4. **Defaults that just work** — minimal config needed for basic usage
5. **Progressive customization** — power users can override defaults later
6. **Explicit bootstrap** — users must run `karate setup` before first use

## **3.2 Command Responsibility Model**

The Rust launcher handles **management commands** natively. All **runtime commands** are delegated to the Karate JAR via JVM.

### **Rust-Native Commands**

Commands fully implemented in Rust:

| Command | Description |
|---------|-------------|
| `karate setup` | Interactive first-run wizard |
| `karate setup --all` | Install JAR + JRE non-interactively |
| `karate setup --item jar` | Install JAR only (use system JRE) |
| `karate setup --item jre` | Install/update JRE only |
| `karate update [--all] [--item <name>]` | Check for and install updates |
| `karate config [--global\|--local\|--show]` | Edit or view configuration |
| `karate jre list` | List installed JREs |
| `karate jre doctor` | Check JRE health |
| `karate plugin install <name>[@version]` | Install a plugin |
| `karate plugin remove <name>` | Remove a plugin |
| `karate plugin list` | List installed plugins |
| `karate doctor [--json]` | Full system diagnostics |
| `karate version` | Show all version info |

### **JAR-Delegated Commands**

Everything else passes through to the JVM:

* `karate run ...` — Run tests
* `karate mock ...` — Start mock server
* `karate mcp ...` — MCP server commands
* `karate init ...` — Project scaffolding
* Any unknown command → delegate to JAR

---

# **4. Requirements**

## **4.1 Functional Requirements**

### **A. Bootstrap & Setup**

* Detect OS & architecture automatically.
* `karate setup` wizard:
  * Download latest Karate fatjar
  * Download matching JustJ JRE
  * Store in `~/.karate/`
  * Offer to add to PATH
* Support automated/non-interactive mode (`--all` or `--item`).
* **Explicit bootstrap required** — running `karate run` without setup shows helpful error.

### **B. Self-Management**

* `karate update`:
  * Check for newer versions of installed components
  * Display current vs latest versions
  * Interactive confirmation before downloading
  * Support `--all` for non-interactive updates
  * Support `--item` for targeted updates
  * Clean unused versions optionally

* `karate doctor`:
  * Show resolved versions, paths, config, plugin lists
  * `--json` mode for LLM/CI consumption

### **C. CLI PATH Setup**

* `karate setup path`:
  * Create symlink or copy to:
    * Unix: `~/.local/bin` or `/usr/local/bin`
    * Windows: `%LOCALAPPDATA%\Programs\Karate`
  * Options:
    * `--bin-dir <path>`
    * `--modify-shell-profile` (Unix)
    * `--add-to-path` (Windows)

### **D. Running Tests (Delegated)**

* `karate run <paths> [options]`:
  * Delegated to Karate JAR
  * Launcher constructs JVM command:
    * JRE path
    * Classpath (fatjar + ext/*.jar + --cp entries)
    * JVM opts from config

### **E. Extensions & Classpath**

* **User extensions:** `~/.karate/ext/` — manually dropped JARs, always added to classpath
* **`--cp` flag:** Additional classpath entries appended after ext JARs. Can be specified multiple times. Useful for IDE integrations and proprietary JARs.
* For v1, extensions are managed manually by dropping JAR files into the `ext/` folder
* Future versions may add managed plugin installation via manifest

**Classpath order:** karate fatjar → `~/.karate/ext/*.jar` → `.karate/ext/*.jar` → `--cp` entries

**Example:**
```bash
# Add a proprietary debug adapter JAR
karate --cp /path/to/karate-ide-v2.jar run features/

# Multiple extra JARs
karate --cp /path/to/a.jar --cp /path/to/b.jar run features/
```

### **F. Config Management**

* Global: `~/.karate/karate-cli.json`
* Project: `./.karate/karate-cli.json`
* CLI precedence: command flag → project config → global config → defaults

* `karate config`:
  * Interactive editor (opens in $EDITOR or simple prompts)
  * `--global` — edit global config
  * `--local` — edit/create project config
  * `--show` — print resolved config (merged)

### **G. Update Notifications**

* On every delegated command, non-blocking background check for updates
* Shows banner if update available: `Update available: 2.1.0 → run 'karate update'`
* Configurable: `"check_updates": false` in config to disable

### **H. Proxy Support**

* V1: Use system proxy settings (environment variables `HTTP_PROXY`, `HTTPS_PROXY`)
* Future: Explicit proxy config in `karate-cli.json`

### **I. ANSI Coloring**

* Fully support colored output (pass-through + launcher messages).
* `--no-color` flag for CI.
* Respect `NO_COLOR` environment variable.

---

## **4.2 Non-Functional Requirements**

### **Performance**

* Startup < 10 ms for launcher.
* JVM launch overhead via JRE (expected): < 200 ms warm.

### **Security**

* All downloads must support:
  * SHA-256 verification
  * HTTPS enforced
* No automatic PATH changes unless explicitly requested.
* All plugin loading is sandboxed via classpath.

### **Durability / Maintainability**

* Launcher should rarely need updates.
* Manifest can evolve without forcing binary replacement.
* Cross-platform builds automated via GitHub Actions.

### **Compliance**

* macOS: Notarized launcher through standard signing pipeline.
* Windows: Signed EXE with existing cert.

---

# **5. CLI Command Reference**

## **5.1 Command Overview**

```text
karate [global-options] <command> [options]

Global Options:
  --no-color             Disable colored output
  --cp <path>            Additional classpath entry (repeatable)

Management Commands (Rust-native):
  setup [subcommand]     First-run wizard or targeted setup
  update                 Check for and install updates
  config                 View or edit configuration
  jre <subcommand>       JRE management
  plugin <subcommand>    Plugin management
  doctor                 System diagnostics
  version                Show version information

Runtime Commands (JAR-delegated):
  run                    Run Karate tests
  mock                   Start mock server
  mcp                    MCP server commands
  init                   Initialize new project
  <other>                Passed to Karate JAR
```

---

## **5.2 Command Details**

### **setup**

```
karate setup [--all] [--item <name>] [--force] [--karate-version <ver>] [--java-version <ver>]
```

Interactive first-run wizard. Downloads JRE and Karate JAR, offers PATH setup.

**Flags:**
* `--all` — Install all components (JAR + JRE) non-interactively
* `--item <name>` — Install specific item: jar, jre
* `--force` — Force download even if components already installed
* `--karate-version <ver>` — Specific Karate JAR version to install (e.g., 1.5.2, 2.0.0)
* `--java-version <ver>` — Specific Java major version (default: 21)

**Examples:**
```
karate setup                                        # Interactive wizard
karate setup --all                                  # Install everything non-interactively
karate setup --item jar                             # JAR only (use system JRE)
karate setup --item jre                             # JRE only
karate setup --item jar --force                     # Force re-download JAR
karate setup --item jar --karate-version 2.0.0      # Install specific Karate version
```

---

### **update**

```
karate update [--all] [--item <name>]
```

Check for updates and download new versions. Interactive by default.

**Flags:**
* `--all` — Update all components non-interactively
* `--item <name>` — Update specific item: jar, jre

---

### **config**

```
karate config [--global | --local | --show]
```

Manage configuration files.

**Flags:**
* `--global` — Edit `~/.karate/karate-cli.json`
* `--local` — Edit `./.karate/karate-cli.json` (creates if needed)
* `--show` — Print resolved (merged) config as JSON

---

### **jre**

```
karate jre list
karate jre doctor
```

JRE inspection commands.

**Subcommands:**
* `list` — Show installed JRE versions
* `doctor` — Check JRE health and compatibility

---

### **plugin**

```
karate plugin install <name>[@version]
karate plugin remove <name>
karate plugin list
```

Manage plugins from manifest.

---

### **doctor**

```
karate doctor [--json]
```

Full system diagnostics showing:
* OS / architecture
* JRE path & version
* Karate JAR path & version
* Plugins resolved
* Extension JARs found
* Config file locations
* PATH / symlink status
* Update availability

---

### **version**

```
karate version [--json]
```

Show versions:
* Launcher version
* Karate JAR version
* JRE version
* Installed plugins

---

# **6. Exit Codes**

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error |
| `2` | Configuration/setup error (not bootstrapped, invalid config) |
| `3` | Network error (download failed, manifest unreachable) |
| `4` | JRE error (missing, corrupt, launch failed) |
| `100+` | Pass-through from JVM process |

---

# **7. Directory Structure & File Layout**

## **7.1 Two-Level Path Resolution**

Karate CLI uses a two-level resolution for resources (dist, jre, ext):

**Global Home** (always present):
1. `KARATE_HOME` environment variable — if set
2. `~/.karate` — default

**Local Override** (optional):
- `.karate/` folder in current working directory

**Resolution per resource:**
- `dist/`: If `.karate/dist/` exists in cwd → use local, otherwise → use global
- `jre/`: If `.karate/jre/` exists in cwd → use local, otherwise → use global
- `ext/`: Extensions from BOTH global `~/.karate/ext/` AND local `.karate/ext/` are loaded (composable, not override)

**Example:** A project with `.karate/ext/` but no `.karate/jre/`:
- Extensions: loaded from BOTH `~/.karate/ext/` (global) AND `.karate/ext/` (local)
- JRE: loaded from `~/.karate/jre/` (global fallback)
- Dist: loaded from `~/.karate/dist/` (global fallback)

This allows:
- Project-specific extensions without duplicating JRE/JAR
- Development testing with `KARATE_HOME=./home/.karate`
- Pinning specific Karate versions per project (via local dist/)
- Standard user-level installation at `~/.karate`

## **7.2 Home Directory Structure**

```
~/.karate/                        # Or KARATE_HOME location
├── dist/
│   └── karate-2.0.0.jar
├── jre/
│   └── 21.0.9-macosx-aarch64/
│       └── bin/java
├── ext/                          # User-provided extension JARs
│   └── custom-lib.jar
├── cache/
│   └── manifest.json             # Cached manifest
├── karate-cli.json               # Config for this home
├── uuid.txt                      # License management (preserved)
└── karate.lic                    # License file (preserved)
```

**Important:** The `uuid.txt` and `karate.lic` files are used for license management. Never delete the entire `~/.karate` folder.

---

## **7.3 Project Local Config (./.karate/)**

```
my-project/
└── .karate/
    └── karate-cli.json            # Project-specific config overrides
```

Note: A `.karate` folder with only `karate-cli.json` is treated as config-only, not a karate home.

---

## **7.4 Configuration Schema**

### **karate-cli.json (Global and Project)**

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

**Fields:**
* `channel` — Release channel: `stable`, `beta`, `nightly` (default: `stable`)
* `karate_version` — Version or `latest` (default: `latest`)
* `jre_path` — Explicit path to JRE directory (default: `null` → uses `~/.karate/jre/`)
* `dist_path` — Explicit path to directory containing Karate JAR (default: `null` → uses `~/.karate/dist/`)
* `jvm_opts` — Additional JVM options (default: none)
* `check_updates` — Check for updates on run (default: `true`)

**Path Override Use Cases:**
* JavaFX installer sets paths to point to bundled JRE/JAR
* Enterprise environments with centrally managed installations
* Development/testing with custom builds

---

# **8. Release Manifest (karate.sh)**

The CLI fetches artifact download URLs from a central manifest hosted at karate.sh. This avoids GitHub API rate limits and provides a single source of truth for all releases.

## **Location**

```
https://karate.sh/manifest.json
```

**Source repository:** [github.com/karatelabs/karate-sh](https://github.com/karatelabs/karate-sh) (private)

## **Manifest Schema**

```json
{
  "schema_version": 1,
  "generated_at": "2025-02-05T00:00:00Z",
  "artifacts": {
    "karate-cli": {
      "description": "Karate CLI - Rust binary launcher",
      "repo": "karatelabs/karate-cli",
      "versions": {
        "0.1.2": {
          "channels": ["stable"],
          "released_at": "2025-11-30T00:00:00Z",
          "platforms": {
            "macos-aarch64": { "url": "https://github.com/.../karate-darwin-arm64.tar.gz", "sha256": "..." },
            "macos-x64":     { "url": "https://github.com/.../karate-darwin-x64.tar.gz", "sha256": "..." },
            "linux-x64":     { "url": "https://github.com/.../karate-linux-x64.tar.gz", "sha256": "..." },
            "linux-aarch64": { "url": "https://github.com/.../karate-linux-arm64.tar.gz", "sha256": "..." },
            "windows-x64":   { "url": "https://github.com/.../karate-windows-x64.zip", "sha256": "..." }
          }
        }
      }
    },
    "karate": {
      "description": "Karate Core - Standalone testing JAR",
      "repo": "karatelabs/karate",
      "versions": {
        "1.5.2": {
          "channels": ["stable"],
          "released_at": "2025-11-30T00:00:00Z",
          "url": "https://github.com/karatelabs/karate/releases/download/v1.5.2/karate-1.5.2.jar",
          "sha256": "ccf4740c64a154c4c2457d6f0fd19a8f37902c29d32aac4e23012e0a878614be"
        }
      }
    }
  },
  "channel_defaults": {
    "stable": { "karate-cli": "0.1.2", "karate": "1.5.2" },
    "beta": {}
  }
}
```

## **Channels**

- **stable** — Production releases
- **beta** — Pre-release versions (RC, alpha, etc.)

Users can switch channels via config:
```bash
karate config --global   # Set "channel": "beta"
```

## **Adding a New Release**

When a new Karate release is published:

1. Clone the `karatelabs/karate-sh` repository
2. Edit `public/manifest.json`:
   - Add new version entry under the artifact
   - Get SHA256 from GitHub release `.sha256` files
   - Update `channel_defaults.stable` if promoting to stable
3. Commit and push to main
4. Netlify auto-deploys to karate.sh

See the [karate-sh README](https://github.com/karatelabs/karate-sh) for detailed instructions.

## **Version Pinning**

Users can pin a specific version in their config:
```json
{
  "channel": "stable",
  "karate_version": "1.5.2"
}
```

Setting `karate_version` to anything other than `"latest"` will use that exact version.

---

# **9. JavaFX Installer Integration**

The Rust CLI is designed to work seamlessly with the JavaFX installer application while remaining fully independent.

## **9.1 Integration Model**

```
┌─────────────────────────────────────────────────────────────────┐
│  JavaFX Installer App (Karate.app / Karate.exe)                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Bundled Resources:                                       │  │
│  │  ├── karate (Rust binary - for extraction)                │  │
│  │  ├── karate-2.0.0.jar                                     │  │
│  │  └── runtime/ (JRE)                                       │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  UI allows user to:                                             │
│  1. Extract `karate` binary → ~/.local/bin (or custom path)     │
│  2. Optionally configure PATH                                   │
│  3. Write config pointing CLI to bundled JRE/JAR                │
└─────────────────────────────────────────────────────────────────┘
                           │
                           ▼ writes
┌─────────────────────────────────────────────────────────────────┐
│  ~/.karate/karate-cli.json                                      │
│  {                                                              │
│    "jre_path": "/Applications/Karate.app/.../runtime",          │
│    "dist_path": "/Applications/Karate.app/.../Resources",       │
│    "check_updates": false                                       │
│  }                                                              │
└─────────────────────────────────────────────────────────────────┘
                           │
                           ▼ reads
┌─────────────────────────────────────────────────────────────────┐
│  ~/.local/bin/karate  (Rust CLI - standalone)                   │
│                                                                 │
│  • Developer-friendly CLI                                       │
│  • Reads config to locate JRE/JAR                               │
│  • Works independently of JavaFX app                            │
│  • Can download own JRE/JAR if paths not configured             │
└─────────────────────────────────────────────────────────────────┘
```

## **9.2 Key Design Principles**

1. **Decoupled Lifecycle** — The Rust CLI binary can be updated independently of the JavaFX app
2. **Config-Driven** — No magic bundle detection; paths are explicit in config
3. **User Control** — JavaFX UI provides opt-in PATH setup, user chooses location
4. **Fallback Behavior** — If `jre_path`/`dist_path` are null, CLI downloads its own

## **9.3 JavaFX Installer Responsibilities**

The JavaFX app handles:
* Extracting the Rust binary to a user-chosen location
* Writing `~/.karate/karate-cli.json` with paths to bundled resources
* Optionally modifying PATH (with user consent)
* Displaying instructions for manual PATH setup if declined

## **9.4 Example Configurations**

**Standalone Mode (downloaded by CLI):**
```json
{
  "channel": "stable",
  "karate_version": "latest"
}
```

**JavaFX-Managed Mode (macOS):**
```json
{
  "jre_path": "/Applications/Karate.app/Contents/Resources/runtime/Contents/Home",
  "dist_path": "/Applications/Karate.app/Contents/Resources",
  "check_updates": false
}
```

**JavaFX-Managed Mode (Windows):**
```json
{
  "jre_path": "C:\\Program Files\\Karate\\runtime",
  "dist_path": "C:\\Program Files\\Karate",
  "check_updates": false
}
```

---

# **10. Distribution Channels**

The Rust CLI enables multiple distribution paths. The goal is **one canonical binary** distributed through various channels.

## **10.1 Primary: karate.sh (Universal Installer)**

Own the install experience with `karate.sh`:

**Unix/macOS:**
```bash
curl -fsSL https://karate.sh/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://karate.sh/install.ps1 | iex
```

**Why this is the primary channel:**
- Full control over install experience
- Works everywhere (curl/PowerShell are universal)
- Can include telemetry, version selection, PATH setup
- Single URL to remember and document
- No approval process or third-party dependencies

**Implementation:**
- `karate.sh` serves a shell script that detects OS/arch
- Downloads the correct binary from GitHub releases
- Optionally runs `karate setup --all` for full bootstrap
- Provides clear instructions for PATH setup

## **10.2 npm Package**

Replace the brittle JBang-based `karate-npm` with a thin wrapper around the Rust binary.

**Current problems with karate-npm:**
- Triple wrapper: npm → Node.js → shelljs → JBang → Maven → JVM
- Windows silent failures (PowerShell execution policies, temp files)
- JBang dependency with its own bugs and JRE management
- shelljs fragility that swallows errors

**New approach:**
```
npm install -g karate
    ↓
postinstall downloads platform-specific Rust binary
    ↓
npm bin → karate (Rust) → JRE/JAR (managed by Rust)
```

**Package structure:**
```
karate-npm/
├── package.json          # npm package definition
├── postinstall.js        # Downloads Rust binary for platform
├── bin/
│   ├── karate            # Unix stub script
│   └── karate.cmd        # Windows stub script
└── dist/                 # Downloaded binaries (gitignored)
    └── karate-{platform} # Platform-specific Rust binary
```

**postinstall.js responsibilities:**
1. Detect OS/arch (darwin-arm64, darwin-x64, linux-x64, win32-x64, etc.)
2. Download matching Rust binary from GitHub releases
3. Verify SHA256 checksum
4. Make executable (Unix)
5. First run triggers `karate setup --all` for JRE/JAR bootstrap

**User experience:**
```bash
npm install -g karate
karate setup      # First-time JRE/JAR download
karate run my.feature
```

## **10.3 Package Managers (Secondary)**

These require maintenance effort but increase discoverability:

| Channel | Effort | Value | Priority |
|---------|--------|-------|----------|
| **Homebrew** | Medium | High (macOS devs) | P1 |
| **Chocolatey** | Medium | High (Windows devs) | P1 |
| **Scoop** | Low | Medium (Windows) | P2 |
| **apt/deb** | High | Medium (Linux) | P3 |
| **rpm** | High | Low | P3 |
| **Cargo** | Low | Low (Rust devs only) | P4 |

**Homebrew formula (example):**
```ruby
class Karate < Formula
  desc "Karate - API testing framework CLI"
  homepage "https://karatelabs.io"
  url "https://github.com/karatelabs/karate-cli/releases/download/v2.0.0/karate-darwin-arm64.tar.gz"
  sha256 "..."

  def install
    bin.install "karate"
  end

  def post_install
    system "#{bin}/karate", "setup", "--all"
  end
end
```

**Chocolatey package (example):**
```powershell
$packageArgs = @{
  packageName   = 'karate'
  url64bit      = 'https://github.com/karatelabs/karate-cli/releases/download/v2.0.0/karate-windows-x64.zip'
  checksum64    = '...'
  unzipLocation = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
}
Install-ChocolateyZipPackage @packageArgs
```

## **10.4 Recommended Strategy**

**Phase 1 (MVP):**
1. ✅ GitHub releases with binaries for all platforms
2. 🔲 `karate.sh` universal installer
3. 🔲 npm package (replace karate-npm)

**Phase 2 (Adoption):**
4. 🔲 Homebrew formula (tap first, then core)
5. 🔲 Chocolatey package
6. 🔲 Scoop manifest

**Phase 3 (Completeness):**
7. 🔲 Docker images
8. 🔲 Linux packages (deb/rpm) if demand exists

## **10.5 Why karate.sh is Enough for Most Users**

| User Type | Best Channel |
|-----------|--------------|
| Quick start / tutorials | `karate.sh` |
| Node.js projects | npm |
| macOS power users | Homebrew |
| Windows enterprises | Chocolatey |
| CI/CD pipelines | `karate.sh` or Docker |
| Air-gapped networks | Direct binary download |

The `karate.sh` approach (like `rustup.sh`, `get.docker.com`) is battle-tested and works for 80%+ of users without requiring package manager submissions.

---

# **11. Replacing karate-npm**

The existing `karate-npm` package wraps JBang, which itself wraps Maven and manages JRE. This creates a fragile chain:

```
npm → Node.js (karate.js) → shelljs → JBang → Maven → JVM
```

**Known issues:**
- Windows silent failures (PowerShell execution policies, temp file creation)
- JBang is another dependency with its own bugs and update cycle
- shelljs swallows errors, making debugging difficult
- No visibility into JRE management
- Complex fallback mechanisms that fail silently

**New architecture:**
```
npm → postinstall.js → downloads Rust binary
npm bin/karate → Rust CLI → JRE/JAR (self-managed)
```

**Benefits:**
- Single native binary, no runtime dependencies
- Explicit error messages with proper exit codes
- User-visible JRE management (`karate jre list`, `karate doctor`)
- Works offline after initial setup
- Same binary whether installed via npm, curl, or Homebrew

**Migration path:**
1. Publish new `karate` package (version 2.0.0+)
2. Deprecate old JBang-based approach
3. Users run `npm update -g karate` to get new version
4. First run prompts `karate setup` for JRE/JAR download

---

# **12. Future Enhancements (Post-MVP)**

## **12.1 Central Manifest at karate.sh** ✅ IMPLEMENTED

The central manifest is now live at `https://karate.sh/manifest.json`:

* Download locations for karate-cli and karate JAR
* SHA-256 checksums for integrity verification
* Channel support (stable, beta) for version management
* Hosted on Netlify, managed via [github.com/karatelabs/karate-sh](https://github.com/karatelabs/karate-sh)

See Section 8 for manifest schema and release workflow.

## **12.2 Rust-Native `init` Command**

Move `karate init` from JAR-delegated to Rust-native. The command scaffolds project structure *before* Java is involved, making it unsuitable for JAR delegation.

```
karate init [name] [--type <type>] [--template <template>]
```

**Arguments:**
* `name` — Project directory name (default: current directory)

**Flags:**
* `--type <type>` — Project type: `standalone`, `maven`, `gradle` (skips interactive prompt)
* `--template <template>` — Template name (e.g., `api`, `openapi`, `spring`)
* `--force` — Overwrite existing files

**Interactive flow:**
```
$ karate init my-project

? Project type:
  > standalone (just Karate, no build tool)
    maven (Java project with pom.xml)
    gradle (Java project with build.gradle)

? Template:
  > api (basic API testing)
    openapi (OpenAPI/Swagger integration)
    spring (Spring Boot integration)

Creating project: my-project/
  ├── karate.json
  ├── src/test/features/
  │   └── example.feature
  └── karate-config.js

Done! Run tests with: cd my-project && karate run
```

**Project type details:**

| Type | Output | Use case |
|------|--------|----------|
| `standalone` | `karate.json`, features only | Quick start, non-Java teams, LLM agents |
| `maven` | `pom.xml` with karate dependency | Java teams, CI integration |
| `gradle` | `build.gradle` with karate dependency | Java teams preferring Gradle |

**Extended templates (future):**
* `karate init --template openapi` — Generate tests from OpenAPI spec
* `karate init --template spring` — Spring Boot integration with test harness
* `karate init --template graphql` — GraphQL testing scaffold
* Custom templates from git repos: `karate init --template https://github.com/...`

## **12.3 Other Enhancements**

* `karate lock` → freeze exact versions + checksums in project
* Shell completions (bash, zsh, fish, PowerShell)
* "Agent mode" improvements for LLM-based automation
* Docker images pre-baked with launcher + runtime
* Local manifest override for air-gapped networks
* Telemetry (opt-in) with auto GitHub issue creation for crashes
* System JRE detection and preference
* Deprecate Maven archetype in favor of `karate init --type maven`

---

# **13. Summary**

The Rust-based Karate CLI Launcher gives us:

* **Zero-dependency installs**
* **Deterministic behavior for agents and CI**
* **First-class plugin architecture**
* **Self-contained JRE management**
* **Unified CLI UX across OSes**
* **Compatibility with both OSS and commercial (Xplorer/MCP) workflows**
* **Flexible enterprise onboarding via JavaFX UI**
* **Convention over configuration with progressive customization**

This design intentionally minimizes moving parts in the launcher itself while giving Karate the lifecycle and extensibility of a modern, developer-friendly CLI tool.
