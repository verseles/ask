# ask installer for Windows
# https://github.com/verseles/ask
#
# Licensed under AGPL-3.0

$ErrorActionPreference = "Stop"

$REPO = "verseles/ask"
$BINARY_NAME = "ask"
$INSTALL_DIR = "$env:USERPROFILE\.local\bin"

function Write-Info {
    param([string]$Message)
    Write-Host "info: " -ForegroundColor Cyan -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "success: " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warn {
    param([string]$Message)
    Write-Host "warning: " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Error {
    param([string]$Message)
    Write-Host "error: " -ForegroundColor Red -NoNewline
    Write-Host $Message
    exit 1
}

function Get-Architecture {
    $arch = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
    switch ($arch) {
        "AMD64" { return "x86_64" }
        "ARM64" { return "aarch64" }
        default { Write-Error "Unsupported architecture: $arch" }
    }
}

function Get-LatestVersion {
    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest"
        return $response.tag_name
    } catch {
        Write-Error "Could not determine latest version: $_"
    }
}

function Get-FileHash256 {
    param([string]$Path)
    return (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLower()
}

function Main {
    Write-Info "Installing $BINARY_NAME..."

    $arch = Get-Architecture
    Write-Info "Detected architecture: $arch"

    # Map to target
    $target = switch ($arch) {
        "x86_64"  { "x86_64-pc-windows-msvc" }
        "aarch64" { "aarch64-pc-windows-msvc" }
        default   { Write-Error "Unsupported architecture: $arch" }
    }

    $version = Get-LatestVersion
    Write-Info "Latest version: $version"

    # Create temp directory
    $tempDir = New-Item -ItemType Directory -Path ([System.IO.Path]::GetTempPath()) -Name ([System.Guid]::NewGuid().ToString())

    try {
        $binaryUrl = "https://github.com/$REPO/releases/download/$version/$BINARY_NAME-$target.exe"
        $checksumUrl = "$binaryUrl.sha256"

        $binaryPath = Join-Path $tempDir "$BINARY_NAME.exe"
        $checksumPath = Join-Path $tempDir "$BINARY_NAME.exe.sha256"

        Write-Info "Downloading $BINARY_NAME..."
        Invoke-WebRequest -Uri $binaryUrl -OutFile $binaryPath
        Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumPath

        # Verify checksum
        $expectedHash = (Get-Content $checksumPath).Split()[0].Trim()
        $actualHash = Get-FileHash256 -Path $binaryPath

        if ($actualHash -ne $expectedHash) {
            Write-Error "Checksum verification failed"
        }
        Write-Success "Checksum verified"

        # Create install directory
        if (-not (Test-Path $INSTALL_DIR)) {
            New-Item -ItemType Directory -Path $INSTALL_DIR -Force | Out-Null
        }

        # Install binary
        $installPath = Join-Path $INSTALL_DIR "$BINARY_NAME.exe"
        Move-Item -Path $binaryPath -Destination $installPath -Force

        Write-Success "Installed $BINARY_NAME to $installPath"

        # Check PATH
        $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
        if (-not $userPath.Contains($INSTALL_DIR)) {
            Write-Warn "$INSTALL_DIR is not in your PATH"
            Write-Host ""

            $addToPath = Read-Host "Add to PATH? (Y/n)"
            if ($addToPath -ne "n" -and $addToPath -ne "N") {
                $newPath = "$INSTALL_DIR;$userPath"
                [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
                $env:PATH = "$INSTALL_DIR;$env:PATH"
                Write-Success "Added to PATH"
            }
        }

        Write-Host ""
        Write-Success "Installation complete!"
        Write-Host ""
        Write-Host "Get started:"
        Write-Host "  $BINARY_NAME init    # Configure API keys"
        Write-Host "  $BINARY_NAME --help  # Show help"
        Write-Host ""

    } finally {
        # Cleanup
        Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Main
