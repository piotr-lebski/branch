<#
.SYNOPSIS
  Installs branch - an interactive git branch and worktree navigator.

.PARAMETER Version
  Release tag to install (e.g. v0.1.0). Defaults to the latest release.

.PARAMETER BinDir
  Directory to install branch.exe. Defaults to $env:LOCALAPPDATA\branch\bin.

.PARAMETER NoShellIntegration
  Skip adding the Invoke-Expression line to your PowerShell profile.

.EXAMPLE
  iwr -useb https://raw.githubusercontent.com/piotr-lebski/branch/main/install.ps1 | iex
#>
param(
    [string]$Version = "",
    [string]$BinDir  = "$env:LOCALAPPDATA\branch\bin",
    [switch]$NoShellIntegration
)

$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

function Write-Ok   { param($msg) Write-Host "  OK $msg" -ForegroundColor Green }
function Write-Warn { param($msg) Write-Host "  ! $msg"  -ForegroundColor Yellow }
function Write-Err  {
    param($msg)
    Write-Host "  x $msg" -ForegroundColor Red
    exit 1
}

if ($env:PROCESSOR_ARCHITECTURE -ne "AMD64") {
    Write-Err "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE. Only AMD64 (x86_64) is supported.`nSee https://github.com/piotr-lebski/branch#install to build from source."
}
$Target = "x86_64-pc-windows-msvc"

if (-not $Version) {
    try {
        $restParams = @{
            Uri     = "https://api.github.com/repos/piotr-lebski/branch/releases/latest"
            Headers = @{ "User-Agent" = "branch-installer" }
        }
        $release = Invoke-RestMethod @restParams
        $Version = $release.tag_name
    } catch {
        Write-Err "Failed to determine latest release version: $_"
    }
}

Write-Host "Installing branch $Version for $Target..."

$TmpDir = $null
try {
    $TmpDir = Join-Path $env:TEMP "branch-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $TmpDir | Out-Null

    $Archive = "branch-$Target-$Version.zip"
    $BaseUrl = "https://github.com/piotr-lebski/branch/releases/download/$Version"

    Invoke-WebRequest -Uri "$BaseUrl/$Archive"        -OutFile "$TmpDir\$Archive"        -UseBasicParsing
    Invoke-WebRequest -Uri "$BaseUrl/$Archive.sha256" -OutFile "$TmpDir\$Archive.sha256" -UseBasicParsing

    $expectedLine = (Get-Content "$TmpDir\$Archive.sha256" -TotalCount 1).Trim()
    $expectedHash = $expectedLine.Split(' ')[0].ToUpper()
    $actualHash   = (Get-FileHash "$TmpDir\$Archive" -Algorithm SHA256).Hash.ToUpper()
    if ($expectedHash -ne $actualHash) {
        Write-Err "Checksum mismatch!`n  Expected: $expectedHash`n  Got:      $actualHash"
    }
    Write-Ok "Checksum verified"

    Expand-Archive -Path "$TmpDir\$Archive" -DestinationPath $TmpDir -Force
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    Move-Item -Path "$TmpDir\branch.exe" -Destination "$BinDir\branch.exe" -Force
    Write-Ok "Installed to $BinDir\branch.exe"

    $currentPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
    $pathParts = if ($currentPath) { $currentPath -split ';' | Where-Object { $_ -ne '' } } else { @() }
    if ($pathParts -notcontains $BinDir) {
        $newPath = if ($currentPath) { "$BinDir;$currentPath" } else { $BinDir }
        [System.Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        $env:PATH = "$BinDir;$env:PATH"
        Write-Ok "Added $BinDir to PATH"
    }

    if (-not $NoShellIntegration) {
        $profileDir = Split-Path $PROFILE
        if (-not (Test-Path $profileDir)) {
            New-Item -ItemType Directory -Path $profileDir -Force | Out-Null
        }
        if (-not (Test-Path $PROFILE)) {
            New-Item -ItemType File -Path $PROFILE -Force | Out-Null
        }
        $initLine = 'Invoke-Expression ((& branch --init) -join "`n")'
        $profileContent = Get-Content $PROFILE -ErrorAction SilentlyContinue
        if ($profileContent -match "branch --init") {
            Write-Ok "Shell integration already present in $PROFILE"
        } else {
            $raw = Get-Content -Path $PROFILE -Raw -ErrorAction SilentlyContinue
            if ($raw -and $raw[-1] -notin @("`r", "`n")) {
                Add-Content -Path $PROFILE -Value ""
            }
            Add-Content -Path $PROFILE -Value $initLine
            Write-Ok "Added shell integration to $PROFILE"
        }
    } else {
        Write-Host ""
        Write-Host "  Shell integration skipped. Add the following to your PowerShell profile manually:"
        Write-Host '    Invoke-Expression ((& branch --init) -join "`n")'
    }

    Write-Host ""
    Write-Ok "branch $Version installed successfully!"
    Write-Host ""
    if (-not $NoShellIntegration) {
        Write-Host "  Restart PowerShell or run:"
        Write-Host "    . `$PROFILE"
    } else {
        Write-Host "  Restart PowerShell to pick up the updated PATH."
    }

} finally {
    if ($TmpDir -and (Test-Path $TmpDir)) {
        Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
    }
}
