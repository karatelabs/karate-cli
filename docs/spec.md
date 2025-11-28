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
https://github.com/karatelabs/karate-cli-manifest/releases/latest/download/manifest.json
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
6. **Explicit bootstrap** — users must run `karate setup` or `karate upgrade` before first use

## **3.2 Command Responsibility Model**

The Rust launcher handles **management commands** natively. All **runtime commands** are delegated to the Karate JAR via JVM.

### **Rust-Native Commands**

Commands fully implemented in Rust:

| Command | Description |
|---------|-------------|
| `karate setup` | Interactive first-run wizard |
| `karate setup path` | Install CLI to PATH only |
| `karate setup jre` | Install/update JRE only |
| `karate upgrade [--yes] [--version <ver>]` | Update to latest/specific version |
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
* Support automated/non-interactive mode (`--yes`).
* **Explicit bootstrap required** — running `karate run` without setup shows helpful error.

### **B. Self-Management**

* `karate upgrade`:
  * Check manifest for updates
  * Download new version(s)
  * Support `--version <ver>` for specific version
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
    * Classpath (fatjar + plugins + ext/*.jar)
    * JVM opts from config

### **E. Extensions Support**

* **User extensions:** `~/.karate/ext/` — manually dropped JARs, always added to classpath
* For v1, extensions are managed manually by dropping JAR files into the `ext/` folder
* Future versions may add managed plugin installation via manifest

### **F. Config Management**

* Global: `~/.karate/karate-cli.json`
* Project: `./.karate/karate.json`
* CLI precedence: command flag → project config → global config → defaults

* `karate config`:
  * Interactive editor (opens in $EDITOR or simple prompts)
  * `--global` — edit global config
  * `--local` — edit/create project config
  * `--show` — print resolved config (merged)

### **G. Update Notifications**

* On every delegated command, non-blocking background check for updates
* Shows banner if update available: `Update available: 2.1.0 → run 'karate upgrade'`
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
karate <command> [options]

Management Commands (Rust-native):
  setup [subcommand]     First-run wizard or targeted setup
  upgrade                Update Karate JAR and JRE
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
karate setup [--yes]
karate setup path [--bin-dir <path>] [--modify-shell-profile]
karate setup jre [--version <ver>]
```

Interactive first-run wizard. Downloads JRE and Karate JAR, offers PATH setup.

**Subcommands:**
* `path` — Only set up PATH/symlinks
* `jre` — Only install/update JRE

**Flags:**
* `--yes` — Non-interactive, accept defaults

---

### **upgrade**

```
karate upgrade [--yes] [--version <ver>]
```

Check for updates and download new versions.

**Flags:**
* `--yes` — Non-interactive
* `--version <ver>` — Install specific version instead of latest

---

### **config**

```
karate config [--global | --local | --show]
```

Manage configuration files.

**Flags:**
* `--global` — Edit `~/.karate/karate-cli.json`
* `--local` — Edit `./.karate/karate.json` (creates if needed)
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
- If `.karate/{resource}/` exists in cwd → use local
- Otherwise → use global `{home}/{resource}/`

**Example:** A project with `.karate/ext/` but no `.karate/jre/`:
- Extensions: loaded from `.karate/ext/` (local)
- JRE: loaded from `~/.karate/jre/` (global fallback)
- Dist: loaded from `~/.karate/dist/` (global fallback)

This allows:
- Project-specific extensions without duplicating JRE/JAR
- Development testing with `KARATE_HOME=./home`
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
    └── karate.json               # Project-specific config overrides
```

Note: A `.karate` folder with only `karate.json` is treated as config-only, not a karate home.

---

## **7.4 Configuration Schema**

### **karate-cli.json (Global) / karate.json (Project)**

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

# **8. GitHub Manifest Format**

## **Location**

```
https://github.com/karatelabs/karate-cli-manifest/releases/latest/download/manifest.json
```

## **Manifest Schema**

```json
{
  "schema_version": 1,
  "channels": {
    "stable": {
      "version": "2.0.0",
      "karate_jar": {
        "url": "https://github.com/karatelabs/karate/releases/download/v2.0.0/karate-2.0.0-all.jar",
        "sha256": "abc123..."
      },
      "jre": {
        "version": "17.0.12",
        "platforms": {
          "macos-aarch64": { "url": "...", "sha256": "..." },
          "macos-x64":     { "url": "...", "sha256": "..." },
          "linux-x64":     { "url": "...", "sha256": "..." },
          "linux-aarch64": { "url": "...", "sha256": "..." },
          "windows-x64":   { "url": "...", "sha256": "..." }
        }
      },
      "plugins": {
        "xplorer": {
          "version": "1.3.0",
          "url": "...",
          "sha256": "..."
        }
      }
    },
    "beta": { ... }
  },
  "defaults": {
    "karate_jar_url_template": "https://github.com/karatelabs/karate/releases/download/v{version}/karate-{version}-all.jar",
    "jre_version": "17.0.12"
  }
}
```

**Note:** The `defaults` section enables the launcher to work without fetching manifest, using URL templates and conventions.

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

# **10. Future Enhancements (Post-MVP)**

* `karate lock` → freeze exact versions + checksums in project
* Shell completions (bash, zsh, fish, PowerShell)
* Templated project scaffolds (`karate init --template spring-openapi`)
* "Agent mode" improvements for LLM-based automation
* Docker images pre-baked with launcher + runtime
* Local manifest override for air-gapped networks
* Telemetry (opt-in) with auto GitHub issue creation for crashes
* System JRE detection and preference

---

# **11. Summary**

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
