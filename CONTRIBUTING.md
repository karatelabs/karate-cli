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

### Option A: Manual Trigger (Recommended)

1. **Ensure CI passes on main**
   ```bash
   git checkout main
   git pull
   ```

2. **Trigger release via GitHub UI**
   - Go to Actions → Release → Run workflow
   - Enter tag name: `v0.2.0`
   - Click "Run workflow"

   The workflow will:
   - Inject the version from the tag into `Cargo.toml` at build time
   - Build binaries for all platforms
   - Create the git tag AND the GitHub Release

   Note: `Cargo.toml` stays at `version = "0.1.0"` in git - the version is injected during the release build only.

### Option B: Tag-Based Release

```bash
git tag v0.2.0
git push origin v0.2.0
```

The release workflow triggers automatically on tag push.

### After Release

1. **Verify the GitHub Release**
   - Binaries appear at: `https://github.com/karatelabs/karate-cli/releases/tag/v0.2.0`
   - GitHub auto-generates release notes from commits
   - Edit release notes manually if needed

2. **Update the manifest at karate.sh**

   The CLI uses `https://karate.sh/manifest.json` to resolve downloads. After every release, update the manifest in the [`karate-sh`](https://github.com/karatelabs/karate-sh) repo:

   ```bash
   cd /path/to/karate-sh

   # Download all .sha256 files from the release
   gh release download v0.2.0 -R karatelabs/karate-cli -p '*.sha256'
   ```

   Then edit `public/manifest.json`:
   - Add a new version entry under `artifacts.karate-cli.versions`
   - Move `"stable"` from the old version's `channels` to the new one (old version gets `[]`)
   - Update `channel_defaults.stable.karate-cli` to the new version
   - Use the SHA256 values from the downloaded checksum files

   ```bash
   # Commit and push (Netlify auto-deploys)
   git add public/manifest.json
   git commit -m "add karate-cli v0.2.0 to manifest"
   git push
   ```

3. **Test the install scripts**

   ```bash
   # macOS/Linux
   curl -fsSL https://karate.sh | sh

   # Windows PowerShell
   irm https://karate.sh/install.ps1 | iex
   ```

### Deleting a Tag (if needed)

```bash
# Delete local tag
git tag -d v0.2.0

# Delete remote tag
git push origin --delete v0.2.0

# Or delete both in one line
git tag -d v0.2.0 && git push origin --delete v0.2.0
```

Note: Deleting a tag does NOT delete the GitHub Release. Delete the release manually from the GitHub UI if needed.

## Testing CLI Self-Update Locally

The `karate update --item cli` command can be tested without a real release using `KARATE_MANIFEST_URL` to point at a local HTTP server. The [`karate-sh`](https://github.com/karatelabs/karate-sh) repo (expected at `../karate-sh`) contains the production `manifest.json`.

```bash
# 1. Build the binary
cargo build

# 2. Create a test directory and copy the binary
mkdir -p /tmp/karate-update-test/serve /tmp/karate-update-test/bin
cp target/debug/karate /tmp/karate-update-test/bin/karate

# 3. Package it as a release archive and get the SHA256
tar czf /tmp/karate-update-test/serve/karate-cli.tar.gz \
  -C /tmp/karate-update-test/bin karate
shasum -a 256 /tmp/karate-update-test/serve/karate-cli.tar.gz
```

4. Create `/tmp/karate-update-test/serve/manifest.json` using the SHA256 from above (replace `YOUR_SHA256` and adjust the platform key for your machine):

```json
{
  "schema_version": 1,
  "generated_at": "2026-01-01T00:00:00Z",
  "artifacts": {
    "karate-cli": {
      "description": "Karate CLI",
      "versions": {
        "0.2.0-test": {
          "channels": ["stable"],
          "released_at": "2026-01-01T00:00:00Z",
          "platforms": {
            "macos-aarch64": {
              "url": "http://localhost:9999/karate-cli.tar.gz",
              "sha256": "YOUR_SHA256"
            }
          }
        }
      }
    },
    "karate": {
      "description": "Karate Core",
      "versions": {
        "1.5.2": {
          "channels": ["stable"],
          "released_at": "2025-11-30T00:00:00Z",
          "url": "https://example.com/karate-1.5.2.jar",
          "sha256": "abc123"
        }
      }
    }
  },
  "channel_defaults": {
    "stable": { "karate-cli": "0.2.0-test", "karate": "1.5.2" }
  }
}
```

```bash
# 5. Start a local HTTP server
cd /tmp/karate-update-test/serve && python3 -m http.server 9999 &

# 6. Test the full self-update flow (runs the copied binary, not cargo run)
KARATE_MANIFEST_URL=http://localhost:9999/manifest.json \
KARATE_HOME=./home/.karate \
  /tmp/karate-update-test/bin/karate update --item cli

# 7. Verify the binary still works after replacement
/tmp/karate-update-test/bin/karate version

# 8. Clean up
kill $(lsof -ti:9999) 2>/dev/null
rm -rf /tmp/karate-update-test
```

Note: The dev binary always reports `0.1.0` (from `Cargo.toml`), so it will always see `0.2.0-test` as an update. In production, the version is injected at release build time.

You can also test just the check/display phase via `cargo run` (no binary replacement):

```bash
KARATE_MANIFEST_URL=http://localhost:9999/manifest.json \
KARATE_HOME=./home/.karate cargo run -- update --item cli
```

## Related Repositories

The [`karate-sh`](https://github.com/karatelabs/karate-sh) website repo is expected at `../karate-sh` relative to this project. It contains:
- `public/manifest.json` - The production manifest used by `karate setup`, `karate update`, and install scripts
- Install scripts (`install.sh`, `install.ps1`)

## Important Notes

- **Never delete `~/.karate`** - Contains license files (`uuid.txt`, `karate.lic`)
- **Use `KARATE_HOME=./home/.karate`** for local development testing
- **Binaries are ~2MB** compressed, ~3.5MB uncompressed
