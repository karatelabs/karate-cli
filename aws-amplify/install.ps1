# Karate CLI Installer for Windows
# Usage: irm https://karate.sh/install.ps1 | iex
#        irm https://karate.sh/install.ps1 | iex -Args '--yes'
#
# Options:
#   -Yes              Non-interactive, accept defaults
#   -InstallDir DIR   Install to custom directory (default: %LOCALAPPDATA%\Programs\Karate)
#   -Version VER      Install specific version (default: latest)

param(
    [switch]$Yes,
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Karate",
    [string]$Version = "latest",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

# Configuration
$GitHubRepo = "karatelabs/karate-cli"

function Write-Info {
    param([string]$Message)
    Write-Host "==> " -ForegroundColor Cyan -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "==> " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warn {
    param([string]$Message)
    Write-Host "Warning: " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Error-And-Exit {
    param([string]$Message)
    Write-Host "Error: " -ForegroundColor Red -NoNewline
    Write-Host $Message
    exit 1
}

function Get-Architecture {
    if ([Environment]::Is64BitOperatingSystem) {
        $arch = $env:PROCESSOR_ARCHITECTURE
        switch ($arch) {
            "AMD64" { return "x64" }
            "ARM64" { return "arm64" }
            default { return "x64" }
        }
    } else {
        Write-Error-And-Exit "32-bit Windows is not supported"
    }
}

function Get-LatestVersion {
    $url = "https://api.github.com/repos/$GitHubRepo/releases/latest"
    try {
        $response = Invoke-RestMethod -Uri $url -UseBasicParsing
        return $response.tag_name -replace "^v", ""
    } catch {
        Write-Error-And-Exit "Failed to fetch latest version from GitHub: $_"
    }
}

function Get-FileHash256 {
    param([string]$Path)
    $hash = Get-FileHash -Path $Path -Algorithm SHA256
    return $hash.Hash.ToLower()
}

function Add-ToUserPath {
    param([string]$Directory)

    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$Directory*") {
        $newPath = "$Directory;$currentPath"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:Path = "$Directory;$env:Path"
        return $true
    }
    return $false
}

function Show-Help {
    Write-Host "Karate CLI Installer for Windows"
    Write-Host ""
    Write-Host "Usage: irm https://karate.sh/install.ps1 | iex"
    Write-Host "       .\install.ps1 [OPTIONS]"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -Yes              Non-interactive, accept defaults"
    Write-Host "  -InstallDir DIR   Install to custom directory"
    Write-Host "                    (default: %LOCALAPPDATA%\Programs\Karate)"
    Write-Host "  -Version VER      Install specific version (default: latest)"
    Write-Host "  -Help             Show this help"
    exit 0
}

function Main {
    if ($Help) {
        Show-Help
    }

    Write-Info "Karate CLI Installer"
    Write-Host ""

    # Detect architecture
    $arch = Get-Architecture
    $platform = "windows-$arch"

    Write-Info "Detected platform: $platform"

    # Get version
    if ($Version -eq "latest") {
        Write-Info "Fetching latest version..."
        $Version = Get-LatestVersion
    }

    Write-Info "Installing Karate CLI v$Version"

    # Construct download URLs
    $zipName = "karate-$platform.zip"
    $downloadUrl = "https://github.com/$GitHubRepo/releases/download/v$Version/$zipName"
    $checksumUrl = "https://github.com/$GitHubRepo/releases/download/v$Version/$zipName.sha256"

    # Create temp directory
    $tempDir = Join-Path $env:TEMP "karate-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

    try {
        # Download zip
        $zipPath = Join-Path $tempDir $zipName
        Write-Info "Downloading $zipName..."
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UseBasicParsing

        # Download and verify checksum
        Write-Info "Verifying checksum..."
        $checksumPath = Join-Path $tempDir "checksum.txt"
        Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumPath -UseBasicParsing

        $expectedChecksum = (Get-Content $checksumPath).Split(" ")[0].ToLower()
        $actualChecksum = Get-FileHash256 -Path $zipPath

        if ($actualChecksum -ne $expectedChecksum) {
            Write-Error-And-Exit "Checksum verification failed!`n  Expected: $expectedChecksum`n  Actual:   $actualChecksum"
        }

        # Extract
        Write-Info "Extracting..."
        $extractDir = Join-Path $tempDir "extract"
        Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force

        # Create install directory
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }

        # Install binary
        $binaryPath = Join-Path $InstallDir "karate.exe"
        Write-Info "Installing to $binaryPath..."
        Copy-Item -Path (Join-Path $extractDir "karate.exe") -Destination $binaryPath -Force

        Write-Success "Karate CLI v$Version installed successfully!"
        Write-Host ""

        # Add to PATH
        $pathAdded = Add-ToUserPath -Directory $InstallDir
        if ($pathAdded) {
            Write-Info "Added $InstallDir to your PATH"
            Write-Host ""
        }

        # Check if in current session PATH
        if ($env:Path -notlike "*$InstallDir*") {
            Write-Warn "$InstallDir is not in your current session PATH."
            Write-Host ""
            Write-Host "Restart your terminal, or run:"
            Write-Host ""
            Write-Host "  `$env:Path = `"$InstallDir;`$env:Path`""
            Write-Host ""
        }

        # Run setup if requested
        if ($Yes) {
            Write-Info "Running karate setup..."
            & $binaryPath setup --yes
        } else {
            Write-Host "Next steps:"
            Write-Host ""
            Write-Host "  karate setup    # Download JRE and Karate JAR" -ForegroundColor White
            Write-Host "  karate doctor   # Verify installation" -ForegroundColor White
            Write-Host "  karate run      # Run your first test" -ForegroundColor White
            Write-Host ""
        }

    } finally {
        # Cleanup
        if (Test-Path $tempDir) {
            Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

Main
