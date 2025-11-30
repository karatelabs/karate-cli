# Contributing to Karate CLI

## Development Setup

```bash
# Clone and build
git clone https://github.com/karatelabs/karate-cli.git
cd karate-cli
cargo build

# Run locally
cargo run -- --help
cargo run -- doctor

# Test with local home directory (avoids touching ~/.karate)
KARATE_HOME=./home/.karate cargo run -- setup
KARATE_HOME=./home/.karate cargo run -- doctor
```

## Project Structure

```
src/
├── main.rs          # Entry point
├── cli.rs           # Clap command definitions
├── commands/        # Rust-native command implementations
│   ├── setup.rs     # karate setup
│   ├── doctor.rs    # karate doctor
│   └── ...
├── delegate.rs      # JAR delegation for run/mock/mcp
├── jre.rs           # JRE detection and management
├── download.rs      # HTTP downloads with progress
├── platform.rs      # OS/arch detection, paths
├── config.rs        # Configuration loading/merging
├── manifest.rs      # Remote manifest parsing
└── error.rs         # Error types and exit codes
```

## Code Quality

Before pushing, ensure:

```bash
cargo fmt --all           # Format code
cargo clippy -- -D warnings   # Lint (warnings are errors)
cargo test                # Run tests
```

## CI Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci.yml` | Push/PR to `main` | Format, lint, test, build check |
| `release.yml` | Git tag `v*` | Build release binaries for all platforms |

### CI Jobs (`ci.yml`)

- **check** - `cargo check`
- **fmt** - `cargo fmt --check`
- **clippy** - `cargo clippy -D warnings`
- **test** - Run tests on Linux, macOS, Windows
- **build-check** - Verify release builds compile

### Release Artifacts (`release.yml`)

Builds binaries for 5 platforms:

| Target | Artifact |
|--------|----------|
| `aarch64-apple-darwin` | `karate-darwin-arm64.tar.gz` |
| `x86_64-apple-darwin` | `karate-darwin-x64.tar.gz` |
| `x86_64-unknown-linux-gnu` | `karate-linux-x64.tar.gz` |
| `aarch64-unknown-linux-gnu` | `karate-linux-arm64.tar.gz` |
| `x86_64-pc-windows-msvc` | `karate-windows-x64.zip` |

Each artifact includes a `.sha256` checksum file.

## Making a Release

1. **Ensure CI passes on main**
   ```bash
   git checkout main
   git pull
   ```

2. **Update version in Cargo.toml**
   ```bash
   # Edit Cargo.toml: version = "0.2.0"
   git add Cargo.toml
   git commit -m "Bump version to 0.2.0"
   git push
   ```

3. **Create and push a tag**
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

4. **Monitor the release workflow**
   - Go to Actions → Release workflow
   - Once complete, binaries appear at:
     `https://github.com/karatelabs/karate-cli/releases/tag/v0.2.0`

5. **Edit release notes** (optional)
   - GitHub auto-generates notes from commits
   - Add highlights or breaking changes manually if needed

## Testing the Install Scripts

After a release, test the installers:

```bash
# macOS/Linux
curl -fsSL https://karate.sh | sh

# Windows PowerShell
irm https://karate.sh/install.ps1 | iex
```

## Important Notes

- **Never delete `~/.karate`** - Contains license files (`uuid.txt`, `karate.lic`)
- **Use `KARATE_HOME=./home/.karate`** for local development testing
- **Binaries are ~15MB** due to static linking (expected)
