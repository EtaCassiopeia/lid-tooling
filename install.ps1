# Install lidc and/or lid-mcp from the latest GitHub release.
#
# Usage (from PowerShell):
#   irm https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.ps1 | iex
#   irm https://raw.githubusercontent.com/EtaCassiopeia/lid-tooling/main/install.ps1 | iex; Install-Lid -Mcp
#
# Parameters:
#   -Mcp        Also install lid-mcp
#   -Dir        Install directory (default: $env:LOCALAPPDATA\lid-tooling\bin)
#   -Version    Specific version tag (e.g. v0.2.3)

param(
    [switch]$Mcp,
    [string]$Dir = "$env:LOCALAPPDATA\lid-tooling\bin",
    [string]$Version = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Target = "x86_64-pc-windows-msvc"

# ── Resolve version ──────────────────────────────────────────────────────────

if (-not $Version) {
    $latest = Invoke-RestMethod "https://api.github.com/repos/EtaCassiopeia/lid-tooling/releases/latest"
    $Version = $latest.tag_name
}

Write-Host "Installing lid-tooling $Version for Windows x64"

$BaseUrl = "https://github.com/EtaCassiopeia/lid-tooling/releases/download/$Version"

# ── Install directory ────────────────────────────────────────────────────────

if (-not (Test-Path $Dir)) {
    New-Item -ItemType Directory -Path $Dir | Out-Null
}

# Add to PATH for the current session and persistently for the user
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$Dir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$Dir", "User")
    $env:PATH = "$env:PATH;$Dir"
    Write-Host "  Added $Dir to PATH"
}

# ── Download helper ──────────────────────────────────────────────────────────

function Install-Bin {
    param([string]$Name)
    Write-Host "  -> $Name"
    $url  = "$BaseUrl/$Name-$Target.exe"
    $dest = Join-Path $Dir "$Name.exe"
    Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing
}

Install-Bin "lidc"
if ($Mcp) { Install-Bin "lid-mcp" }

Write-Host ""
Write-Host "Done!  Try:  lidc --version"
if ($Mcp) { Write-Host "       and:  lid-mcp --help" }
Write-Host ""
Write-Host "Note: open a new terminal for the PATH change to take effect."
